use super::*;

#[test]
fn engine_bridge_has_one_fixed_capability_cell_contract() {
    let ids = ENGINE_BRIDGE_CAPABILITY_PORTS
        .iter()
        .map(|contract| contract.id)
        .collect::<Vec<_>>();
    assert_eq!(
        ids,
        [
            "input",
            "timeSimulation",
            "sceneEntities",
            "voxelAssetsBuffers",
            "camera",
            "gameplay",
            "projection",
            "workspaceAuthoring",
            "runtimeProjectLifecycle",
            "replayEvidence",
        ]
    );
    let unique_ids = ids.iter().copied().collect::<BTreeSet<_>>();
    assert_eq!(unique_ids.len(), ids.len());

    let lifecycle = ENGINE_BRIDGE_CAPABILITY_PORTS
        .iter()
        .find(|contract| contract.id == "runtimeProjectLifecycle")
        .unwrap();
    assert_eq!(lifecycle.initialization, "createsEngine");
    assert_eq!(lifecycle.runtime_project, "ownsProjectLifecycle");
    assert_eq!(lifecycle.snapshot_hash, "activeProjectContent");
    assert_eq!(lifecycle.resource_lifetime, "session");

    let workspace_authoring = ENGINE_BRIDGE_CAPABILITY_PORTS
        .iter()
        .find(|contract| contract.id == "workspaceAuthoring")
        .unwrap();
    assert_eq!(workspace_authoring.initialization, "createsEngine");
    assert_eq!(workspace_authoring.runtime_project, "ownsProjectLifecycle");
    assert_eq!(
        workspace_authoring.snapshot_hash,
        "workspaceAuthoringAuthority"
    );

    let buffers = ENGINE_BRIDGE_CAPABILITY_PORTS
        .iter()
        .find(|contract| contract.id == "voxelAssetsBuffers")
        .unwrap();
    assert_eq!(buffers.resource_lifetime, "mixedExplicitAndSession");

    let bridge = EngineBridge::new();
    assert!(bridge.runtime_project.engine.is_none());
    assert!(bridge.scene.scene_document.is_none());
    assert!(bridge.voxel.voxel.is_none());
    assert!(bridge.gameplay.fps_session.is_none());
    assert!(bridge.projection.projection_frame.is_none());
}

#[test]
fn step_before_init_is_typed_error() {
    let mut bridge = EngineBridge::new();
    let err = bridge
        .step_simulation(StepInputEnvelope { tick: 1 })
        .unwrap_err();
    assert_eq!(err.kind, RuntimeBridgeErrorKind::NotInitialized);
    assert_eq!(err.category(), ErrorCategory::Unsupported);
}

#[test]
fn enemy_direct_nav_movement_routes_through_rust_entity_authority() {
    let mut bridge = EngineBridge::new();
    bridge.initialize_engine(EngineConfig { seed: 7 }).unwrap();

    let first = bridge
        .apply_enemy_direct_nav_movement(EnemyDirectNavMovementRequest {
            entity: 777,
            seed_position: Vec3::new(0.0, 0.5, -2.6),
            target: Vec3::new(0.0, 1.62, 1.25),
            max_step_units: 0.35,
        })
        .unwrap();
    assert_eq!(
        first.authority_source,
        EnemyDirectNavAuthoritySource::SeededFromRequest
    );
    assert_eq!(first.from, Vec3::new(0.0, 0.5, -2.6));
    assert_eq!(first.next_waypoint, Vec3::new(0.0, 0.598, -2.264));
    assert_eq!(first.path_hash, 0x69ed_74d6_9292_2db7);
    assert_ne!(first.transform_hash, 0);

    let second = bridge
        .apply_enemy_direct_nav_movement(EnemyDirectNavMovementRequest {
            entity: 777,
            seed_position: Vec3::new(99.0, 99.0, 99.0),
            target: Vec3::new(0.0, 1.62, 1.25),
            max_step_units: 0.35,
        })
        .unwrap();
    assert_eq!(
        second.authority_source,
        EnemyDirectNavAuthoritySource::RustEntityStore
    );
    assert_eq!(
        second.from, first.next_waypoint,
        "Rust store, not a stale TS seed, owns the next starting transform"
    );
    assert_ne!(second.next_waypoint, first.next_waypoint);
}

