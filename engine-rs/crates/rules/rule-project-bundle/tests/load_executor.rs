//! Integration tests for the ordered project-bundle load executor (#2361).
//!
//! Exercises a minimal valid bundle through the *real* executor (not a plan
//! builder), plus the classified failure paths: missing durable artifact, an
//! invalid scene, a missing asset lock, an unsupported version, and an
//! out-of-order plan. A golden stage summary pins the executed-stage readback.

use core_ids::{RuntimeSessionId, SceneId, SceneNodeId};
use core_scene::{encode, SceneMetadata, SceneNode, SceneNodeKind, SceneTree};
use svc_serialization::{LoadPlan, LoadStage, LoadStep};

use rule_project_bundle::{
    compose_voxel_edit_history_artifact, execute_load_plan, BundleArtifacts, LoadExecutionError,
    ProjectBundleStage,
};

/// A small, valid two-node scene (scene id 100), encoded as canonical JSON.
fn sample_scene_json() -> String {
    let tree = SceneTree {
        id: SceneId::new(100),
        schema_version: 1,
        metadata: SceneMetadata {
            name: Some("load-fixture".into()),
            authoring_format_version: 1,
        },
        dependencies: vec![],
        roots: vec![
            SceneNode::leaf(SceneNodeId::new(1), SceneNodeKind::EmptyGroup).with_children(vec![
                SceneNode::leaf(SceneNodeId::new(2), SceneNodeKind::EmptyGroup),
            ]),
        ],
    };
    encode(&tree.to_flat())
}

/// The canonical mandatory-stage plan for the sample bundle.
fn sample_plan() -> LoadPlan {
    LoadPlan {
        steps: vec![
            LoadStep::ValidateVersions {
                bundle_schema_version: 2,
                protocol_version: 1,
            },
            LoadStep::LoadAssetLock {
                artifact: "assets/lock.json".into(),
                asset_count: 1,
            },
            LoadStep::LoadSceneDocument {
                artifact: "scene/scene.json".into(),
                scene: SceneId::new(100),
            },
            LoadStep::BootstrapScene {
                scene: SceneId::new(100),
                runtime_session: RuntimeSessionId::new(7),
            },
            LoadStep::ValidateFinalState,
        ],
    }
}

fn sample_artifacts() -> BundleArtifacts {
    BundleArtifacts::new()
        .with_artifact("assets/lock.json", "{ \"entries\": [] }\n")
        .with_artifact("scene/scene.json", sample_scene_json())
}

#[test]
fn minimal_valid_bundle_loads_into_authority() {
    let result = execute_load_plan(&sample_plan(), &sample_artifacts()).expect("load succeeds");
    // Two scene nodes → two runtime entities, each with a source trace.
    assert_eq!(result.spatial_session.entity_count(), 2);
    assert_eq!(result.bootstrap.source_trace.len(), 2);
    assert_eq!(
        result.bootstrap.runtime_session_id,
        RuntimeSessionId::new(7)
    );
    assert!(result.voxel.is_none());
    // Every mandatory stage ran, in order.
    let stages: Vec<LoadStage> = result.stages.iter().map(|s| s.stage).collect();
    assert_eq!(
        stages,
        vec![
            LoadStage::Versions,
            LoadStage::AssetLock,
            LoadStage::SceneDocument,
            LoadStage::Bootstrap,
            LoadStage::FinalValidation,
        ]
    );
}

#[test]
fn stage_summary_matches_golden() {
    let result = execute_load_plan(&sample_plan(), &sample_artifacts()).unwrap();
    let summary = result.render_summary();
    let spatial_session_hash = result.spatial_session_hash.0;
    let expected = format!(
        "stage versions schema=2 protocol=1\n\
         stage assetLock artifact=assets/lock.json expectedAssets=1\n\
         stage sceneDocument artifact=scene/scene.json nodes=2\n\
         stage bootstrap runtimeSession=7 entities=2\n\
         stage finalValidation spatialSessionHash={spatial_session_hash:016x} ok\n\
         result entities=2 voxel=false spatialSessionHash={spatial_session_hash:016x}\n\
         voxelAnnotations count=0\n\
         voxelHistory none\n\
         runtimeEntities none\n\
         sourceTrace count=2\n"
    );
    assert_eq!(summary, expected);
}

