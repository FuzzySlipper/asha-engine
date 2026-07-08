//! End-to-end runtime session-state snapshot save → reload through the real load
//! executor (post-launchable-02, Den task #2484).
//!
//! Builds a mixed runtime entity store (scene-sourced + runtime-created, spatial +
//! non-spatial, rendered/collider/logical, contained/attached, asset-bound),
//! composes the durable `sessionStateSnapshot` artifact, drives the real
//! [`execute_load_plan`] over a plan that includes the
//! [`LoadStep::RestoreSessionState`] stage, and asserts the reloaded runtime entity
//! authority reproduces the pre-save fingerprint exactly. A corrupted snapshot
//! must fail closed with a classified [`LoadExecutionError::SessionStateDecode`]
//! rather than partially mutate authority.

use core_entity::{
    ControllerCapability, EntityLifecycleCommand, EntitySource, EntityStore, EntityTransform,
    RelationCommand,
};
use core_ids::{EntityId, ProcessId, SceneId, SceneNodeId, SubjectId, TagId, WorldId};
use core_scene::{encode, SceneMetadata, SceneNode, SceneNodeKind, SceneTree};
use svc_serialization::{LoadPlan, LoadStep};

use rule_world_bundle::{
    compose_session_state_snapshot, execute_load_plan, BundleArtifacts, LoadExecutionError,
    SESSION_STATE_SNAPSHOT_PATH,
};

/// A small valid scene (id 100), matching the bootstrap baseline.
fn sample_scene_json() -> String {
    let tree = SceneTree {
        id: SceneId::new(100),
        schema_version: 1,
        metadata: SceneMetadata {
            name: Some("session-state-fixture".into()),
            authoring_format_version: 1,
        },
        dependencies: vec![],
        roots: vec![SceneNode::leaf(
            SceneNodeId::new(1),
            SceneNodeKind::EmptyGroup,
        )],
    };
    encode(&tree.to_flat())
}

/// The mixed runtime authority store: every #2484 fixture vocabulary class.
fn mixed_runtime_store() -> EntityStore {
    let mut store = EntityStore::new();
    let mk = |store: &mut EntityStore, id: u64, source, labels: Vec<u64>| {
        store
            .apply(EntityLifecycleCommand::Create {
                id: EntityId::new(id),
                source,
                labels: labels.into_iter().map(TagId::new).collect(),
            })
            .unwrap();
    };

    // spatial rendered (scene-sourced, transform diverged from origin)
    mk(
        &mut store,
        1,
        EntitySource::SceneBootstrap {
            node: SceneNodeId::new(1),
        },
        vec![3],
    );
    store.attach_transform(
        EntityId::new(1),
        EntityTransform::at(core_vec3(4.0, 0.0, -1.0)),
    );
    store.attach_render_projection(EntityId::new(1), true);

    // spatial non-rendered collider (runtime-created)
    mk(
        &mut store,
        2,
        EntitySource::RuntimeCreated {
            by: Some(ProcessId::new(9)),
        },
        vec![],
    );
    store.attach_transform(EntityId::new(2), EntityTransform::IDENTITY);
    store.attach_collision(EntityId::new(2), true);

    // non-spatial logical (no transform)
    mk(
        &mut store,
        3,
        EntitySource::PolicyProposed {
            by: SubjectId::new(2),
        },
        vec![1],
    );
    store.attach_controller(
        EntityId::new(3),
        ControllerCapability::Subject(SubjectId::new(2)),
    );

    // contained member
    mk(
        &mut store,
        4,
        EntitySource::RuntimeCreated { by: None },
        vec![],
    );
    store
        .apply_relation(RelationCommand::SetContainment {
            member: EntityId::new(4),
            container: EntityId::new(2),
        })
        .unwrap();

    // attached child + asset binding + source ancestry
    mk(
        &mut store,
        5,
        EntitySource::RuntimeCreated { by: None },
        vec![],
    );
    store.attach_transform(
        EntityId::new(5),
        EntityTransform::at(core_vec3(0.0, 1.0, 0.0)),
    );
    store.attach_asset_binding(
        EntityId::new(5),
        core_assets::AssetReference::new(
            core_assets::AssetId::parse("mesh/crate").unwrap(),
            core_assets::AssetVersionReq::Any,
            None,
        ),
    );
    store
        .apply_relation(RelationCommand::AttachTransformParent {
            child: EntityId::new(5),
            parent: EntityId::new(1),
        })
        .unwrap();
    store
        .apply_relation(RelationCommand::SetDerivedFrom {
            derived: EntityId::new(5),
            origin: EntityId::new(4),
        })
        .unwrap();

    store
}

fn core_vec3(x: f32, y: f32, z: f32) -> core_math::Vec3 {
    core_math::Vec3::new(x, y, z)
}

/// A plan that bootstraps the scene baseline then restores the runtime snapshot.
fn plan_with_spatial_session_state() -> LoadPlan {
    LoadPlan {
        steps: vec![
            LoadStep::ValidateVersions {
                bundle_schema_version: 1,
                protocol_version: 1,
            },
            LoadStep::LoadAssetLock {
                artifact: "assets/lock.json".into(),
                asset_count: 0,
            },
            LoadStep::LoadSceneDocument {
                artifact: "scene/scene.json".into(),
                scene: SceneId::new(100),
            },
            LoadStep::BootstrapScene {
                scene: SceneId::new(100),
                world: WorldId::new(7),
            },
            LoadStep::RestoreSessionState {
                artifact: SESSION_STATE_SNAPSHOT_PATH.into(),
            },
            LoadStep::ValidateFinalState,
        ],
    }
}

