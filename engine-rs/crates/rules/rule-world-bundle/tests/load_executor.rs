//! Integration tests for the ordered world-bundle load executor (#2361).
//!
//! Exercises a minimal valid bundle through the *real* executor (not a plan
//! builder), plus the classified failure paths: missing durable artifact, an
//! invalid scene, a missing asset lock, an unsupported version, and an
//! out-of-order plan. A golden stage summary pins the executed-stage readback.

use core_ids::{SceneId, SceneNodeId, WorldId};
use core_scene::{encode, SceneMetadata, SceneNode, SceneNodeKind, SceneTree};
use svc_serialization::{LoadPlan, LoadStage, LoadStep};

use rule_world_bundle::{execute_load_plan, BundleArtifacts, LoadExecutionError, WorldStage};

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
                bundle_schema_version: 1,
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
                world: WorldId::new(7),
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
    assert_eq!(result.world.entity_count(), 2);
    assert_eq!(result.bootstrap.source_trace.len(), 2);
    assert_eq!(result.bootstrap.world_id, WorldId::new(7));
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
    let world_hash = result.world_hash.0;
    let expected = format!(
        "stage versions schema=1 protocol=1\n\
         stage assetLock artifact=assets/lock.json expectedAssets=1\n\
         stage sceneDocument artifact=scene/scene.json nodes=2\n\
         stage bootstrap world=7 entities=2\n\
         stage finalValidation worldHash={world_hash:016x} ok\n\
         result entities=2 voxel=false worldHash={world_hash:016x}\n\
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
                bundle_schema_version: 1,
                protocol_version: 1,
            },
            LoadStep::LoadAssetLock {
                artifact: "assets/lock.json".into(),
                asset_count: 1,
            },
            LoadStep::BootstrapScene {
                scene: SceneId::new(100),
                world: WorldId::new(7),
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
        },
    );

    let artifacts = sample_artifacts()
        .with_artifact("voxel/edits.log", encode_edit_log(&events))
        .with_voxel_spec(spec);

    let result = execute_load_plan(&plan, &artifacts).expect("voxel load succeeds");
    let voxel = result.voxel.expect("voxel authority present");
    assert!(voxel.tracked_len() >= 1, "the generated chunk is resident");
    // Scene authority is still intact alongside voxel authority.
    assert_eq!(result.world.entity_count(), 2);
}

#[test]
fn voxel_section_without_spec_fails_closed() {
    let mut plan = sample_plan();
    plan.steps.insert(
        3,
        LoadStep::ApplyVoxelEdits {
            edit_logs: vec!["voxel/edits.log".into()],
            snapshots: vec![],
        },
    );
    // Provide the log but no voxel spec.
    let artifacts = sample_artifacts().with_artifact("voxel/edits.log", "voxel-log\n");
    let err = execute_load_plan(&plan, &artifacts).unwrap_err();
    assert!(matches!(err, LoadExecutionError::VoxelSpecMissing));
}

#[test]
fn staged_commit_swaps_only_on_success() {
    let mut stage = WorldStage::empty();
    // First load commits a live world.
    stage
        .load_and_commit(&sample_plan(), &sample_artifacts())
        .expect("first load commits");
    assert!(stage.has_live());
    let original_hash = stage.live_world_hash().unwrap();

    // A second, failing load (missing scene artifact) must NOT mutate the live
    // world: the previous world stays committed, unchanged.
    let broken = BundleArtifacts::new().with_artifact("assets/lock.json", "{}\n");
    let err = stage.load_and_commit(&sample_plan(), &broken).unwrap_err();
    assert!(matches!(err, LoadExecutionError::MissingArtifact { .. }));
    assert_eq!(
        stage.live_world_hash().unwrap(),
        original_hash,
        "a failed load must leave the live world unchanged (no partial commit)"
    );
    assert_eq!(stage.live().unwrap().world.entity_count(), 2);
}

#[test]
fn optional_cache_absence_does_not_block_load() {
    // The plan never references a cache artifact, so a bundle missing its cache
    // loads identically. Proven by the valid load above + this explicit check
    // that no cache path is required.
    let artifacts = sample_artifacts(); // no cache/* entries at all
    assert!(execute_load_plan(&sample_plan(), &artifacts).is_ok());
}
