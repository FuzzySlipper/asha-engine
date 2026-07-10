use super::*;

fn set_voxel(coord: VoxelCoord, material: u16) -> VoxelCommand {
    VoxelCommand::SetVoxel {
        grid: GridId::new(1),
        coord,
        value: VoxelValue::solid_raw(material),
    }
}

#[test]
fn submit_before_init_fails_closed() {
    let mut bridge = EngineBridge::new();
    let err = bridge.submit_commands(CommandBatch::default()).unwrap_err();
    assert_eq!(err.kind, RuntimeBridgeErrorKind::NotInitialized);
}

#[test]
fn accepted_voxel_command_mutates_authority_and_marks_dirty() {
    let mut bridge = init_bridge();
    // The batch carries a generated VoxelCommand — not a `{ kind }` placeholder.
    let result = bridge
        .submit_commands(CommandBatch {
            commands: vec![set_voxel(VoxelCoord::new(0, 0, 0), 1)],
        })
        .unwrap();
    assert_eq!(result.accepted, 1);
    assert_eq!(result.rejected, 0);
    assert!(result.rejections.is_empty());

    let world = bridge.voxel.as_ref().unwrap();
    let chunk = world.get(ChunkCoord::new(0, 0, 0)).unwrap();
    assert_eq!(
        chunk.get(LocalVoxelCoord::new(0, 0, 0)),
        Some(VoxelValue::solid_raw(1)),
        "authority voxel state changed"
    );
    assert!(
        world.is_dirty(ChunkCoord::new(0, 0, 0)),
        "the edited chunk is marked dirty"
    );
}

#[test]
fn rejected_unknown_material_is_classified_and_does_not_mutate() {
    let mut bridge = init_bridge();
    let before = bridge
        .voxel
        .as_ref()
        .unwrap()
        .get(ChunkCoord::new(0, 0, 0))
        .unwrap()
        .content_hash();

    let result = bridge
        .submit_commands(CommandBatch {
            commands: vec![set_voxel(VoxelCoord::new(0, 0, 0), 99)],
        })
        .unwrap();
    assert_eq!(result.accepted, 0);
    assert_eq!(result.rejected, 1);
    assert!(matches!(
        result.rejections[0],
        VoxelEditRejection::UnknownMaterial(_)
    ));

    let after = bridge
        .voxel
        .as_ref()
        .unwrap()
        .get(ChunkCoord::new(0, 0, 0))
        .unwrap()
        .content_hash();
    assert_eq!(
        before, after,
        "a rejected command must not mutate authority"
    );
}

#[test]
fn rejected_non_resident_chunk_is_classified() {
    let mut bridge = init_bridge();
    let result = bridge
        .submit_commands(CommandBatch {
            commands: vec![set_voxel(VoxelCoord::new(100, 0, 0), 1)],
        })
        .unwrap();
    assert_eq!(result.rejected, 1);
    assert!(matches!(
        result.rejections[0],
        VoxelEditRejection::ChunkNotResident { .. }
    ));
}

