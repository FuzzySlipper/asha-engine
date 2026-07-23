//! Typed border for durable project content edited before RuntimeSession start.
//!
//! The document union is intentionally closed. It reuses the existing entity,
//! catalog, prefab, gameplay-binding, trigger, and scene-reference contracts;
//! it is not a JSON value bus or an arbitrary property-path API.

#![forbid(unsafe_code)]

use core_ids::{SceneId, SceneNodeId};
use protocol_assets::StoredAssetCatalog;
use protocol_entity_authoring::EntityDefinition;
use protocol_game_extension::{
    GameplayContractRef, GameplayModuleBinding, GameplayModuleBindingOverride, GameplayModuleRef,
};
use protocol_project_bundle::{GameplayTriggerDefinition, PrefabRegistry};
use protocol_scene::{FlatSceneDocumentDto, SceneTransformDto};
use protocol_voxel_asset::{
    VoxelAssetAuthoringMetadata, VoxelAssetMaterialBinding, VoxelVolumeAsset,
};

pub const PROJECT_CONTENT_SCHEMA_VERSION: u32 = 1;
pub const AUTHORED_BEHAVIOR_PACKAGE_SCHEMA_VERSION: u32 = 1;
pub const AUTHORED_BEHAVIOR_VOCABULARY_VERSION: u32 = 1;
pub const AUTHORED_BEHAVIOR_VOCABULARY_HASH: &str =
    "asha.authored-behavior.v1:typed-semantic-refs;symbolic-state;direct-owner-verbs";
pub const AUTHORED_SIGNAL_PREFAB_PART_INTERACTED: &str = "asha.signal.prefab-part-interacted";
pub const AUTHORED_PREDICATE_STATE_IS: &str = "asha.predicate.state-is";
pub const AUTHORED_VERB_TRANSITION_STATE: &str = "asha.verb.transition-state";
pub const AUTHORED_VERB_SET_RELATIVE_TRANSLATION: &str = "asha.verb.set-relative-translation";
pub const AUTHORED_VERB_SET_CAPABILITY_ACTIVE: &str = "asha.verb.set-capability-active";
pub const AUTHORED_BEHAVIOR_MAX_MACHINES: u32 = 16;
pub const AUTHORED_BEHAVIOR_MAX_BEHAVIORS: u32 = 64;
pub const AUTHORED_BEHAVIOR_MAX_STATES_PER_MACHINE: u32 = 8;
pub const AUTHORED_BEHAVIOR_MAX_TRANSITIONS_PER_MACHINE: u32 = 16;
pub const AUTHORED_BEHAVIOR_MAX_STEPS_PER_BEHAVIOR: u32 = 8;
pub const AUTHORED_BEHAVIOR_MAX_OPERATIONS_PER_STEP: u32 = 8;
pub const AUTHORED_BEHAVIOR_MAX_ARGUMENTS: u32 = 8;
pub const AUTHORED_BEHAVIOR_MAX_DELAY_TICKS: u32 = 3_600;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectContentDocumentKind {
    EntityDefinition,
    AssetCatalog,
    PrefabRegistry,
    GameplayConfiguration,
    PresentationCatalog,
    InputCatalog,
    BehaviorPackage,
}

