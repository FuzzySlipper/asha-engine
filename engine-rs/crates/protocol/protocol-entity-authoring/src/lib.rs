//! Cross-boundary schema for **generic entity authoring** (post-launchable-03,
//! Den task #2485).
//!
//! # Lane
//!
//! `contract-steward` — the border shape a UI/devtools authoring surface uses to
//! **propose** generic entity lifecycle/capability/relation/movement changes, and
//! the classified outcome authority reports back. Like `protocol-policy-view` it
//! depends on `core-ids` only and carries **no authority logic**: validation and
//! application live in `svc-entity-authoring`, over `core-entity`'s atomic,
//! fail-closed operations. TypeScript can build these proposals and read these
//! outcomes; it can never mutate authority.
//!
//! # Authoring vs. policy
//!
//! `protocol-policy-view` exposes the deliberately *narrow* set a sandboxed policy
//! may propose. This crate is the *fuller* operator/agent authoring surface —
//! create, destroy, attach capabilities, relate, transform, move — that devtools
//! drives. Both are proposal-only and both route through Rust validation.
//!
//! # Single home for stable vocabularies
//!
//! Each routing string — command kind, capability kind, source kind, event kind,
//! rejection reason — has its single home here as a `const` table plus a closed
//! enum, with a test pinning the two together. `protocol-codegen` sources the
//! vocabularies so the generated TypeScript and Rust can never disagree.

#![forbid(unsafe_code)]

use core_ids::{EntityId, ProcessId, SceneNodeId, SubjectId, TagId};

// ── Value shapes ──────────────────────────────────────────────────────────────

/// A runtime transform on the authoring border: translation, rotation `(x,y,z,w)`,
/// scale. Mirrors `protocol_policy_view::PolicyTransform` so the two borders agree.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AuthoringTransform {
    pub translation: [f32; 3],
    pub rotation: [f32; 4],
    pub scale: [f32; 3],
}

/// Where an authored entity comes from. Mirrors `core_entity::EntitySource` on the
/// wire (the asset reference is carried as its canonical id string).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthoringSource {
    SceneBootstrap { node: SceneNodeId },
    RuntimeCreated { by: Option<ProcessId> },
    Imported { asset: String },
    DiagnosticTooling,
    PolicyProposed { by: SubjectId },
}

/// Stable discriminants for [`AuthoringSource`].
pub const SOURCE_KINDS: &[&str] = &[
    "sceneBootstrap",
    "runtimeCreated",
    "imported",
    "diagnosticTooling",
    "policyProposed",
];

impl AuthoringSource {
    pub fn kind(&self) -> &'static str {
        match self {
            AuthoringSource::SceneBootstrap { .. } => "sceneBootstrap",
            AuthoringSource::RuntimeCreated { .. } => "runtimeCreated",
            AuthoringSource::Imported { .. } => "imported",
            AuthoringSource::DiagnosticTooling => "diagnosticTooling",
            AuthoringSource::PolicyProposed { .. } => "policyProposed",
        }
    }
}

// ── Commands ────────────────────────────────────────────────────────────────--

/// The capability an `attachCapability` command attaches. Capability attach is an
/// authoring op on a live entity (it does not validate transform-eligibility — it
/// *establishes* it); the value-carrying transform attach is its own verb.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AuthoringCapability {
    Transform { transform: AuthoringTransform },
    Render { visible: bool },
    Collision { static_collider: bool },
    Bounds { min: [f32; 3], max: [f32; 3] },
}

/// Stable discriminants for [`AuthoringCapability`].
pub const CAPABILITY_KINDS: &[&str] = &["transform", "render", "collision", "bounds"];

impl AuthoringCapability {
    pub fn kind(&self) -> &'static str {
        match self {
            AuthoringCapability::Transform { .. } => "transform",
            AuthoringCapability::Render { .. } => "render",
            AuthoringCapability::Collision { .. } => "collision",
            AuthoringCapability::Bounds { .. } => "bounds",
        }
    }
}

