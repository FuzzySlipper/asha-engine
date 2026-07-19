//! Project-bundle serialization: manifest format, deterministic load plan, and
//! save/compaction plan model (scene-capability-02, epic #2310).
//!
//! # Lane
//!
//! `rust-service` — authority-relevant serialization structures. Depends only on
//! `core-ids`, `core-assets`, `core-error`, and `core-scene`; it must not reach
//! into protocol/render/wasm/bridge. The *execution* that needs voxel
//! persistence (`rule-voxel-edit`) — actual snapshot/edit-log composition,
//! compaction reconstruction, and regenerate-and-replay diagnostics — lives in
//! the higher `rule-project-bundle` crate so the format/plan model stays low and
//! reusable.
//!
//! # Scope
//!
//! * [`ProjectBundleManifest`] — the inspectable directory/manifest index with the
//!   classified [`ArtifactEntry`] table, bundle/protocol versions, project/scene
//!   identities, asset lock, and optional authoring provenance. Validation fails **closed** on
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
pub mod prefab;
pub mod prefab_json;
pub mod save_plan;
pub mod source_batch;
pub mod write_set;

pub use artifact::{ArtifactClass, ArtifactEntry, ArtifactRole};
pub use hash::BundleHash;
pub use json::{decode, encode, ManifestDecodeError};
pub use load_plan::{LoadPlan, LoadPlanError, LoadStage, LoadStep};
pub use manifest::{
    AssetLockSection, GeneratorMetadata, ManifestError, ProjectBundleManifest, ProjectSection,
    SceneSection, BUNDLE_SCHEMA_VERSION, LEGACY_BUNDLE_SCHEMA_VERSION, SUPPORTED_PROTOCOL_VERSION,
};
pub use prefab::{
    validate_prefab_registry, PrefabDefinition, PrefabDiagnostic, PrefabDiagnosticCode,
    PrefabInstanceRecord, PrefabOverride, PrefabOverrideValue, PrefabPart, PrefabPartReference,
    PrefabPartRoleBinding, PrefabPartSource, PrefabRegistry, PrefabRegistryValidationContext,
    PrefabTransform, PrefabValidationReport, PrefabVariantDelta, ValidatedPrefabRegistry,
    PREFAB_DEFINITION_SCHEMA_VERSION, PREFAB_REGISTRY_SCHEMA_VERSION,
};
pub use prefab_json::{
    encode_prefab_registry, load_prefab_registry, PrefabRegistryDecodeError,
    PrefabRegistryLoadError,
};
pub use save_plan::{CompactionPlan, SavePlan};
pub use source_batch::{
    validate_runtime_project_source_batch, AdmittedRuntimeProjectSourceBatch,
    ProjectResourceHandle, ProjectResourceStaging, ProjectResourceTransaction,
    ProjectSourceBatchError, ProjectSourceBatchErrorCode, ProjectSourceBody,
    RuntimeProjectSourceBatch, StagedProjectResource, ValidatedRuntimeProjectSourceBatch,
    PROJECT_SOURCE_INLINE_BODY_MAX_BYTES, PROJECT_SOURCE_INLINE_TOTAL_MAX_BYTES,
    PROJECT_SOURCE_MANIFEST_MAX_BYTES, PROJECT_SOURCE_MAX_BODIES,
    PROJECT_SOURCE_RESOURCE_MAX_BYTES, PROJECT_SOURCE_RESOURCE_TOTAL_MAX_BYTES,
};
pub use write_set::{
    AuthorizedProjectWriteCandidate, CanonicalProjectDelete, CanonicalProjectMove,
    CanonicalProjectWrite, ProjectArtifactExpectation, ProjectStoreIdentity, ProjectWriteCandidate,
    ProjectWriteConfirmation, ProjectWriteSetDraft, ProjectWriteSetError,
    PROJECT_BUNDLE_MANIFEST_PATH,
};

#[cfg(test)]
mod tests {
    use super::*;
    use core_ids::{ProjectId, SceneId};

