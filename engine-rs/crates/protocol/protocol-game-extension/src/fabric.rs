//! Successor gameplay-fabric contracts for statically composed Rust modules.
//!
//! These types describe immutable Session topology and diagnostic projections.
//! They do not register TypeScript callbacks, dispatch handlers, or grant
//! mutation authority.

use core_ids::{EntityId, PrefabId, PrefabInstanceId};
use protocol_diagnostics::DiagnosticSeverity;
pub use protocol_project_bundle::PrefabPartReference;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

pub const GAMEPLAY_INVOCATION_FAMILIES: &[&str] = &["observe", "guard", "transform", "react"];

pub const GAMEPLAY_EVENT_PHASES: &[&str] = &["postCommit", "decisionMoment", "scheduledMoment"];

pub const GAMEPLAY_READ_VIEW_KINDS: &[&str] = &[
    "eventIdentity",
    "entityCapability",
    "moduleNamed",
    "relationship",
    "prefabPart",
    "selection",
    "ownerQuery",
];

pub const GAMEPLAY_READ_SELECTOR_CAPABILITIES: &[&str] = &[
    "eventSource",
    "eventSubject",
    "eventTarget",
    "knownEntity",
    "lifecycleCapability",
    "transformCapability",
    "collisionCapability",
    "controllerCapability",
    "transformParent",
    "containment",
    "sourceAncestry",
    "prefabPartRole",
    "tagSelection",
    "scopeSelection",
    "moduleStateScope",
    "ownerQuery",
];

pub const GAMEPLAY_REGISTRY_DIAGNOSTIC_CODES: &[&str] = &[
    "invalidIdentifier",
    "invalidNamespace",
    "overlappingNamespace",
    "duplicateModule",
    "duplicateProvider",
    "missingProvider",
    "providerManifestMismatch",
    "foreignNamespaceWrite",
    "duplicateEventKind",
    "schemaHashMismatch",
    "missingCodec",
    "duplicateCodec",
    "unknownSubscription",
    "duplicateSubscription",
    "missingInvocation",
    "invalidSubscriptionInvocation",
    "duplicateInvocation",
    "invalidBudget",
    "missingProposalOwner",
    "multipleProposalOwners",
    "proposalOwnerMismatch",
    "missingReadViewProvider",
    "multipleReadViewProviders",
    "readViewProviderMismatch",
    "readViewKindMismatch",
    "missingReadViewSelector",
    "missingReadViewField",
    "missingStateOwner",
    "multipleStateOwners",
    "stateOwnerMismatch",
    "unknownOrderingTarget",
    "orderingCycle",
];

pub const GAMEPLAY_MODULE_BINDING_SCHEMA_VERSION: u32 = 1;

pub const GAMEPLAY_MODULE_BINDING_DIAGNOSTIC_CODES: &[&str] = &[
    "invalidRegistryHash",
    "duplicateConfiguration",
    "duplicateBinding",
    "unknownConfiguration",
    "moduleMismatch",
    "providerMismatch",
    "configurationSchemaMismatch",
    "configurationCodecMismatch",
    "stateSchemaMismatch",
    "readContractMismatch",
    "outputContractMismatch",
    "unresolvedTarget",
    "ineligibleTarget",
    "invalidOverride",
    "duplicateStateScope",
    "stateInitializationRejected",
    "snapshotMismatch",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GameplayInvocationFamily {
    Observe,
    Guard,
    Transform,
    React,
}

impl GameplayInvocationFamily {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Observe => "observe",
            Self::Guard => "guard",
            Self::Transform => "transform",
            Self::React => "react",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GameplayEventPhase {
    PostCommit,
    DecisionMoment,
    ScheduledMoment,
}

impl GameplayEventPhase {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PostCommit => "postCommit",
            Self::DecisionMoment => "decisionMoment",
            Self::ScheduledMoment => "scheduledMoment",
        }
    }
}

/// Open, immutable namespaced contract identity. New downstream meanings do
/// not extend an engine enum; they add another validated value of this shape.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameplayContractRef {
    pub namespace: String,
    pub name: String,
    pub version: u32,
    pub schema_hash: String,
}