// ── Stored EntityDefinition schema ───────────────────────────────────────────

/// Where a stored entity definition was read from inside a durable ProjectBundle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityDefinitionSourceTrace {
    pub project_bundle: String,
    pub relative_path: String,
}

/// Small string metadata entry for Studio/project readout. This is intentionally
/// display/authoring metadata, not arbitrary runtime authority state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityDefinitionMetadataEntry {
    pub key: String,
    pub value: String,
}

/// A stored capability declaration with an initial value. `Unknown` exists so
/// decoded or hand-authored bad data can be represented and rejected explicitly
/// instead of disappearing before validation.
#[derive(Debug, Clone, PartialEq)]
pub enum EntityDefinitionCapability {
    Transform { transform: AuthoringTransform },
    Render { visible: bool },
    Collision { static_collider: bool },
    Bounds { min: [f32; 3], max: [f32; 3] },
    Unknown { capability_kind: String },
}

/// Stable discriminants for valid stored entity definition capabilities.
pub const ENTITY_DEFINITION_CAPABILITY_KINDS: &[&str] =
    &["transform", "render", "collision", "bounds"];

impl EntityDefinitionCapability {
    pub fn kind(&self) -> &str {
        match self {
            EntityDefinitionCapability::Transform { .. } => "transform",
            EntityDefinitionCapability::Render { .. } => "render",
            EntityDefinitionCapability::Collision { .. } => "collision",
            EntityDefinitionCapability::Bounds { .. } => "bounds",
            EntityDefinitionCapability::Unknown { capability_kind } => capability_kind,
        }
    }
}

/// Durable stored entity definition authored in a ProjectBundle/catalog and later
/// validated by Rust authority before it can seed runtime CapabilityState.
#[derive(Debug, Clone, PartialEq)]
pub struct EntityDefinition {
    pub stable_id: String,
    pub display_name: String,
    pub source: EntityDefinitionSourceTrace,
    pub tags: Vec<TagId>,
    pub metadata: Vec<EntityDefinitionMetadataEntry>,
    pub capabilities: Vec<EntityDefinitionCapability>,
}

/// Classified validation diagnostic for stored entity definitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityDefinitionDiagnosticCode {
    MissingStableId,
    MissingDisplayName,
    MissingSourceTrace,
    UnknownCapability,
    DuplicateCapability,
    NonFiniteInitialValue,
    InvalidInitialValue,
}

/// Stable discriminants for [`EntityDefinitionDiagnosticCode`].
pub const ENTITY_DEFINITION_DIAGNOSTIC_CODES: &[&str] = &[
    "missingStableId",
    "missingDisplayName",
    "missingSourceTrace",
    "unknownCapability",
    "duplicateCapability",
    "nonFiniteInitialValue",
    "invalidInitialValue",
];

impl EntityDefinitionDiagnosticCode {
    pub fn as_str(self) -> &'static str {
        match self {
            EntityDefinitionDiagnosticCode::MissingStableId => "missingStableId",
            EntityDefinitionDiagnosticCode::MissingDisplayName => "missingDisplayName",
            EntityDefinitionDiagnosticCode::MissingSourceTrace => "missingSourceTrace",
            EntityDefinitionDiagnosticCode::UnknownCapability => "unknownCapability",
            EntityDefinitionDiagnosticCode::DuplicateCapability => "duplicateCapability",
            EntityDefinitionDiagnosticCode::NonFiniteInitialValue => "nonFiniteInitialValue",
            EntityDefinitionDiagnosticCode::InvalidInitialValue => "invalidInitialValue",
        }
    }
}

/// One stored EntityDefinition validation diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityDefinitionDiagnostic {
    pub code: EntityDefinitionDiagnosticCode,
    pub path: String,
    pub message: String,
}

/// Validation outcome for stored EntityDefinitions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntityDefinitionValidationOutcome {
    Valid,
    Invalid {
        diagnostics: Vec<EntityDefinitionDiagnostic>,
    },
}

