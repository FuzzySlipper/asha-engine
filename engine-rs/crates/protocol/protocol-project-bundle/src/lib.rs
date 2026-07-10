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

use core_ids::{ProjectId, RuntimeSessionId, SceneId};

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

/// Terrain generator provenance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratorMetadata {
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
    pub scene: SceneSection,
    pub asset_lock: AssetLockSection,
    pub generator: GeneratorMetadata,
    pub artifacts: Vec<ArtifactEntry>,
}

/// Classified manifest validation failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManifestError {
    UnsupportedSchema { found: u32, supported: u32 },
    UnsupportedProtocol { found: u32, supported: u32 },
    DuplicateArtifact { path: String },
    MissingArtifact { role: String, path: String },
    DurableMissingHash { path: String },
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
