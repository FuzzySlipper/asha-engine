//! Cross-boundary schema for project bundles (scene-capability-super, epic #2351,
//! subtask #2366).
//!
//! # Lane
//!
//! `contract-steward` — owns the border shape TypeScript devtools use to
//! **display** project-bundle manifests, ordered load plans, save/compaction
//! summaries, version-compatibility findings, and the regenerate-and-replay
//! generator diagnostic. Like `protocol-render`/`protocol-scene` it depends on
//! `core-ids` only and carries **no authority logic**: manifest validation, load
//! planning, save composition, and generator replay all stay in
//! `svc-serialization` and the project-bundle load/save rule lane. TS can read
//! these shapes; it cannot mutate bundle state.
//!
//! # Single home for stable vocabularies
//!
//! Every string a reader routes on — artifact class/role tags, load-stage
//! labels, manifest/load-plan error codes, generator suggested actions — has its
//! single home here as a `const` table plus a closed enum, with a test pinning
//! the two together. `protocol-codegen` sources the tables so the generated
//! TypeScript and Rust can never disagree.

#![forbid(unsafe_code)]

// ── Workspace authoring lifecycle ───────────────────────────────────────────

/// Stable project identity for one non-gameplay workspace-authoring cell.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkspaceAuthoringProjectIdentity {
    pub game_id: String,
    pub workspace_id: String,
}