#[test]
fn missing_durable_artifact_fails_closed() {
    // Drop the scene artifact from the source.
    let artifacts = BundleArtifacts::new().with_artifact("assets/lock.json", "{}\n");
    let err = execute_load_plan(&sample_plan(), &artifacts).unwrap_err();
    match err {
        LoadExecutionError::MissingArtifact { stage, path } => {
            assert_eq!(stage, LoadStage::SceneDocument);
            assert_eq!(path, "scene/scene.json");
        }
        other => panic!("expected MissingArtifact, got {other:?}"),
    }
}

#[test]
fn missing_asset_lock_fails_closed() {
    let artifacts = BundleArtifacts::new().with_artifact("scene/scene.json", sample_scene_json());
    let err = execute_load_plan(&sample_plan(), &artifacts).unwrap_err();
    assert!(matches!(
        err,
        LoadExecutionError::MissingArtifact {
            stage: LoadStage::AssetLock,
            ..
        }
    ));
}

#[test]
fn invalid_scene_is_classified() {
    // A scene whose node names a parent that does not exist fails validation.
    let bad_scene = r#"{
  "schemaVersion": 1,
  "id": 100,
  "metadata": { "name": null, "authoringFormatVersion": 0 },
  "dependencies": [],
  "nodes": [
    { "id": 1, "parent": 999, "childOrder": 0, "label": null, "tags": [], "transform": { "translation": [0, 0, 0], "rotation": [0, 0, 0, 1], "scale": [1, 1, 1] }, "kind": { "kind": "emptyGroup" } }
  ]
}
"#;
    let artifacts = BundleArtifacts::new()
        .with_artifact("assets/lock.json", "{}\n")
        .with_artifact("scene/scene.json", bad_scene);
    let err = execute_load_plan(&sample_plan(), &artifacts).unwrap_err();
    match err {
        LoadExecutionError::SceneInvalid { report, .. } => assert!(!report.is_ok()),
        other => panic!("expected SceneInvalid, got {other:?}"),
    }
}

#[test]
fn unsupported_version_fails_closed() {
    let mut plan = sample_plan();
    plan.steps[0] = LoadStep::ValidateVersions {
        bundle_schema_version: 99,
        protocol_version: 1,
    };
    let err = execute_load_plan(&plan, &sample_artifacts()).unwrap_err();
    assert!(matches!(
        err,
        LoadExecutionError::VersionUnsupported {
            bundle_schema: 99,
            ..
        }
    ));
}

#[test]
fn out_of_order_plan_is_rejected_before_execution() {
    // Bootstrap before the scene document violates authority load order.
    let plan = LoadPlan {
        steps: vec![
            LoadStep::ValidateVersions {
                bundle_schema_version: 2,
                protocol_version: 1,
            },
            LoadStep::LoadAssetLock {
                artifact: "assets/lock.json".into(),
                asset_count: 1,
            },
            LoadStep::BootstrapScene {
                scene: SceneId::new(100),
                runtime_session: RuntimeSessionId::new(7),
            },
            LoadStep::LoadSceneDocument {
                artifact: "scene/scene.json".into(),
                scene: SceneId::new(100),
            },
            LoadStep::ValidateFinalState,
        ],
    };
    let err = execute_load_plan(&plan, &sample_artifacts()).unwrap_err();
    assert!(matches!(err, LoadExecutionError::PlanInvalid(_)));
}