#[test]
fn collision_constrained_camera_blocks_terrain_and_allows_empty_space() {
    use protocol_view::{CameraPose, PerspectiveProjection, ViewportSize};

    let mut bridge = init_bridge();
    let camera = bridge
        .create_camera(CameraCreateRequest {
            initial_pose: CameraPose {
                position: [1.5, 1.5, 1.3],
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
        })
        .unwrap();
    let shape = CameraCollisionShape {
        half_extents: [0.2, 0.2, 0.2],
    };
    let policy = CameraCollisionPolicy {
        mode: CameraCollisionPolicyMode::AxisSeparableSlide,
        max_iterations: 3,
    };
    let blocked = bridge
        .apply_collision_constrained_camera_input(CollisionConstrainedCameraInputEnvelope {
            camera: camera.camera,
            grid: 1,
            input: FirstPersonCameraInput {
                move_forward: 1.0,
                move_right: 0.0,
                move_up: 0.0,
                yaw_delta_degrees: 0.0,
                pitch_delta_degrees: 0.0,
                dt_seconds: 1.0,
                move_speed_units_per_second: 1.0,
            },
            tick: 1,
            shape,
            policy,
        })
        .unwrap();
    assert!(blocked.collision.collided);
    assert_eq!(blocked.collision.blocked_axes, vec![CollisionAxis::Z]);
    assert_eq!(blocked.after.pose.position, camera.pose.position);
    assert!(blocked.movement_hash.starts_with("fnv1a64:"));

    let clear = bridge
        .apply_collision_constrained_camera_input(CollisionConstrainedCameraInputEnvelope {
            camera: camera.camera,
            grid: 1,
            input: FirstPersonCameraInput {
                move_forward: -1.0,
                move_right: 0.0,
                move_up: 0.0,
                yaw_delta_degrees: 0.0,
                pitch_delta_degrees: 0.0,
                dt_seconds: 1.0,
                move_speed_units_per_second: 1.0,
            },
            tick: 2,
            shape,
            policy,
        })
        .unwrap();
    assert!(!clear.collision.collided);
    assert_eq!(clear.collision.blocked_axes, Vec::<CollisionAxis>::new());
    assert_eq!(clear.after.pose.position, [1.5, 1.5, 2.3]);
}

#[test]
fn select_voxel_derives_center_ray_and_edit_anchor_from_camera() {
    use protocol_view::{CameraPose, PerspectiveProjection, ViewportSize};

    let mut bridge = init_bridge();
    let camera = bridge
        .create_camera(CameraCreateRequest {
            initial_pose: CameraPose {
                position: [1.5, 1.5, 4.0],
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
        })
        .unwrap();
    let selection = bridge
        .select_voxel(ScreenPointToPickRayRequest {
            camera: camera.camera,
            grid: 1,
            viewport: None,
            screen_point: ScreenPoint {
                x: 0.5,
                y: 0.5,
                space: ScreenPointSpace::Normalized01,
            },
            max_distance: 10.0,
        })
        .unwrap();
    assert_eq!(selection.pick_ray.direction, [0.0, 0.0, -1.0]);
    assert_eq!(selection.selected_voxel, Some(VoxelCoord::new(1, 1, 0)));
    assert_eq!(selection.selected_face, Some(Face::PosZ));
    assert_eq!(selection.edit_anchor, Some(VoxelCoord::new(1, 1, 1)));
    assert!(selection
        .pick_ray
        .camera_projection_hash
        .starts_with("fnv1a64:"));
    assert!(selection.selection_hash.starts_with("fnv1a64:"));
}

#[test]
fn select_voxel_reports_miss_for_out_of_range_crosshair() {
    use protocol_view::{CameraPose, PerspectiveProjection, ViewportSize};

    let mut bridge = init_bridge();
    let camera = bridge
        .create_camera(CameraCreateRequest {
            initial_pose: CameraPose {
                position: [1.5, 1.5, 4.0],
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
        })
        .unwrap();
    let selection = bridge
        .select_voxel(ScreenPointToPickRayRequest {
            camera: camera.camera,
            grid: 1,
            viewport: None,
            screen_point: ScreenPoint {
                x: 0.5,
                y: 0.5,
                space: ScreenPointSpace::Normalized01,
            },
            max_distance: 1.0,
        })
        .unwrap();
    assert_eq!(selection.outcome, VoxelSelectionOutcome::Miss);
    assert_eq!(selection.selected_voxel, None);
    assert_eq!(selection.edit_anchor, None);
}

#[test]
fn mesh_evidence_reports_fixture_chunks_and_changes_after_edit() {
    let mut bridge = init_bridge();
    let before = bridge
        .read_voxel_mesh_evidence(VoxelMeshEvidenceRequest {
            grid: 1,
            chunks: vec![ChunkCoord::new(0, 0, 0)],
        })
        .unwrap();
    assert_eq!(before.fixture_id, "basic-voxel-landscape-interaction");
    assert_eq!(before.voxel_state_hash, "27f89a36b51a8cb7");
    assert_eq!(before.meshing_strategy, "visible-face");
    assert_eq!(before.chunks.len(), 1);
    let before_chunk = &before.chunks[0];
    assert!(before_chunk.resident);
    assert!(before_chunk.visible);
    let before_hash = before_chunk.mesh_hash.clone().expect("mesh hash");
    assert_eq!(before_chunk.material_slots, vec![1]);
    assert_eq!(before_chunk.stats.unwrap().quads, 12);

    bridge
        .submit_commands(CommandBatch {
            commands: vec![set_voxel(VoxelCoord::new(1, 1, 1), 2)],
        })
        .unwrap();
    let after = bridge
        .read_voxel_mesh_evidence(VoxelMeshEvidenceRequest {
            grid: 1,
            chunks: vec![ChunkCoord::new(0, 0, 0)],
        })
        .unwrap();
    let after_chunk = &after.chunks[0];
    assert_ne!(after.voxel_state_hash, before.voxel_state_hash);
    assert_ne!(after_chunk.mesh_hash.as_ref().unwrap(), &before_hash);
    assert_eq!(after_chunk.material_slots, vec![1, 2]);
    assert!(after_chunk.stats.unwrap().quads > before_chunk.stats.unwrap().quads);
}

#[test]
fn mesh_evidence_fails_closed_before_init_and_unknown_grid() {
    let bridge = EngineBridge::new();
    assert_eq!(
        bridge
            .read_voxel_mesh_evidence(VoxelMeshEvidenceRequest {
                grid: 1,
                chunks: Vec::new(),
            })
            .unwrap_err()
            .kind,
        RuntimeBridgeErrorKind::NotInitialized
    );

    let bridge = init_bridge();
    assert_eq!(
        bridge
            .read_voxel_mesh_evidence(VoxelMeshEvidenceRequest {
                grid: 999,
                chunks: Vec::new(),
            })
            .unwrap_err()
            .kind,
        RuntimeBridgeErrorKind::InvalidInput
    );
}

#[test]
fn mixed_batch_accepts_valid_and_classifies_invalid_in_order() {
    let mut bridge = init_bridge();
    let result = bridge
        .submit_commands(CommandBatch {
            commands: vec![
                set_voxel(VoxelCoord::new(1, 0, 0), 2), // resident, known material → accept
                set_voxel(VoxelCoord::new(0, 0, 0), 77), // unknown material → reject
            ],
        })
        .unwrap();
    assert_eq!(result.accepted, 1);
    assert_eq!(result.rejected, 1);
    assert!(matches!(
        result.rejections[0],
        VoxelEditRejection::UnknownMaterial(_)
    ));
}

// ── Voxel picking → Rust authority raycast (launchable-voxel, #2437) ──

/// A ray from x=-5 toward +X along y=0.5,z=0.5 — through voxel (0,0,0)'s span.
fn pick_ray_plus_x() -> PickRay {
    PickRay {
        grid: 1,
        origin: [-5.0, 0.5, 0.5],
        direction: [1.0, 0.0, 0.0],
        max_distance: 100.0,
    }
}

#[test]
fn pick_before_init_fails_closed() {
    let bridge = EngineBridge::new();
    let err = bridge.pick_voxel(pick_ray_plus_x()).unwrap_err();
    assert_eq!(err.kind, RuntimeBridgeErrorKind::NotInitialized);
}

#[test]
fn pick_hits_solid_voxel_with_authoritative_face() {
    let mut bridge = init_bridge();
    bridge
        .submit_commands(CommandBatch {
            commands: vec![set_voxel(VoxelCoord::new(0, 0, 0), 1)],
        })
        .unwrap();
    match bridge.pick_voxel(pick_ray_plus_x()).unwrap() {
        PickResult::Hit(hit) => {
            assert_eq!(hit.grid, 1);
            assert_eq!(hit.voxel, VoxelCoord::new(0, 0, 0));
            assert_eq!(hit.chunk, ChunkCoord::new(0, 0, 0));
            // The +X-travelling ray strikes the voxel's -X face.
            assert_eq!(hit.face, Face::NegX);
            assert!((hit.distance - 5.0).abs() < 1e-6);
        }
        PickResult::Miss(r) => panic!("expected a hit, got {r:?}"),
    }
}

#[test]
fn pick_empty_space_misses() {
    // The canonical launch terrain occupies z=0 only; a ray above the slab misses.
    let bridge = init_bridge();
    let mut ray = pick_ray_plus_x();
    ray.origin = [-5.0, 0.5, 1.5];
    assert_eq!(
        bridge.pick_voxel(ray).unwrap(),
        PickResult::Miss(PickRejection::NoHit)
    );
}

#[test]
fn pick_unknown_grid_fails_closed() {
    let bridge = init_bridge();
    let mut ray = pick_ray_plus_x();
    ray.grid = 999;
    let err = bridge.pick_voxel(ray).unwrap_err();
    assert_eq!(err.kind, RuntimeBridgeErrorKind::InvalidInput);
}

#[test]
fn buffer_view_round_trips_and_unknown_handle_errors() {
    let mut bridge = EngineBridge::new();
    bridge
        .initialize_engine(EngineConfig { seed: 0x01020304 })
        .unwrap();
    let view = bridge.get_buffer(RuntimeBufferHandle::new(0)).unwrap();
    assert_eq!(view.bytes, &0x01020304u64.to_le_bytes());
    let err = bridge.get_buffer(RuntimeBufferHandle::new(99)).unwrap_err();
    assert_eq!(err.kind, RuntimeBridgeErrorKind::UnknownHandle);
}