/// A proposed generic entity authoring change. Proposal-only: authority validates
/// and applies or rejects (atomic, fail-closed). One verb per atomic authority op.
#[derive(Debug, Clone, PartialEq)]
pub enum EntityAuthoringCommand {
    Create {
        id: EntityId,
        source: AuthoringSource,
        labels: Vec<TagId>,
    },
    Destroy {
        id: EntityId,
    },
    Disable {
        id: EntityId,
    },
    Enable {
        id: EntityId,
    },
    AddLabel {
        id: EntityId,
        tag: TagId,
    },
    RemoveLabel {
        id: EntityId,
        tag: TagId,
    },
    AttachCapability {
        id: EntityId,
        capability: AuthoringCapability,
    },
    SetTransform {
        id: EntityId,
        transform: AuthoringTransform,
    },
    Move {
        id: EntityId,
        delta: [f32; 3],
    },
    AttachTransformParent {
        child: EntityId,
        parent: EntityId,
    },
    DetachTransformParent {
        child: EntityId,
    },
    SetContainment {
        member: EntityId,
        container: EntityId,
    },
    ClearContainment {
        member: EntityId,
    },
    SetDerivedFrom {
        derived: EntityId,
        origin: EntityId,
    },
}

/// Stable discriminants for [`EntityAuthoringCommand`].
pub const COMMAND_KINDS: &[&str] = &[
    "create",
    "destroy",
    "disable",
    "enable",
    "addLabel",
    "removeLabel",
    "attachCapability",
    "setTransform",
    "move",
    "attachTransformParent",
    "detachTransformParent",
    "setContainment",
    "clearContainment",
    "setDerivedFrom",
];

impl EntityAuthoringCommand {
    pub fn kind(&self) -> &'static str {
        match self {
            EntityAuthoringCommand::Create { .. } => "create",
            EntityAuthoringCommand::Destroy { .. } => "destroy",
            EntityAuthoringCommand::Disable { .. } => "disable",
            EntityAuthoringCommand::Enable { .. } => "enable",
            EntityAuthoringCommand::AddLabel { .. } => "addLabel",
            EntityAuthoringCommand::RemoveLabel { .. } => "removeLabel",
            EntityAuthoringCommand::AttachCapability { .. } => "attachCapability",
            EntityAuthoringCommand::SetTransform { .. } => "setTransform",
            EntityAuthoringCommand::Move { .. } => "move",
            EntityAuthoringCommand::AttachTransformParent { .. } => "attachTransformParent",
            EntityAuthoringCommand::DetachTransformParent { .. } => "detachTransformParent",
            EntityAuthoringCommand::SetContainment { .. } => "setContainment",
            EntityAuthoringCommand::ClearContainment { .. } => "clearContainment",
            EntityAuthoringCommand::SetDerivedFrom { .. } => "setDerivedFrom",
        }
    }
}

// ── Outcomes ────────────────────────────────────────────────────────────────--

/// The kind of accepted authoring change (compact; the inspector re-reads the
/// store snapshot for full detail).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthoringEventKind {
    Created,
    Destroyed,
    Disabled,
    Enabled,
    LabelAdded,
    LabelRemoved,
    CapabilityAttached,
    TransformSet,
    Moved,
    RelationSet,
    RelationCleared,
}

/// Stable discriminants for [`AuthoringEventKind`].
pub const EVENT_KINDS: &[&str] = &[
    "created",
    "destroyed",
    "disabled",
    "enabled",
    "labelAdded",
    "labelRemoved",
    "capabilityAttached",
    "transformSet",
    "moved",
    "relationSet",
    "relationCleared",
];