#[test]
fn voxel_section_reconstructs_authority() {
    use core_events::VoxelEditEvent;
    use core_space::{ChunkCoord, ChunkDims, GridId, VoxelCoord, VoxelGridSpec};
    use core_voxel::VoxelValue;
    use rule_voxel_edit::generate_chunk;
    use rule_voxel_edit::persist::encode_edit_log;

    let spec = VoxelGridSpec::new(GridId::new(0), 1.0, ChunkDims::cubic(4).unwrap()).unwrap();
    let g = GridId::new(0);
    let chunk = ChunkCoord::new(0, 0, 0);
    let gen = generate_chunk(&spec, chunk, 7, 1);
    let events = vec![
        VoxelEditEvent::ChunkGenerated {
            grid: g,
            chunk,
            seed: 7,
            generator_version: 1,
            hash: gen.content_hash().0,
        },
        VoxelEditEvent::VoxelSet {
            grid: g,
            coord: VoxelCoord::new(0, 3, 0),
            value: VoxelValue::solid_raw(2),
        },
    ];

    // Insert a voxel stage before bootstrap.
    let mut plan = sample_plan();
    plan.steps.insert(
        3,
        LoadStep::ApplyVoxelEdits {
            edit_logs: vec!["voxel/edits.log".into()],
            snapshots: vec![],
            histories: vec![],
        },
    );

    let artifacts = sample_artifacts()
        .with_artifact("voxel/edits.log", encode_edit_log(&events))
        .with_voxel_spec(spec);

    let result = execute_load_plan(&plan, &artifacts).expect("voxel load succeeds");
    let voxel = result.voxel.expect("voxel authority present");
    assert!(voxel.tracked_len() >= 1, "the generated chunk is resident");
    // Scene authority is still intact alongside voxel authority.
    assert_eq!(result.spatial_session.entity_count(), 2);
}

#[test]
fn voxel_history_survives_bundle_reopen_with_redo_tail() {
    use core_events::VoxelEditEvent;
    use core_space::{ChunkCoord, ChunkDims, GridId, VoxelCoord, VoxelGridSpec};
    use core_voxel::VoxelValue;
    use rule_voxel_edit::history::VoxelEditHistory;
    use rule_voxel_edit::persist::{encode_chunk_snapshot, encode_edit_log};
    use rule_voxel_edit::{apply_all, voxel_world_hash, VoxelEditTransactionMode};
    use svc_spatial::VoxelWorld;
    use svc_volume::VoxelChunk;

    fn accepted_receipt(
        world: &mut VoxelWorld,
        event: VoxelEditEvent,
    ) -> rule_voxel_edit::VoxelEditTransactionReceipt {
        let before_hash = voxel_world_hash(world);
        let events = vec![event];
        apply_all(world, &events).expect("event applies");
        let after_hash = voxel_world_hash(world);
        rule_voxel_edit::VoxelEditTransactionReceipt {
            mode: VoxelEditTransactionMode::Apply,
            applied: true,
            accepted: 1,
            rejected: 0,
            event_count: events.len() as u32,
            touched_voxels: 1,
            before_hash,
            projected_hash: after_hash,
            after_hash,
            transaction_hash: before_hash ^ after_hash,
            events,
            rejections: Vec::new(),
        }
    }

    let spec = VoxelGridSpec::new(GridId::new(0), 1.0, ChunkDims::cubic(4).unwrap()).unwrap();
    let chunk = ChunkCoord::new(0, 0, 0);
    let mut base = VoxelWorld::new(spec);
    base.insert(chunk, VoxelChunk::from_spec(&spec));
    base.drain_dirty();

    let mut external = base.clone();
    let mut history = VoxelEditHistory::new(base.clone());
    let first = accepted_receipt(
        &mut external,
        VoxelEditEvent::VoxelSet {
            grid: GridId::new(0),
            coord: VoxelCoord::new(0, 0, 0),
            value: VoxelValue::solid_raw(1),
        },
    );
    history.append_accepted(first.clone()).unwrap();
    let second = accepted_receipt(
        &mut external,
        VoxelEditEvent::VoxelSet {
            grid: GridId::new(0),
            coord: VoxelCoord::new(1, 0, 0),
            value: VoxelValue::solid_raw(2),
        },
    );
    history.append_accepted(second).unwrap();
    history.undo_one().unwrap();

    let history_artifact = compose_voxel_edit_history_artifact(&history, "materials:test", 0);
    let base_chunk = base.get(chunk).expect("base chunk present");
    let mut plan = sample_plan();
    plan.steps.insert(
        3,
        LoadStep::ApplyVoxelEdits {
            edit_logs: vec!["voxel/edits.log".into()],
            snapshots: vec!["voxel/chunk_0_0_0.snapshot".into()],
            histories: vec![history_artifact.path.clone()],
        },
    );
    let artifacts = sample_artifacts()
        .with_artifact(
            "voxel/chunk_0_0_0.snapshot",
            encode_chunk_snapshot(base_chunk),
        )
        .with_artifact("voxel/edits.log", encode_edit_log(&first.events))
        .with_artifact(history_artifact.path, history_artifact.text)
        .with_voxel_spec(spec)
        .with_voxel_material_catalog_hash("materials:test");

    let result = execute_load_plan(&plan, &artifacts).expect("voxel history load succeeds");
    let loaded_history = result.voxel_history.expect("history authority present");
    let loaded_voxel = result.voxel.expect("voxel authority present");

    assert_eq!(loaded_history.cursor().index, 1);
    assert_eq!(loaded_history.cursor().redo_depth, 1);
    assert_eq!(
        loaded_history.current_world_hash(),
        rule_voxel_edit::voxel_world_hash(&loaded_voxel)
    );
}

