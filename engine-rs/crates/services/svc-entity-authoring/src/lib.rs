//! Validate and apply proposed generic **entity authoring** commands
//! (post-launchable-03, Den task #2485).
//!
//! # Lane
//!
//! `rust-service`. A UI/devtools authoring surface only *proposes*: it builds a
//! [`protocol_entity_authoring::EntityAuthoringCommand`] and hands it here.
//! Authority validates each proposal against the live [`EntityStore`] and either
//! applies it (returning an accepted event) or refuses it (returning a classified
//! rejection). Validation reuses `core-entity`'s atomic, fail-closed authority
//! operations — a rejected command mutates nothing — so an authoring surface can
//! never bypass lifecycle/transform/relation/movement rules or corrupt state, and
//! never holds a second copy of authority.
//!
//! This mirrors `svc-policy-view`'s validate/apply role, but for the fuller
//! operator/agent authoring surface (create/destroy/attach/relate/move) rather
//! than the narrow sandboxed policy set.

#![forbid(unsafe_code)]

mod activation;
pub use activation::{apply_rule_owned_capability_activation, project_capability_activation};

use std::collections::BTreeSet;

use core_assets::{AssetId, AssetReference, AssetVersionReq};
use core_entity::{
    Aabb, EntityLifecycleCommand, EntityLifecycleError, EntitySource, EntityStore, EntityTransform,
    MovementCommand, MovementError, Quat, RelationCommand, RelationError, TransformCommand,
    TransformError,
};
use core_math::Vec3;
use protocol_entity_authoring::{
    AuthoringCapability, AuthoringEventKind, AuthoringRejectionReason, AuthoringSource,
    AuthoringTransform, EntityAuthoringCommand, EntityAuthoringEvent, EntityAuthoringOutcome,
    EntityAuthoringRejection, EntityDefinition, EntityDefinitionCapability,
    EntityDefinitionDiagnostic, EntityDefinitionDiagnosticCode, EntityDefinitionSourceTrace,
    EntityDefinitionValidationOutcome,
};

// ── Border ⇄ core value mapping ───────────────────────────────────────────────

fn to_entity_transform(t: &AuthoringTransform) -> EntityTransform {
    EntityTransform {
        translation: Vec3::new(t.translation[0], t.translation[1], t.translation[2]),
        rotation: Quat {
            x: t.rotation[0],
            y: t.rotation[1],
            z: t.rotation[2],
            w: t.rotation[3],
        },
        scale: Vec3::new(t.scale[0], t.scale[1], t.scale[2]),
    }
}

fn to_entity_source(source: &AuthoringSource) -> Result<EntitySource, AuthoringRejectionReason> {
    Ok(match source {
        AuthoringSource::SceneBootstrap { node } => EntitySource::SceneBootstrap { node: *node },
        AuthoringSource::RuntimeCreated { by } => EntitySource::RuntimeCreated { by: *by },
        AuthoringSource::Imported { asset } => {
            let id = AssetId::parse(asset).map_err(|_| AuthoringRejectionReason::InvalidAsset)?;
            EntitySource::Imported {
                asset: AssetReference::new(id, AssetVersionReq::Any, None),
            }
        }
        AuthoringSource::DiagnosticTooling => EntitySource::DiagnosticTooling,
        AuthoringSource::PolicyProposed { by } => EntitySource::PolicyProposed { by: *by },
    })
}

// ── Error mapping ─────────────────────────────────────────────────────────────

fn map_lifecycle(err: EntityLifecycleError) -> AuthoringRejectionReason {
    match err {
        EntityLifecycleError::AlreadyExists { .. } => AuthoringRejectionReason::AlreadyExists,
        EntityLifecycleError::IdRetired { .. } => AuthoringRejectionReason::IdRetired,
        EntityLifecycleError::UnknownEntity { .. } => AuthoringRejectionReason::UnknownEntity,
        EntityLifecycleError::Tombstoned { .. } => AuthoringRejectionReason::Tombstoned,
        EntityLifecycleError::InvalidTransition { .. } => {
            AuthoringRejectionReason::InvalidTransition
        }
        EntityLifecycleError::LabelAlreadyPresent { .. } => {
            AuthoringRejectionReason::LabelAlreadyPresent
        }
        EntityLifecycleError::LabelAbsent { .. } => AuthoringRejectionReason::LabelAbsent,
    }
}

fn map_transform(err: TransformError) -> AuthoringRejectionReason {
    match err {
        TransformError::UnknownEntity { .. } => AuthoringRejectionReason::UnknownEntity,
        TransformError::Tombstoned { .. } => AuthoringRejectionReason::Tombstoned,
        TransformError::Disabled { .. } => AuthoringRejectionReason::InvalidTransition,
        TransformError::NotTransformEligible { .. } => {
            AuthoringRejectionReason::NotTransformEligible
        }
        TransformError::Immovable { .. } => AuthoringRejectionReason::Immovable,
        TransformError::NonFinite { .. } => AuthoringRejectionReason::NonFinite,
    }
}

fn map_movement(err: MovementError) -> AuthoringRejectionReason {
    match err {
        MovementError::UnknownEntity { .. } => AuthoringRejectionReason::UnknownEntity,
        MovementError::Tombstoned { .. } => AuthoringRejectionReason::Tombstoned,
        MovementError::Disabled { .. } => AuthoringRejectionReason::InvalidTransition,
        MovementError::NotSpatial { .. } => AuthoringRejectionReason::NotSpatial,
        MovementError::NoCollider { .. } => AuthoringRejectionReason::NoCollider,
        MovementError::Immovable { .. } => AuthoringRejectionReason::Immovable,
        MovementError::NonFinite { .. } => AuthoringRejectionReason::NonFinite,
    }
}

fn map_relation(err: RelationError) -> AuthoringRejectionReason {
    match err {
        RelationError::UnknownEntity { .. } => AuthoringRejectionReason::UnknownEntity,
        RelationError::Tombstoned { .. } => AuthoringRejectionReason::Tombstoned,
        RelationError::Cycle { .. } => AuthoringRejectionReason::RelationCycle,
        RelationError::NotTransformEligible { .. } => {
            AuthoringRejectionReason::EndpointNotTransformEligible
        }
        RelationError::SelfRelation { .. } => AuthoringRejectionReason::SelfRelation,
        RelationError::NoSuchRelation { .. } => AuthoringRejectionReason::NoSuchRelation,
        RelationError::ProjectionOnly { .. } => AuthoringRejectionReason::ProjectionOnly,
    }
}

// ── Outcome helpers ───────────────────────────────────────────────────────────

fn accepted(kind: AuthoringEventKind, entity: core_ids::EntityId) -> EntityAuthoringOutcome {
    EntityAuthoringOutcome::Accepted {
        event: EntityAuthoringEvent { kind, entity },
    }
}

fn rejected(
    reason: AuthoringRejectionReason,
    entity: core_ids::EntityId,
) -> EntityAuthoringOutcome {
    EntityAuthoringOutcome::Rejected {
        rejection: EntityAuthoringRejection { reason, entity },
    }
}