/// Bounded ProjectBundle identity used to seed authoring without loading a
/// gameplay RuntimeSession.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkspaceAuthoringProjectBundleRef {
    pub bundle_schema_version: u32,
    pub protocol_version: u32,
    pub scene_id: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkspaceAuthoringCompositionStatus {
    pub loaded_project_bundle: Option<u64>,
    pub fatal_count: u32,
    pub total_count: u32,
    pub blocks_load: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkspaceAuthoringOpenRequest {
    pub authoring_id: String,
    pub seed: u64,
    pub project: WorkspaceAuthoringProjectIdentity,
    pub project_bundle: WorkspaceAuthoringProjectBundleRef,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkspaceAuthoringIdentity {
    pub kind: String,
    pub authoring_id: String,
    pub mode: String,
    pub generation: u64,
    pub seed: u64,
    pub project: WorkspaceAuthoringProjectIdentity,
    pub project_bundle: WorkspaceAuthoringProjectBundleRef,
    pub non_claims: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkspaceAuthoringStateSummary {
    pub kind: String,
    pub status: String,
    pub identity: WorkspaceAuthoringIdentity,
    pub composition: WorkspaceAuthoringCompositionStatus,
    pub working_revision: u64,
    pub stored_revision: u64,
    pub dirty: bool,
    pub last_stored_canonical_json_hash: Option<String>,
    pub authority_snapshot_hash: String,
    pub lifecycle_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkspaceAuthoringProjectionRequest {
    pub expected_workspace_id: String,
    pub expected_generation: u64,
    pub expected_working_revision: u64,
    pub cursor: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkspaceAuthoringProjectionReceipt {
    pub kind: String,
    pub workspace_id: String,
    pub generation: u64,
    pub working_revision: u64,
    pub cursor: u64,
    pub next_cursor: u64,
    pub delivery: String,
    /// Canonical render-bridge JSON decoded by the public transport facade.
    pub frame_json: String,
    pub render_diff_count: u64,
    pub projection_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkspaceAuthoringStoredConfirmationRequest {
    pub expected_workspace_id: String,
    pub expected_generation: u64,
    pub host_path: String,
    pub canonical_json_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkspaceAuthoringStoredConfirmationReceipt {
    pub kind: String,
    pub accepted: bool,
    pub workspace_id: String,
    pub generation: u64,
    pub host_path: String,
    pub canonical_json_hash: String,
    pub stored_revision: u64,
    pub lifecycle_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkspaceAuthoringCloseRequest {
    pub expected_workspace_id: String,
    pub expected_generation: u64,
    #[serde(default)]
    pub discard_unsaved_working_state: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkspaceAuthoringCloseReceipt {
    pub kind: String,
    pub closed: bool,
    pub workspace_id: String,
    pub generation: u64,
    pub discarded_unsaved_working_state: bool,
    pub lifecycle_hash: String,
}

// ── Artifact classification ───────────────────────────────────────────────────

/// Stable on-disk discriminant for each artifact class. Mirrors
/// `svc_serialization::ArtifactClass::tag`.
pub const ARTIFACT_CLASSES: &[&str] = &["durable", "generated", "cache"];

/// What an artifact's persistence guarantee is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtifactClass {
    Durable,
    Generated,
    Cache,
}

impl ArtifactClass {
    pub fn as_str(self) -> &'static str {
        match self {
            ArtifactClass::Durable => "durable",
            ArtifactClass::Generated => "generated",
            ArtifactClass::Cache => "cache",
        }
    }
}

/// Every [`ArtifactClass`] in declaration order.
pub const ALL_ARTIFACT_CLASSES: &[ArtifactClass] = &[
    ArtifactClass::Durable,
    ArtifactClass::Generated,
    ArtifactClass::Cache,
];

/// The artifact roles this build names. The wire role is an open string (unknown
/// roles are carried verbatim by `svc_serialization::ArtifactRole::Other`), so
/// the border types the field as `string`; this table is the *known* vocabulary
/// for routing/display. Mirrors `svc_serialization::ArtifactRole::tag`.
pub const KNOWN_ARTIFACT_ROLES: &[&str] = &[
    "sceneDocument",
    "assetLock",
    "prefabRegistry",
    "projectContent",
    "entityDefinitionCatalog",
    "materialCatalog",
    "voxelVolumeAsset",
    "sessionStateSnapshot",
    "voxelChunkSnapshot",
    "voxelEditLog",
    "voxelEditHistory",
    "voxelAnnotationLayer",
    "replayRecord",
    "generatedMetadata",
    "cache",
];

// ── Load plan ─────────────────────────────────────────────────────────────────

/// Stable label for each ordered load stage. Mirrors
/// `svc_serialization::LoadStage::label`; order matters (authority load order).
pub const LOAD_STAGES: &[&str] = &[
    "versions",
    "assetLock",
    "sceneDocument",
    "terrainGeneration",
    "voxelEdits",
    "voxelAnnotations",
    "bootstrap",
    "sessionStateSnapshot",
    "finalValidation",
];

/// One ordered stage of an authority load.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadStage {
    Versions,
    AssetLock,
    SceneDocument,
    TerrainGeneration,
    VoxelEdits,
    /// Validate stored voxel annotation layers against their target voxel-volume
    /// artifacts before any consumer can reference semantic region ids.
    VoxelAnnotations,
    Bootstrap,
    /// Restore the runtime-diverged session-state snapshot over the bootstrapped
    /// scene baseline. Optional: present only when a save carried runtime
    /// divergence (#2484).
    SessionStateSnapshot,
    FinalValidation,
}

impl LoadStage {
    pub fn as_str(self) -> &'static str {
        match self {
            LoadStage::Versions => "versions",
            LoadStage::AssetLock => "assetLock",
            LoadStage::SceneDocument => "sceneDocument",
            LoadStage::TerrainGeneration => "terrainGeneration",
            LoadStage::VoxelEdits => "voxelEdits",
            LoadStage::VoxelAnnotations => "voxelAnnotations",
            LoadStage::Bootstrap => "bootstrap",
            LoadStage::SessionStateSnapshot => "sessionStateSnapshot",
            LoadStage::FinalValidation => "finalValidation",
        }
    }
}

/// Every [`LoadStage`] in canonical load order.
pub const ALL_LOAD_STAGES: &[LoadStage] = &[
    LoadStage::Versions,
    LoadStage::AssetLock,
    LoadStage::SceneDocument,
    LoadStage::TerrainGeneration,
    LoadStage::VoxelEdits,
    LoadStage::VoxelAnnotations,
    LoadStage::Bootstrap,
    LoadStage::SessionStateSnapshot,
    LoadStage::FinalValidation,
];

/// Stable discriminant for each load-step variant. Mirrors the `svc_serialization::LoadStep` enum.
pub const LOAD_STEP_KINDS: &[&str] = &[
    "validateVersions",
    "loadAssetLock",
    "loadSceneDocument",
    "generateTerrain",
    "applyVoxelEdits",
    "loadVoxelAnnotations",
    "bootstrapScene",
    "restoreSessionState",
    "validateFinalState",
];

// ── Error codes ───────────────────────────────────────────────────────────────

/// Stable manifest-validation error codes. Mirrors `svc_serialization::ManifestError`.
pub const MANIFEST_ERROR_CODES: &[&str] = &[
    "unsupportedSchema",
    "unsupportedProtocol",
    "duplicateArtifact",
    "missingArtifact",
    "durableMissingHash",
    "loadRequiredMissingHash",
    "invalidArtifactPath",
    "duplicateScene",
    "missingEntryScene",
    "sceneArtifactMismatch",
    "unreferencedSceneArtifact",
    "unknownArtifactRole",
    "duplicateArtifactRole",
    "artifactClassMismatch",
];

/// Stable load-plan error codes. Mirrors `svc_serialization::LoadPlanError`.
pub const LOAD_PLAN_ERROR_CODES: &[&str] = &[
    "manifest",
    "missingPrerequisiteArtifact",
    "outOfOrder",
    "missingStage",
];

// ── Generator diagnostic ──────────────────────────────────────────────────────

/// Stable suggested-action codes for an edit conflict. Mirrors
/// the project-bundle load/save rule lane's suggested-action label.
pub const SUGGESTED_ACTIONS: &[&str] = &["keepEdit", "reviewConflict"];

/// What to do about an edit whose generated context changed under a new generator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuggestedAction {
    KeepEdit,
    ReviewConflict,
}

impl SuggestedAction {
    pub fn as_str(self) -> &'static str {
        match self {
            SuggestedAction::KeepEdit => "keepEdit",
            SuggestedAction::ReviewConflict => "reviewConflict",
        }
    }
}

/// Every [`SuggestedAction`] in declaration order.
pub const ALL_SUGGESTED_ACTIONS: &[SuggestedAction] =
    &[SuggestedAction::KeepEdit, SuggestedAction::ReviewConflict];

// The DTOs below are the source-owned project-bundle border. Authority services
// may use richer internal types, but generated consumers derive only from these
// inert declarations.

use core_ids::{PrefabId, PrefabInstanceId, PrefabPartId, ProjectId, RuntimeSessionId, SceneId};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Durable schema for semantic trigger roles authored with a ProjectBundle.
pub const GAMEPLAY_TRIGGER_DEFINITION_SCHEMA_VERSION: u32 = 2;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GameplayTriggerDefinition {
    pub schema_version: u32,
    /// Stable authored SceneEntityInstance identity. Runtime EntityId allocation
    /// is resolved from the validated scene bootstrap record.
    pub scene_instance_id: String,
    pub scope: String,
    pub tags: Vec<String>,
}

/// Project-bundle border form of a voxel coordinate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProjectBundleVoxelCoord {
    pub x: i64,
    pub y: i64,
    pub z: i64,
}

/// Project-bundle border form of a voxel value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectBundleVoxelValue {
    Empty,
    Solid { material: u16 },
}

/// One row of the manifest artifact table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactEntry {
    pub path: String,
    pub class: ArtifactClass,
    pub role: String,
    pub content_hash: Option<String>,
}