#[test]
fn voxel_history_material_hash_drift_fails_closed() {
    use core_space::{ChunkCoord, ChunkDims, GridId, VoxelGridSpec};
    use rule_voxel_edit::history::VoxelEditHistory;
    use rule_voxel_edit::persist::encode_chunk_snapshot;
    use svc_spatial::VoxelWorld;
    use svc_volume::VoxelChunk;

    let spec = VoxelGridSpec::new(GridId::new(0), 1.0, ChunkDims::cubic(4).unwrap()).unwrap();
    let chunk = ChunkCoord::new(0, 0, 0);
    let mut base = VoxelWorld::new(spec);
    base.insert(chunk, VoxelChunk::from_spec(&spec));
    base.drain_dirty();
    let history = VoxelEditHistory::new(base.clone());
    let history_artifact = compose_voxel_edit_history_artifact(&history, "materials:old", 0);
    let base_chunk = base.get(chunk).expect("base chunk present");

    let mut plan = sample_plan();
    plan.steps.insert(
        3,
        LoadStep::ApplyVoxelEdits {
            edit_logs: vec![],
            snapshots: vec!["voxel/chunk_0_0_0.snapshot".into()],
            histories: vec![history_artifact.path.clone()],
        },
    );
    let artifacts = sample_artifacts()
        .with_artifact(
            "voxel/chunk_0_0_0.snapshot",
            encode_chunk_snapshot(base_chunk),
        )
        .with_artifact(history_artifact.path, history_artifact.text)
        .with_voxel_spec(spec)
        .with_voxel_material_catalog_hash("materials:new");

    let err = execute_load_plan(&plan, &artifacts).unwrap_err();
    match err {
        LoadExecutionError::VoxelHistory { detail, .. } => {
            assert!(detail.contains("material catalog hash mismatch"));
        }
        other => panic!("expected material hash drift, got {other:?}"),
    }
}

#[test]
fn voxel_section_without_spec_fails_closed() {
    let mut plan = sample_plan();
    plan.steps.insert(
        3,
        LoadStep::ApplyVoxelEdits {
            edit_logs: vec!["voxel/edits.log".into()],
            snapshots: vec![],
            histories: vec![],
        },
    );
    // Provide the log but no voxel spec.
    let artifacts = sample_artifacts().with_artifact("voxel/edits.log", "voxel-log\n");
    let err = execute_load_plan(&plan, &artifacts).unwrap_err();
    assert!(matches!(err, LoadExecutionError::VoxelSpecMissing));
}

