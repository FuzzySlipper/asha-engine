//! World-bundle serialization: manifest format, deterministic load plan, and
//! save/compaction plan model (scene-capability-02, epic #2310).
//!
//! # Lane
//!
//! `rust-service` — authority-relevant serialization structures. Depends only on
//! `core-ids`, `core-assets`, `core-error`, and `core-scene`; it must not reach
//! into protocol/render/wasm/bridge. The *execution* that needs voxel
//! persistence (`rule-voxel-edit`) — actual snapshot/edit-log composition,
//! compaction reconstruction, and regenerate-and-replay diagnostics — lives in
//! the higher `rule-world-bundle` crate so the format/plan model stays low and
//! reusable.
//!
//! # Scope
//!
//! * [`WorldBundleManifest`] — the inspectable directory/manifest index with the
//!   classified [`ArtifactEntry`] table, bundle/protocol versions, world/scene
//!   identity, asset lock, and generator metadata. Validation fails **closed** on
//!   unknown newer versions (subtask #2318).
//! * [`json`] — std-only canonical manifest encode/decode (subtask #2318).
//! * [`LoadPlan`] — the deterministic, ordered, typed authority-load sequence with
//!   out-of-order / missing-prerequisite diagnostics (subtask #2319).
//! * [`SavePlan`] / [`CompactionPlan`] — the declarative save & explicit-compaction
//!   description, voxel-agnostic (subtask #2320).
//!
//! # Directory vs archive
//!
//! The directory/manifest layout is canonical for development — agents inspect,
//! diff, and repair individual files. A single-file `.asha` archive is only a
//! transport wrapper around the same files (`directory -> archive -> stage ->
//! validate -> load`); the directory is truth and the two must round-trip. Only
//! the directory form is implemented here.

#![forbid(unsafe_code)]

pub mod artifact;
pub mod hash;
pub mod json;
pub mod load_plan;
pub mod manifest;
pub mod save_plan;

pub use artifact::{ArtifactClass, ArtifactEntry, ArtifactRole};
pub use hash::BundleHash;
pub use json::{decode, encode, ManifestDecodeError};
pub use load_plan::{LoadPlan, LoadPlanError, LoadStage, LoadStep};
pub use manifest::{
    AssetLockSection, GeneratorMetadata, ManifestError, SceneSection, WorldBundleManifest,
    WorldSection, BUNDLE_SCHEMA_VERSION, SUPPORTED_PROTOCOL_VERSION,
};
pub use save_plan::{CompactionPlan, SavePlan};

#[cfg(test)]
mod tests {
    use super::*;
    use core_ids::{ProjectId, SceneId};

    /// A minimal but representative manifest: scene + asset lock (durable),
    /// one generated chunk snapshot, one durable edit log, and one disposable
    /// cache artifact. Abstract fixture nouns only.
    fn sample_manifest() -> WorldBundleManifest {
        WorldBundleManifest {
            bundle_schema_version: BUNDLE_SCHEMA_VERSION,
            protocol_version: SUPPORTED_PROTOCOL_VERSION,
            world: WorldSection {
                id: ProjectId::new(7),
                name: Some("sample-world".into()),
            },
            scene: SceneSection {
                id: SceneId::new(100),
                schema_version: 1,
                artifact: "scene/scene.json".into(),
            },
            asset_lock: AssetLockSection {
                artifact: "assets/lock.json".into(),
                asset_count: 1,
            },
            generator: GeneratorMetadata {
                seed: 42,
                version: 1,
                params: "default".into(),
            },
            artifacts: vec![
                ArtifactEntry::durable(
                    "scene/scene.json",
                    ArtifactRole::SceneDocument,
                    b"scene-bytes",
                ),
                ArtifactEntry::durable("assets/lock.json", ArtifactRole::AssetLock, b"lock-bytes"),
                ArtifactEntry::durable(
                    "voxel/edits.log",
                    ArtifactRole::VoxelEditLog,
                    b"edit-bytes",
                ),
                ArtifactEntry::generated(
                    "voxel/chunk_0_0_0.snapshot",
                    ArtifactRole::VoxelChunkSnapshot,
                    b"chunk-bytes",
                ),
                ArtifactEntry::cache("cache/mesh_0_0_0.bin", ArtifactRole::Cache),
            ],
        }
    }

    #[test]
    fn minimal_manifest_validates() {
        assert_eq!(sample_manifest().validate(), Ok(()));
    }

    #[test]
    fn newer_schema_fails_closed() {
        let mut m = sample_manifest();
        m.bundle_schema_version = BUNDLE_SCHEMA_VERSION + 1;
        assert!(matches!(
            m.validate(),
            Err(ManifestError::UnsupportedSchema { .. })
        ));
    }

    #[test]
    fn newer_protocol_fails_closed() {
        let mut m = sample_manifest();
        m.protocol_version = SUPPORTED_PROTOCOL_VERSION + 1;
        assert!(matches!(
            m.validate(),
            Err(ManifestError::UnsupportedProtocol { .. })
        ));
    }

    #[test]
    fn duplicate_artifact_path_is_rejected() {
        let mut m = sample_manifest();
        m.artifacts.push(ArtifactEntry::cache(
            "scene/scene.json",
            ArtifactRole::Cache,
        ));
        assert!(matches!(
            m.validate(),
            Err(ManifestError::DuplicateArtifact { .. })
        ));
    }

