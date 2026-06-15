//! Cross-boundary schema for world bundles (scene-capability-super, epic #2351,
//! subtask #2366).
//!
//! # Lane
//!
//! `contract-steward` — owns the border shape TypeScript devtools use to
//! **display** world-bundle manifests, ordered load plans, save/compaction
//! summaries, version-compatibility findings, and the regenerate-and-replay
//! generator diagnostic. Like `protocol-render`/`protocol-scene` it depends on
//! `core-ids` only and carries **no authority logic**: manifest validation, load
//! planning, save composition, and generator replay all stay in
//! `svc-serialization` and `rule-world-bundle`. TS can read these shapes; it
//! cannot mutate bundle state.
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
    "worldStateSnapshot",
    "voxelChunkSnapshot",
    "voxelEditLog",
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
    "bootstrap",
    "worldStateSnapshot",
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
    Bootstrap,
    /// Restore the runtime-diverged world-state snapshot over the bootstrapped
    /// scene baseline. Optional: present only when a save carried runtime
    /// divergence (#2484).
    WorldStateSnapshot,
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
            LoadStage::Bootstrap => "bootstrap",
            LoadStage::WorldStateSnapshot => "worldStateSnapshot",
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
    LoadStage::Bootstrap,
    LoadStage::WorldStateSnapshot,
    LoadStage::FinalValidation,
];

/// Stable discriminant for each load-step variant. Mirrors the `svc_serialization::LoadStep` enum.
pub const LOAD_STEP_KINDS: &[&str] = &[
    "validateVersions",
    "loadAssetLock",
    "loadSceneDocument",
    "generateTerrain",
    "applyVoxelEdits",
    "bootstrapScene",
    "restoreWorldState",
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
/// `rule_world_bundle::SuggestedAction::label`.
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