impl AuthoringEventKind {
    pub fn as_str(self) -> &'static str {
        match self {
            AuthoringEventKind::Created => "created",
            AuthoringEventKind::Destroyed => "destroyed",
            AuthoringEventKind::Disabled => "disabled",
            AuthoringEventKind::Enabled => "enabled",
            AuthoringEventKind::LabelAdded => "labelAdded",
            AuthoringEventKind::LabelRemoved => "labelRemoved",
            AuthoringEventKind::CapabilityAttached => "capabilityAttached",
            AuthoringEventKind::TransformSet => "transformSet",
            AuthoringEventKind::Moved => "moved",
            AuthoringEventKind::RelationSet => "relationSet",
            AuthoringEventKind::RelationCleared => "relationCleared",
        }
    }
}

/// The accepted authoring event: what happened, to which entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntityAuthoringEvent {
    pub kind: AuthoringEventKind,
    pub entity: EntityId,
}

/// The classified reason authority refused a proposed authoring command. A UI
/// reflects this; it never decides acceptance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthoringRejectionReason {
    UnknownEntity,
    AlreadyExists,
    IdRetired,
    Tombstoned,
    EntityNotAlive,
    InvalidTransition,
    LabelAlreadyPresent,
    LabelAbsent,
    NotTransformEligible,
    Immovable,
    NonFinite,
    NotSpatial,
    NoCollider,
    SelfRelation,
    RelationCycle,
    EndpointNotTransformEligible,
    NoSuchRelation,
    ProjectionOnly,
    InvalidAsset,
}

/// Stable discriminants for [`AuthoringRejectionReason`].
pub const REJECTION_REASONS: &[&str] = &[
    "unknownEntity",
    "alreadyExists",
    "idRetired",
    "tombstoned",
    "entityNotAlive",
    "invalidTransition",
    "labelAlreadyPresent",
    "labelAbsent",
    "notTransformEligible",
    "immovable",
    "nonFinite",
    "notSpatial",
    "noCollider",
    "selfRelation",
    "relationCycle",
    "endpointNotTransformEligible",
    "noSuchRelation",
    "projectionOnly",
    "invalidAsset",
];

impl AuthoringRejectionReason {
    pub fn as_str(self) -> &'static str {
        match self {
            AuthoringRejectionReason::UnknownEntity => "unknownEntity",
            AuthoringRejectionReason::AlreadyExists => "alreadyExists",
            AuthoringRejectionReason::IdRetired => "idRetired",
            AuthoringRejectionReason::Tombstoned => "tombstoned",
            AuthoringRejectionReason::EntityNotAlive => "entityNotAlive",
            AuthoringRejectionReason::InvalidTransition => "invalidTransition",
            AuthoringRejectionReason::LabelAlreadyPresent => "labelAlreadyPresent",
            AuthoringRejectionReason::LabelAbsent => "labelAbsent",
            AuthoringRejectionReason::NotTransformEligible => "notTransformEligible",
            AuthoringRejectionReason::Immovable => "immovable",
            AuthoringRejectionReason::NonFinite => "nonFinite",
            AuthoringRejectionReason::NotSpatial => "notSpatial",
            AuthoringRejectionReason::NoCollider => "noCollider",
            AuthoringRejectionReason::SelfRelation => "selfRelation",
            AuthoringRejectionReason::RelationCycle => "relationCycle",
            AuthoringRejectionReason::EndpointNotTransformEligible => {
                "endpointNotTransformEligible"
            }
            AuthoringRejectionReason::NoSuchRelation => "noSuchRelation",
            AuthoringRejectionReason::ProjectionOnly => "projectionOnly",
            AuthoringRejectionReason::InvalidAsset => "invalidAsset",
        }
    }
}

/// The classified refusal: a reason plus the primary entity it concerns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntityAuthoringRejection {
    pub reason: AuthoringRejectionReason,
    pub entity: EntityId,
}

