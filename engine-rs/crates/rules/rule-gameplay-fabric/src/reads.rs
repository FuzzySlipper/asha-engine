//! Declared, bounded read assembly for gameplay-module invocation waves.

use core_entity::{ActivatableCapabilityKind, ControllerCapability, EntityStore};
use core_ids::{EntityId, PrefabId, PrefabInstanceId, PrefabPartId, TagId};
use protocol_game_extension::{
    GameplayContractRef, GameplayEventEnvelope, GameplayReadSelectorCapability,
    GameplayReadViewKind,
};
use protocol_project_bundle::PrefabPartReference;
use rule_trigger_volume::TriggerVolumeRule;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use svc_gameplay_fabric::GameplayFabricRegistry;
use svc_serialization::ValidatedPrefabRegistry;

use crate::{gameplay_module_payload_hash, GameplayModuleStateScope, GameplayModuleStateStore};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameplayReadPlan {
    pub module_id: String,
    pub invocation_id: String,
    pub event_id: String,
    pub wave: u32,
    pub requests: Vec<GameplayReadRequest>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameplayReadRequest {
    pub request_id: String,
    pub view: GameplayContractRef,
    pub fields: Vec<String>,
    pub selector: GameplayReadSelector,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameplayEventEntityBinding {
    Source,
    Subject { index: u32 },
    Target { index: u32 },
    Known(EntityId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameplayCapabilityReadKind {
    Lifecycle,
    Transform,
    Collision,
    Controller,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameplayRelationshipReadKind {
    TransformParent,
    Containment,
    SourceAncestry,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameplayOwnerQuery {
    NearbyEntities {
        anchor: GameplayEventEntityBinding,
        radius_millimeters: u64,
        required_tags: Vec<TagId>,
        max_items: u32,
    },
    LineOfSight {
        source: GameplayEventEntityBinding,
        target: GameplayEventEntityBinding,
    },
    PathBetween {
        source: GameplayEventEntityBinding,
        target: GameplayEventEntityBinding,
        max_steps: u32,
    },
    CurrentTriggerOverlaps {
        trigger: GameplayEventEntityBinding,
        max_items: u32,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameplayReadSelector {
    EventIdentity {
        binding: GameplayEventEntityBinding,
    },
    Capability {
        binding: GameplayEventEntityBinding,
        capability: GameplayCapabilityReadKind,
    },
    Related {
        binding: GameplayEventEntityBinding,
        relationship: GameplayRelationshipReadKind,
    },
    PrefabPart {
        instance: PrefabInstanceId,
        reference: PrefabPartReference,
    },
    Tags {
        required_tags: Vec<TagId>,
        max_items: u32,
    },
    Scope {
        scope: String,
        max_items: u32,
    },
    ModuleNamed {
        scope: GameplayModuleStateScope,
    },
    OwnerQuery {
        query: GameplayOwnerQuery,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameplayCapabilityReadout {
    pub entity: u64,
    pub capability: String,
    pub entity_lifecycle: String,
    pub presence: String,
    pub effective_active: bool,
    pub fields: BTreeMap<String, GameplayScalarReadValue>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "camelCase")]
pub enum GameplayScalarReadValue {
    Boolean(bool),
    Unsigned(u64),
    UnsignedList(Vec<u64>),
    Text(String),
    FloatBits(Vec<u32>),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum GameplayOwnerQueryResult {
    NearbyEntities {
        entities: Vec<u64>,
        provider_revision: u64,
    },
    LineOfSight {
        visible: bool,
        blocker: Option<u64>,
        provider_revision: u64,
    },
    PathBetween {
        reachable: bool,
        steps: Vec<[i64; 3]>,
        provider_revision: u64,
    },
    CurrentTriggerOverlaps {
        trigger: u64,
        subjects: Vec<u64>,
        provider_revision: u64,
        overlap_hash: String,
    },
}

impl GameplayOwnerQueryResult {
    fn item_count(&self) -> usize {
        match self {
            Self::NearbyEntities { entities, .. } => entities.len(),
            Self::LineOfSight { .. } => 1,
            Self::PathBetween { steps, .. } => steps.len(),
            Self::CurrentTriggerOverlaps { subjects, .. } => subjects.len(),
        }
    }
}

pub trait GameplayOwnerQueryProvider {
    fn provider_id(&self) -> &str;

    fn query(
        &self,
        request: GameplayResolvedOwnerQuery,
    ) -> Result<GameplayOwnerQueryResult, GameplayReadProviderError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameplayResolvedOwnerQuery {
    NearbyEntities {
        anchor: EntityId,
        radius_millimeters: u64,
        required_tags: Vec<TagId>,
        max_items: u32,
    },
    LineOfSight {
        source: EntityId,
        target: EntityId,
    },
    PathBetween {
        source: EntityId,
        target: EntityId,
        max_steps: u32,
    },
    CurrentTriggerOverlaps {
        trigger: EntityId,
        max_items: u32,
    },
}

/// Internal adapter from the collision-owned trigger state into the generic
/// closed owner-query boundary. Modules receive only the frozen typed result.
pub struct GameplayTriggerOverlapQueryProvider<'a> {
    provider_id: String,
    triggers: &'a TriggerVolumeRule,
}

impl<'a> GameplayTriggerOverlapQueryProvider<'a> {
    pub fn new(provider_id: impl Into<String>, triggers: &'a TriggerVolumeRule) -> Self {
        Self {
            provider_id: provider_id.into(),
            triggers,
        }
    }
}

impl GameplayOwnerQueryProvider for GameplayTriggerOverlapQueryProvider<'_> {
    fn provider_id(&self) -> &str {
        &self.provider_id
    }

    fn query(
        &self,
        request: GameplayResolvedOwnerQuery,
    ) -> Result<GameplayOwnerQueryResult, GameplayReadProviderError> {
        let GameplayResolvedOwnerQuery::CurrentTriggerOverlaps { trigger, max_items } = request
        else {
            return Err(GameplayReadProviderError {
                code: "unsupportedQuery".to_owned(),
                message: "trigger overlap provider only serves current trigger overlaps".to_owned(),
            });
        };
        let readout = self
            .triggers
            .current_overlaps(trigger, max_items)
            .map_err(|error| GameplayReadProviderError {
                code: error
                    .diagnostics
                    .first()
                    .map(|diagnostic| diagnostic.code.as_str())
                    .unwrap_or("triggerReadRejected")
                    .to_owned(),
                message: error.to_string(),
            })?;
        Ok(GameplayOwnerQueryResult::CurrentTriggerOverlaps {
            trigger: readout.trigger,
            subjects: readout.subjects,
            provider_revision: readout.revision,
            overlap_hash: readout.overlap_hash,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameplayReadProviderError {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum GameplayReadValue {
    EventIdentity {
        entity: u64,
    },
    Capability {
        readout: GameplayCapabilityReadout,
    },
    RelatedEntity {
        relationship: String,
        from: u64,
        entity: u64,
    },
    PrefabPart {
        instance: u64,
        prefab: u64,
        role: String,
        part: u64,
        entity: u64,
    },
    EntitySelection {
        entities: Vec<u64>,
    },
    ModuleNamed {
        scope: GameplayModuleStateScope,
        revision: u64,
        canonical_payload: Vec<u8>,
        view_hash: String,
    },
    OwnerQuery {
        result: GameplayOwnerQueryResult,
    },
    Missing {
        reason: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameplayFrozenRead {
    pub request_id: String,
    pub view: GameplayContractRef,
    pub provider_id: String,
    pub fields: Vec<String>,
    pub value: GameplayReadValue,
    pub value_hash: String,
}

impl GameplayFrozenRead {
    pub fn decode_named_view<T: DeserializeOwned>(&self) -> Result<T, GameplayReadDecodeError> {
        let GameplayReadValue::ModuleNamed {
            canonical_payload, ..
        } = &self.value
        else {
            return Err(GameplayReadDecodeError::NotNamedView);
        };
        serde_json::from_slice(canonical_payload)
            .map_err(|error| GameplayReadDecodeError::Decode(error.to_string()))
    }

    pub fn canonical_value_hash(&self) -> String {
        hash_serializable(&self.value)
    }

    pub fn value_hash_is_valid(&self) -> bool {
        self.value_hash == self.canonical_value_hash()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameplayReadDecodeError {
    NotNamedView,
    Decode(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameplayFrozenReadSet {
    pub registry_digest: String,
    pub module_id: String,
    pub invocation_id: String,
    pub event_id: String,
    pub wave: u32,
    pub reads: Vec<GameplayFrozenRead>,
    pub read_set_hash: String,
}

impl GameplayFrozenReadSet {
    pub fn canonical_hash(&self) -> String {
        let mut canonical = self.clone();
        canonical.read_set_hash.clear();
        hash_serializable(&canonical)
    }

    pub fn nested_hashes_are_valid(&self) -> bool {
        self.reads
            .iter()
            .all(GameplayFrozenRead::value_hash_is_valid)
            && self.read_set_hash == self.canonical_hash()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum GameplayReadDiagnosticCode {
    UnknownModule,
    EventMismatch,
    DuplicateRequest,
    UndeclaredRead,
    MissingProvider,
    ProviderMismatch,
    UnsupportedViewKind,
    UnsupportedSelector,
    MissingField,
    QuotaExceeded,
    MissingIdentity,
    StaleIdentity,
    MissingPrefab,
    MissingPrefabRole,
    MissingPrefabInstance,
    ForeignPrefabInstance,
    MissingModuleView,
    MissingOwnerQueryProvider,
    OwnerQueryRejected,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameplayReadDiagnostic {
    pub code: GameplayReadDiagnosticCode,
    pub request_id: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameplayReadAssemblyError {
    pub diagnostics: Vec<GameplayReadDiagnostic>,
}

impl core::fmt::Display for GameplayReadAssemblyError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            formatter,
            "gameplay read plan rejected with {} diagnostic(s)",
            self.diagnostics.len()
        )
    }
}

impl std::error::Error for GameplayReadAssemblyError {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameplayReadPlanEntryReadout {
    pub request_id: String,
    pub view: String,
    pub view_schema_hash: String,
    pub provider_id: String,
    pub kind: GameplayReadViewKind,
    pub selectors: Vec<GameplayReadSelectorCapability>,
    pub fields: Vec<String>,
    pub max_items: u32,
    pub ordering: String,
    pub provider_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameplayReadPlanReadout {
    pub registry_digest: String,
    pub module_id: String,
    pub invocation_id: String,
    pub entries: Vec<GameplayReadPlanEntryReadout>,
    pub plan_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameplayPrefabInstanceBinding {
    pub prefab: PrefabId,
    pub part_entities: BTreeMap<PrefabPartId, EntityId>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GameplayPrefabInstanceIndex {
    bindings: BTreeMap<PrefabInstanceId, GameplayPrefabInstanceBinding>,
}

impl GameplayPrefabInstanceIndex {
    pub fn insert(
        &mut self,
        instance: PrefabInstanceId,
        binding: GameplayPrefabInstanceBinding,
    ) -> Result<(), GameplayReadDiagnosticCode> {
        if self.bindings.contains_key(&instance) {
            return Err(GameplayReadDiagnosticCode::MissingPrefabInstance);
        }
        self.bindings.insert(instance, binding);
        Ok(())
    }

    fn get(&self, instance: PrefabInstanceId) -> Option<&GameplayPrefabInstanceBinding> {
        self.bindings.get(&instance)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GameplayEntityScopeIndex {
    scopes: BTreeMap<String, BTreeSet<EntityId>>,
}

impl GameplayEntityScopeIndex {
    pub fn bind(&mut self, scope: impl Into<String>, entity: EntityId) {
        self.scopes.entry(scope.into()).or_default().insert(entity);
    }

    fn entities(&self, scope: &str) -> impl Iterator<Item = EntityId> + '_ {
        self.scopes
            .get(scope)
            .into_iter()
            .flat_map(|entities| entities.iter().copied())
    }
}

pub struct GameplayReadAssembler<'a, 'registry> {
    registry: &'registry GameplayFabricRegistry,
    entities: &'a EntityStore,
    module_state: &'a GameplayModuleStateStore,
    prefab_registry: &'a ValidatedPrefabRegistry,
    prefab_instances: &'a GameplayPrefabInstanceIndex,
    scopes: &'a GameplayEntityScopeIndex,
    owner_queries: BTreeMap<String, &'a dyn GameplayOwnerQueryProvider>,
}

impl<'a, 'registry> GameplayReadAssembler<'a, 'registry> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        registry: &'registry GameplayFabricRegistry,
        entities: &'a EntityStore,
        module_state: &'a GameplayModuleStateStore,
        prefab_registry: &'a ValidatedPrefabRegistry,
        prefab_instances: &'a GameplayPrefabInstanceIndex,
        scopes: &'a GameplayEntityScopeIndex,
        owner_query_providers: Vec<&'a dyn GameplayOwnerQueryProvider>,
    ) -> Result<Self, GameplayReadAssemblyError> {
        let mut owner_queries = BTreeMap::new();
        for provider in owner_query_providers {
            if owner_queries
                .insert(provider.provider_id().to_owned(), provider)
                .is_some()
            {
                return Err(error(
                    "provider-registration",
                    GameplayReadDiagnosticCode::ProviderMismatch,
                    format!(
                        "duplicate owner query provider `{}`",
                        provider.provider_id()
                    ),
                ));
            }
        }
        Ok(Self {
            registry,
            entities,
            module_state,
            prefab_registry,
            prefab_instances,
            scopes,
            owner_queries,
        })
    }

    pub fn read_plan_readout(
        &self,
        plan: &GameplayReadPlan,
    ) -> Result<GameplayReadPlanReadout, GameplayReadAssemblyError> {
        self.validate_plan_header(plan)?;
        let mut entries = Vec::new();
        let mut seen = BTreeSet::new();
        for request in &plan.requests {
            if !seen.insert(request.request_id.as_str()) {
                return Err(error(
                    &request.request_id,
                    GameplayReadDiagnosticCode::DuplicateRequest,
                    "request ids must be unique within one read plan",
                ));
            }
            let metadata = self.validate_request(plan, request)?;
            entries.push(GameplayReadPlanEntryReadout {
                request_id: request.request_id.clone(),
                view: request.view.key(),
                view_schema_hash: request.view.schema_hash.clone(),
                provider_id: metadata.provider_id,
                kind: metadata.kind,
                selectors: metadata.selectors,
                fields: metadata.fields,
                max_items: metadata.max_items,
                ordering: metadata.ordering,
                provider_hash: metadata.provider_hash,
            });
        }
        entries.sort_by(|a, b| a.request_id.cmp(&b.request_id));
        let mut readout = GameplayReadPlanReadout {
            registry_digest: self.registry.registry_digest().to_owned(),
            module_id: plan.module_id.clone(),
            invocation_id: plan.invocation_id.clone(),
            entries,
            plan_hash: String::new(),
        };
        readout.plan_hash = hash_serializable(&readout);
        Ok(readout)
    }

    pub fn assemble(
        &self,
        plan: &GameplayReadPlan,
        event: &GameplayEventEnvelope,
    ) -> Result<GameplayFrozenReadSet, GameplayReadAssemblyError> {
        self.validate_plan_header(plan)?;
        if plan.event_id != event.event_id || plan.wave != event.wave {
            return Err(error(
                "plan",
                GameplayReadDiagnosticCode::EventMismatch,
                "read plan event identity or wave does not match the delivered event",
            ));
        }
        let mut seen = BTreeSet::new();
        let mut reads = Vec::new();
        for request in &plan.requests {
            if !seen.insert(request.request_id.as_str()) {
                return Err(error(
                    &request.request_id,
                    GameplayReadDiagnosticCode::DuplicateRequest,
                    "request ids must be unique within one read plan",
                ));
            }
            let metadata = self.validate_request(plan, request)?;
            let value = self.resolve(request, event, metadata.max_items)?;
            if value_item_count(&value) > metadata.max_items as usize {
                return Err(error(
                    &request.request_id,
                    GameplayReadDiagnosticCode::QuotaExceeded,
                    "provider result exceeded the closed registry item quota",
                ));
            }
            let value_hash = hash_serializable(&value);
            reads.push(GameplayFrozenRead {
                request_id: request.request_id.clone(),
                view: request.view.clone(),
                provider_id: metadata.provider_id,
                fields: metadata.fields,
                value,
                value_hash,
            });
        }
        reads.sort_by(|a, b| a.request_id.cmp(&b.request_id));
        let mut frozen = GameplayFrozenReadSet {
            registry_digest: self.registry.registry_digest().to_owned(),
            module_id: plan.module_id.clone(),
            invocation_id: plan.invocation_id.clone(),
            event_id: plan.event_id.clone(),
            wave: plan.wave,
            reads,
            read_set_hash: String::new(),
        };
        frozen.read_set_hash = hash_serializable(&frozen);
        Ok(frozen)
    }

    fn validate_plan_header(
        &self,
        plan: &GameplayReadPlan,
    ) -> Result<(), GameplayReadAssemblyError> {
        let Some(module) = self.registry.module(&plan.module_id) else {
            return Err(error(
                "plan",
                GameplayReadDiagnosticCode::UnknownModule,
                format!("unknown module `{}`", plan.module_id),
            ));
        };
        let Some(_invocation) = module
            .invocations
            .iter()
            .find(|invocation| invocation.invocation_id == plan.invocation_id)
        else {
            return Err(error(
                "plan",
                GameplayReadDiagnosticCode::UndeclaredRead,
                format!(
                    "module does not declare invocation `{}`",
                    plan.invocation_id
                ),
            ));
        };
        Ok(())
    }

    fn validate_request(
        &self,
        plan: &GameplayReadPlan,
        request: &GameplayReadRequest,
    ) -> Result<ValidatedReadMetadata, GameplayReadAssemblyError> {
        let module = self
            .registry
            .module(&plan.module_id)
            .expect("header checked");
        let invocation = module
            .invocations
            .iter()
            .find(|invocation| invocation.invocation_id == plan.invocation_id)
            .expect("header checked");
        let Some(invocation_requirement) = invocation
            .read_requirements
            .iter()
            .find(|requirement| requirement.request_id == request.request_id)
        else {
            return Err(error(
                &request.request_id,
                GameplayReadDiagnosticCode::UndeclaredRead,
                format!(
                    "invocation `{}` does not declare read request `{}`",
                    plan.invocation_id, request.request_id
                ),
            ));
        };
        if invocation_requirement.view != request.view {
            return Err(error(
                &request.request_id,
                GameplayReadDiagnosticCode::UndeclaredRead,
                format!(
                    "invocation read request `{}` is bound to `{}` rather than `{}`",
                    request.request_id,
                    invocation_requirement.view.key(),
                    request.view.key()
                ),
            ));
        }
        let Some(requirement) = module
            .read_views
            .iter()
            .find(|requirement| requirement.view == request.view)
        else {
            return Err(error(
                &request.request_id,
                GameplayReadDiagnosticCode::UndeclaredRead,
                format!("module does not declare read `{}`", request.view.key()),
            ));
        };
        let Some(provider) = self.registry.read_view_provider(&request.view) else {
            return Err(error(
                &request.request_id,
                GameplayReadDiagnosticCode::MissingProvider,
                format!(
                    "read `{}` has no closed-registry provider",
                    request.view.key()
                ),
            ));
        };
        let (kind, selectors, item_limit) = selector_metadata(&request.selector);
        if requirement.provider_id != provider.provider_id || requirement.kind != provider.kind {
            return Err(error(
                &request.request_id,
                GameplayReadDiagnosticCode::ProviderMismatch,
                "manifest and closed-registry provider metadata disagree",
            ));
        }
        if kind != requirement.kind {
            return Err(error(
                &request.request_id,
                GameplayReadDiagnosticCode::UnsupportedViewKind,
                format!("selector cannot consume `{}`", requirement.kind.as_str()),
            ));
        }
        for selector in &selectors {
            if !requirement.selector_capabilities.contains(selector)
                || !provider.selector_capabilities.contains(selector)
            {
                return Err(error(
                    &request.request_id,
                    GameplayReadDiagnosticCode::UnsupportedSelector,
                    format!("selector `{}` is not declared", selector.as_str()),
                ));
            }
        }
        let declared_fields: BTreeSet<&str> =
            requirement.fields.iter().map(String::as_str).collect();
        let provider_fields: BTreeSet<&str> = provider.fields.iter().map(String::as_str).collect();
        for field in &request.fields {
            if !declared_fields.contains(field.as_str())
                || !provider_fields.contains(field.as_str())
            {
                return Err(error(
                    &request.request_id,
                    GameplayReadDiagnosticCode::MissingField,
                    format!("field `{field}` is unavailable or undeclared"),
                ));
            }
        }
        let max_items = requirement.max_items.min(provider.max_items);
        if item_limit > max_items {
            return Err(error(
                &request.request_id,
                GameplayReadDiagnosticCode::QuotaExceeded,
                format!("request asks for {item_limit} items but quota is {max_items}"),
            ));
        }
        let provider_hash = self
            .registry
            .readout()
            .read_view_provider_details
            .iter()
            .find(|detail| detail.view == request.view.key())
            .map(|detail| detail.provider_hash.clone())
            .ok_or_else(|| {
                error(
                    &request.request_id,
                    GameplayReadDiagnosticCode::MissingProvider,
                    "provider metadata readout is unavailable",
                )
            })?;
        let mut fields = request.fields.clone();
        fields.sort();
        fields.dedup();
        Ok(ValidatedReadMetadata {
            provider_id: provider.provider_id.clone(),
            kind,
            selectors,
            fields,
            max_items,
            ordering: provider.ordering.clone(),
            provider_hash,
        })
    }

    fn resolve(
        &self,
        request: &GameplayReadRequest,
        event: &GameplayEventEnvelope,
        max_items: u32,
    ) -> Result<GameplayReadValue, GameplayReadAssemblyError> {
        match &request.selector {
            GameplayReadSelector::EventIdentity { binding } => {
                let entity = self.resolve_entity(*binding, event, &request.request_id)?;
                Ok(GameplayReadValue::EventIdentity {
                    entity: entity.raw(),
                })
            }
            GameplayReadSelector::Capability {
                binding,
                capability,
            } => {
                let entity = self.resolve_entity(*binding, event, &request.request_id)?;
                Ok(GameplayReadValue::Capability {
                    readout: self.capability_readout(entity, *capability, &request.fields),
                })
            }
            GameplayReadSelector::Related {
                binding,
                relationship,
            } => {
                let from = self.resolve_entity(*binding, event, &request.request_id)?;
                let related = match relationship {
                    GameplayRelationshipReadKind::TransformParent => {
                        self.entities.transform_parent_of(from)
                    }
                    GameplayRelationshipReadKind::Containment => {
                        self.entities.containment(from).map(|value| value.container)
                    }
                    GameplayRelationshipReadKind::SourceAncestry => {
                        self.entities.derived_from(from)
                    }
                };
                let Some(entity) = related else {
                    return Ok(GameplayReadValue::Missing {
                        reason: "relationshipAbsent".to_owned(),
                    });
                };
                self.require_live_entity(entity, &request.request_id)?;
                Ok(GameplayReadValue::RelatedEntity {
                    relationship: relationship_label(*relationship).to_owned(),
                    from: from.raw(),
                    entity: entity.raw(),
                })
            }
            GameplayReadSelector::PrefabPart {
                instance,
                reference,
            } => self.resolve_prefab_part(*instance, reference, &request.request_id),
            GameplayReadSelector::Tags {
                required_tags,
                max_items: requested,
            } => {
                let entities = self
                    .entities
                    .entities()
                    .filter(|entity| {
                        entity.lifecycle.is_alive()
                            && required_tags.iter().all(|tag| entity.has_label(*tag))
                    })
                    .map(|entity| entity.id.raw())
                    .collect::<Vec<_>>();
                let limit = (*requested).min(max_items) as usize;
                if entities.len() > limit {
                    return Err(error(
                        &request.request_id,
                        GameplayReadDiagnosticCode::QuotaExceeded,
                        format!(
                            "tag selection matched {} entities but its bound is {limit}",
                            entities.len()
                        ),
                    ));
                }
                Ok(GameplayReadValue::EntitySelection { entities })
            }
            GameplayReadSelector::Scope {
                scope,
                max_items: requested,
            } => {
                let mut entities = Vec::new();
                for entity in self.scopes.entities(scope) {
                    self.require_live_entity(entity, &request.request_id)?;
                    entities.push(entity.raw());
                }
                let limit = (*requested).min(max_items) as usize;
                if entities.len() > limit {
                    return Err(error(
                        &request.request_id,
                        GameplayReadDiagnosticCode::QuotaExceeded,
                        format!(
                            "scope selection matched {} entities but its bound is {limit}",
                            entities.len()
                        ),
                    ));
                }
                Ok(GameplayReadValue::EntitySelection { entities })
            }
            GameplayReadSelector::ModuleNamed { scope } => {
                let view = self
                    .module_state
                    .named_view_by_contract(&request.view, scope)
                    .map_err(|error_value| {
                        error(
                            &request.request_id,
                            GameplayReadDiagnosticCode::MissingModuleView,
                            format!("module named view rejected: {error_value}"),
                        )
                    })?;
                Ok(GameplayReadValue::ModuleNamed {
                    scope: view.scope,
                    revision: view.revision,
                    canonical_payload: view.canonical_payload,
                    view_hash: view.view_hash,
                })
            }
            GameplayReadSelector::OwnerQuery { query } => {
                let provider_id = self
                    .registry
                    .read_view_provider(&request.view)
                    .expect("request validation checked provider")
                    .provider_id
                    .as_str();
                let Some(provider) = self.owner_queries.get(provider_id) else {
                    return Err(error(
                        &request.request_id,
                        GameplayReadDiagnosticCode::MissingOwnerQueryProvider,
                        format!("owner query provider `{provider_id}` is not installed"),
                    ));
                };
                let resolved = self.resolve_owner_query(query, event, &request.request_id)?;
                let mut result = provider.query(resolved.clone()).map_err(|provider_error| {
                    error(
                        &request.request_id,
                        GameplayReadDiagnosticCode::OwnerQueryRejected,
                        format!("{}: {}", provider_error.code, provider_error.message),
                    )
                })?;
                self.validate_owner_query_result(&resolved, &mut result, &request.request_id)?;
                if result.item_count() > max_items as usize {
                    return Err(error(
                        &request.request_id,
                        GameplayReadDiagnosticCode::QuotaExceeded,
                        "owner query result exceeded its registered quota",
                    ));
                }
                Ok(GameplayReadValue::OwnerQuery { result })
            }
        }
    }

    fn resolve_entity(
        &self,
        binding: GameplayEventEntityBinding,
        event: &GameplayEventEnvelope,
        request_id: &str,
    ) -> Result<EntityId, GameplayReadAssemblyError> {
        let entity = match binding {
            GameplayEventEntityBinding::Source => event.source.as_ref().map(|item| item.entity),
            GameplayEventEntityBinding::Subject { index } => {
                event.subjects.get(index as usize).map(|item| item.entity)
            }
            GameplayEventEntityBinding::Target { index } => {
                event.targets.get(index as usize).map(|item| item.entity)
            }
            GameplayEventEntityBinding::Known(entity) => Some(entity),
        }
        .ok_or_else(|| {
            error(
                request_id,
                GameplayReadDiagnosticCode::MissingIdentity,
                "event-bound identity is absent or index is out of range",
            )
        })?;
        self.require_live_entity(entity, request_id)?;
        Ok(entity)
    }

    fn require_live_entity(
        &self,
        entity: EntityId,
        request_id: &str,
    ) -> Result<(), GameplayReadAssemblyError> {
        if !self.entities.contains(entity) || !self.entities.is_alive(entity) {
            return Err(error(
                request_id,
                GameplayReadDiagnosticCode::StaleIdentity,
                format!("entity {} is missing or tombstoned", entity.raw()),
            ));
        }
        Ok(())
    }

    fn capability_readout(
        &self,
        entity: EntityId,
        capability: GameplayCapabilityReadKind,
        requested_fields: &[String],
    ) -> GameplayCapabilityReadout {
        let lifecycle = self
            .entities
            .lifecycle(entity)
            .expect("entity was checked live");
        let mut fields = BTreeMap::new();
        let (capability_name, presence, effective_active) = match capability {
            GameplayCapabilityReadKind::Lifecycle => {
                insert_if_requested(
                    &mut fields,
                    requested_fields,
                    "source",
                    GameplayScalarReadValue::Text(
                        self.entities
                            .core(entity)
                            .expect("entity was checked live")
                            .source
                            .label()
                            .to_owned(),
                    ),
                );
                insert_if_requested(
                    &mut fields,
                    requested_fields,
                    "labels",
                    GameplayScalarReadValue::UnsignedList(
                        self.entities
                            .core(entity)
                            .expect("entity was checked live")
                            .labels
                            .iter()
                            .map(|tag| tag.raw())
                            .collect(),
                    ),
                );
                ("lifecycle", "present", lifecycle.label() == "active")
            }
            GameplayCapabilityReadKind::Transform => {
                if let Some(transform) = self.entities.transform(entity) {
                    let value = transform.transform;
                    insert_if_requested(
                        &mut fields,
                        requested_fields,
                        "translation",
                        GameplayScalarReadValue::FloatBits(vec![
                            value.translation.x.to_bits(),
                            value.translation.y.to_bits(),
                            value.translation.z.to_bits(),
                        ]),
                    );
                    ("transform", "present", lifecycle.label() == "active")
                } else {
                    ("transform", "absent", false)
                }
            }
            GameplayCapabilityReadKind::Collision => {
                let activation = self
                    .entities
                    .capability_activation(entity, ActivatableCapabilityKind::Collision)
                    .expect("entity was checked live");
                if let Some(collision) = self.entities.collision(entity) {
                    insert_if_requested(
                        &mut fields,
                        requested_fields,
                        "staticCollider",
                        GameplayScalarReadValue::Boolean(collision.static_collider),
                    );
                }
                (
                    "collision",
                    activation.presence.label(),
                    activation.effective_active,
                )
            }
            GameplayCapabilityReadKind::Controller => {
                let activation = self
                    .entities
                    .capability_activation(entity, ActivatableCapabilityKind::Controller)
                    .expect("entity was checked live");
                if let Some(controller) = self.entities.controller(entity) {
                    let (kind, id) = match controller {
                        ControllerCapability::Process(id) => ("process", id.raw()),
                        ControllerCapability::Subject(id) => ("subject", id.raw()),
                    };
                    insert_if_requested(
                        &mut fields,
                        requested_fields,
                        "controllerKind",
                        GameplayScalarReadValue::Text(kind.to_owned()),
                    );
                    insert_if_requested(
                        &mut fields,
                        requested_fields,
                        "controllerId",
                        GameplayScalarReadValue::Unsigned(id),
                    );
                }
                (
                    "controller",
                    activation.presence.label(),
                    activation.effective_active,
                )
            }
        };
        GameplayCapabilityReadout {
            entity: entity.raw(),
            capability: capability_name.to_owned(),
            entity_lifecycle: lifecycle.label().to_owned(),
            presence: presence.to_owned(),
            effective_active,
            fields,
        }
    }

    fn resolve_prefab_part(
        &self,
        instance: PrefabInstanceId,
        reference: &PrefabPartReference,
        request_id: &str,
    ) -> Result<GameplayReadValue, GameplayReadAssemblyError> {
        let definition = self
            .prefab_registry
            .as_registry()
            .definitions
            .iter()
            .find(|definition| definition.id.raw() == reference.prefab.raw())
            .ok_or_else(|| {
                error(
                    request_id,
                    GameplayReadDiagnosticCode::MissingPrefab,
                    format!("prefab {} is not registered", reference.prefab.raw()),
                )
            })?;
        let (role_definition, removed) = if let Some(variant) = &definition.variant {
            let base = self
                .prefab_registry
                .as_registry()
                .definitions
                .iter()
                .find(|candidate| candidate.id == variant.base)
                .expect("validated registry has the variant base");
            (base, variant.removed_roles.contains(&reference.role))
        } else {
            (definition, false)
        };
        let part = (!removed)
            .then(|| {
                role_definition
                    .part_roles
                    .iter()
                    .find(|binding| binding.role == reference.role)
                    .map(|binding| binding.part)
            })
            .flatten()
            .ok_or_else(|| {
                error(
                    request_id,
                    GameplayReadDiagnosticCode::MissingPrefabRole,
                    format!("prefab role `{}` is unavailable", reference.role),
                )
            })?;
        let binding = self.prefab_instances.get(instance).ok_or_else(|| {
            error(
                request_id,
                GameplayReadDiagnosticCode::MissingPrefabInstance,
                format!("prefab instance {} is not indexed", instance.raw()),
            )
        })?;
        if binding.prefab.raw() != reference.prefab.raw() {
            return Err(error(
                request_id,
                GameplayReadDiagnosticCode::ForeignPrefabInstance,
                "prefab selector does not match the indexed instance prefab",
            ));
        }
        let entity = *binding.part_entities.get(&part).ok_or_else(|| {
            error(
                request_id,
                GameplayReadDiagnosticCode::MissingPrefabRole,
                "resolved prefab part has no runtime entity binding",
            )
        })?;
        self.require_live_entity(entity, request_id)?;
        Ok(GameplayReadValue::PrefabPart {
            instance: instance.raw(),
            prefab: reference.prefab.raw(),
            role: reference.role.clone(),
            part: part.raw(),
            entity: entity.raw(),
        })
    }

    fn resolve_owner_query(
        &self,
        query: &GameplayOwnerQuery,
        event: &GameplayEventEnvelope,
        request_id: &str,
    ) -> Result<GameplayResolvedOwnerQuery, GameplayReadAssemblyError> {
        match query {
            GameplayOwnerQuery::NearbyEntities {
                anchor,
                radius_millimeters,
                required_tags,
                max_items,
            } => Ok(GameplayResolvedOwnerQuery::NearbyEntities {
                anchor: self.resolve_entity(*anchor, event, request_id)?,
                radius_millimeters: *radius_millimeters,
                required_tags: required_tags.clone(),
                max_items: *max_items,
            }),
            GameplayOwnerQuery::LineOfSight { source, target } => {
                Ok(GameplayResolvedOwnerQuery::LineOfSight {
                    source: self.resolve_entity(*source, event, request_id)?,
                    target: self.resolve_entity(*target, event, request_id)?,
                })
            }
            GameplayOwnerQuery::PathBetween {
                source,
                target,
                max_steps,
            } => Ok(GameplayResolvedOwnerQuery::PathBetween {
                source: self.resolve_entity(*source, event, request_id)?,
                target: self.resolve_entity(*target, event, request_id)?,
                max_steps: *max_steps,
            }),
            GameplayOwnerQuery::CurrentTriggerOverlaps { trigger, max_items } => {
                Ok(GameplayResolvedOwnerQuery::CurrentTriggerOverlaps {
                    trigger: self.resolve_entity(*trigger, event, request_id)?,
                    max_items: *max_items,
                })
            }
        }
    }

    fn validate_owner_query_result(
        &self,
        request: &GameplayResolvedOwnerQuery,
        result: &mut GameplayOwnerQueryResult,
        request_id: &str,
    ) -> Result<(), GameplayReadAssemblyError> {
        match (request, result) {
            (
                GameplayResolvedOwnerQuery::NearbyEntities { .. },
                GameplayOwnerQueryResult::NearbyEntities { entities, .. },
            ) => {
                entities.sort_unstable();
                entities.dedup();
                for raw in entities {
                    self.require_live_entity(EntityId::new(*raw), request_id)?;
                }
            }
            (
                GameplayResolvedOwnerQuery::LineOfSight { .. },
                GameplayOwnerQueryResult::LineOfSight { blocker, .. },
            ) => {
                if let Some(raw) = blocker {
                    self.require_live_entity(EntityId::new(*raw), request_id)?;
                }
            }
            (
                GameplayResolvedOwnerQuery::PathBetween { .. },
                GameplayOwnerQueryResult::PathBetween { .. },
            ) => {}
            (
                GameplayResolvedOwnerQuery::CurrentTriggerOverlaps { trigger, .. },
                GameplayOwnerQueryResult::CurrentTriggerOverlaps {
                    trigger: returned,
                    subjects,
                    ..
                },
            ) => {
                if trigger.raw() != *returned {
                    return Err(error(
                        request_id,
                        GameplayReadDiagnosticCode::OwnerQueryRejected,
                        "trigger overlap provider returned a different trigger identity",
                    ));
                }
                subjects.sort_unstable();
                subjects.dedup();
                for subject in subjects {
                    self.require_live_entity(EntityId::new(*subject), request_id)?;
                }
            }
            _ => {
                return Err(error(
                    request_id,
                    GameplayReadDiagnosticCode::OwnerQueryRejected,
                    "owner query provider returned a receipt for another query kind",
                ));
            }
        }
        Ok(())
    }
}

struct ValidatedReadMetadata {
    provider_id: String,
    kind: GameplayReadViewKind,
    selectors: Vec<GameplayReadSelectorCapability>,
    fields: Vec<String>,
    max_items: u32,
    ordering: String,
    provider_hash: String,
}

fn selector_metadata(
    selector: &GameplayReadSelector,
) -> (
    GameplayReadViewKind,
    Vec<GameplayReadSelectorCapability>,
    u32,
) {
    match selector {
        GameplayReadSelector::EventIdentity { binding } => (
            GameplayReadViewKind::EventIdentity,
            vec![binding_capability(*binding)],
            1,
        ),
        GameplayReadSelector::Capability {
            binding,
            capability,
        } => (
            GameplayReadViewKind::EntityCapability,
            vec![
                binding_capability(*binding),
                capability_selector(*capability),
            ],
            1,
        ),
        GameplayReadSelector::Related {
            binding,
            relationship,
        } => (
            GameplayReadViewKind::Relationship,
            vec![
                binding_capability(*binding),
                match relationship {
                    GameplayRelationshipReadKind::TransformParent => {
                        GameplayReadSelectorCapability::TransformParent
                    }
                    GameplayRelationshipReadKind::Containment => {
                        GameplayReadSelectorCapability::Containment
                    }
                    GameplayRelationshipReadKind::SourceAncestry => {
                        GameplayReadSelectorCapability::SourceAncestry
                    }
                },
            ],
            1,
        ),
        GameplayReadSelector::PrefabPart { .. } => (
            GameplayReadViewKind::PrefabPart,
            vec![GameplayReadSelectorCapability::PrefabPartRole],
            1,
        ),
        GameplayReadSelector::Tags { max_items, .. } => (
            GameplayReadViewKind::Selection,
            vec![GameplayReadSelectorCapability::TagSelection],
            *max_items,
        ),
        GameplayReadSelector::Scope { max_items, .. } => (
            GameplayReadViewKind::Selection,
            vec![GameplayReadSelectorCapability::ScopeSelection],
            *max_items,
        ),
        GameplayReadSelector::ModuleNamed { .. } => (
            GameplayReadViewKind::ModuleNamed,
            vec![GameplayReadSelectorCapability::ModuleStateScope],
            1,
        ),
        GameplayReadSelector::OwnerQuery { query } => {
            let mut selectors = vec![GameplayReadSelectorCapability::OwnerQuery];
            let limit = match query {
                GameplayOwnerQuery::NearbyEntities {
                    anchor, max_items, ..
                } => {
                    selectors.push(binding_capability(*anchor));
                    *max_items
                }
                GameplayOwnerQuery::LineOfSight { source, target } => {
                    selectors.push(binding_capability(*source));
                    selectors.push(binding_capability(*target));
                    1
                }
                GameplayOwnerQuery::PathBetween {
                    source,
                    target,
                    max_steps,
                } => {
                    selectors.push(binding_capability(*source));
                    selectors.push(binding_capability(*target));
                    *max_steps
                }
                GameplayOwnerQuery::CurrentTriggerOverlaps { trigger, max_items } => {
                    selectors.push(binding_capability(*trigger));
                    *max_items
                }
            };
            selectors.sort();
            selectors.dedup();
            (GameplayReadViewKind::OwnerQuery, selectors, limit)
        }
    }
}

fn capability_selector(capability: GameplayCapabilityReadKind) -> GameplayReadSelectorCapability {
    match capability {
        GameplayCapabilityReadKind::Lifecycle => {
            GameplayReadSelectorCapability::LifecycleCapability
        }
        GameplayCapabilityReadKind::Transform => {
            GameplayReadSelectorCapability::TransformCapability
        }
        GameplayCapabilityReadKind::Collision => {
            GameplayReadSelectorCapability::CollisionCapability
        }
        GameplayCapabilityReadKind::Controller => {
            GameplayReadSelectorCapability::ControllerCapability
        }
    }
}

fn binding_capability(binding: GameplayEventEntityBinding) -> GameplayReadSelectorCapability {
    match binding {
        GameplayEventEntityBinding::Source => GameplayReadSelectorCapability::EventSource,
        GameplayEventEntityBinding::Subject { .. } => GameplayReadSelectorCapability::EventSubject,
        GameplayEventEntityBinding::Target { .. } => GameplayReadSelectorCapability::EventTarget,
        GameplayEventEntityBinding::Known(_) => GameplayReadSelectorCapability::KnownEntity,
    }
}

fn relationship_label(relationship: GameplayRelationshipReadKind) -> &'static str {
    match relationship {
        GameplayRelationshipReadKind::TransformParent => "transformParent",
        GameplayRelationshipReadKind::Containment => "containment",
        GameplayRelationshipReadKind::SourceAncestry => "sourceAncestry",
    }
}

fn value_item_count(value: &GameplayReadValue) -> usize {
    match value {
        GameplayReadValue::EntitySelection { entities } => entities.len(),
        GameplayReadValue::OwnerQuery { result } => result.item_count(),
        _ => 1,
    }
}

fn insert_if_requested(
    fields: &mut BTreeMap<String, GameplayScalarReadValue>,
    requested: &[String],
    name: &str,
    value: GameplayScalarReadValue,
) {
    if requested.iter().any(|field| field == name) {
        fields.insert(name.to_owned(), value);
    }
}

fn hash_serializable<T: Serialize>(value: &T) -> String {
    let bytes = serde_json::to_vec(value).expect("gameplay read evidence serializes");
    gameplay_module_payload_hash(&bytes)
}

fn error(
    request_id: impl Into<String>,
    code: GameplayReadDiagnosticCode,
    message: impl Into<String>,
) -> GameplayReadAssemblyError {
    GameplayReadAssemblyError {
        diagnostics: vec![GameplayReadDiagnostic {
            code,
            request_id: request_id.into(),
            message: message.into(),
        }],
    }
}