impl GameplayContractRef {
    pub fn key(&self) -> String {
        format!("{}.{}.v{}", self.namespace, self.name, self.version)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameplayModuleRef {
    pub module_id: String,
    pub namespace: String,
    pub version: String,
    pub sdk_hash: String,
    pub contract_hash: String,
    pub artifact_hash: String,
    pub provider_id: String,
}

/// Durable authored configuration bytes. These bytes seed module state once;
/// they are not live gameplay authority after activation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameplayModuleConfiguration {
    pub configuration_id: String,
    pub module: GameplayModuleRef,
    pub configuration: GameplayContractRef,
    pub codec_id: String,
    pub canonical_config: Vec<u8>,
    pub config_hash: String,
}

/// Stable authored targets. Prefab part bindings deliberately share the
/// `{prefab, role}` selector used by declared reads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum GameplayModuleBindingTarget {
    Session,
    EntityDefinition {
        stable_id: String,
    },
    Prefab {
        #[serde(
            serialize_with = "serialize_prefab_id",
            deserialize_with = "deserialize_prefab_id"
        )]
        prefab: PrefabId,
    },
    PrefabPart {
        part: PrefabPartReference,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameplayModuleBinding {
    pub binding_id: String,
    pub module_id: String,
    pub configuration_id: String,
    pub state_schema: GameplayContractRef,
    pub target: GameplayModuleBindingTarget,
    pub required_reads: Vec<GameplayReadViewRequirement>,
    pub output_contracts: Vec<GameplayContractRef>,
    pub enabled: bool,
}

/// A prefab-instance layer may replace configuration and/or eligibility for one
/// stored binding without mutating the prefab definition or base binding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameplayModuleBindingOverride {
    pub binding_id: String,
    #[serde(
        serialize_with = "serialize_prefab_instance_id",
        deserialize_with = "deserialize_prefab_instance_id"
    )]
    pub prefab_instance: PrefabInstanceId,
    pub configuration_id: Option<String>,
    pub enabled: Option<bool>,
}

fn serialize_prefab_id<S>(id: &PrefabId, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_u64(id.raw())
}

fn deserialize_prefab_id<'de, D>(deserializer: D) -> Result<PrefabId, D::Error>
where
    D: Deserializer<'de>,
{
    u64::deserialize(deserializer).map(PrefabId::new)
}

fn serialize_prefab_instance_id<S>(id: &PrefabInstanceId, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_u64(id.raw())
}

fn deserialize_prefab_instance_id<'de, D>(deserializer: D) -> Result<PrefabInstanceId, D::Error>
where
    D: Deserializer<'de>,
{
    u64::deserialize(deserializer).map(PrefabInstanceId::new)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameplayModuleBindingRegistry {
    pub schema_version: u32,
    pub configurations: Vec<GameplayModuleConfiguration>,
    pub bindings: Vec<GameplayModuleBinding>,
    pub overrides: Vec<GameplayModuleBindingOverride>,
    pub registry_hash: String,
}

/// Selects how authored content is matched to the statically linked gameplay
/// composition. Compatible is the normal product-load policy; Exact is an
/// explicit replay/certification/deployment pin.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GameplayCompositionLoadMode {
    #[default]
    Compatible,
    Exact,
}

impl GameplayCompositionLoadMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Compatible => "compatible",
            Self::Exact => "exact",
        }
    }
}

/// Authored compatibility expectation carried by a ProjectBundle load.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameplayCompositionRequirement {
    pub load_mode: GameplayCompositionLoadMode,
    pub semantic_compatibility_digest: String,
    pub artifact_provenance_digest: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GameplayCompositionDiagnosticCode {
    LegacyCompatibilityDefaulted,
    SemanticCompatibilityMismatch,
    ArtifactProvenanceMismatch,
    MissingExactArtifactProvenance,
}

impl GameplayCompositionDiagnosticCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::LegacyCompatibilityDefaulted => "legacyCompatibilityDefaulted",
            Self::SemanticCompatibilityMismatch => "semanticCompatibilityMismatch",
            Self::ArtifactProvenanceMismatch => "artifactProvenanceMismatch",
            Self::MissingExactArtifactProvenance => "missingExactArtifactProvenance",
        }
    }
}

/// Public load/readout evidence. Error-severity diagnostics reject before
/// activation; warning-severity diagnostics remain visible after activation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameplayCompositionDiagnostic {
    pub code: GameplayCompositionDiagnosticCode,
    pub severity: DiagnosticSeverity,
    pub path: String,
    pub expected: Option<String>,
    pub actual: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GameplayModuleBindingDiagnosticCode {
    InvalidRegistryHash,
    DuplicateConfiguration,
    DuplicateBinding,
    UnknownConfiguration,
    ModuleMismatch,
    ProviderMismatch,
    ConfigurationSchemaMismatch,
    ConfigurationCodecMismatch,
    StateSchemaMismatch,
    ReadContractMismatch,
    OutputContractMismatch,
    UnresolvedTarget,
    IneligibleTarget,
    InvalidOverride,
    DuplicateStateScope,
    StateInitializationRejected,
    SnapshotMismatch,
}

impl GameplayModuleBindingDiagnosticCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InvalidRegistryHash => "invalidRegistryHash",
            Self::DuplicateConfiguration => "duplicateConfiguration",
            Self::DuplicateBinding => "duplicateBinding",
            Self::UnknownConfiguration => "unknownConfiguration",
            Self::ModuleMismatch => "moduleMismatch",
            Self::ProviderMismatch => "providerMismatch",
            Self::ConfigurationSchemaMismatch => "configurationSchemaMismatch",
            Self::ConfigurationCodecMismatch => "configurationCodecMismatch",
            Self::StateSchemaMismatch => "stateSchemaMismatch",
            Self::ReadContractMismatch => "readContractMismatch",
            Self::OutputContractMismatch => "outputContractMismatch",
            Self::UnresolvedTarget => "unresolvedTarget",
            Self::IneligibleTarget => "ineligibleTarget",
            Self::InvalidOverride => "invalidOverride",
            Self::DuplicateStateScope => "duplicateStateScope",
            Self::StateInitializationRejected => "stateInitializationRejected",
            Self::SnapshotMismatch => "snapshotMismatch",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameplayModuleBindingDiagnostic {
    pub code: GameplayModuleBindingDiagnosticCode,
    pub path: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameplayModuleBindingReadout {
    pub binding_id: String,
    pub module_id: String,
    pub configuration_id: String,
    pub target: GameplayModuleBindingTarget,
    pub resolved_scopes: Vec<String>,
    pub active: bool,
    pub provenance_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameplayModuleBindingActivationReceipt {
    pub binding_registry_hash: String,
    pub gameplay_registry_digest: String,
    pub semantic_compatibility_digest: String,
    pub artifact_provenance_digest: String,
    pub compatibility_diagnostics: Vec<GameplayCompositionDiagnostic>,
    pub readouts: Vec<GameplayModuleBindingReadout>,
    pub module_state_hash: String,
    pub receipt_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameplayOwnerRef {
    pub owner_id: String,
    pub provider_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameplayEventSchemaDeclaration {
    pub event: GameplayContractRef,
    pub codec_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameplayEntityRef {
    #[serde(
        serialize_with = "serialize_entity_id",
        deserialize_with = "deserialize_entity_id"
    )]
    pub entity: EntityId,
}

fn serialize_entity_id<S>(id: &EntityId, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_u64(id.raw())
}

fn deserialize_entity_id<'de, D>(deserializer: D) -> Result<EntityId, D::Error>
where
    D: Deserializer<'de>,
{
    u64::deserialize(deserializer).map(EntityId::new)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum GameplayEmitterRef {
    Owner { owner_id: String },
    Module { module_id: String },
    Scheduler { scheduler_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameplayCausationRef {
    pub root_id: String,
    pub parent_event_id: Option<String>,
    pub decision_id: Option<String>,
}

/// Immutable, type-erased queue/replay envelope. Module edges recover the
/// canonical payload through a registered Rust codec.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameplayEventEnvelope {
    pub event_id: String,
    pub event: GameplayContractRef,
    pub tick: u64,
    pub root_sequence: u64,
    pub wave: u32,
    pub event_sequence: u32,
    pub phase: GameplayEventPhase,
    pub emitter: GameplayEmitterRef,
    pub causation: GameplayCausationRef,
    pub source: Option<GameplayEntityRef>,
    pub subjects: Vec<GameplayEntityRef>,
    pub targets: Vec<GameplayEntityRef>,
    pub scope: Option<String>,
    pub tags: Vec<String>,
    pub canonical_payload: Vec<u8>,
    pub payload_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameplayHeaderSelector {
    pub source: Option<GameplayEntityRef>,
    pub target: Option<GameplayEntityRef>,
    pub scope: Option<String>,
    pub required_tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameplaySubscriptionDeclaration {
    pub subscription_id: String,
    pub event: GameplayContractRef,
    pub invocation_id: String,
    pub selector: GameplayHeaderSelector,
    pub max_deliveries_per_root: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameplayInvocationReadRequirement {
    pub request_id: String,
    pub view: GameplayContractRef,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameplayInvocationDescriptor {
    pub invocation_id: String,
    pub family: GameplayInvocationFamily,
    pub input_contract: GameplayContractRef,
    pub output_contract: GameplayContractRef,
    #[serde(default)]
    pub read_requirements: Vec<GameplayInvocationReadRequirement>,
    pub max_outputs: u32,
    pub max_payload_bytes: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameplayProposalDeclaration {
    pub proposal: GameplayContractRef,
    pub owner: GameplayOwnerRef,
}

/// Immutable pending proposal. The authority owner is resolved from the
/// registry rather than trusted from the emitting module.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameplayProposalEnvelope {
    pub proposal_id: String,
    pub proposal: GameplayContractRef,
    pub tick: u64,
    pub root_sequence: u64,
    pub wave: u32,
    pub proposal_sequence: u32,
    pub emitter: GameplayEmitterRef,
    pub causation: GameplayCausationRef,
    pub originating_event_id: Option<String>,
    pub source: Option<GameplayEntityRef>,
    pub targets: Vec<GameplayEntityRef>,
    pub canonical_payload: Vec<u8>,
    pub payload_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameplayReadViewRequirement {
    pub view: GameplayContractRef,
    pub provider_id: String,
    pub kind: GameplayReadViewKind,
    pub fields: Vec<String>,
    pub selector_capabilities: Vec<GameplayReadSelectorCapability>,
    pub max_items: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GameplayReadViewKind {
    EventIdentity,
    EntityCapability,
    ModuleNamed,
    Relationship,
    PrefabPart,
    Selection,
    OwnerQuery,
}

impl GameplayReadViewKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::EventIdentity => "eventIdentity",
            Self::EntityCapability => "entityCapability",
            Self::ModuleNamed => "moduleNamed",
            Self::Relationship => "relationship",
            Self::PrefabPart => "prefabPart",
            Self::Selection => "selection",
            Self::OwnerQuery => "ownerQuery",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GameplayReadSelectorCapability {
    EventSource,
    EventSubject,
    EventTarget,
    KnownEntity,
    LifecycleCapability,
    TransformCapability,
    CollisionCapability,
    ControllerCapability,
    TransformParent,
    Containment,
    SourceAncestry,
    PrefabPartRole,
    TagSelection,
    ScopeSelection,
    ModuleStateScope,
    OwnerQuery,
}

impl GameplayReadSelectorCapability {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::EventSource => "eventSource",
            Self::EventSubject => "eventSubject",
            Self::EventTarget => "eventTarget",
            Self::KnownEntity => "knownEntity",
            Self::LifecycleCapability => "lifecycleCapability",
            Self::TransformCapability => "transformCapability",
            Self::CollisionCapability => "collisionCapability",
            Self::ControllerCapability => "controllerCapability",
            Self::TransformParent => "transformParent",
            Self::Containment => "containment",
            Self::SourceAncestry => "sourceAncestry",
            Self::PrefabPartRole => "prefabPartRole",
            Self::TagSelection => "tagSelection",
            Self::ScopeSelection => "scopeSelection",
            Self::ModuleStateScope => "moduleStateScope",
            Self::OwnerQuery => "ownerQuery",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameplayOwnedSchemaDeclaration {
    pub schema: GameplayContractRef,
    pub owner: GameplayOwnerRef,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameplayOrderingConstraint {
    pub before_module: String,
    pub after_module: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameplayExecutionBudget {
    pub max_waves: u32,
    pub max_events_per_root: u32,
    pub max_proposals_per_root: u32,
    pub max_invocations_per_root: u32,
    pub max_payload_bytes_per_root: u32,
}

/// Successor to the legacy hook-shaped `GameRuleModuleManifest`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameplayModuleManifest {
    pub module_ref: GameplayModuleRef,
    pub published_events: Vec<GameplayEventSchemaDeclaration>,
    pub subscriptions: Vec<GameplaySubscriptionDeclaration>,
    pub invocations: Vec<GameplayInvocationDescriptor>,
    pub read_views: Vec<GameplayReadViewRequirement>,
    pub proposal_kinds: Vec<GameplayProposalDeclaration>,
    pub state_schemas: Vec<GameplayOwnedSchemaDeclaration>,
    pub fact_schemas: Vec<GameplayOwnedSchemaDeclaration>,
    pub ordering: Vec<GameplayOrderingConstraint>,
    pub budget: GameplayExecutionBudget,
    pub deterministic_requirements: Vec<String>,
    pub source_hash: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GameplayRegistryDiagnosticCode {
    InvalidIdentifier,
    InvalidNamespace,
    OverlappingNamespace,
    DuplicateModule,
    DuplicateProvider,
    MissingProvider,
    ProviderManifestMismatch,
    ForeignNamespaceWrite,
    DuplicateEventKind,
    SchemaHashMismatch,
    MissingCodec,
    DuplicateCodec,
    UnknownSubscription,
    DuplicateSubscription,
    MissingInvocation,
    InvalidSubscriptionInvocation,
    DuplicateInvocation,
    DuplicateInvocationRead,
    MissingInvocationReadView,
    InvalidBudget,
    MissingProposalOwner,
    MultipleProposalOwners,
    ProposalOwnerMismatch,
    MissingReadViewProvider,
    MultipleReadViewProviders,
    ReadViewProviderMismatch,
    ReadViewKindMismatch,
    MissingReadViewSelector,
    MissingReadViewField,
    MissingStateOwner,
    MultipleStateOwners,
    StateOwnerMismatch,
    UnknownOrderingTarget,
    OrderingCycle,
}

impl GameplayRegistryDiagnosticCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InvalidIdentifier => "invalidIdentifier",
            Self::InvalidNamespace => "invalidNamespace",
            Self::OverlappingNamespace => "overlappingNamespace",
            Self::DuplicateModule => "duplicateModule",
            Self::DuplicateProvider => "duplicateProvider",
            Self::MissingProvider => "missingProvider",
            Self::ProviderManifestMismatch => "providerManifestMismatch",
            Self::ForeignNamespaceWrite => "foreignNamespaceWrite",
            Self::DuplicateEventKind => "duplicateEventKind",
            Self::SchemaHashMismatch => "schemaHashMismatch",
            Self::MissingCodec => "missingCodec",
            Self::DuplicateCodec => "duplicateCodec",
            Self::UnknownSubscription => "unknownSubscription",
            Self::DuplicateSubscription => "duplicateSubscription",
            Self::MissingInvocation => "missingInvocation",
            Self::InvalidSubscriptionInvocation => "invalidSubscriptionInvocation",
            Self::DuplicateInvocation => "duplicateInvocation",
            Self::DuplicateInvocationRead => "duplicateInvocationRead",
            Self::MissingInvocationReadView => "missingInvocationReadView",
            Self::InvalidBudget => "invalidBudget",
            Self::MissingProposalOwner => "missingProposalOwner",
            Self::MultipleProposalOwners => "multipleProposalOwners",
            Self::ProposalOwnerMismatch => "proposalOwnerMismatch",
            Self::MissingReadViewProvider => "missingReadViewProvider",
            Self::MultipleReadViewProviders => "multipleReadViewProviders",
            Self::ReadViewProviderMismatch => "readViewProviderMismatch",
            Self::ReadViewKindMismatch => "readViewKindMismatch",
            Self::MissingReadViewSelector => "missingReadViewSelector",
            Self::MissingReadViewField => "missingReadViewField",
            Self::MissingStateOwner => "missingStateOwner",
            Self::MultipleStateOwners => "multipleStateOwners",
            Self::StateOwnerMismatch => "stateOwnerMismatch",
            Self::UnknownOrderingTarget => "unknownOrderingTarget",
            Self::OrderingCycle => "orderingCycle",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameplayRegistryDiagnostic {
    pub code: GameplayRegistryDiagnosticCode,
    pub severity: DiagnosticSeverity,
    pub path: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameplayTopologyEdge {
    pub kind: String,
    pub from: String,
    pub to: String,
    pub contract: Option<String>,
}

/// Projection/readout only. It explains the immutable Session topology but
/// exposes no registry mutation operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameplayRegistryReadout {
    pub registry_digest: String,
    pub semantic_compatibility_digest: String,
    pub artifact_provenance_digest: String,
    pub module_ids: Vec<String>,
    pub event_kinds: Vec<String>,
    pub subscription_ids: Vec<String>,
    pub proposal_owners: Vec<String>,
    pub read_view_providers: Vec<String>,
    pub read_view_provider_details: Vec<GameplayReadViewProviderReadout>,
    pub state_owners: Vec<String>,
    pub ordering: Vec<GameplayOrderingConstraint>,
    pub topology: Vec<GameplayTopologyEdge>,
    pub topology_dump: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameplayReadViewProviderReadout {
    pub view: String,
    pub provider_id: String,
    pub kind: GameplayReadViewKind,
    pub fields: Vec<String>,
    pub selector_capabilities: Vec<GameplayReadSelectorCapability>,
    pub max_items: u32,
    pub ordering: String,
    pub provider_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum GameplayRegistryValidationOutcome {
    Valid {
        readout: Box<GameplayRegistryReadout>,
    },
    Invalid {
        diagnostics: Vec<GameplayRegistryDiagnostic>,
    },
}