    #[test]
    fn durable_artifact_must_be_hashed() {
        let mut m = sample_manifest();
        m.artifacts[0].content_hash = None;
        assert!(matches!(
            m.validate(),
            Err(ManifestError::DurableMissingHash { .. })
        ));
    }

    #[test]
    fn missing_scene_artifact_is_rejected() {
        let mut m = sample_manifest();
        m.scene.artifact = "scene/missing.json".into();
        assert!(matches!(
            m.validate(),
            Err(ManifestError::MissingArtifact { .. })
        ));
    }

    #[test]
    fn cache_removal_preserves_durable_load_set() {
        let m = sample_manifest();
        let load_before: Vec<String> = m
            .load_required_artifacts()
            .iter()
            .map(|a| a.path.clone())
            .collect();
        let stripped = m.without_cache();
        assert!(stripped.validate().is_ok());
        let load_after: Vec<String> = stripped
            .load_required_artifacts()
            .iter()
            .map(|a| a.path.clone())
            .collect();
        assert_eq!(load_before, load_after);
        // The durable identity hash is unchanged by cache disposal.
        assert_eq!(m.durable_hash(), stripped.durable_hash());
    }

    #[test]
    fn durable_hash_changes_when_durable_content_changes() {
        let m = sample_manifest();
        let mut changed = m.clone();
        changed.artifacts[0] = ArtifactEntry::durable(
            "scene/scene.json",
            ArtifactRole::SceneDocument,
            b"different",
        );
        assert_ne!(m.durable_hash(), changed.durable_hash());
    }

    #[test]
    fn manifest_json_round_trips_through_decode_encode() {
        let m = sample_manifest();
        let encoded = encode(&m);
        let decoded = decode(&encoded).expect("decode");
        // Encode is canonical; re-encoding the decoded manifest is a fixed point.
        assert_eq!(encode(&decoded), encoded);
        assert_eq!(decoded.canonical(), m.canonical());
        assert!(decoded.validate().is_ok());
    }

    #[test]
    fn decode_rejects_unknown_class() {
        let m = sample_manifest();
        let encoded = encode(&m).replace("\"durable\"", "\"eternal\"");
        assert!(matches!(
            decode(&encoded),
            Err(ManifestDecodeError::UnknownClass(c)) if c == "eternal"
        ));
    }

    #[test]
    fn load_plan_is_deterministic_and_ordered() {
        let m = sample_manifest();
        let plan = LoadPlan::build(&m).expect("plan");
        assert_eq!(plan, LoadPlan::build(&m).expect("plan again"));
        // Stages are non-decreasing.
        let stages: Vec<u8> = plan.steps.iter().map(|s| s.stage().index()).collect();
        let mut sorted = stages.clone();
        sorted.sort_unstable();
        assert_eq!(stages, sorted);
        // Bootstrap references the same scene the document load does.
        assert!(matches!(
            plan.steps.last(),
            Some(LoadStep::ValidateFinalState)
        ));
    }

    #[test]
    fn out_of_order_plan_is_classified() {
        let m = sample_manifest();
        let mut plan = LoadPlan::build(&m).unwrap();
        // Move Bootstrap before the scene-document load.
        plan.steps.swap(2, 5);
        assert!(matches!(
            plan.verify_order(),
            Err(LoadPlanError::OutOfOrder { .. })
        ));
    }

    #[test]
    fn missing_mandatory_stage_is_classified() {
        let m = sample_manifest();
        let mut plan = LoadPlan::build(&m).unwrap();
        plan.steps.retain(|s| s.stage() != LoadStage::Bootstrap);
        assert!(matches!(
            plan.verify_order(),
            Err(LoadPlanError::MissingStage {
                stage: LoadStage::Bootstrap
            })
        ));
    }

    #[test]
    fn load_plan_build_fails_closed_on_bad_manifest() {
        let mut m = sample_manifest();
        m.bundle_schema_version = BUNDLE_SCHEMA_VERSION + 1;
        assert!(matches!(
            LoadPlan::build(&m),
            Err(LoadPlanError::Manifest(
                ManifestError::UnsupportedSchema { .. }
            ))
        ));
    }

    #[test]
    fn save_plan_describes_writes_and_compaction() {
        let writes = vec![
            ArtifactEntry::durable(
                "world/state.snapshot",
                ArtifactRole::SessionStateSnapshot,
                b"s",
            ),
            ArtifactEntry::generated(
                "voxel/chunk_0_0_0.snapshot",
                ArtifactRole::VoxelChunkSnapshot,
                b"c",
            ),
            ArtifactEntry::durable("voxel/recent.log", ArtifactRole::VoxelEditLog, b"e"),
            ArtifactEntry::cache("cache/mesh.bin", ArtifactRole::Cache),
        ];
        let plan = SavePlan::new(
            writes,
            CompactionPlan {
                compacted_edits: 8,
                retained_edits: 2,
                snapshot_chunks: vec!["0,0,0".into()],
            },
        );
        assert_eq!(plan.count(ArtifactClass::Cache), 1);
        assert_eq!(plan.durable_writes().count(), 2);
        let desc = plan.describe();
        assert!(desc.contains("fold 8 edits"));
        assert!(desc.contains("retain 2 recent edit"));
        // Writes are path-sorted (deterministic).
        let paths: Vec<&str> = plan.writes.iter().map(|a| a.path.as_str()).collect();
        let mut sorted = paths.clone();
        sorted.sort_unstable();
        assert_eq!(paths, sorted);
    }
}