#[test]
fn enemy_direct_nav_movement_fails_closed_on_invalid_request() {
    let mut bridge = EngineBridge::new();
    let before_init = bridge
        .apply_enemy_direct_nav_movement(EnemyDirectNavMovementRequest {
            entity: 1,
            seed_position: Vec3::ZERO,
            target: Vec3::ZERO,
            max_step_units: 0.35,
        })
        .unwrap_err();
    assert_eq!(before_init.kind, RuntimeBridgeErrorKind::NotInitialized);

    bridge.initialize_engine(EngineConfig { seed: 7 }).unwrap();
    let invalid_entity = bridge
        .apply_enemy_direct_nav_movement(EnemyDirectNavMovementRequest {
            entity: 0,
            seed_position: Vec3::ZERO,
            target: Vec3::ZERO,
            max_step_units: 0.35,
        })
        .unwrap_err();
    assert_eq!(invalid_entity.kind, RuntimeBridgeErrorKind::InvalidInput);

    let invalid_step = bridge
        .apply_enemy_direct_nav_movement(EnemyDirectNavMovementRequest {
            entity: 1,
            seed_position: Vec3::ZERO,
            target: Vec3::new(1.0, 0.0, 0.0),
            max_step_units: 0.0,
        })
        .unwrap_err();
    assert_eq!(invalid_step.kind, RuntimeBridgeErrorKind::InvalidInput);
}

#[test]
fn camera_view_surface_round_trips_and_fails_closed() {
    use protocol_view::{
        CameraHandle, CameraPose, FirstPersonCameraInput, PerspectiveProjection, ViewportSize,
    };

    let mut bridge = EngineBridge::new();
    let request = CameraCreateRequest {
        initial_pose: CameraPose {
            position: [0.0, 1.6, 0.0],
            yaw_degrees: 0.0,
            pitch_degrees: 0.0,
        },
        projection: PerspectiveProjection {
            fov_y_degrees: 60.0,
            near: 0.1,
            far: 1000.0,
        },
        viewport: ViewportSize {
            width: 1280,
            height: 720,
        },
    };
    assert_eq!(
        bridge.create_camera(request).unwrap_err().kind,
        RuntimeBridgeErrorKind::NotInitialized
    );

    bridge.initialize_engine(EngineConfig { seed: 1 }).unwrap();
    let created = bridge.create_camera(request).unwrap();
    assert_eq!(created.camera.raw(), 1);
    assert_eq!(created.pose, request.initial_pose);

    let moved = bridge
        .apply_first_person_camera_input(FirstPersonCameraInputEnvelope {
            camera: created.camera,
            tick: 1,
            input: FirstPersonCameraInput {
                move_forward: 1.0,
                move_right: 0.0,
                move_up: 0.0,
                yaw_delta_degrees: 15.0,
                pitch_delta_degrees: -5.0,
                dt_seconds: 1.0 / 60.0,
                move_speed_units_per_second: 3.0,
            },
        })
        .unwrap();
    assert_eq!(moved.tick, 1);
    assert_ne!(moved.pose, created.pose);

    let projected = bridge
        .read_camera_projection(CameraProjectionRequest {
            camera: moved.camera,
            viewport: None,
        })
        .unwrap();
    assert_eq!(projected.view_matrix.len(), 16);
    assert_eq!(projected.projection_hash, "fnv1a64:071327a4920ab097");

    assert_eq!(
        bridge
            .read_camera_projection(CameraProjectionRequest {
                camera: moved.camera,
                viewport: Some(ViewportSize {
                    width: 1280,
                    height: 0,
                }),
            })
            .unwrap_err()
            .kind,
        RuntimeBridgeErrorKind::InvalidInput
    );

    assert_eq!(
        bridge
            .read_camera_projection(CameraProjectionRequest {
                camera: CameraHandle::new(999),
                viewport: None,
            })
            .unwrap_err()
            .kind,
        RuntimeBridgeErrorKind::UnknownHandle
    );
}

#[test]
fn init_then_step_is_deterministic() {
    let mut bridge = EngineBridge::new();
    let h = bridge.initialize_engine(EngineConfig { seed: 7 }).unwrap();
    assert_eq!(h.raw(), 7);
    let r = bridge
        .step_simulation(StepInputEnvelope { tick: 6 })
        .unwrap();
    assert_eq!(
        r,
        StepResult {
            tick: 6,
            diff_count: 0
        }
    );
}