    /// A minimal but representative manifest: scene + asset lock (durable),
    /// one generated chunk snapshot, one durable edit log, and one disposable
    /// cache artifact. Abstract fixture nouns only.
    fn sample_manifest() -> ProjectBundleManifest {
        ProjectBundleManifest {
            bundle_schema_version: BUNDLE_SCHEMA_VERSION,
            protocol_version: SUPPORTED_PROTOCOL_VERSION,
            project: ProjectSection {
                id: ProjectId::new(7),
                name: Some("sample-project".into()),
            },
            entry_scene: SceneId::new(100),
            scenes: vec![SceneSection {
                id: SceneId::new(100),
                schema_version: 1,
                artifact: "scene/scene.json".into(),
            }],
            asset_lock: AssetLockSection {
                artifact: "assets/lock.json".into(),
                asset_count: 1,
            },
            generation_provenance: Some(GeneratorMetadata {
                provider: "asha.environment.sample".into(),
                seed: 42,
                version: 1,
                params: "default".into(),
            }),
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
                ArtifactEntry::durable(
                    "annotations/semantic.avann.json",
                    ArtifactRole::VoxelAnnotationLayer,
                    b"annotation-bytes",
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
    fn prefab_registry_is_a_single_durable_bundle_artifact() {
        let mut manifest = sample_manifest();
        manifest.artifacts.push(ArtifactEntry::durable(
            "prefabs/registry.json",
            ArtifactRole::PrefabRegistry,
            b"prefab-registry",
        ));
        assert!(manifest.validate().is_ok());

        manifest.artifacts.push(ArtifactEntry::durable(
            "prefabs/second-registry.json",
            ArtifactRole::PrefabRegistry,
            b"second-prefab-registry",
        ));
        assert!(matches!(
            manifest.validate(),
            Err(ManifestError::DuplicateArtifactRole { .. })
        ));

        manifest.artifacts.pop();
        manifest.artifacts.pop();
        manifest.artifacts.push(ArtifactEntry::generated(
            "prefabs/registry.json",
            ArtifactRole::PrefabRegistry,
            b"generated-prefab-registry",
        ));
        assert!(matches!(
            manifest.validate(),
            Err(ManifestError::ArtifactClassMismatch { .. })
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
        m.scenes[0].artifact = "scene/missing.json".into();
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
    fn generator_free_multi_scene_voxel_closure_is_canonical() {
        let mut manifest = sample_manifest();
        manifest.generation_provenance = None;
        manifest.scenes.push(SceneSection {
            id: SceneId::new(101),
            schema_version: 1,
            artifact: "scene/secondary.json".into(),
        });
        manifest.artifacts.extend([
            ArtifactEntry::durable(
                "scene/secondary.json",
                ArtifactRole::SceneDocument,
                b"secondary-scene",
            ),
            ArtifactEntry::durable(
                "content/gameplay-module.json",
                ArtifactRole::ProjectContent,
                b"project-content",
            ),
            ArtifactEntry::durable(
                "voxel/house.avox.json",
                ArtifactRole::VoxelVolumeAsset,
                b"voxel-house",
            ),
            ArtifactEntry::durable(
                "resources/house-albedo.png",
                ArtifactRole::Resource("resource:texture".into()),
                b"texture-bytes",
            ),
        ]);

        assert_eq!(manifest.validate(), Ok(()));
        let encoded = encode(&manifest);
        assert!(encoded.contains("\"generationProvenance\": null"));
        assert!(encoded.contains("\"entryScene\": 100"));
        assert_eq!(decode(&encoded).expect("decode"), manifest.canonical());

        let plan = LoadPlan::build(&manifest).expect("generator-free load plan");
        assert!(!plan
            .steps
            .iter()
            .any(|step| matches!(step, LoadStep::GenerateTerrain { .. })));
    }

    #[test]
    fn manifest_scene_table_and_paths_fail_closed() {
        let mut missing_entry = sample_manifest();
        missing_entry.entry_scene = SceneId::new(999);
        assert!(matches!(
            missing_entry.validate(),
            Err(ManifestError::MissingEntryScene { scene: 999 })
        ));

        let mut duplicate_scene = sample_manifest();
        duplicate_scene.scenes.push(SceneSection {
            id: SceneId::new(100),
            schema_version: 1,
            artifact: "scene/other.json".into(),
        });
        duplicate_scene.artifacts.push(ArtifactEntry::durable(
            "scene/other.json",
            ArtifactRole::SceneDocument,
            b"other",
        ));
        assert!(matches!(
            duplicate_scene.validate(),
            Err(ManifestError::DuplicateScene { scene: 100 })
        ));

        let mut traversal = sample_manifest();
        traversal.artifacts[0].path = "../scene.json".into();
        traversal.scenes[0].artifact = "../scene.json".into();
        assert!(matches!(
            traversal.validate(),
            Err(ManifestError::InvalidArtifactPath { .. })
        ));

        let mut opaque_role = sample_manifest();
        opaque_role.artifacts.push(ArtifactEntry::durable(
            "future/input.bin",
            ArtifactRole::Other("mysteryInput".into()),
            b"mystery",
        ));
        assert!(matches!(
            opaque_role.validate(),
            Err(ManifestError::UnknownArtifactRole { .. })
        ));
    }

    #[test]
    fn durable_hash_covers_path_role_and_scene_linkage() {
        let manifest = sample_manifest();

        let mut moved = manifest.clone();
        moved.artifacts[0].path = "scene/moved.json".into();
        moved.scenes[0].artifact = "scene/moved.json".into();
        assert_ne!(manifest.durable_hash(), moved.durable_hash());

        let mut changed_role = manifest.clone();
        changed_role.artifacts[2].role = ArtifactRole::ProjectContent;
        assert_ne!(manifest.durable_hash(), changed_role.durable_hash());

        let mut changed_entry = manifest.clone();
        changed_entry.entry_scene = SceneId::new(101);
        assert_ne!(manifest.durable_hash(), changed_entry.durable_hash());

        let mut added = manifest.clone();
        added.artifacts.push(ArtifactEntry::durable(
            "content/added.json",
            ArtifactRole::ProjectContent,
            b"added",
        ));
        assert_ne!(manifest.durable_hash(), added.durable_hash());

        let mut deleted = manifest.clone();
        deleted.artifacts.remove(2);
        assert_ne!(manifest.durable_hash(), deleted.durable_hash());
    }

    #[test]
    fn legacy_v1_manifest_migrates_to_v2_and_strict_decode_rejects_unknown_fields() {
        let legacy = r#"{
  "bundleSchemaVersion": 1,
  "protocolVersion": 1,
  "project": { "id": 7, "name": null },
  "scene": { "id": 100, "schemaVersion": 1, "artifact": "scene/scene.json" },
  "assetLock": { "artifact": "assets/lock.json", "assetCount": 0 },
  "generator": { "seed": 42, "version": 3, "params": "legacy" },
  "artifacts": [
    { "path": "scene/scene.json", "class": "durable", "role": "sceneDocument", "contentHash": "1723540f7db7a459" },
    { "path": "assets/lock.json", "class": "durable", "role": "assetLock", "contentHash": "422f72d827e3137c" }
  ]
}"#;
        let migrated = decode(legacy).expect("v1 compatibility decode");
        assert_eq!(migrated.bundle_schema_version, BUNDLE_SCHEMA_VERSION);
        assert_eq!(migrated.entry_scene, SceneId::new(100));
        assert_eq!(migrated.scenes.len(), 1);
        assert_eq!(
            migrated
                .generation_provenance
                .as_ref()
                .expect("legacy provenance")
                .provider,
            "legacy.terrain-generator"
        );
        assert!(encode(&migrated).contains("\"bundleSchemaVersion\": 2"));

        let with_unknown = legacy.replace(
            "\"protocolVersion\": 1,",
            "\"protocolVersion\": 1, \"hostRoleMirror\": {},",
        );
        assert!(matches!(
            decode(&with_unknown),
            Err(ManifestDecodeError::Field(message)) if message.contains("hostRoleMirror")
        ));

        let future = legacy.replace("\"bundleSchemaVersion\": 1", "\"bundleSchemaVersion\": 99");
        assert!(matches!(
            decode(&future),
            Err(ManifestDecodeError::UnsupportedSchema {
                found: 99,
                supported: BUNDLE_SCHEMA_VERSION
            })
        ));
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
        let annotation_artifacts = plan
            .steps
            .iter()
            .find_map(|step| match step {
                LoadStep::LoadVoxelAnnotations { artifacts } => Some(artifacts),
                _ => None,
            })
            .expect("annotation load step");
        assert_eq!(
            annotation_artifacts,
            &["annotations/semantic.avann.json".to_string()]
        );
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
                "project/state.snapshot",
                ArtifactRole::SessionStateSnapshot,
                b"s",
            ),
            ArtifactEntry::generated(
                "voxel/chunk_0_0_0.snapshot",
                ArtifactRole::VoxelChunkSnapshot,
                b"c",
            ),
            ArtifactEntry::durable("voxel/recent.log", ArtifactRole::VoxelEditLog, b"e"),
            ArtifactEntry::durable(
                "annotations/semantic.avann.json",
                ArtifactRole::VoxelAnnotationLayer,
                b"a",
            ),
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
        assert_eq!(plan.durable_writes().count(), 3);
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