#[test]
fn voxel_annotation_layer_survives_bundle_load_without_touching_voxel_authority() {
    let annotation_text = sample_annotation_json("fnv1a64:target");
    let mut plan = sample_plan();
    plan.steps.insert(
        3,
        LoadStep::LoadVoxelAnnotations {
            artifacts: vec!["annotations/semantic.avann.json".into()],
        },
    );

    let artifacts = sample_artifacts()
        .with_artifact("annotations/semantic.avann.json", annotation_text)
        .with_voxel_volume_data_hash("voxel-volume/test-room", "fnv1a64:target");

    let result = execute_load_plan(&plan, &artifacts).expect("annotation load succeeds");
    assert!(
        result.voxel.is_none(),
        "annotations must not create voxel occupancy"
    );
    assert_eq!(result.voxel_annotations.len(), 1);
    assert_eq!(
        result.voxel_annotations[0].layer_id,
        "voxel-annotation/test-room/semantic"
    );
    assert_eq!(result.voxel_annotations[0].regions.len(), 1);
    let stages: Vec<LoadStage> = result.stages.iter().map(|s| s.stage).collect();
    assert_eq!(
        stages,
        vec![
            LoadStage::Versions,
            LoadStage::AssetLock,
            LoadStage::SceneDocument,
            LoadStage::VoxelAnnotations,
            LoadStage::Bootstrap,
            LoadStage::FinalValidation,
        ]
    );
}

#[test]
fn voxel_annotation_missing_target_volume_fails_closed() {
    let mut plan = sample_plan();
    plan.steps.insert(
        3,
        LoadStep::LoadVoxelAnnotations {
            artifacts: vec!["annotations/semantic.avann.json".into()],
        },
    );
    let artifacts = sample_artifacts().with_artifact(
        "annotations/semantic.avann.json",
        sample_annotation_json("fnv1a64:target"),
    );

    let err = execute_load_plan(&plan, &artifacts).unwrap_err();
    match err {
        LoadExecutionError::VoxelAnnotationTargetMissing { path, asset_id } => {
            assert_eq!(path, "annotations/semantic.avann.json");
            assert_eq!(asset_id, "voxel-volume/test-room");
        }
        other => panic!("expected missing annotation target, got {other:?}"),
    }
}

#[test]
fn voxel_annotation_stale_target_hash_fails_closed() {
    let mut plan = sample_plan();
    plan.steps.insert(
        3,
        LoadStep::LoadVoxelAnnotations {
            artifacts: vec!["annotations/semantic.avann.json".into()],
        },
    );
    let artifacts = sample_artifacts()
        .with_artifact(
            "annotations/semantic.avann.json",
            sample_annotation_json("fnv1a64:old-target"),
        )
        .with_voxel_volume_data_hash("voxel-volume/test-room", "fnv1a64:new-target");

    let err = execute_load_plan(&plan, &artifacts).unwrap_err();
    match err {
        LoadExecutionError::VoxelAnnotationInvalid {
            path, diagnostics, ..
        } => {
            assert_eq!(path, "annotations/semantic.avann.json");
            assert!(diagnostics.iter().any(|diagnostic| {
                diagnostic.code
                    == protocol_voxel_annotation::VoxelAnnotationDiagnosticCode::TargetVoxelHashMismatch
            }));
        }
        other => panic!("expected stale annotation target hash, got {other:?}"),
    }
}