/// The outcome authority reports for one proposed authoring command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityAuthoringOutcome {
    Accepted { event: EntityAuthoringEvent },
    Rejected { rejection: EntityAuthoringRejection },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_kind_table_matches_a_representative_set() {
        // Each kind() string appears in the COMMAND_KINDS table.
        let samples = [
            EntityAuthoringCommand::Destroy {
                id: EntityId::new(1),
            }
            .kind(),
            EntityAuthoringCommand::SetContainment {
                member: EntityId::new(1),
                container: EntityId::new(2),
            }
            .kind(),
        ];
        for s in samples {
            assert!(COMMAND_KINDS.contains(&s), "missing {s}");
        }
        assert_eq!(COMMAND_KINDS.len(), 14);
    }

    #[test]
    fn event_kind_table_matches_variants() {
        let all = [
            AuthoringEventKind::Created,
            AuthoringEventKind::Destroyed,
            AuthoringEventKind::Disabled,
            AuthoringEventKind::Enabled,
            AuthoringEventKind::LabelAdded,
            AuthoringEventKind::LabelRemoved,
            AuthoringEventKind::CapabilityAttached,
            AuthoringEventKind::TransformSet,
            AuthoringEventKind::Moved,
            AuthoringEventKind::RelationSet,
            AuthoringEventKind::RelationCleared,
        ];
        let from: Vec<&str> = all.iter().map(|k| k.as_str()).collect();
        assert_eq!(from, EVENT_KINDS);
    }

    #[test]
    fn rejection_reason_table_matches_variants() {
        let all = [
            AuthoringRejectionReason::UnknownEntity,
            AuthoringRejectionReason::AlreadyExists,
            AuthoringRejectionReason::IdRetired,
            AuthoringRejectionReason::Tombstoned,
            AuthoringRejectionReason::EntityNotAlive,
            AuthoringRejectionReason::InvalidTransition,
            AuthoringRejectionReason::LabelAlreadyPresent,
            AuthoringRejectionReason::LabelAbsent,
            AuthoringRejectionReason::NotTransformEligible,
            AuthoringRejectionReason::Immovable,
            AuthoringRejectionReason::NonFinite,
            AuthoringRejectionReason::NotSpatial,
            AuthoringRejectionReason::NoCollider,
            AuthoringRejectionReason::SelfRelation,
            AuthoringRejectionReason::RelationCycle,
            AuthoringRejectionReason::EndpointNotTransformEligible,
            AuthoringRejectionReason::NoSuchRelation,
            AuthoringRejectionReason::ProjectionOnly,
            AuthoringRejectionReason::InvalidAsset,
        ];
        let from: Vec<&str> = all.iter().map(|r| r.as_str()).collect();
        assert_eq!(from, REJECTION_REASONS);
    }

    #[test]
    fn vocabulary_tables_are_unique() {
        for table in [
            SOURCE_KINDS,
            CAPABILITY_KINDS,
            ENTITY_DEFINITION_CAPABILITY_KINDS,
            COMMAND_KINDS,
            EVENT_KINDS,
            REJECTION_REASONS,
            ENTITY_DEFINITION_DIAGNOSTIC_CODES,
        ] {
            let mut sorted = table.to_vec();
            sorted.sort_unstable();
            sorted.dedup();
            assert_eq!(sorted.len(), table.len(), "duplicate in {table:?}");
        }
    }

    #[test]
    fn entity_definition_diagnostic_table_matches_variants() {
        let all = [
            EntityDefinitionDiagnosticCode::MissingStableId,
            EntityDefinitionDiagnosticCode::MissingDisplayName,
            EntityDefinitionDiagnosticCode::MissingSourceTrace,
            EntityDefinitionDiagnosticCode::UnknownCapability,
            EntityDefinitionDiagnosticCode::DuplicateCapability,
            EntityDefinitionDiagnosticCode::NonFiniteInitialValue,
            EntityDefinitionDiagnosticCode::InvalidInitialValue,
        ];
        let from: Vec<&str> = all.iter().map(|code| code.as_str()).collect();
        assert_eq!(from, ENTITY_DEFINITION_DIAGNOSTIC_CODES);
    }
}