/// Optional authoring-only procedural generation provenance. Runtime admission
/// consumes the materialized scene and resource artifacts, never this provider.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratorMetadata {
    pub provider: String,
    pub seed: u64,
    pub version: u32,
    pub params: String,
}

/// Project identity in a bundle manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectSection {
    pub id: ProjectId,
    pub name: Option<String>,
}

/// Scene identity and artifact location in a bundle manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SceneSection {
    pub id: SceneId,
    pub schema_version: u32,
    pub artifact: String,
}

/// Asset-lock artifact metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetLockSection {
    pub artifact: String,
    pub asset_count: u32,
}

/// Inspectable project-bundle manifest border.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectBundleManifest {
    pub bundle_schema_version: u32,
    pub protocol_version: u32,
    pub project: ProjectSection,
    pub entry_scene: SceneId,
    pub scenes: Vec<SceneSection>,
    pub asset_lock: AssetLockSection,
    pub generation_provenance: Option<GeneratorMetadata>,
    pub artifacts: Vec<ArtifactEntry>,
}

// ── Canonical project source batch ─────────────────────────────────────────

/// Stable classified errors for manifest-owned source batch admission.
pub const PROJECT_SOURCE_BATCH_ERROR_CODES: &[&str] = &[
    "manifestTooLarge",
    "manifestDecodeFailed",
    "manifestInvalid",
    "tooManyBodies",
    "duplicateBody",
    "duplicateResourceHandle",
    "missingBody",
    "extraBody",
    "inlineBodyTooLarge",
    "inlineBodyForbidden",
    "inlineQuotaExceeded",
    "resourceBodyTooLarge",
    "resourceQuotaExceeded",
    "unknownResourceHandle",
    "resourceGenerationMismatch",
    "resourceVersionMismatch",
    "resourceLengthMismatch",
    "resourceManifestMismatch",
    "resourcePathMismatch",
    "contentHashMismatch",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ProjectSourceBatchErrorCode {
    ManifestTooLarge,
    ManifestDecodeFailed,
    ManifestInvalid,
    TooManyBodies,
    DuplicateBody,
    DuplicateResourceHandle,
    MissingBody,
    ExtraBody,
    InlineBodyTooLarge,
    InlineBodyForbidden,
    InlineQuotaExceeded,
    ResourceBodyTooLarge,
    ResourceQuotaExceeded,
    UnknownResourceHandle,
    ResourceGenerationMismatch,
    ResourceVersionMismatch,
    ResourceLengthMismatch,
    ResourceManifestMismatch,
    ResourcePathMismatch,
    ContentHashMismatch,
}

/// Opaque staged-resource identity. It deliberately carries no role, kind, or
/// content hash; the manifest remains the only owner of those facts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StagedProjectResourceRef {
    pub handle: u64,
    pub generation: u64,
    pub version: u32,
    pub byte_len: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectResourceBeginRequest {
    pub manifest_json: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectResourceTransactionReceipt {
    pub generation: u64,
    pub manifest_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectResourceStageRequest {
    pub generation: u64,
    pub path: String,
    pub bytes: Vec<u8>,
}

/// One manifest-relative body supplied by a host adapter.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum ProjectSourceBody {
    Inline {
        path: String,
        bytes: Vec<u8>,
    },
    Resource {
        path: String,
        resource: StagedProjectResourceRef,
    },
}

/// Host-neutral raw batch consumed by Rust source-closure validation.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeProjectSourceBatch {
    pub manifest_json: String,
    pub resource_generation: Option<u64>,
    pub bodies: Vec<ProjectSourceBody>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectSourceBatchDiagnostic {
    pub code: ProjectSourceBatchErrorCode,
    pub path: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectSourceBatchValidationReceipt {
    pub accepted: bool,
    pub manifest_hash: Option<String>,
    pub paths: Vec<String>,
    pub diagnostics: Vec<ProjectSourceBatchDiagnostic>,
}

// ── Durable prefab registry ──────────────────────────────────────────────────

/// Schema version for the prefab registry artifact carried by a ProjectBundle.
pub const PREFAB_REGISTRY_SCHEMA_VERSION: u32 = 1;

/// Schema version for each stored prefab definition.
pub const PREFAB_DEFINITION_SCHEMA_VERSION: u32 = 1;

/// Stable classified prefab validation codes.
pub const PREFAB_DIAGNOSTIC_CODES: &[&str] = &[
    "unsupportedRegistrySchema",
    "unsupportedDefinitionSchema",
    "duplicatePrefabId",
    "missingDisplayName",
    "duplicatePartId",
    "invalidPartNamespace",
    "duplicatePartNamespace",
    "missingParentPart",
    "partHierarchyCycle",
    "invalidPartTransform",
    "unknownAsset",
    "assetKindMismatch",
    "unknownEntityDefinition",
    "invalidPartRole",
    "duplicatePartRole",
    "danglingPartRole",
    "missingBasePrefab",
    "variantCycle",
    "variantDepthExceeded",
    "variantDefinesParts",
    "unknownRemovedRole",
    "duplicateRemovedRole",
    "unsafePartRemoval",
    "invalidOverrideTarget",
    "duplicateOverride",
    "invalidOverrideValue",
    "deletedRoleReferenced",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrefabDiagnosticCode {
    UnsupportedRegistrySchema,
    UnsupportedDefinitionSchema,
    DuplicatePrefabId,
    MissingDisplayName,
    DuplicatePartId,
    InvalidPartNamespace,
    DuplicatePartNamespace,
    MissingParentPart,
    PartHierarchyCycle,
    InvalidPartTransform,
    UnknownAsset,
    AssetKindMismatch,
    UnknownEntityDefinition,
    InvalidPartRole,
    DuplicatePartRole,
    DanglingPartRole,
    MissingBasePrefab,
    VariantCycle,
    VariantDepthExceeded,
    VariantDefinesParts,
    UnknownRemovedRole,
    DuplicateRemovedRole,
    UnsafePartRemoval,
    InvalidOverrideTarget,
    DuplicateOverride,
    InvalidOverrideValue,
    DeletedRoleReferenced,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PrefabTransform {
    pub translation: [f32; 3],
    pub rotation: [f32; 4],
    pub scale: [f32; 3],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrefabPartSource {
    Scene { asset: String },
    EntityDefinition { stable_id: String },
    VoxelObject { asset: String },
}

#[derive(Debug, Clone, PartialEq)]
pub struct PrefabPart {
    pub id: PrefabPartId,
    pub namespace: String,
    pub display_name: String,
    pub parent: Option<PrefabPartId>,
    pub transform: PrefabTransform,
    pub source: PrefabPartSource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrefabPartRoleBinding {
    pub role: String,
    pub part: PrefabPartId,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PrefabOverrideValue {
    Transform { transform: PrefabTransform },
    EntityDefinition { stable_id: String },
    Asset { asset: String },
    Material { asset: String },
    Activation { active: bool },
}

#[derive(Debug, Clone, PartialEq)]
pub struct PrefabOverride {
    pub target_role: String,
    pub value: PrefabOverrideValue,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PrefabVariantDelta {
    /// Stable authored key used by scene documents and Studio. The enclosing
    /// definition's numeric `PrefabId` remains the runtime authority key.
    pub variant_id: String,
    pub base: PrefabId,
    pub removed_roles: Vec<String>,
    pub overrides: Vec<PrefabOverride>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PrefabDefinition {
    pub id: PrefabId,
    pub schema_version: u32,
    pub display_name: String,
    pub parts: Vec<PrefabPart>,
    pub part_roles: Vec<PrefabPartRoleBinding>,
    pub variant: Option<PrefabVariantDelta>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PrefabRegistry {
    pub schema_version: u32,
    pub definitions: Vec<PrefabDefinition>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PrefabInstanceRecord {
    pub instance: PrefabInstanceId,
    pub prefab: PrefabId,
    pub seed: u64,
    pub transform: PrefabTransform,
    pub overrides: Vec<PrefabOverride>,
}

/// Durable selector used by declared reads and authored module bindings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PrefabPartReference {
    #[serde(
        serialize_with = "serialize_prefab_id",
        deserialize_with = "deserialize_prefab_id"
    )]
    pub prefab: PrefabId,
    pub role: String,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrefabDiagnostic {
    pub code: PrefabDiagnosticCode,
    pub path: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrefabValidationOutcome {
    Valid,
    Invalid { diagnostics: Vec<PrefabDiagnostic> },
}

/// Classified manifest validation failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManifestError {
    UnsupportedSchema {
        found: u32,
        supported: u32,
    },
    UnsupportedProtocol {
        found: u32,
        supported: u32,
    },
    DuplicateArtifact {
        path: String,
    },
    MissingArtifact {
        role: String,
        path: String,
    },
    DurableMissingHash {
        path: String,
    },
    DuplicateArtifactRole {
        role: String,
    },
    ArtifactClassMismatch {
        path: String,
        expected: String,
        found: String,
    },
}

/// All manifest errors produced by one validation pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestValidationReport {
    pub errors: Vec<ManifestError>,
}

/// One ordered authority load-plan step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadStep {
    ValidateVersions {
        bundle_schema_version: u32,
        protocol_version: u32,
    },
    LoadAssetLock {
        artifact: String,
        asset_count: u32,
    },
    LoadSceneDocument {
        artifact: String,
        scene: SceneId,
    },
    GenerateTerrain {
        seed: u64,
        version: u32,
        params: String,
    },
    ApplyVoxelEdits {
        edit_logs: Vec<String>,
        snapshots: Vec<String>,
        histories: Vec<String>,
    },
    LoadVoxelAnnotations {
        artifacts: Vec<String>,
    },
    BootstrapScene {
        scene: SceneId,
        runtime_session: RuntimeSessionId,
    },
    RestoreSessionState {
        artifact: String,
    },
    ValidateFinalState,
}

/// Deterministic ordered authority load plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadPlan {
    pub steps: Vec<LoadStep>,
}

/// Why a load plan could not be built or verified.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadPlanError {
    Manifest { error: ManifestError },
    MissingPrerequisiteArtifact { role: String },
    OutOfOrder { step: LoadStage, after: LoadStage },
    MissingStage { stage: LoadStage },
}

/// Save-time compaction summary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompactionSummary {
    pub compacted_edits: u64,
    pub retained_edits: u64,
    pub snapshot_chunks: Vec<String>,
}

/// Artifacts written by save plus compaction evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveSummary {
    pub writes: Vec<ArtifactEntry>,
    pub compaction: CompactionSummary,
}

/// Fail-closed generator version mismatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GeneratorMismatch {
    pub saved_version: u32,
    pub current_version: u32,
}

/// One authored edit whose generated context changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EditConflict {
    pub event_id: u64,
    pub coord: ProjectBundleVoxelCoord,
    pub old_generated: ProjectBundleVoxelValue,
    pub new_generated: ProjectBundleVoxelValue,
    pub edit_value: ProjectBundleVoxelValue,
    pub suggested: SuggestedAction,
}

/// Regenerate-and-replay diagnostic outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegenConflictReport {
    pub saved_version: u32,
    pub new_version: u32,
    pub conflicts: Vec<EditConflict>,
    pub replayed_edits: u64,
    pub staging_session_hash: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn artifact_class_table_matches_variants() {
        let from: Vec<&str> = ALL_ARTIFACT_CLASSES.iter().map(|c| c.as_str()).collect();
        assert_eq!(from, ARTIFACT_CLASSES);
    }

    #[test]
    fn load_stage_table_matches_variants_and_order() {
        let from: Vec<&str> = ALL_LOAD_STAGES.iter().map(|s| s.as_str()).collect();
        assert_eq!(from, LOAD_STAGES);
    }

    #[test]
    fn suggested_action_table_matches_variants() {
        let from: Vec<&str> = ALL_SUGGESTED_ACTIONS.iter().map(|a| a.as_str()).collect();
        assert_eq!(from, SUGGESTED_ACTIONS);
    }

    #[test]
    fn vocabulary_tables_are_nonempty_and_unique() {
        for table in [
            ARTIFACT_CLASSES,
            KNOWN_ARTIFACT_ROLES,
            LOAD_STAGES,
            LOAD_STEP_KINDS,
            MANIFEST_ERROR_CODES,
            LOAD_PLAN_ERROR_CODES,
            SUGGESTED_ACTIONS,
        ] {
            assert!(!table.is_empty());
            let mut sorted = table.to_vec();
            sorted.sort_unstable();
            sorted.dedup();
            assert_eq!(sorted.len(), table.len(), "duplicate in {table:?}");
        }
    }
}