fn artifacts_with(snapshot_text: &str) -> BundleArtifacts {
    BundleArtifacts::new()
        .with_artifact("assets/lock.json", "{ \"entries\": [] }\n")
        .with_artifact("scene/scene.json", sample_scene_json())
        .with_artifact(SESSION_STATE_SNAPSHOT_PATH, snapshot_text.to_string())
}

#[test]
fn mixed_runtime_spatial_session_state_survives_save_reload() {
    let store = mixed_runtime_store();
    let artifact = compose_session_state_snapshot(&store.snapshot_durable());

    let result = execute_load_plan(
        &plan_with_spatial_session_state(),
        &artifacts_with(&artifact.text),
    )
    .expect("load with session-state restore succeeds");

    let restored = result
        .runtime_entities
        .expect("runtime entities restored from session-state snapshot");
    assert_eq!(
        restored.hash(),
        store.hash(),
        "reloaded runtime authority must reproduce the pre-save entity fingerprint"
    );
    // The scene baseline still bootstrapped one entity from the scene document.
    assert_eq!(result.world.entity_count(), 1);
    // Capability presence/absence survived: the logical entity has no transform.
    assert!(restored.transform(EntityId::new(3)).is_none());
    assert!(restored.collision(EntityId::new(2)).is_some());
    assert_eq!(
        restored.containment(EntityId::new(4)).map(|c| c.container),
        Some(EntityId::new(2))
    );
    assert_eq!(
        restored.transform_parent_of(EntityId::new(5)),
        Some(EntityId::new(1))
    );
    assert_eq!(
        restored.derived_from(EntityId::new(5)),
        Some(EntityId::new(4))
    );
}

#[test]
fn corrupt_session_state_snapshot_fails_closed_classified() {
    let store = mixed_runtime_store();
    let artifact = compose_session_state_snapshot(&store.snapshot_durable());
    // Corrupt the source discriminant: a plausible on-disk edit that no longer
    // names a known source kind.
    let tampered = artifact.text.replace("sceneBootstrap", "bogusSourceKind");

    let err = execute_load_plan(
        &plan_with_spatial_session_state(),
        &artifacts_with(&tampered),
    )
    .expect_err("a corrupt snapshot must not load");
    assert!(
        matches!(err, LoadExecutionError::SessionStateDecode { .. }),
        "expected a classified SessionStateDecode error, got {err:?}"
    );
}

#[test]
fn voxel_edit_and_entity_change_survive_the_same_save() {
    use core_events::VoxelEditEvent;
    use core_space::{ChunkCoord, ChunkDims, GridId, VoxelCoord, VoxelGridSpec};
    use core_voxel::VoxelValue;
    use rule_voxel_edit::generate_chunk;
    use rule_voxel_edit::persist::encode_edit_log;

    let spec = VoxelGridSpec::new(GridId::new(0), 1.0, ChunkDims::cubic(4).unwrap()).unwrap();
    let chunk = ChunkCoord::new(0, 0, 0);
    let gen = generate_chunk(&spec, chunk, 7, 1);
    let edit_log = vec![
        VoxelEditEvent::ChunkGenerated {
            grid: GridId::new(0),
            chunk,
            seed: 7,
            generator_version: 1,
            hash: gen.content_hash().0,
        },
        VoxelEditEvent::VoxelSet {
            grid: GridId::new(0),
            coord: VoxelCoord::new(0, 3, 0),
            value: VoxelValue::solid_raw(2),
        },
    ];

    let store = mixed_runtime_store();
    let snapshot = compose_session_state_snapshot(&store.snapshot_durable());

    // One bundle, one save: a voxel section AND a runtime session-state snapshot.
    let artifacts = BundleArtifacts::new()
        .with_artifact("assets/lock.json", "{ \"entries\": [] }\n")
        .with_artifact("scene/scene.json", sample_scene_json())
        .with_artifact("voxel/edits.log", encode_edit_log(&edit_log))
        .with_artifact(SESSION_STATE_SNAPSHOT_PATH, snapshot.text.clone())
        .with_voxel_spec(spec);

    let plan = LoadPlan {
        steps: vec![
            LoadStep::ValidateVersions {
                bundle_schema_version: 1,
                protocol_version: 1,
            },
            LoadStep::LoadAssetLock {
                artifact: "assets/lock.json".into(),
                asset_count: 0,
            },
            LoadStep::LoadSceneDocument {
                artifact: "scene/scene.json".into(),
                scene: SceneId::new(100),
            },
            LoadStep::ApplyVoxelEdits {
                edit_logs: vec!["voxel/edits.log".into()],
                snapshots: vec![],
            },
            LoadStep::BootstrapScene {
                scene: SceneId::new(100),
                world: WorldId::new(7),
            },
            LoadStep::RestoreSessionState {
                artifact: SESSION_STATE_SNAPSHOT_PATH.into(),
            },
            LoadStep::ValidateFinalState,
        ],
    };

    let result = execute_load_plan(&plan, &artifacts).expect("mixed bundle loads");
    // Both authority lanes reloaded: voxel content and the runtime entity store.
    assert!(result.voxel.is_some(), "voxel authority must reload");
    assert_eq!(
        result.runtime_entities.expect("runtime entities").hash(),
        store.hash(),
        "entity authority must reload alongside voxel edits"
    );
}

#[test]
fn spatial_session_state_stage_is_skipped_when_no_divergence() {
    // A plan without the restore step (no divergence saved) loads cleanly and
    // carries no runtime entities — the scene-only baseline.
    let plan = LoadPlan {
        steps: plan_with_spatial_session_state()
            .steps
            .into_iter()
            .filter(|s| !matches!(s, LoadStep::RestoreSessionState { .. }))
            .collect(),
    };
    let result = execute_load_plan(&plan, &artifacts_with("")).expect("scene-only load succeeds");
    assert!(result.runtime_entities.is_none());
}