/// Immutable provenance retained with a TypeScript-authored behavior package.
/// Rust validates these identities and includes them in canonical content;
/// none of them are executable module registration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthoredBehaviorProvenanceDto {
    pub sdk_id: String,
    pub sdk_version: u32,
    pub vocabulary_hash: String,
    /// Consumer package/module that owns the readable declaration.
    pub source_module: String,
    /// Stable project-relative TypeScript source path used in diagnostics.
    pub source_path: String,
    pub source_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthoredBehaviorStateDto {
    pub state_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthoredBehaviorTransitionDto {
    pub transition_id: String,
    pub from_state_id: String,
    pub to_state_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthoredBehaviorStateMachineDto {
    pub machine_id: String,
    /// Stable scene-instance identity. Runtime admission resolves this to the
    /// EntityStore entity created by canonical scene bootstrap.
    pub target_scene_instance_id: String,
    pub initial_state_id: String,
    pub states: Vec<AuthoredBehaviorStateDto>,
    pub transitions: Vec<AuthoredBehaviorTransitionDto>,
}

/// Open, versioned reference to one Rust-published authored meaning. Admission
/// resolves this reference against the closed Engine semantic catalog; runtime
/// never dispatches an arbitrary method name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthoredBehaviorSemanticRefDto {
    pub semantic_id: String,
    pub version: u32,
}

/// Typed values supplied to a Rust-published signal, predicate, or verb.
/// The selected semantic descriptor owns the exact argument names and types.
#[derive(Debug, Clone, PartialEq)]
pub enum AuthoredBehaviorValueDto {
    SceneEntity {
        scene_instance_id: String,
    },
    PrefabPart {
        scene_instance_id: String,
        role: String,
    },
    StateMachine {
        machine_id: String,
    },
    State {
        machine_id: String,
        state_id: String,
    },
    Text {
        value: String,
    },
    Boolean {
        value: bool,
    },
    Integer {
        value: i64,
    },
    Number {
        value: f64,
    },
    Vector3 {
        value: [f32; 3],
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct AuthoredBehaviorArgumentDto {
    pub name: String,
    pub value: AuthoredBehaviorValueDto,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AuthoredBehaviorSignalDto {
    pub signal: AuthoredBehaviorSemanticRefDto,
    pub arguments: Vec<AuthoredBehaviorArgumentDto>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AuthoredBehaviorConditionDto {
    pub predicate: AuthoredBehaviorSemanticRefDto,
    pub arguments: Vec<AuthoredBehaviorArgumentDto>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AuthoredBehaviorOperationDto {
    pub verb: AuthoredBehaviorSemanticRefDto,
    pub arguments: Vec<AuthoredBehaviorArgumentDto>,
}

/// One bounded, atomically executed operation group. Dependencies form a DAG;
/// delayed groups are persisted by the existing Rust scheduler.
#[derive(Debug, Clone, PartialEq)]
pub struct AuthoredBehaviorStepDto {
    pub step_id: String,
    pub after_step_ids: Vec<String>,
    pub delay_ticks: u32,
    pub operations: Vec<AuthoredBehaviorOperationDto>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AuthoredBehaviorDefinitionDto {
    pub behavior_id: String,
    pub signal: AuthoredBehaviorSignalDto,
    pub conditions: Vec<AuthoredBehaviorConditionDto>,
    pub steps: Vec<AuthoredBehaviorStepDto>,
}

/// First-version authored behavior vocabulary. This is intentionally a small
/// semantic family for signal-driven state transitions, not a universal graph.
#[derive(Debug, Clone, PartialEq)]
pub struct AuthoredBehaviorPackageDto {
    pub schema_version: u32,
    pub package_id: String,
    pub provenance: AuthoredBehaviorProvenanceDto,
    pub state_machines: Vec<AuthoredBehaviorStateMachineDto>,
    pub behaviors: Vec<AuthoredBehaviorDefinitionDto>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectContentSourceDto {
    /// Manifest-owned storage location. This is deliberately independent from
    /// the stable document identity declared by the artifact envelope.
    pub source_path: String,
    pub document_id: String,
    pub kind: ProjectContentDocumentKind,
    pub source_text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectConfigurationValueKind {
    Boolean,
    Integer,
    Number,
    String,
    Reference,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectContentReferenceKind {
    Asset,
    EntityDefinition,
    InstantiatedEntityDefinition,
    InstantiatedBoundedEntityDefinition,
    /// A bounded definition instantiated by the entry scene and classified as
    /// Player by the same built-in FPS domain semantics used at activation.
    EntrySceneFpsPlayerEntityDefinition,
    SceneInstance,
    Prefab,
    PrefabPart,
    PresentationResource,
}

/// One Rust-resolved target that is valid for a provider-owned reference
/// field. Studio consumes this catalog instead of reimplementing scene,
/// capability, or gameplay-domain eligibility rules.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectContentReferenceOptionDto {
    pub target_id: String,
    pub label: String,
}

/// Provider-owned field metadata. Engine validates the shape and references;
/// providers retain the semantic meaning and codec identity.
#[derive(Debug, Clone, PartialEq)]
pub struct ProjectConfigurationFieldDto {
    pub field_id: String,
    pub label: String,
    pub value_kind: ProjectConfigurationValueKind,
    pub required: bool,
    pub reference_kind: Option<ProjectContentReferenceKind>,
    pub integer_min: Option<i64>,
    pub integer_max: Option<i64>,
    pub number_min: Option<f64>,
    pub number_max: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProjectConfigurationSchemaDto {
    pub schema_id: String,
    pub module_id: String,
    pub provider_id: String,
    pub contract: GameplayContractRef,
    pub codec_id: String,
    pub fields: Vec<ProjectConfigurationFieldDto>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProjectConfigurationValueDto {
    Boolean {
        value: bool,
    },
    Integer {
        value: i64,
    },
    Number {
        value: f64,
    },
    String {
        value: String,
    },
    Reference {
        reference_kind: ProjectContentReferenceKind,
        target_id: String,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProjectConfigurationFieldValueDto {
    pub field_id: String,
    pub value: ProjectConfigurationValueDto,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProjectGameplayConfigurationDto {
    pub configuration_id: String,
    pub module: GameplayModuleRef,
    pub schema_id: String,
    pub values: Vec<ProjectConfigurationFieldValueDto>,
}

/// Human-authored gameplay selection. Canonical provider bytes and hashes are
/// derived by Rust and are deliberately absent from stored source.
#[derive(Debug, Clone, PartialEq)]
pub struct ProjectGameplayConfigurationDocumentDto {
    pub schema_version: u32,
    pub configurations: Vec<ProjectGameplayConfigurationDto>,
    pub bindings: Vec<GameplayModuleBinding>,
    pub overrides: Vec<GameplayModuleBindingOverride>,
    pub triggers: Vec<GameplayTriggerDefinition>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectPresentationResourceKind {
    AnimatedMesh,
    Audio,
    Particle,
    Font,
    Overlay,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectAnimatedMeshRuntimeFormat {
    Glb,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProjectAnimationClipDescriptorDto {
    pub id: String,
    pub name: Option<String>,
    pub duration_seconds: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectMeshMaterialSlotDto {
    pub slot: u16,
    pub material: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProjectMeshBoundsDescriptorDto {
    pub min: [f32; 3],
    pub max: [f32; 3],
}

/// Stored renderer-neutral animated-mesh metadata. Runtime projection maps this
/// contract into the live render descriptor only after project admission.
#[derive(Debug, Clone, PartialEq)]
pub struct ProjectAnimatedMeshDescriptorDto {
    pub asset: String,
    pub runtime_format: ProjectAnimatedMeshRuntimeFormat,
    pub content_hash: Option<String>,
    pub clips: Vec<ProjectAnimationClipDescriptorDto>,
    pub default_clip: Option<String>,
    pub material_slots: Vec<ProjectMeshMaterialSlotDto>,
    pub bounds: ProjectMeshBoundsDescriptorDto,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProjectPresentationResourceDto {
    pub resource_id: String,
    pub kind: ProjectPresentationResourceKind,
    pub asset_id: String,
    pub source_path: String,
    pub content_hash: String,
    pub license_path: Option<String>,
    /// Renderer-neutral descriptor required for animated-mesh resources and
    /// forbidden for every other resource kind.
    pub animated_mesh: Option<ProjectAnimatedMeshDescriptorDto>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProjectPresentationSignalDomain {
    Audio,
    Particle,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectPresentationSignalDto {
    pub domain: ProjectPresentationSignalDomain,
    pub signal_id: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProjectPresentationCueDto {
    Animation {
        cue_id: String,
        resource_id: String,
        clip_id: String,
        looped: bool,
        at_seconds: f32,
        signal: ProjectPresentationSignalDto,
    },
    Audio {
        cue_id: String,
        signal_id: String,
        resource_id: String,
        gain: f32,
    },
    Particle {
        cue_id: String,
        signal_id: String,
        resource_id: String,
        scale: f32,
    },
    Overlay {
        cue_id: String,
        resource_id: String,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProjectPresentationCatalogDto {
    pub schema_version: u32,
    pub resources: Vec<ProjectPresentationResourceDto>,
    pub cues: Vec<ProjectPresentationCueDto>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProjectContentDocumentDto {
    EntityDefinition {
        document_id: String,
        definition: EntityDefinition,
    },
    AssetCatalog {
        document_id: String,
        catalog: StoredAssetCatalog,
    },
    PrefabRegistry {
        document_id: String,
        registry: PrefabRegistry,
    },
    GameplayConfiguration {
        document_id: String,
        document: ProjectGameplayConfigurationDocumentDto,
    },
    PresentationCatalog {
        document_id: String,
        catalog: ProjectPresentationCatalogDto,
    },
    InputCatalog {
        document_id: String,
        catalog: protocol_input::ProjectInputCatalog,
    },
    BehaviorPackage {
        document_id: String,
        package: AuthoredBehaviorPackageDto,
    },
}

impl ProjectContentDocumentDto {
    pub fn document_id(&self) -> &str {
        match self {
            Self::EntityDefinition { document_id, .. }
            | Self::AssetCatalog { document_id, .. }
            | Self::PrefabRegistry { document_id, .. }
            | Self::GameplayConfiguration { document_id, .. }
            | Self::PresentationCatalog { document_id, .. }
            | Self::InputCatalog { document_id, .. }
            | Self::BehaviorPackage { document_id, .. } => document_id,
        }
    }

    pub fn kind(&self) -> ProjectContentDocumentKind {
        match self {
            Self::EntityDefinition { .. } => ProjectContentDocumentKind::EntityDefinition,
            Self::AssetCatalog { .. } => ProjectContentDocumentKind::AssetCatalog,
            Self::PrefabRegistry { .. } => ProjectContentDocumentKind::PrefabRegistry,
            Self::GameplayConfiguration { .. } => ProjectContentDocumentKind::GameplayConfiguration,
            Self::PresentationCatalog { .. } => ProjectContentDocumentKind::PresentationCatalog,
            Self::InputCatalog { .. } => ProjectContentDocumentKind::InputCatalog,
            Self::BehaviorPackage { .. } => ProjectContentDocumentKind::BehaviorPackage,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProjectContentDecodeRequestDto {
    pub sources: Vec<ProjectContentSourceDto>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProjectContentEncodeRequestDto {
    pub documents: Vec<ProjectContentDocumentDto>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectContentDiagnosticCode {
    InvalidJson,
    UnknownField,
    InvalidField,
    DuplicateDocument,
    InvalidDocument,
    UnknownReference,
    ReferenceKindMismatch,
    StaleRevision,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectContentDiagnosticDto {
    pub code: ProjectContentDiagnosticCode,
    pub document_id: Option<String>,
    pub path: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectContentCanonicalFileDto {
    /// Retained manifest path when the file belongs to an opened authoring
    /// project. Pure document encoding has no storage ownership and returns
    /// `None` here.
    pub source_path: Option<String>,
    pub document_id: String,
    pub kind: ProjectContentDocumentKind,
    pub canonical_json: String,
    pub content_hash: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProjectContentFieldMetadataDto {
    pub document_id: String,
    /// Stable Rust-owned field identity within `schema_id`. Studio uses this
    /// instead of deriving mutation semantics from JSON path spelling.
    pub field_id: String,
    pub path: String,
    pub label: String,
    pub value_kind: ProjectConfigurationValueKind,
    pub required: bool,
    pub editable: bool,
    pub reference_kind: Option<ProjectContentReferenceKind>,
    pub reference_options: Vec<ProjectContentReferenceOptionDto>,
    pub configuration_id: Option<String>,
    pub schema_id: Option<String>,
    pub module_id: Option<String>,
    pub provider_id: Option<String>,
    pub contract: Option<GameplayContractRef>,
    pub codec_id: Option<String>,
    pub integer_min: Option<i64>,
    pub integer_max: Option<i64>,
    pub number_min: Option<f64>,
    pub number_max: Option<f64>,
}

/// Closed typed edits for the canonical entity-appearance binding. Rust owns
/// compatible-resource selection, dependent clip normalization, and numeric
/// bounds; authoring clients only select one generated operation.
#[derive(Debug, Clone, PartialEq)]
pub enum ProjectEntityAppearanceUpdateDto {
    Resource { resource_id: String },
    InitialClip { initial_clip_id: Option<String> },
    ModelScale { axis: u8, value: f32 },
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProjectContentCodecResultDto {
    pub accepted: bool,
    pub documents: Vec<ProjectContentDocumentDto>,
    pub canonical_files: Vec<ProjectContentCanonicalFileDto>,
    pub set_hash: Option<String>,
    /// Read-only catalog derived from the statically composed Rust providers.
    /// Requests never supply or amend these schemas.
    pub provider_schemas: Vec<ProjectConfigurationSchemaDto>,
    pub field_metadata: Vec<ProjectContentFieldMetadataDto>,
    pub diagnostics: Vec<ProjectContentDiagnosticDto>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveRuntimeProjectDomainKind {
    Fps,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveRuntimeProjectEntityRole {
    Player,
    Enemy,
    Neutral,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveRuntimeProjectEntityRoleReadoutDto {
    pub entity: u64,
    pub role: ActiveRuntimeProjectEntityRole,
}

/// Rust-owned status for one statically installed gameplay domain. Entity roles
/// are resolved by that domain's adapter and are projection facts, not TS
/// inference or a caller-supplied bootstrap registry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveRuntimeProjectDomainReadoutDto {
    pub kind: ActiveRuntimeProjectDomainKind,
    pub entity_roles: Vec<ActiveRuntimeProjectEntityRoleReadoutDto>,
}

/// Rust-owned projection of the canonical content and entry scene currently
/// backing one active RuntimeSession. This is read-only accepted state, not a
/// second authoring workspace or a caller-replayable bootstrap request.
#[derive(Debug, Clone, PartialEq)]
pub struct ActiveRuntimeProjectContentReadoutDto {
    pub project_id: u64,
    pub manifest_hash: String,
    pub content_set_hash: String,
    pub entry_scene: FlatSceneDocumentDto,
    pub content: ProjectContentCodecResultDto,
    pub active_domains: Vec<ActiveRuntimeProjectDomainReadoutDto>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProjectContentAuthoringCommandDto {
    Upsert {
        /// Explicit storage location for a newly inserted document or a typed
        /// relocation of an existing document. Never inferred from document id.
        source_path: String,
        document: ProjectContentDocumentDto,
    },
    Delete {
        document_id: String,
        document_kind: ProjectContentDocumentKind,
    },
    UpdateEntityAppearance {
        document_id: String,
        projection_id: String,
        update: ProjectEntityAppearanceUpdateDto,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProjectContentAuthoringRequestDto {
    pub expected_workspace_id: String,
    pub expected_generation: u64,
    pub expected_working_revision: u64,
    pub expected_set_hash: String,
    pub command: ProjectContentAuthoringCommandDto,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProjectContentAuthoringResultDto {
    pub accepted: bool,
    pub documents: Vec<ProjectContentDocumentDto>,
    pub canonical_files: Vec<ProjectContentCanonicalFileDto>,
    pub set_hash: Option<String>,
    /// Read-only catalog derived from the statically composed Rust providers.
    pub provider_schemas: Vec<ProjectConfigurationSchemaDto>,
    pub field_metadata: Vec<ProjectContentFieldMetadataDto>,
    pub diagnostics: Vec<ProjectContentDiagnosticDto>,
}

/// Caller-selected bounds for one procedural materialization request. Rust
/// applies stricter provider limits when they are lower than these values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProceduralEnvironmentLimitsDto {
    pub max_voxels: u64,
    pub max_sparse_runs: u64,
    pub max_markers: u64,
}

/// Deterministic mapping from one provider marker to one stored scene marker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProceduralEnvironmentMarkerTargetDto {
    pub source_marker_id: String,
    pub node_id: SceneNodeId,
    pub marker_id: String,
    pub child_order: u32,
}

/// Explicit stored artifact identities and placement for materialization.
#[derive(Debug, Clone, PartialEq)]
pub struct ProceduralEnvironmentTargetDto {
    pub scene_id: SceneId,
    pub scene_path: String,
    pub asset_id: String,
    pub asset_path: String,
    pub voxel_node_id: SceneNodeId,
    pub voxel_parent_id: Option<SceneNodeId>,
    pub voxel_child_order: u32,
    pub voxel_label: Option<String>,
    pub voxel_transform: SceneTransformDto,
    pub marker_targets: Vec<ProceduralEnvironmentMarkerTargetDto>,
}

/// Pure preview request bound to one Rust workspace revision and one
/// Engine-owned canonical scene.
#[derive(Debug, Clone, PartialEq)]
pub struct ProceduralEnvironmentPreviewRequestDto {
    pub expected_workspace_id: String,
    pub expected_generation: u64,
    pub expected_working_revision: u64,
    pub expected_scene_content_hash: String,
    pub provider_id: String,
    pub preset_id: String,
    pub seed: u64,
    pub target: ProceduralEnvironmentTargetDto,
    pub material_palette: Vec<VoxelAssetMaterialBinding>,
    pub authoring: VoxelAssetAuthoringMetadata,
    pub limits: ProceduralEnvironmentLimitsDto,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProceduralEnvironmentDiagnosticCode {
    MissingScene,
    StaleScene,
    UnknownProvider,
    UnknownPreset,
    RecipeMismatch,
    InvalidTarget,
    LimitExceeded,
    InvalidGeneratedAsset,
    InvalidGeneratedScene,
    StaleCandidate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProceduralEnvironmentDiagnosticDto {
    pub code: ProceduralEnvironmentDiagnosticCode,
    pub path: String,
    pub message: String,
}

/// Durable recipe and generated-output identity retained with the artifacts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProceduralEnvironmentProvenanceDto {
    pub provider_id: String,
    pub provider_version: u32,
    pub preset_id: String,
    pub seed: u64,
    pub config_hash: String,
    pub output_hash: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProceduralEnvironmentMarkerReadoutDto {
    pub source_marker_id: String,
    pub marker_id: String,
    pub node_id: SceneNodeId,
    pub local_position: [f32; 3],
    pub yaw_degrees: i32,
}

/// Renderer-neutral and simulation-neutral derivation evidence. These hashes
/// identify the exact saved voxel source used to build both consumers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProceduralEnvironmentSourceReadoutDto {
    pub voxel_data_hash: String,
    pub collision_source_hash: String,
    pub navigation_source_hash: String,
    pub solid_voxel_count: u64,
    pub walkable_voxel_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProceduralEnvironmentCanonicalFileDto {
    pub path: String,
    pub media_type: String,
    pub canonical_json: String,
    pub content_hash: String,
}

/// Complete immutable candidate owned by Rust between preview and apply.
#[derive(Debug, Clone, PartialEq)]
pub struct ProceduralEnvironmentArtifactCandidateDto {
    pub candidate_hash: String,
    pub scene_file: ProceduralEnvironmentCanonicalFileDto,
    pub voxel_file: ProceduralEnvironmentCanonicalFileDto,
    pub artifact_set_hash: String,
    pub scene: FlatSceneDocumentDto,
    pub asset: VoxelVolumeAsset,
    pub provenance: ProceduralEnvironmentProvenanceDto,
    pub markers: Vec<ProceduralEnvironmentMarkerReadoutDto>,
    pub sources: ProceduralEnvironmentSourceReadoutDto,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProceduralEnvironmentPreviewResultDto {
    pub accepted: bool,
    pub candidate: Option<ProceduralEnvironmentArtifactCandidateDto>,
    pub preview_frame: Option<protocol_render::RenderFrameDiff>,
    pub preview_projection_hash: Option<String>,
    pub preview_diff_count: u64,
    pub diagnostics: Vec<ProceduralEnvironmentDiagnosticDto>,
}

/// Apply consumes the Engine-owned candidate by identity. Artifact bytes are
/// deliberately absent so callers cannot substitute a different valid set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProceduralEnvironmentApplyRequestDto {
    pub expected_workspace_id: String,
    pub expected_generation: u64,
    pub expected_working_revision: u64,
    pub candidate_hash: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProceduralEnvironmentApplyResultDto {
    pub accepted: bool,
    pub working_revision: u64,
    pub save_candidate_hash: Option<String>,
    pub candidate: Option<ProceduralEnvironmentArtifactCandidateDto>,
    pub diagnostics: Vec<ProceduralEnvironmentDiagnosticDto>,
}