// ── ECRP Rule ownership / mutation rights ────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EcrpRuleOwner {
    EntityBootstrap,
    LifecycleRule,
    TransformRule,
    MovementRule,
    CollisionRule,
    ControllerRule,
    RenderProjectionRule,
    RelationRule,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EcrpCapabilityMutation {
    Lifecycle,
    AttachTransform,
    AttachBounds,
    AttachRenderProjection,
    AttachCollision,
    ActivateCollision,
    ActivateController,
    SetTransform,
    Move,
    Relation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EcrpRuleMutationDiagnostic {
    pub owner: EcrpRuleOwner,
    pub command_kind: &'static str,
    pub mutation: EcrpCapabilityMutation,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuleOwnedEntityAuthoringOutcome {
    Accepted {
        event: EntityAuthoringEvent,
    },
    Rejected {
        rejection: EntityAuthoringRejection,
    },
    Forbidden {
        diagnostic: EcrpRuleMutationDiagnostic,
    },
}

impl From<EntityAuthoringOutcome> for RuleOwnedEntityAuthoringOutcome {
    fn from(value: EntityAuthoringOutcome) -> Self {
        match value {
            EntityAuthoringOutcome::Accepted { event } => Self::Accepted { event },
            EntityAuthoringOutcome::Rejected { rejection } => Self::Rejected { rejection },
        }
    }
}

/// Validate a named ECRP Rule owner before applying an authoring command. This is
/// the authority-facing mutation-right gate for rule paths; UI/devtools proposal
/// paths still use `validate_and_apply` and should remain separate from Rules.
pub fn validate_and_apply_rule_owned(
    store: &mut EntityStore,
    owner: EcrpRuleOwner,
    command: &EntityAuthoringCommand,
) -> RuleOwnedEntityAuthoringOutcome {
    if let Err(diagnostic) = validate_rule_mutation_right(owner, command) {
        return RuleOwnedEntityAuthoringOutcome::Forbidden { diagnostic };
    }
    validate_and_apply(store, command).into()
}

pub fn validate_rule_mutation_right(
    owner: EcrpRuleOwner,
    command: &EntityAuthoringCommand,
) -> Result<(), EcrpRuleMutationDiagnostic> {
    let mutation = command_mutation(command);
    if rule_owner_allows(owner, mutation) {
        Ok(())
    } else {
        Err(EcrpRuleMutationDiagnostic {
            owner,
            command_kind: command.kind(),
            mutation,
            message: format!(
                "{owner:?} cannot apply {mutation:?}; ECRP capability mutation requires its owning Rule"
            ),
        })
    }
}

fn command_mutation(command: &EntityAuthoringCommand) -> EcrpCapabilityMutation {
    match command {
        EntityAuthoringCommand::Create { .. }
        | EntityAuthoringCommand::Destroy { .. }
        | EntityAuthoringCommand::Disable { .. }
        | EntityAuthoringCommand::Enable { .. }
        | EntityAuthoringCommand::AddLabel { .. }
        | EntityAuthoringCommand::RemoveLabel { .. } => EcrpCapabilityMutation::Lifecycle,
        EntityAuthoringCommand::AttachCapability { capability, .. } => match capability {
            AuthoringCapability::Transform { .. } => EcrpCapabilityMutation::AttachTransform,
            AuthoringCapability::Render { .. } => EcrpCapabilityMutation::AttachRenderProjection,
            AuthoringCapability::Collision { .. } => EcrpCapabilityMutation::AttachCollision,
            AuthoringCapability::Bounds { .. } => EcrpCapabilityMutation::AttachBounds,
        },
        EntityAuthoringCommand::SetTransform { .. } => EcrpCapabilityMutation::SetTransform,
        EntityAuthoringCommand::Move { .. } => EcrpCapabilityMutation::Move,
        EntityAuthoringCommand::AttachTransformParent { .. }
        | EntityAuthoringCommand::DetachTransformParent { .. }
        | EntityAuthoringCommand::SetContainment { .. }
        | EntityAuthoringCommand::ClearContainment { .. }
        | EntityAuthoringCommand::SetDerivedFrom { .. } => EcrpCapabilityMutation::Relation,
    }
}

pub(crate) fn rule_owner_allows(owner: EcrpRuleOwner, mutation: EcrpCapabilityMutation) -> bool {
    match owner {
        EcrpRuleOwner::EntityBootstrap => matches!(
            mutation,
            EcrpCapabilityMutation::Lifecycle
                | EcrpCapabilityMutation::AttachTransform
                | EcrpCapabilityMutation::AttachBounds
                | EcrpCapabilityMutation::AttachRenderProjection
                | EcrpCapabilityMutation::AttachCollision
        ),
        EcrpRuleOwner::LifecycleRule => matches!(mutation, EcrpCapabilityMutation::Lifecycle),
        EcrpRuleOwner::TransformRule => matches!(mutation, EcrpCapabilityMutation::SetTransform),
        EcrpRuleOwner::MovementRule => matches!(mutation, EcrpCapabilityMutation::Move),
        EcrpRuleOwner::CollisionRule => matches!(
            mutation,
            EcrpCapabilityMutation::AttachCollision
                | EcrpCapabilityMutation::AttachBounds
                | EcrpCapabilityMutation::ActivateCollision
        ),
        EcrpRuleOwner::ControllerRule => {
            matches!(mutation, EcrpCapabilityMutation::ActivateController)
        }
        EcrpRuleOwner::RenderProjectionRule => {
            matches!(mutation, EcrpCapabilityMutation::AttachRenderProjection)
        }
        EcrpRuleOwner::RelationRule => matches!(mutation, EcrpCapabilityMutation::Relation),
    }
}

// ── Stored EntityDefinition validation/bootstrap ─────────────────────────────

/// Authority-side bootstrap failure for a stored EntityDefinition. Invalid stored
/// data is reported separately from an otherwise-valid definition rejected by the
/// live runtime store (for example, an already allocated runtime entity id).
#[derive(Debug, Clone, PartialEq)]
pub enum EntityDefinitionBootstrapError {
    Invalid {
        diagnostics: Vec<EntityDefinitionDiagnostic>,
    },
    Rejected {
        rejection: EntityAuthoringRejection,
    },
}

/// Deterministic readout for one stored EntityDefinition bootstrap into runtime
/// entity/capability state.
#[derive(Debug, Clone, PartialEq)]
pub struct EntityDefinitionBootstrapRecord {
    pub stable_id: String,
    pub display_name: String,
    pub entity: core_ids::EntityId,
    pub source: EntityDefinitionSourceTrace,
    pub applied_capabilities: Vec<String>,
    pub entity_hash: core_entity::EntityHash,
    pub replay_unit_label: &'static str,
}

/// One stored EntityDefinition selected for ProjectBundle bootstrap into a
/// deterministic runtime entity id.
#[derive(Debug, Clone, PartialEq)]
pub struct ProjectBundleEntityDefinitionBootstrapEntry {
    pub entity: core_ids::EntityId,
    pub definition: EntityDefinition,
}

/// ProjectBundle-shaped batch bootstrap request. This remains a Rust authority
/// service shape for now; downstream TS/demo access should go through a public
/// RuntimeSession readout task rather than raw store handles.
#[derive(Debug, Clone, PartialEq)]
pub struct ProjectBundleEntityDefinitionBootstrapRequest {
    pub project_bundle: String,
    pub entries: Vec<ProjectBundleEntityDefinitionBootstrapEntry>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectBundleEntityDefinitionBootstrapDiagnosticCode {
    MissingProjectBundle,
    EmptyDefinitions,
    DefinitionInvalid,
    SourceProjectBundleMismatch,
    DuplicateDefinitionStableId,
    DuplicateRuntimeEntity,
    RuntimeEntityAlreadyExists,
    RuntimeEntityMissing,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProjectBundleEntityDefinitionBootstrapDiagnostic {
    pub code: ProjectBundleEntityDefinitionBootstrapDiagnosticCode,
    pub path: String,
    pub stable_id: Option<String>,
    pub entity: Option<core_ids::EntityId>,
    pub message: String,
    pub definition_diagnostics: Vec<EntityDefinitionDiagnostic>,
}

/// Authority-side batch bootstrap failure. `Invalid` is a preflight failure and
/// always leaves the live store untouched; `Rejected` means a staged authority
/// apply was unexpectedly refused and is also not committed to the live store.
#[derive(Debug, Clone, PartialEq)]
pub enum ProjectBundleEntityDefinitionBootstrapError {
    Invalid {
        diagnostics: Vec<ProjectBundleEntityDefinitionBootstrapDiagnostic>,
    },
    Rejected {
        stable_id: String,
        entity: core_ids::EntityId,
        rejection: EntityAuthoringRejection,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProjectBundleEntityDefinitionBootstrapRecord {
    pub project_bundle: String,
    pub records: Vec<EntityDefinitionBootstrapRecord>,
    pub entity_hash: core_entity::EntityHash,
    pub replay_unit_label: &'static str,
}

pub type ProjectBundleEntityDefinitionBootstrapResult = Result<
    ProjectBundleEntityDefinitionBootstrapRecord,
    ProjectBundleEntityDefinitionBootstrapError,
>;

/// Validate durable stored EntityDefinition data before it can seed runtime
/// authority. This is ProjectBundle/catalog input validation, not live mutation.
pub fn validate_entity_definition(
    definition: &EntityDefinition,
) -> EntityDefinitionValidationOutcome {
    let mut diagnostics = Vec::new();
    if definition.stable_id.trim().is_empty() {
        diagnostics.push(entity_definition_diag(
            EntityDefinitionDiagnosticCode::MissingStableId,
            "stable_id",
            "EntityDefinition stable_id is required",
        ));
    }
    if definition.display_name.trim().is_empty() {
        diagnostics.push(entity_definition_diag(
            EntityDefinitionDiagnosticCode::MissingDisplayName,
            "display_name",
            "EntityDefinition display_name is required",
        ));
    }
    if definition.source.project_bundle.trim().is_empty()
        || definition.source.relative_path.trim().is_empty()
    {
        diagnostics.push(entity_definition_diag(
            EntityDefinitionDiagnosticCode::MissingSourceTrace,
            "source",
            "EntityDefinition source.project_bundle and source.relative_path are required",
        ));
    }

    let mut seen_capabilities = BTreeSet::new();
    for (index, capability) in definition.capabilities.iter().enumerate() {
        let path = format!("capabilities[{index}]");
        let kind = capability.kind().to_string();
        if !matches!(capability, EntityDefinitionCapability::Unknown { .. })
            && !seen_capabilities.insert(kind.clone())
        {
            diagnostics.push(entity_definition_diag(
                EntityDefinitionDiagnosticCode::DuplicateCapability,
                format!("{path}.kind"),
                format!("duplicate capability declaration \"{kind}\""),
            ));
        }
        validate_entity_definition_capability(capability, &path, &mut diagnostics);
    }

    if diagnostics.is_empty() {
        EntityDefinitionValidationOutcome::Valid
    } else {
        EntityDefinitionValidationOutcome::Invalid { diagnostics }
    }
}

/// Validate and bootstrap one stored EntityDefinition into runtime authority.
/// The function validates first; invalid stored data leaves `store` untouched.
pub fn bootstrap_entity_definition(
    store: &mut EntityStore,
    entity: core_ids::EntityId,
    definition: &EntityDefinition,
) -> Result<EntityDefinitionBootstrapRecord, EntityDefinitionBootstrapError> {
    if let EntityDefinitionValidationOutcome::Invalid { diagnostics } =
        validate_entity_definition(definition)
    {
        return Err(EntityDefinitionBootstrapError::Invalid { diagnostics });
    }

    let mut staging = store.clone();
    let create = validate_and_apply_rule_owned(
        &mut staging,
        EcrpRuleOwner::EntityBootstrap,
        &EntityAuthoringCommand::Create {
            id: entity,
            source: AuthoringSource::RuntimeCreated { by: None },
            labels: definition.tags.clone(),
        },
    );
    match create {
        RuleOwnedEntityAuthoringOutcome::Accepted { .. } => {}
        RuleOwnedEntityAuthoringOutcome::Rejected { rejection } => {
            return Err(EntityDefinitionBootstrapError::Rejected { rejection });
        }
        RuleOwnedEntityAuthoringOutcome::Forbidden { diagnostic } => {
            panic!("EntityBootstrap rule unexpectedly rejected create: {diagnostic:?}");
        }
    }

    let mut applied_capabilities = Vec::with_capacity(definition.capabilities.len());
    for capability in &definition.capabilities {
        let Some(authoring_capability) = to_authoring_capability(capability) else {
            // Valid domain-owned stored capabilities are consumed by their
            // statically installed Rule adapter. They are not generic
            // EntityStore attachments and must never be treated as unknown.
            continue;
        };
        let outcome = validate_and_apply_rule_owned(
            &mut staging,
            EcrpRuleOwner::EntityBootstrap,
            &EntityAuthoringCommand::AttachCapability {
                id: entity,
                capability: authoring_capability,
            },
        );
        match outcome {
            RuleOwnedEntityAuthoringOutcome::Accepted { .. } => {
                applied_capabilities.push(capability.kind().to_string());
            }
            RuleOwnedEntityAuthoringOutcome::Rejected { rejection } => {
                return Err(EntityDefinitionBootstrapError::Rejected { rejection });
            }
            RuleOwnedEntityAuthoringOutcome::Forbidden { diagnostic } => {
                panic!(
                    "EntityBootstrap rule unexpectedly rejected capability attach: {diagnostic:?}"
                );
            }
        }
    }

    let entity_hash = staging.hash();
    *store = staging;
    Ok(EntityDefinitionBootstrapRecord {
        stable_id: definition.stable_id.clone(),
        display_name: definition.display_name.clone(),
        entity,
        source: definition.source.clone(),
        applied_capabilities,
        entity_hash,
        replay_unit_label: "entity_definition.bootstrap",
    })
}

/// Validate and bootstrap a ProjectBundle batch of stored EntityDefinitions into
/// runtime Entity/CapabilityState. The batch is atomic: all definitions and ids
/// are preflighted, then applied to a staging store. The live store is replaced
/// only after every entry succeeds.
pub fn bootstrap_project_bundle_entity_definitions(
    store: &mut EntityStore,
    request: &ProjectBundleEntityDefinitionBootstrapRequest,
) -> ProjectBundleEntityDefinitionBootstrapResult {
    let diagnostics = validate_project_bundle_bootstrap_request(
        store,
        request,
        ProjectBundleBootstrapTarget::Create,
    );
    if !diagnostics.is_empty() {
        return Err(ProjectBundleEntityDefinitionBootstrapError::Invalid { diagnostics });
    }

    let mut staging = store.clone();
    let mut records = Vec::with_capacity(request.entries.len());
    for (index, entry) in request.entries.iter().enumerate() {
        match bootstrap_entity_definition(&mut staging, entry.entity, &entry.definition) {
            Ok(record) => records.push(record),
            Err(EntityDefinitionBootstrapError::Invalid { diagnostics }) => {
                return Err(ProjectBundleEntityDefinitionBootstrapError::Invalid {
                    diagnostics: vec![ProjectBundleEntityDefinitionBootstrapDiagnostic {
                        code:
                            ProjectBundleEntityDefinitionBootstrapDiagnosticCode::DefinitionInvalid,
                        path: format!("entries[{index}].definition"),
                        stable_id: Some(entry.definition.stable_id.clone()),
                        entity: Some(entry.entity),
                        message: "EntityDefinition failed staged validation".into(),
                        definition_diagnostics: diagnostics,
                    }],
                });
            }
            Err(EntityDefinitionBootstrapError::Rejected { rejection }) => {
                return Err(ProjectBundleEntityDefinitionBootstrapError::Rejected {
                    stable_id: entry.definition.stable_id.clone(),
                    entity: entry.entity,
                    rejection,
                });
            }
        }
    }

    let entity_hash = staging.hash();
    *store = staging;
    Ok(ProjectBundleEntityDefinitionBootstrapRecord {
        project_bundle: request.project_bundle.clone(),
        records,
        entity_hash,
        replay_unit_label: "project_bundle.entity_definitions.bootstrap",
    })
}

/// Bind typed stored definitions to entities already created by canonical
/// scene/bootstrap admission. This is the canonical RuntimeSession seam: it
/// validates the same definition batch without allocating a second entity
/// graph or reapplying base capabilities.
pub fn bind_project_bundle_entity_definitions(
    store: &EntityStore,
    request: &ProjectBundleEntityDefinitionBootstrapRequest,
) -> ProjectBundleEntityDefinitionBootstrapResult {
    let diagnostics = validate_project_bundle_bootstrap_request(
        store,
        request,
        ProjectBundleBootstrapTarget::Existing,
    );
    if !diagnostics.is_empty() {
        return Err(ProjectBundleEntityDefinitionBootstrapError::Invalid { diagnostics });
    }
    let entity_hash = store.hash();
    let records = request
        .entries
        .iter()
        .map(|entry| EntityDefinitionBootstrapRecord {
            stable_id: entry.definition.stable_id.clone(),
            display_name: entry.definition.display_name.clone(),
            entity: entry.entity,
            source: entry.definition.source.clone(),
            applied_capabilities: entry
                .definition
                .capabilities
                .iter()
                .filter(|capability| {
                    matches!(
                        capability,
                        EntityDefinitionCapability::Transform { .. }
                            | EntityDefinitionCapability::Collision { .. }
                            | EntityDefinitionCapability::Bounds { .. }
                    )
                })
                .map(|capability| capability.kind().to_owned())
                .collect(),
            entity_hash,
            replay_unit_label: "entity_definition.canonical_binding",
        })
        .collect();
    Ok(ProjectBundleEntityDefinitionBootstrapRecord {
        project_bundle: request.project_bundle.clone(),
        records,
        entity_hash,
        replay_unit_label: "project_bundle.entity_definitions.canonical_binding",
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProjectBundleBootstrapTarget {
    Create,
    Existing,
}

fn validate_project_bundle_bootstrap_request(
    store: &EntityStore,
    request: &ProjectBundleEntityDefinitionBootstrapRequest,
    target: ProjectBundleBootstrapTarget,
) -> Vec<ProjectBundleEntityDefinitionBootstrapDiagnostic> {
    let mut diagnostics = Vec::new();
    if request.project_bundle.trim().is_empty() {
        diagnostics.push(project_bundle_bootstrap_diag(
            ProjectBundleEntityDefinitionBootstrapDiagnosticCode::MissingProjectBundle,
            "project_bundle",
            None,
            None,
            "ProjectBundle id is required",
            Vec::new(),
        ));
    }
    if request.entries.is_empty() {
        diagnostics.push(project_bundle_bootstrap_diag(
            ProjectBundleEntityDefinitionBootstrapDiagnosticCode::EmptyDefinitions,
            "entries",
            None,
            None,
            "at least one EntityDefinition entry is required",
            Vec::new(),
        ));
    }

    let mut seen_stable_ids = BTreeSet::new();
    let mut seen_entities = BTreeSet::new();
    for (index, entry) in request.entries.iter().enumerate() {
        let definition = &entry.definition;
        let entry_path = format!("entries[{index}]");
        if let EntityDefinitionValidationOutcome::Invalid {
            diagnostics: definition_diagnostics,
        } = validate_entity_definition(definition)
        {
            diagnostics.push(project_bundle_bootstrap_diag(
                ProjectBundleEntityDefinitionBootstrapDiagnosticCode::DefinitionInvalid,
                format!("{entry_path}.definition"),
                Some(definition.stable_id.clone()),
                Some(entry.entity),
                "EntityDefinition is invalid",
                definition_diagnostics,
            ));
        }
        if definition.source.project_bundle != request.project_bundle {
            diagnostics.push(project_bundle_bootstrap_diag(
                ProjectBundleEntityDefinitionBootstrapDiagnosticCode::SourceProjectBundleMismatch,
                format!("{entry_path}.definition.source.project_bundle"),
                Some(definition.stable_id.clone()),
                Some(entry.entity),
                "EntityDefinition source.project_bundle must match the ProjectBundle bootstrap request",
                Vec::new(),
            ));
        }
        if !definition.stable_id.trim().is_empty()
            && !seen_stable_ids.insert(definition.stable_id.clone())
        {
            diagnostics.push(project_bundle_bootstrap_diag(
                ProjectBundleEntityDefinitionBootstrapDiagnosticCode::DuplicateDefinitionStableId,
                format!("{entry_path}.definition.stable_id"),
                Some(definition.stable_id.clone()),
                Some(entry.entity),
                "EntityDefinition stable_id must be unique within a ProjectBundle bootstrap batch",
                Vec::new(),
            ));
        }
        if !seen_entities.insert(entry.entity) {
            diagnostics.push(project_bundle_bootstrap_diag(
                ProjectBundleEntityDefinitionBootstrapDiagnosticCode::DuplicateRuntimeEntity,
                format!("{entry_path}.entity"),
                Some(definition.stable_id.clone()),
                Some(entry.entity),
                "runtime entity id must be unique within a ProjectBundle bootstrap batch",
                Vec::new(),
            ));
        }
        if target == ProjectBundleBootstrapTarget::Create && store.contains(entry.entity) {
            diagnostics.push(project_bundle_bootstrap_diag(
                ProjectBundleEntityDefinitionBootstrapDiagnosticCode::RuntimeEntityAlreadyExists,
                format!("{entry_path}.entity"),
                Some(definition.stable_id.clone()),
                Some(entry.entity),
                "runtime entity id is already allocated in SessionState",
                Vec::new(),
            ));
        }
        if target == ProjectBundleBootstrapTarget::Existing && !store.contains(entry.entity) {
            diagnostics.push(project_bundle_bootstrap_diag(
                ProjectBundleEntityDefinitionBootstrapDiagnosticCode::RuntimeEntityMissing,
                format!("{entry_path}.entity"),
                Some(definition.stable_id.clone()),
                Some(entry.entity),
                "canonical runtime entity id is missing from SessionState",
                Vec::new(),
            ));
        }
    }
    diagnostics
}

fn project_bundle_bootstrap_diag(
    code: ProjectBundleEntityDefinitionBootstrapDiagnosticCode,
    path: impl Into<String>,
    stable_id: Option<String>,
    entity: Option<core_ids::EntityId>,
    message: impl Into<String>,
    definition_diagnostics: Vec<EntityDefinitionDiagnostic>,
) -> ProjectBundleEntityDefinitionBootstrapDiagnostic {
    ProjectBundleEntityDefinitionBootstrapDiagnostic {
        code,
        path: path.into(),
        stable_id,
        entity,
        message: message.into(),
        definition_diagnostics,
    }
}

fn validate_entity_definition_capability(
    capability: &EntityDefinitionCapability,
    path: &str,
    diagnostics: &mut Vec<EntityDefinitionDiagnostic>,
) {
    match capability {
        EntityDefinitionCapability::Transform { transform } => {
            if !authoring_transform_is_finite(transform) {
                diagnostics.push(entity_definition_diag(
                    EntityDefinitionDiagnosticCode::NonFiniteInitialValue,
                    path,
                    "transform initial value must be finite",
                ));
            }
            if transform.scale.contains(&0.0) {
                diagnostics.push(entity_definition_diag(
                    EntityDefinitionDiagnosticCode::InvalidInitialValue,
                    path,
                    "transform scale axes must be non-zero",
                ));
            }
        }
        EntityDefinitionCapability::Render { .. }
        | EntityDefinitionCapability::Collision { .. } => {}
        EntityDefinitionCapability::Bounds { min, max } => {
            if !min.iter().chain(max.iter()).all(|value| value.is_finite()) {
                diagnostics.push(entity_definition_diag(
                    EntityDefinitionDiagnosticCode::NonFiniteInitialValue,
                    path,
                    "bounds initial value must be finite",
                ));
            }
            if min.iter().zip(max.iter()).any(|(lo, hi)| lo > hi) {
                diagnostics.push(entity_definition_diag(
                    EntityDefinitionDiagnosticCode::InvalidInitialValue,
                    path,
                    "bounds min must be less than or equal to max on every axis",
                ));
            }
        }
        EntityDefinitionCapability::Controller { controller_id } => {
            validate_required_capability_id(controller_id, path, "controller id", diagnostics);
        }
        EntityDefinitionCapability::Health { current, max } => {
            if *max == 0 || current > max {
                diagnostics.push(entity_definition_diag(
                    EntityDefinitionDiagnosticCode::InvalidInitialValue,
                    path,
                    "health requires a non-zero max and current less than or equal to max",
                ));
            }
        }
        EntityDefinitionCapability::WeaponMount {
            weapon_id,
            damage,
            range_units,
            ammo: _,
            cooldown_ticks_after_fire: _,
        } => {
            validate_required_capability_id(weapon_id, path, "weapon id", diagnostics);
            if *damage == 0 || *range_units == 0 {
                diagnostics.push(entity_definition_diag(
                    EntityDefinitionDiagnosticCode::InvalidInitialValue,
                    path,
                    "weapon mount requires non-zero damage and range",
                ));
            }
        }
        EntityDefinitionCapability::RenderProjection {
            projection_id,
            appearance,
            ..
        } => {
            validate_required_capability_id(
                projection_id,
                path,
                "render projection id",
                diagnostics,
            );
            if let Some(appearance) = appearance {
                validate_required_capability_id(
                    &appearance.resource_id,
                    path,
                    "appearance resource id",
                    diagnostics,
                );
                if appearance
                    .initial_clip_id
                    .as_ref()
                    .is_some_and(|clip| clip.trim().is_empty())
                {
                    diagnostics.push(entity_definition_diag(
                        EntityDefinitionDiagnosticCode::InvalidInitialValue,
                        path,
                        "appearance initial clip id must be non-empty when present",
                    ));
                }
                if !appearance
                    .model_scale
                    .iter()
                    .all(|value| value.is_finite() && *value > 0.0)
                {
                    diagnostics.push(entity_definition_diag(
                        EntityDefinitionDiagnosticCode::InvalidInitialValue,
                        path,
                        "appearance model scale must contain finite positive values",
                    ));
                }
            }
        }
        EntityDefinitionCapability::PolicyBinding {
            binding_id,
            policy_id,
            view_kind,
            view_version,
            allowed_intents,
            runtime_moment,
        } => {
            for (label, value) in [
                ("binding id", binding_id),
                ("policy id", policy_id),
                ("view kind", view_kind),
                ("view version", view_version),
                ("runtime moment", runtime_moment),
            ] {
                validate_required_capability_id(value, path, label, diagnostics);
            }
            if allowed_intents.is_empty()
                || allowed_intents
                    .iter()
                    .any(|intent| intent.trim().is_empty())
            {
                diagnostics.push(entity_definition_diag(
                    EntityDefinitionDiagnosticCode::InvalidInitialValue,
                    path,
                    "policy binding requires non-empty allowed intent ids",
                ));
            }
        }
        EntityDefinitionCapability::SpawnMarker { marker_id } => {
            validate_required_capability_id(marker_id, path, "spawn marker id", diagnostics);
        }
        EntityDefinitionCapability::Faction { faction_id } => {
            validate_required_capability_id(faction_id, path, "faction id", diagnostics);
        }
        EntityDefinitionCapability::Unknown { capability_kind } => {
            diagnostics.push(entity_definition_diag(
                EntityDefinitionDiagnosticCode::UnknownCapability,
                format!("{path}.kind"),
                format!("unknown capability declaration \"{capability_kind}\""),
            ));
        }
    }
}

fn validate_required_capability_id(
    value: &str,
    path: &str,
    label: &str,
    diagnostics: &mut Vec<EntityDefinitionDiagnostic>,
) {
    if value.trim().is_empty() {
        diagnostics.push(entity_definition_diag(
            EntityDefinitionDiagnosticCode::InvalidInitialValue,
            path,
            format!("{label} is required"),
        ));
    }
}

fn to_authoring_capability(capability: &EntityDefinitionCapability) -> Option<AuthoringCapability> {
    match capability {
        EntityDefinitionCapability::Transform { transform } => {
            Some(AuthoringCapability::Transform {
                transform: *transform,
            })
        }
        EntityDefinitionCapability::Render { visible } => {
            Some(AuthoringCapability::Render { visible: *visible })
        }
        EntityDefinitionCapability::Collision { static_collider } => {
            Some(AuthoringCapability::Collision {
                static_collider: *static_collider,
            })
        }
        EntityDefinitionCapability::Bounds { min, max } => Some(AuthoringCapability::Bounds {
            min: *min,
            max: *max,
        }),
        EntityDefinitionCapability::Controller { .. }
        | EntityDefinitionCapability::Health { .. }
        | EntityDefinitionCapability::WeaponMount { .. }
        | EntityDefinitionCapability::RenderProjection { .. }
        | EntityDefinitionCapability::PolicyBinding { .. }
        | EntityDefinitionCapability::SpawnMarker { .. }
        | EntityDefinitionCapability::Faction { .. } => None,
        EntityDefinitionCapability::Unknown { .. } => None,
    }
}

fn authoring_transform_is_finite(transform: &AuthoringTransform) -> bool {
    transform
        .translation
        .iter()
        .chain(transform.rotation.iter())
        .chain(transform.scale.iter())
        .all(|value| value.is_finite())
}

fn entity_definition_diag(
    code: EntityDefinitionDiagnosticCode,
    path: impl Into<String>,
    message: impl Into<String>,
) -> EntityDefinitionDiagnostic {
    EntityDefinitionDiagnostic {
        code,
        path: path.into(),
        message: message.into(),
    }
}

// ── Validate + apply ──────────────────────────────────────────────────────────

/// Validate a single proposed authoring command against `store` and, if accepted,
/// apply it. On rejection the store is left untouched (the underlying authority
/// operations are atomic and fail-closed). Returns the classified outcome either
/// way.
pub fn validate_and_apply(
    store: &mut EntityStore,
    command: &EntityAuthoringCommand,
) -> EntityAuthoringOutcome {
    use AuthoringEventKind as E;
    match command {
        EntityAuthoringCommand::Create { id, source, labels } => {
            let source = match to_entity_source(source) {
                Ok(s) => s,
                Err(reason) => return rejected(reason, *id),
            };
            match store.apply(EntityLifecycleCommand::Create {
                id: *id,
                source,
                labels: labels.clone(),
            }) {
                Ok(_) => accepted(E::Created, *id),
                Err(e) => rejected(map_lifecycle(e), *id),
            }
        }
        EntityAuthoringCommand::Destroy { id } => lifecycle(
            store,
            EntityLifecycleCommand::Destroy { id: *id },
            E::Destroyed,
            *id,
        ),
        EntityAuthoringCommand::Disable { id } => lifecycle(
            store,
            EntityLifecycleCommand::Disable { id: *id },
            E::Disabled,
            *id,
        ),
        EntityAuthoringCommand::Enable { id } => lifecycle(
            store,
            EntityLifecycleCommand::Enable { id: *id },
            E::Enabled,
            *id,
        ),
        EntityAuthoringCommand::AddLabel { id, tag } => lifecycle(
            store,
            EntityLifecycleCommand::AddLabel { id: *id, tag: *tag },
            E::LabelAdded,
            *id,
        ),
        EntityAuthoringCommand::RemoveLabel { id, tag } => lifecycle(
            store,
            EntityLifecycleCommand::RemoveLabel { id: *id, tag: *tag },
            E::LabelRemoved,
            *id,
        ),
        EntityAuthoringCommand::AttachCapability { id, capability } => {
            attach_capability(store, *id, capability)
        }
        EntityAuthoringCommand::SetTransform { id, transform } => {
            let cmd = TransformCommand::Set {
                id: *id,
                transform: to_entity_transform(transform),
            };
            match store.apply_transform(cmd) {
                Ok(_) => accepted(E::TransformSet, *id),
                Err(e) => rejected(map_transform(e), *id),
            }
        }
        EntityAuthoringCommand::Move { id, delta } => {
            let cmd = MovementCommand {
                id: *id,
                delta: Vec3::new(delta[0], delta[1], delta[2]),
            };
            match store.apply_movement(cmd) {
                Ok(_) => accepted(E::Moved, *id),
                Err(e) => rejected(map_movement(e), *id),
            }
        }
        EntityAuthoringCommand::AttachTransformParent { child, parent } => relation(
            store,
            RelationCommand::AttachTransformParent {
                child: *child,
                parent: *parent,
            },
            E::RelationSet,
            *child,
        ),
        EntityAuthoringCommand::DetachTransformParent { child } => relation(
            store,
            RelationCommand::DetachTransformParent { child: *child },
            E::RelationCleared,
            *child,
        ),
        EntityAuthoringCommand::SetContainment { member, container } => relation(
            store,
            RelationCommand::SetContainment {
                member: *member,
                container: *container,
            },
            E::RelationSet,
            *member,
        ),
        EntityAuthoringCommand::ClearContainment { member } => relation(
            store,
            RelationCommand::ClearContainment { member: *member },
            E::RelationCleared,
            *member,
        ),
        EntityAuthoringCommand::SetDerivedFrom { derived, origin } => relation(
            store,
            RelationCommand::SetDerivedFrom {
                derived: *derived,
                origin: *origin,
            },
            E::RelationSet,
            *derived,
        ),
    }
}

fn lifecycle(
    store: &mut EntityStore,
    cmd: EntityLifecycleCommand,
    on_ok: AuthoringEventKind,
    id: core_ids::EntityId,
) -> EntityAuthoringOutcome {
    match store.apply(cmd) {
        Ok(_) => accepted(on_ok, id),
        Err(e) => rejected(map_lifecycle(e), id),
    }
}

fn relation(
    store: &mut EntityStore,
    cmd: RelationCommand,
    on_ok: AuthoringEventKind,
    id: core_ids::EntityId,
) -> EntityAuthoringOutcome {
    match store.apply_relation(cmd) {
        Ok(()) => accepted(on_ok, id),
        Err(e) => rejected(map_relation(e), id),
    }
}

/// Capability attach is a no-op on a dead/unknown entity; classify those rather
/// than silently dropping the proposal.
fn attach_capability(
    store: &mut EntityStore,
    id: core_ids::EntityId,
    capability: &AuthoringCapability,
) -> EntityAuthoringOutcome {
    match store.lifecycle(id) {
        None => return rejected(AuthoringRejectionReason::UnknownEntity, id),
        Some(core_entity::EntityLifecycle::Tombstoned) => {
            return rejected(AuthoringRejectionReason::Tombstoned, id)
        }
        Some(_) => {}
    }
    let attached = match capability {
        AuthoringCapability::Transform { transform } => {
            store.attach_transform(id, to_entity_transform(transform))
        }
        AuthoringCapability::Render { visible } => store.attach_render_projection(id, *visible),
        AuthoringCapability::Collision { static_collider } => {
            store.attach_collision(id, *static_collider)
        }
        AuthoringCapability::Bounds { min, max } => store.attach_bounds(
            id,
            Aabb::new(
                Vec3::new(min[0], min[1], min[2]),
                Vec3::new(max[0], max[1], max[2]),
            ),
        ),
    };
    if attached {
        accepted(AuthoringEventKind::CapabilityAttached, id)
    } else {
        // Lifecycle check above already excluded unknown/tombstoned; a false here
        // means disabled (attach is alive-only).
        rejected(AuthoringRejectionReason::EntityNotAlive, id)
    }
}

// ── Eligibility preview (capability discipline, no mutation) ───────────────────

/// Whether a transform/movement-style command would be accepted for `id`, without
/// applying anything — for a UI to disable an ineligible control and explain why.
pub fn transform_eligible(
    store: &EntityStore,
    id: core_ids::EntityId,
) -> Result<(), AuthoringRejectionReason> {
    store.transform_eligible(id).map_err(map_transform)
}

/// Whether a kinematic move would be accepted for `id`, without applying it.
pub fn movement_eligible(
    store: &EntityStore,
    id: core_ids::EntityId,
) -> Result<(), AuthoringRejectionReason> {
    store.movement_eligible(id).map_err(map_movement)
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_ids::{EntityId, TagId};
    use protocol_entity_authoring::{EntityAuthoringOutcome as O, EntityDefinitionMetadataEntry};

    fn ident() -> AuthoringTransform {
        AuthoringTransform {
            translation: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            scale: [1.0, 1.0, 1.0],
        }
    }

    fn create(store: &mut EntityStore, id: u64) -> EntityAuthoringOutcome {
        validate_and_apply(
            store,
            &EntityAuthoringCommand::Create {
                id: EntityId::new(id),
                source: AuthoringSource::RuntimeCreated { by: None },
                labels: vec![],
            },
        )
    }

    fn minimal_entity_definition() -> EntityDefinition {
        EntityDefinition {
            stable_id: "actor/demo-player".into(),
            display_name: "Demo Player".into(),
            source: EntityDefinitionSourceTrace {
                project_bundle: "asha-demo".into(),
                relative_path: "catalogs/actors/demo-player.entity.json".into(),
            },
            tags: vec![TagId::new(11)],
            metadata: vec![EntityDefinitionMetadataEntry {
                key: "readout".into(),
                value: "skeleton".into(),
            }],
            capabilities: vec![EntityDefinitionCapability::Transform { transform: ident() }],
        }
    }

    fn target_entity_definition() -> EntityDefinition {
        EntityDefinition {
            stable_id: "actor/demo-target".into(),
            display_name: "Demo Target".into(),
            source: EntityDefinitionSourceTrace {
                project_bundle: "asha-demo".into(),
                relative_path: "catalogs/actors/demo-target.entity.json".into(),
            },
            tags: vec![TagId::new(12)],
            metadata: vec![EntityDefinitionMetadataEntry {
                key: "readout".into(),
                value: "target".into(),
            }],
            capabilities: vec![
                EntityDefinitionCapability::Transform {
                    transform: AuthoringTransform {
                        translation: [1.0, 0.0, -2.0],
                        ..ident()
                    },
                },
                EntityDefinitionCapability::Render { visible: true },
                EntityDefinitionCapability::Collision {
                    static_collider: true,
                },
                EntityDefinitionCapability::Bounds {
                    min: [-0.25, 0.0, -0.25],
                    max: [0.25, 1.0, 0.25],
                },
            ],
        }
    }

    fn semantic_entity_definition() -> EntityDefinition {
        let mut definition = minimal_entity_definition();
        definition.capabilities.extend([
            EntityDefinitionCapability::Controller {
                controller_id: "player_input".into(),
            },
            EntityDefinitionCapability::Health {
                current: 100,
                max: 100,
            },
            EntityDefinitionCapability::WeaponMount {
                weapon_id: "weapon/demo".into(),
                damage: 10,
                range_units: 12,
                ammo: 3,
                cooldown_ticks_after_fire: 2,
            },
            EntityDefinitionCapability::RenderProjection {
                projection_id: "first_person_camera".into(),
                visible: true,
                appearance: None,
            },
            EntityDefinitionCapability::PolicyBinding {
                binding_id: "player:policy".into(),
                policy_id: "policy/player".into(),
                view_kind: "runtime_session.fps.policy_view.v0".into(),
                view_version: "v0".into(),
                allowed_intents: vec!["runtime.intent.primary_fire.v0".into()],
                runtime_moment: "player_input".into(),
            },
            EntityDefinitionCapability::SpawnMarker {
                marker_id: "spawn.player".into(),
            },
            EntityDefinitionCapability::Faction {
                faction_id: "player".into(),
            },
        ]);
        definition
    }

    fn project_bundle_request(
        entries: Vec<(u64, EntityDefinition)>,
    ) -> ProjectBundleEntityDefinitionBootstrapRequest {
        ProjectBundleEntityDefinitionBootstrapRequest {
            project_bundle: "asha-demo".into(),
            entries: entries
                .into_iter()
                .map(
                    |(entity, definition)| ProjectBundleEntityDefinitionBootstrapEntry {
                        entity: EntityId::new(entity),
                        definition,
                    },
                )
                .collect(),
        }
    }

    #[test]
    fn create_then_attach_then_transform_is_accepted() {
        let mut store = EntityStore::new();
        assert!(matches!(create(&mut store, 1), O::Accepted { .. }));
        assert!(matches!(
            validate_and_apply(
                &mut store,
                &EntityAuthoringCommand::AttachCapability {
                    id: EntityId::new(1),
                    capability: AuthoringCapability::Transform { transform: ident() },
                }
            ),
            O::Accepted { .. }
        ));
        let out = validate_and_apply(
            &mut store,
            &EntityAuthoringCommand::SetTransform {
                id: EntityId::new(1),
                transform: AuthoringTransform {
                    translation: [3.0, 0.0, 0.0],
                    ..ident()
                },
            },
        );
        assert!(matches!(out, O::Accepted { .. }));
    }

    #[test]
    fn transform_on_non_spatial_entity_is_classified_not_eligible() {
        let mut store = EntityStore::new();
        create(&mut store, 1); // no transform capability attached
        let out = validate_and_apply(
            &mut store,
            &EntityAuthoringCommand::SetTransform {
                id: EntityId::new(1),
                transform: ident(),
            },
        );
        assert_eq!(
            out,
            rejected(
                AuthoringRejectionReason::NotTransformEligible,
                EntityId::new(1)
            )
        );
    }

    #[test]
    fn rejected_command_mutates_nothing() {
        let mut store = EntityStore::new();
        create(&mut store, 1);
        let hash_before = store.hash();
        // SetTransform on a non-spatial entity is rejected → no mutation.
        let out = validate_and_apply(
            &mut store,
            &EntityAuthoringCommand::SetTransform {
                id: EntityId::new(1),
                transform: ident(),
            },
        );
        assert!(matches!(out, O::Rejected { .. }));
        assert_eq!(
            store.hash(),
            hash_before,
            "a rejected command must not mutate authority"
        );
    }

    #[test]
    fn duplicate_create_is_classified_already_exists() {
        let mut store = EntityStore::new();
        create(&mut store, 1);
        let out = create(&mut store, 1);
        assert_eq!(
            out,
            rejected(AuthoringRejectionReason::AlreadyExists, EntityId::new(1))
        );
    }

    #[test]
    fn containment_and_source_relations_are_accepted_and_distinct() {
        let mut store = EntityStore::new();
        create(&mut store, 1);
        create(&mut store, 2);
        let contain = validate_and_apply(
            &mut store,
            &EntityAuthoringCommand::SetContainment {
                member: EntityId::new(1),
                container: EntityId::new(2),
            },
        );
        assert!(matches!(contain, O::Accepted { .. }));
        assert_eq!(
            store.containment(EntityId::new(1)).map(|c| c.container),
            Some(EntityId::new(2))
        );
        let derive = validate_and_apply(
            &mut store,
            &EntityAuthoringCommand::SetDerivedFrom {
                derived: EntityId::new(1),
                origin: EntityId::new(2),
            },
        );
        assert!(matches!(derive, O::Accepted { .. }));
        // Distinct relation taxonomy: containment is not source ancestry.
        assert_eq!(store.derived_from(EntityId::new(1)), Some(EntityId::new(2)));
    }

    #[test]
    fn self_containment_is_classified_self_relation() {
        let mut store = EntityStore::new();
        create(&mut store, 1);
        let out = validate_and_apply(
            &mut store,
            &EntityAuthoringCommand::SetContainment {
                member: EntityId::new(1),
                container: EntityId::new(1),
            },
        );
        assert_eq!(
            out,
            rejected(AuthoringRejectionReason::SelfRelation, EntityId::new(1))
        );
    }

    #[test]
    fn add_label_round_trips_through_authority() {
        let mut store = EntityStore::new();
        create(&mut store, 1);
        let out = validate_and_apply(
            &mut store,
            &EntityAuthoringCommand::AddLabel {
                id: EntityId::new(1),
                tag: TagId::new(7),
            },
        );
        assert!(matches!(out, O::Accepted { .. }));
        assert!(store
            .core(EntityId::new(1))
            .unwrap()
            .has_label(TagId::new(7)));
    }

    #[test]
    fn eligibility_preview_does_not_mutate() {
        let mut store = EntityStore::new();
        create(&mut store, 1);
        let before = store.hash();
        assert_eq!(
            transform_eligible(&store, EntityId::new(1)),
            Err(AuthoringRejectionReason::NotTransformEligible)
        );
        assert_eq!(store.hash(), before);
    }

    #[test]
    fn rule_owned_entity_bootstrap_can_create_and_attach_capability_state() {
        let mut store = EntityStore::new();

        let created = validate_and_apply_rule_owned(
            &mut store,
            EcrpRuleOwner::EntityBootstrap,
            &EntityAuthoringCommand::Create {
                id: EntityId::new(1),
                source: AuthoringSource::RuntimeCreated { by: None },
                labels: vec![],
            },
        );
        let attached = validate_and_apply_rule_owned(
            &mut store,
            EcrpRuleOwner::EntityBootstrap,
            &EntityAuthoringCommand::AttachCapability {
                id: EntityId::new(1),
                capability: AuthoringCapability::Transform { transform: ident() },
            },
        );

        assert!(matches!(
            created,
            RuleOwnedEntityAuthoringOutcome::Accepted { .. }
        ));
        assert!(matches!(
            attached,
            RuleOwnedEntityAuthoringOutcome::Accepted { .. }
        ));
        assert!(store.transform(EntityId::new(1)).is_some());
    }

    #[test]
    fn rule_owned_mutation_rejects_forbidden_cross_rule_capability_write() {
        let mut store = EntityStore::new();
        create(&mut store, 1);
        let before = store.hash();

        let forbidden = validate_and_apply_rule_owned(
            &mut store,
            EcrpRuleOwner::MovementRule,
            &EntityAuthoringCommand::AttachCapability {
                id: EntityId::new(1),
                capability: AuthoringCapability::Collision {
                    static_collider: false,
                },
            },
        );

        let RuleOwnedEntityAuthoringOutcome::Forbidden { diagnostic } = forbidden else {
            panic!("expected forbidden MovementRule collision attach");
        };
        assert_eq!(diagnostic.owner, EcrpRuleOwner::MovementRule);
        assert_eq!(diagnostic.mutation, EcrpCapabilityMutation::AttachCollision);
        assert_eq!(diagnostic.command_kind, "attachCapability");
        assert_eq!(store.hash(), before);
        assert!(store.collision(EntityId::new(1)).is_none());
    }

    #[test]
    fn rule_owned_movement_rule_can_apply_movement_only_after_capability_setup() {
        let mut store = EntityStore::new();
        create(&mut store, 1);
        validate_and_apply_rule_owned(
            &mut store,
            EcrpRuleOwner::EntityBootstrap,
            &EntityAuthoringCommand::AttachCapability {
                id: EntityId::new(1),
                capability: AuthoringCapability::Transform { transform: ident() },
            },
        );
        validate_and_apply_rule_owned(
            &mut store,
            EcrpRuleOwner::EntityBootstrap,
            &EntityAuthoringCommand::AttachCapability {
                id: EntityId::new(1),
                capability: AuthoringCapability::Collision {
                    static_collider: false,
                },
            },
        );

        let moved = validate_and_apply_rule_owned(
            &mut store,
            EcrpRuleOwner::MovementRule,
            &EntityAuthoringCommand::Move {
                id: EntityId::new(1),
                delta: [1.0, 0.0, 0.0],
            },
        );
        let wrong_owner = validate_and_apply_rule_owned(
            &mut store,
            EcrpRuleOwner::RenderProjectionRule,
            &EntityAuthoringCommand::Move {
                id: EntityId::new(1),
                delta: [1.0, 0.0, 0.0],
            },
        );

        assert!(matches!(
            moved,
            RuleOwnedEntityAuthoringOutcome::Accepted { .. }
        ));
        assert_eq!(
            store
                .transform(EntityId::new(1))
                .expect("transform after move")
                .transform
                .translation,
            Vec3::new(1.0, 0.0, 0.0)
        );
        assert!(matches!(
            wrong_owner,
            RuleOwnedEntityAuthoringOutcome::Forbidden { .. }
        ));
    }

    #[test]
    fn entity_definition_bootstrap_seeds_runtime_capability_state() {
        let mut store = EntityStore::new();
        let definition = minimal_entity_definition();

        let record =
            bootstrap_entity_definition(&mut store, EntityId::new(77), &definition).unwrap();

        assert_eq!(record.stable_id, "actor/demo-player");
        assert_eq!(record.display_name, "Demo Player");
        assert_eq!(record.entity, EntityId::new(77));
        assert_eq!(record.replay_unit_label, "entity_definition.bootstrap");
        assert_eq!(record.source.project_bundle, "asha-demo");
        assert_eq!(record.applied_capabilities, vec!["transform".to_string()]);
        assert_eq!(record.entity_hash, store.hash());

        let core = store
            .core(EntityId::new(77))
            .expect("runtime entity exists");
        assert_eq!(
            core.source,
            EntitySource::RuntimeCreated { by: None },
            "stored source trace is recorded on the bootstrap record until core source provenance grows a stored-definition variant"
        );
        assert!(core.has_label(TagId::new(11)));
        assert_eq!(
            store.transform(EntityId::new(77)).unwrap().transform,
            to_entity_transform(&ident())
        );
    }

    #[test]
    fn semantic_capabilities_are_valid_without_becoming_generic_store_attachments() {
        let mut store = EntityStore::new();
        let definition = semantic_entity_definition();

        let record = bootstrap_entity_definition(&mut store, EntityId::new(80), &definition)
            .expect("valid domain-owned capabilities must not panic");

        assert_eq!(record.applied_capabilities, vec!["transform".to_string()]);
        assert_eq!(store.alive_count(), 1);
        assert!(store.transform(EntityId::new(80)).is_some());
        assert_eq!(record.entity_hash, store.hash());
    }

    #[test]
    fn single_definition_bootstrap_rejection_never_partially_publishes() {
        let mut store = EntityStore::new();
        create(&mut store, 81);
        let before = store.hash();

        let result = bootstrap_entity_definition(
            &mut store,
            EntityId::new(81),
            &semantic_entity_definition(),
        );

        assert!(matches!(
            result,
            Err(EntityDefinitionBootstrapError::Rejected { .. })
        ));
        assert_eq!(store.hash(), before);
        assert_eq!(store.alive_count(), 1);
    }

    #[test]
    fn project_bundle_bootstrap_seeds_multiple_entity_definitions_atomically() {
        let mut store = EntityStore::new();
        let request = project_bundle_request(vec![
            (77, minimal_entity_definition()),
            (78, target_entity_definition()),
        ]);

        let record = bootstrap_project_bundle_entity_definitions(&mut store, &request).unwrap();

        assert_eq!(record.project_bundle, "asha-demo");
        assert_eq!(
            record.replay_unit_label,
            "project_bundle.entity_definitions.bootstrap"
        );
        assert_eq!(record.records.len(), 2);
        assert_eq!(record.records[0].stable_id, "actor/demo-player");
        assert_eq!(record.records[0].entity, EntityId::new(77));
        assert_eq!(record.records[1].stable_id, "actor/demo-target");
        assert_eq!(record.records[1].entity, EntityId::new(78));
        assert_eq!(record.records[1].source.project_bundle, "asha-demo");
        assert_eq!(
            record.records[1].applied_capabilities,
            vec![
                "transform".to_string(),
                "render".to_string(),
                "collision".to_string(),
                "bounds".to_string()
            ]
        );
        assert_eq!(record.entity_hash, store.hash());
        assert_eq!(store.alive_count(), 2);
        assert!(store.transform(EntityId::new(77)).is_some());
        assert!(store.transform(EntityId::new(78)).is_some());
        assert!(store.render_projection(EntityId::new(78)).is_some());
        assert!(store.collision(EntityId::new(78)).is_some());
        assert!(store.bounds(EntityId::new(78)).is_some());
    }

    #[test]
    fn project_bundle_bootstrap_accepts_domain_owned_semantic_capabilities() {
        let mut store = EntityStore::new();
        let request = project_bundle_request(vec![
            (80, semantic_entity_definition()),
            (82, target_entity_definition()),
        ]);

        let record = bootstrap_project_bundle_entity_definitions(&mut store, &request)
            .expect("valid semantic capabilities must survive batch bootstrap");

        assert_eq!(record.records[0].applied_capabilities, vec!["transform"]);
        assert_eq!(record.records[1].applied_capabilities.len(), 4);
        assert_eq!(store.alive_count(), 2);
        assert_eq!(record.entity_hash, store.hash());
    }

    #[test]
    fn project_bundle_bootstrap_preflight_rejects_invalid_batch_without_mutation() {
        let mut store = EntityStore::new();
        create(&mut store, 7);
        let before = store.hash();
        let mut duplicate_definition = target_entity_definition();
        duplicate_definition.stable_id = "actor/demo-player".into();
        let mut wrong_bundle_definition = target_entity_definition();
        wrong_bundle_definition.stable_id = "actor/wrong-bundle".into();
        wrong_bundle_definition.source.project_bundle = "other-demo".into();
        let mut invalid_definition = target_entity_definition();
        invalid_definition.stable_id = "actor/invalid".into();
        invalid_definition
            .capabilities
            .push(EntityDefinitionCapability::Unknown {
                capability_kind: "health".into(),
            });
        let request = project_bundle_request(vec![
            (7, minimal_entity_definition()),
            (8, duplicate_definition),
            (8, wrong_bundle_definition),
            (9, invalid_definition),
        ]);

        let result = bootstrap_project_bundle_entity_definitions(&mut store, &request);

        let Err(ProjectBundleEntityDefinitionBootstrapError::Invalid { diagnostics }) = result
        else {
            panic!("expected invalid ProjectBundle bootstrap batch");
        };
        let codes: Vec<_> = diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code)
            .collect();
        assert!(codes.contains(
            &ProjectBundleEntityDefinitionBootstrapDiagnosticCode::RuntimeEntityAlreadyExists
        ));
        assert!(codes.contains(
            &ProjectBundleEntityDefinitionBootstrapDiagnosticCode::DuplicateDefinitionStableId
        ));
        assert!(codes.contains(
            &ProjectBundleEntityDefinitionBootstrapDiagnosticCode::DuplicateRuntimeEntity
        ));
        assert!(codes.contains(
            &ProjectBundleEntityDefinitionBootstrapDiagnosticCode::SourceProjectBundleMismatch
        ));
        assert!(diagnostics.iter().any(|diagnostic| diagnostic
            .definition_diagnostics
            .iter()
            .any(|nested| nested.code == EntityDefinitionDiagnosticCode::UnknownCapability)));
        assert_eq!(store.hash(), before);
        assert!(store.core(EntityId::new(8)).is_none());
        assert!(store.core(EntityId::new(9)).is_none());
    }

    #[test]
    fn invalid_entity_definition_rejects_unknown_duplicate_and_invalid_capability() {
        let mut definition = minimal_entity_definition();
        definition.capabilities = vec![
            EntityDefinitionCapability::Transform {
                transform: AuthoringTransform {
                    scale: [0.0, 1.0, 1.0],
                    ..ident()
                },
            },
            EntityDefinitionCapability::Transform {
                transform: AuthoringTransform {
                    translation: [f32::NAN, 0.0, 0.0],
                    ..ident()
                },
            },
            EntityDefinitionCapability::Unknown {
                capability_kind: "health".into(),
            },
        ];

        let outcome = validate_entity_definition(&definition);
        let EntityDefinitionValidationOutcome::Invalid { diagnostics } = outcome else {
            panic!("expected invalid EntityDefinition");
        };
        let codes: Vec<_> = diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code)
            .collect();
        assert!(codes.contains(&EntityDefinitionDiagnosticCode::UnknownCapability));
        assert!(codes.contains(&EntityDefinitionDiagnosticCode::DuplicateCapability));
        assert!(codes.contains(&EntityDefinitionDiagnosticCode::NonFiniteInitialValue));
        assert!(codes.contains(&EntityDefinitionDiagnosticCode::InvalidInitialValue));
    }

    #[test]
    fn invalid_entity_definition_bootstrap_mutates_nothing() {
        let mut store = EntityStore::new();
        let mut definition = minimal_entity_definition();
        definition.stable_id.clear();
        definition
            .capabilities
            .push(EntityDefinitionCapability::Unknown {
                capability_kind: "combat".into(),
            });
        let before = store.hash();

        let result = bootstrap_entity_definition(&mut store, EntityId::new(99), &definition);

        assert!(matches!(
            result,
            Err(EntityDefinitionBootstrapError::Invalid { .. })
        ));
        assert_eq!(store.hash(), before);
        assert!(store.core(EntityId::new(99)).is_none());
    }
}