#[test]
fn staged_commit_swaps_only_on_success() {
    let mut stage = ProjectBundleStage::empty();
    // First load commits a live ProjectBundle load.
    stage
        .load_and_commit(&sample_plan(), &sample_artifacts())
        .expect("first load commits");
    assert!(stage.has_live());
    let original_hash = stage.live_spatial_session_hash().unwrap();

    // A second, failing load (missing scene artifact) must NOT mutate the live
    // load: the previous ProjectBundle load stays committed, unchanged.
    let broken = BundleArtifacts::new().with_artifact("assets/lock.json", "{}\n");
    let err = stage.load_and_commit(&sample_plan(), &broken).unwrap_err();
    assert!(matches!(err, LoadExecutionError::MissingArtifact { .. }));
    assert_eq!(
        stage.live_spatial_session_hash().unwrap(),
        original_hash,
        "a failed load must leave the live ProjectBundle load unchanged (no partial commit)"
    );
    assert_eq!(stage.live().unwrap().spatial_session.entity_count(), 2);
}

#[test]
fn optional_cache_absence_does_not_block_load() {
    // The plan never references a cache artifact, so a bundle missing its cache
    // loads identically. Proven by the valid load above + this explicit check
    // that no cache path is required.
    let artifacts = sample_artifacts(); // no cache/* entries at all
    assert!(execute_load_plan(&sample_plan(), &artifacts).is_ok());
}

fn sample_annotation_json(target_hash: &str) -> String {
    use protocol_voxel_annotation::{
        VoxelAnnotationBounds, VoxelAnnotationContentHashes, VoxelAnnotationCoord,
        VoxelAnnotationKind, VoxelAnnotationLayer, VoxelAnnotationProvenanceKind,
        VoxelAnnotationProvenanceRef, VoxelAnnotationRegion, VoxelAnnotationSelection,
        VoxelAnnotationSparseRun, VOXEL_ANNOTATION_MEDIA_TYPE, VOXEL_ANNOTATION_SCHEMA_VERSION,
    };

    fn coord(x: i64, y: i64, z: i64) -> VoxelAnnotationCoord {
        VoxelAnnotationCoord { x, y, z }
    }

    fn bounds(
        min_x: i64,
        min_y: i64,
        min_z: i64,
        max_x: i64,
        max_y: i64,
        max_z: i64,
    ) -> VoxelAnnotationBounds {
        VoxelAnnotationBounds {
            min: coord(min_x, min_y, min_z),
            max: coord(max_x, max_y, max_z),
        }
    }

    let layer = VoxelAnnotationLayer {
        layer_id: "voxel-annotation/test-room/semantic".to_string(),
        schema_version: VOXEL_ANNOTATION_SCHEMA_VERSION,
        media_type: VOXEL_ANNOTATION_MEDIA_TYPE.to_string(),
        target_voxel_volume_asset_id: "voxel-volume/test-room".to_string(),
        target_voxel_data_hash: target_hash.to_string(),
        target_bounds: bounds(0, 0, 0, 9, 9, 9),
        regions: vec![VoxelAnnotationRegion {
            region_id: "region/spawn".to_string(),
            label: "Spawn".to_string(),
            kind: VoxelAnnotationKind::SpawnArea,
            tags: vec!["entry".to_string()],
            parent_region_id: None,
            bounds: bounds(1, 1, 1, 3, 1, 1),
            selection: VoxelAnnotationSelection {
                sparse_runs: vec![VoxelAnnotationSparseRun {
                    start: coord(1, 1, 1),
                    length: 3,
                }],
            },
        }],
        provenance: vec![VoxelAnnotationProvenanceRef {
            kind: VoxelAnnotationProvenanceKind::Authored,
            uri: "asha://fixture/annotation".to_string(),
            content_hash: "fnv1a64:source".to_string(),
        }],
        content_hashes: VoxelAnnotationContentHashes {
            canonical_json: String::new(),
            membership_data: String::new(),
        },
        validation_diagnostics: Vec::new(),
    };
    let layer = svc_voxel_annotation::with_computed_hashes(&layer);
    serde_json::to_string_pretty(&layer).expect("annotation layer serializes")
}
