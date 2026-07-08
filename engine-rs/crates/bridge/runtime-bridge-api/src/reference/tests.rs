use super::*;

#[test]
fn step_before_init_is_typed_error() {
    let mut bridge = ReferenceBridge::new();
    let err = bridge
        .step_simulation(StepInputEnvelope { tick: 1 })
        .unwrap_err();
    assert_eq!(err.kind, RuntimeBridgeErrorKind::NotInitialized);
    assert_eq!(err.category(), ErrorCategory::Unsupported);
}

#[test]
fn save_before_load_fails_closed() {
    let mut bridge = ReferenceBridge::new();
    let err = bridge.save_project_bundle().unwrap_err();
    assert_eq!(err.kind, RuntimeBridgeErrorKind::NotInitialized);
    // And status reflects no loaded ProjectBundle.
    assert_eq!(
        bridge
            .get_project_bundle_composition_status()
            .unwrap()
            .loaded_project_bundle,
        None
    );
}

#[test]
fn enemy_direct_nav_movement_routes_through_rust_entity_authority() {
    let mut bridge = ReferenceBridge::new();
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
    let mut bridge = ReferenceBridge::new();
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

    let mut bridge = ReferenceBridge::new();
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
fn load_save_status_unload_round_trip() {
    let mut bridge = ReferenceBridge::new();
    let status = bridge
        .load_project_bundle(ProjectBundleLoadRequest {
            bundle_schema_version: 1,
            protocol_version: 1,
            scene_id: 100,
        })
        .unwrap();
    assert_eq!(status.loaded_project_bundle, Some(100));
    assert!(!status.blocks_load);

    let save = bridge.save_project_bundle().unwrap();
    assert_eq!(save.artifacts_written, 3);

    assert_eq!(
        bridge
            .get_project_bundle_composition_status()
            .unwrap()
            .loaded_project_bundle,
        Some(100)
    );

    bridge.unload_project_bundle().unwrap();
    assert_eq!(
        bridge
            .get_project_bundle_composition_status()
            .unwrap()
            .loaded_project_bundle,
        None
    );
    // Save after unload fails closed again.
    assert_eq!(
        bridge.save_project_bundle().unwrap_err().kind,
        RuntimeBridgeErrorKind::NotInitialized
    );
}

#[test]
fn load_unsupported_version_fails_closed_without_mutating() {
    let mut bridge = ReferenceBridge::new();
    // Load a valid ProjectBundle first.
    bridge
        .load_project_bundle(ProjectBundleLoadRequest {
            bundle_schema_version: 1,
            protocol_version: 1,
            scene_id: 7,
        })
        .unwrap();
    // A too-new bundle is rejected and must NOT replace the loaded ProjectBundle.
    let err = bridge
        .load_project_bundle(ProjectBundleLoadRequest {
            bundle_schema_version: 99,
            protocol_version: 1,
            scene_id: 8,
        })
        .unwrap_err();
    assert_eq!(err.kind, RuntimeBridgeErrorKind::InvalidInput);
    assert_eq!(
        bridge
            .get_project_bundle_composition_status()
            .unwrap()
            .loaded_project_bundle,
        Some(7),
        "a failed load must not swap out the prior ProjectBundle"
    );
}

#[test]
fn init_then_step_is_deterministic() {
    let mut bridge = ReferenceBridge::new();
    let h = bridge.initialize_engine(EngineConfig { seed: 7 }).unwrap();
    assert_eq!(h.raw(), 7);
    let r = bridge
        .step_simulation(StepInputEnvelope { tick: 6 })
        .unwrap();
    assert_eq!(
        r,
        StepResult {
            tick: 6,
            diff_count: 2
        }
    );
}

// ── Voxel command submission → Rust authority (launchable-voxel, #2436) ──

use core_space::{LocalVoxelCoord, VoxelCoord};
use core_voxel::VoxelValue;

pub(super) fn init_bridge() -> ReferenceBridge {
    let mut bridge = ReferenceBridge::new();
    bridge.initialize_engine(EngineConfig { seed: 1 }).unwrap();
    bridge
}

fn project_voxel_conversion_request(grid: u64) -> VoxelConversionPlanRequest {
    VoxelConversionPlanRequest {
        source: protocol_voxel_conversion::VoxelConversionSourceRef {
            asset_id: "mesh/import-fixture-a".to_string(),
            asset_kind: "mesh".to_string(),
            asset_version: 1,
            source_hash: "sha256:import-fixture-a".to_string(),
            mesh_primitive: Some("default".to_string()),
        },
        target: protocol_voxel_conversion::VoxelConversionTargetRef {
            grid,
            volume_asset_id: Some("voxel/generated".to_string()),
            origin: protocol_voxel_conversion::VoxelConversionCoord { x: 0, y: 0, z: 0 },
        },
        settings: protocol_voxel_conversion::VoxelConversionSettings {
            mode: protocol_voxel_conversion::VoxelConversionMode::Surface,
            fit_policy: protocol_voxel_conversion::VoxelConversionFitPolicy::Contain,
            origin_policy: protocol_voxel_conversion::VoxelConversionOriginPolicy::TargetMin,
            resolution: [4, 4, 1],
            voxel_size: 1.0,
            max_output_voxels: 16,
            transform: [
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
            ],
            material_map: protocol_voxel_conversion::VoxelConversionMaterialMap {
                entries: vec![protocol_voxel_conversion::VoxelConversionMaterialMapEntry {
                    source_material_slot: 0,
                    source_material_id: Some("material/surface-a".to_string()),
                    voxel_material: 3,
                }],
                texture_assets: Vec::new(),
                texture_bindings: Vec::new(),
                default_voxel_material: None,
            },
        },
    }
}

fn studio_registered_source_request() -> VoxelConversionSourceRegistrationRequest {
    VoxelConversionSourceRegistrationRequest {
        source: protocol_voxel_conversion::VoxelConversionSourceRef {
            asset_id: "mesh/studio-registered-triangle".to_string(),
            asset_kind: "mesh".to_string(),
            asset_version: 3,
            source_hash: "sha256:studio-registered-triangle".to_string(),
            mesh_primitive: Some("default".to_string()),
        },
        positions: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        triangles: vec![protocol_voxel_conversion::VoxelConversionSourceTriangle {
            indices: [0, 1, 2],
            source_material_slot: 4,
        }],
        material_slots: vec![VoxelConversionSourceMaterialSlot {
            source_material_slot: 4,
            source_material_id: Some("material/studio-copper".to_string()),
        }],
    }
}

fn project_mesh_asset_registration_request(
) -> protocol_voxel_conversion::VoxelConversionMeshAssetRegistrationRequest {
    protocol_voxel_conversion::VoxelConversionMeshAssetRegistrationRequest {
        source: protocol_voxel_conversion::VoxelConversionSourceRef {
            asset_id: "mesh/project-quad".to_string(),
            asset_kind: "mesh".to_string(),
            asset_version: 5,
            source_hash: "sha256:project-quad".to_string(),
            mesh_primitive: Some("default".to_string()),
        },
        mesh_asset: protocol_voxel_conversion::VoxelConversionMeshAsset {
            asset_id: "mesh/project-quad".to_string(),
            source_path: Some("assets/meshes/project-quad.mesh.json".to_string()),
            positions: vec![
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [1.0, 1.0, 0.0],
                [0.0, 1.0, 0.0],
            ],
            normals: Vec::new(),
            indices: vec![0, 1, 2, 0, 2, 3],
            groups: vec![protocol_voxel_conversion::VoxelConversionMeshAssetGroup {
                material_slot: 2,
                start: 0,
                count: 6,
            }],
            material_slots: vec![VoxelConversionSourceMaterialSlot {
                source_material_slot: 2,
                source_material_id: Some("material/project-brick".to_string()),
            }],
        },
    }
}

fn larger_registered_grid_source_request() -> VoxelConversionSourceRegistrationRequest {
    let mut positions = Vec::new();
    for y in 0..3 {
        for x in 0..3 {
            positions.push([x as f32, y as f32, 0.0]);
        }
    }

    let mut triangles = Vec::new();
    for y in 0..2 {
        for x in 0..2 {
            let a = y * 3 + x;
            let b = a + 1;
            let c = a + 3;
            let d = c + 1;
            triangles.push(protocol_voxel_conversion::VoxelConversionSourceTriangle {
                indices: [a, b, d],
                source_material_slot: 0,
            });
            triangles.push(protocol_voxel_conversion::VoxelConversionSourceTriangle {
                indices: [a, d, c],
                source_material_slot: 0,
            });
        }
    }

    VoxelConversionSourceRegistrationRequest {
        source: protocol_voxel_conversion::VoxelConversionSourceRef {
            asset_id: "mesh/registered-grid-3x3".to_string(),
            asset_kind: "mesh".to_string(),
            asset_version: 1,
            source_hash: "sha256:registered-grid-3x3".to_string(),
            mesh_primitive: Some("default".to_string()),
        },
        positions,
        triangles,
        material_slots: vec![VoxelConversionSourceMaterialSlot {
            source_material_slot: 0,
            source_material_id: Some("material/grid-stone".to_string()),
        }],
    }
}

fn registered_source_plan_request(
    registration: &VoxelConversionSourceRegistrationRequest,
) -> VoxelConversionPlanRequest {
    let mut request = project_voxel_conversion_request(7);
    request.source = registration.source.clone();
    request.settings.material_map.entries =
        vec![protocol_voxel_conversion::VoxelConversionMaterialMapEntry {
            source_material_slot: 4,
            source_material_id: Some("material/studio-copper".to_string()),
            voxel_material: 9,
        }];
    request.settings.material_map.default_voxel_material = None;
    request
}

fn larger_registered_grid_plan_request(
    registration: &VoxelConversionSourceRegistrationRequest,
) -> VoxelConversionPlanRequest {
    let mut request = project_voxel_conversion_request(7);
    request.source = registration.source.clone();
    request.settings.resolution = [3, 3, 1];
    request.settings.max_output_voxels = 16;
    request.settings.material_map.entries =
        vec![protocol_voxel_conversion::VoxelConversionMaterialMapEntry {
            source_material_slot: 0,
            source_material_id: Some("material/grid-stone".to_string()),
            voxel_material: 3,
        }];
    request.settings.material_map.default_voxel_material = None;
    request
}

fn project_mesh_asset_plan_request(
    registration: &protocol_voxel_conversion::VoxelConversionMeshAssetRegistrationRequest,
) -> VoxelConversionPlanRequest {
    let mut request = project_voxel_conversion_request(7);
    request.source = registration.source.clone();
    request.settings.resolution = [4, 4, 1];
    request.settings.material_map.entries =
        vec![protocol_voxel_conversion::VoxelConversionMaterialMapEntry {
            source_material_slot: 2,
            source_material_id: Some("material/project-brick".to_string()),
            voxel_material: 11,
        }];
    request.settings.material_map.default_voxel_material = None;
    request
}

fn hand_authored_voxel_volume_asset() -> VoxelVolumeAsset {
    let asset = VoxelVolumeAsset {
        asset_id: "voxel-volume/hand-authored-room".to_string(),
        schema_version: protocol_voxel_asset::VOXEL_ASSET_SCHEMA_VERSION,
        media_type: protocol_voxel_asset::VOXEL_ASSET_MEDIA_TYPE.to_string(),
        grid: VoxelAssetGrid {
            origin: [0.0, 0.0, 0.0],
            cell_size: 1.0,
            coordinate_system: svc_voxel_asset::VOXEL_ASSET_COORDINATE_SYSTEM.to_string(),
        },
        bounds: VoxelAssetBounds {
            min: VoxelAssetCoord { x: 0, y: 0, z: 0 },
            max: VoxelAssetCoord { x: 1, y: 0, z: 0 },
        },
        representation: VoxelAssetRepresentation {
            kind: VoxelAssetRepresentationKind::SparseRuns,
            sparse_runs: vec![VoxelAssetSparseRun {
                start: VoxelAssetCoord { x: 0, y: 0, z: 0 },
                length: 2,
                material: 1,
            }],
        },
        material_palette: vec![VoxelAssetMaterialBinding {
            voxel_material: 1,
            material_asset_id: "material/concrete".to_string(),
        }],
        provenance: vec![VoxelAssetProvenanceRef {
            kind: VoxelAssetProvenanceKind::Authored,
            uri: "asha://project-bundle/assets/voxel-volume/hand-authored-room".to_string(),
            content_hash: "fnv1a64:authored-room".to_string(),
        }],
        authoring: VoxelAssetAuthoringMetadata {
            label: Some("Hand authored room".to_string()),
            created_by: Some("runtime-bridge-api-test".to_string()),
            source_tool: Some("fixture".to_string()),
        },
        validation_diagnostics: Vec::new(),
        content_hashes: VoxelAssetContentHashes {
            canonical_json: String::new(),
            voxel_data: String::new(),
        },
    };
    svc_voxel_asset::with_computed_hashes(&asset)
}

pub(super) fn fps_load_request(enemy_health: u32) -> FpsRuntimeSessionLoadRequest {
    FpsRuntimeSessionLoadRequest {
        project_bundle: "custom-demo".to_string(),
        definitions: vec![
            FpsBridgeStoredEntityDefinition {
                entity: 101,
                stable_id: "actor/custom-player".to_string(),
                display_name: "Custom Player".to_string(),
                source_path: "catalogs/actors/player.entity.json".to_string(),
                tags: vec!["player".to_string()],
                role: FpsBridgeRole::Player,
                transform: Some(FpsBridgeTransformCapability {
                    translation: [0.0, 1.5, 0.0],
                    rotation: [0.0, 0.0, 0.0, 1.0],
                    scale: [1.0, 1.0, 1.0],
                }),
                bounds: Some(FpsBridgeBoundsCapability {
                    min: [2.2, 1.0, 1.0],
                    max: [2.8, 2.0, 2.0],
                }),
                render_visible: Some(true),
                static_collider: Some(false),
                health: Some(FpsBridgeHealth {
                    current: 88,
                    max: 88,
                }),
                weapon: Some(FpsBridgeWeaponMount {
                    weapon_id: "weapon.custom.primary".to_string(),
                    damage: 75,
                    range_units: 16,
                    ammo: 3,
                    cooldown_ticks_after_fire: 4,
                }),
                policy_binding: None,
            },
            FpsBridgeStoredEntityDefinition {
                entity: 777,
                stable_id: "actor/custom-enemy".to_string(),
                display_name: "Custom Enemy".to_string(),
                source_path: "catalogs/actors/enemy.entity.json".to_string(),
                tags: vec!["enemy".to_string()],
                role: FpsBridgeRole::Enemy,
                transform: Some(FpsBridgeTransformCapability {
                    translation: [0.0, 1.5, 5.2],
                    rotation: [0.0, 0.0, 0.0, 1.0],
                    scale: [1.0, 1.0, 1.0],
                }),
                bounds: Some(FpsBridgeBoundsCapability {
                    min: [2.2, 1.0, 5.0],
                    max: [2.8, 2.0, 5.8],
                }),
                render_visible: Some(true),
                static_collider: Some(false),
                health: Some(FpsBridgeHealth {
                    current: enemy_health,
                    max: enemy_health,
                }),
                weapon: None,
                policy_binding: Some(FpsBridgePolicyBinding {
                    binding_id: "binding.enemy.custom.v0".to_string(),
                    policy_id: "policy.enemy.custom.v0".to_string(),
                    view_kind: "runtime_session.nav_policy_view.v0".to_string(),
                    view_version: "v0".to_string(),
                    allowed_intents: vec![
                        "runtime.intent.move_direct_nav.v0".to_string(),
                        "runtime.intent.primary_fire.v0".to_string(),
                    ],
                    runtime_moment: "runtime.tick.enemy_policy.v0".to_string(),
                }),
            },
        ],
        game_rule_modules: Vec::new(),
    }
}

#[test]
fn fps_runtime_session_loads_project_bundle_through_rust_authority() {
    let mut bridge = init_bridge();
    let snapshot = bridge
        .load_fps_runtime_session(fps_load_request(75))
        .expect("fps session loads");

    assert_eq!(snapshot.backend, "reference_bridge_rust");
    assert_eq!(
        snapshot.authority_surface,
        "runtime_session.fps.authority.v0"
    );
    assert_eq!(snapshot.session_epoch, 1);
    assert_eq!(snapshot.player_entity, 101);
    assert_eq!(snapshot.enemy_entity, 777);
    assert_eq!(
        snapshot.health,
        vec![
            FpsEntityHealthReadout {
                entity: 101,
                current: 88,
                max: 88,
            },
            FpsEntityHealthReadout {
                entity: 777,
                current: 75,
                max: 75,
            },
        ]
    );
    assert_eq!(snapshot.policy_bindings.len(), 1);
    assert_eq!(snapshot.policy_bindings[0].entity, 777);
    assert_eq!(
        snapshot.replay_records[0].replay_unit,
        "runtime_session.fps.bootstrap.v0"
    );
    assert_ne!(snapshot.replay_hash, 0);
    assert!(snapshot
        .read_sets
        .iter()
        .any(|view| view.owner == "rule-lifecycle"));
}

#[test]
fn fps_primary_fire_receipt_comes_from_rust_combat_lifecycle_and_replay() {
    let mut bridge = init_bridge();
    bridge
        .load_fps_runtime_session(fps_load_request(75))
        .unwrap();
    let receipt = bridge
        .apply_fps_primary_fire(FpsPrimaryFireRequest {
            tick: 9,
            origin: [2.5, 1.5, 1.5],
            direction: [0.0, 0.0, 1.0],
        })
        .expect("primary fire applies");

    assert_eq!(receipt.backend, "reference_bridge_rust");
    assert_eq!(receipt.mutation_owner, "rule-lifecycle + svc-combat");
    assert_eq!(receipt.shooter, 101);
    assert_eq!(receipt.target, Some(777));
    assert_eq!(
        receipt.target_health_before,
        Some(FpsBridgeHealth {
            current: 75,
            max: 75,
        })
    );
    assert_eq!(
        receipt.target_health_after,
        Some(FpsBridgeHealth {
            current: 0,
            max: 75,
        })
    );
    assert_eq!(
        receipt.lifecycle_status,
        FpsBridgeLifecycleStatus::EnemyDefeated {
            entity: 777,
            tick: 9,
        }
    );
    assert_eq!(receipt.target_render_visible, Some(false));
    assert_ne!(receipt.replay_hash, 0);

    let snapshot = bridge.read_fps_runtime_session().unwrap();
    assert_eq!(snapshot.replay_records.len(), 2);
    assert_eq!(snapshot.replay_hash, receipt.replay_hash);
}

#[test]
fn fps_encounter_transition_is_rule_lifecycle_authority() {
    let mut bridge = init_bridge();
    bridge
        .load_fps_runtime_session(fps_load_request(75))
        .unwrap();
    let active_lifecycle = FpsEncounterLifecycleInput {
        outcome_kind: "in_progress".to_string(),
        terminal: false,
        enemy_dead: false,
        player_dead: false,
        lifecycle_hash: "fnv1a64:active".to_string(),
    };
    let pending = bridge
        .read_fps_encounter_director(active_lifecycle.clone())
        .unwrap();
    assert_eq!(pending.backend, "reference_bridge_rust");
    assert_eq!(
        pending.authority_surface,
        "runtime_session.fps.encounter_director.v0"
    );
    assert_eq!(pending.state.status, "pending");
    assert_eq!(pending.read_sets[0].owner, "rule-lifecycle");

    let activated = bridge
        .apply_fps_encounter_transition(FpsEncounterTransitionRequest {
            preset_id: "generated-tunnel-small-encounter".to_string(),
            action: "activate".to_string(),
            lifecycle: active_lifecycle,
        })
        .unwrap();
    assert!(activated.accepted);
    assert_eq!(
        activated.event_kind.as_deref(),
        Some("runtime_encounter.activated.v0")
    );
    assert_eq!(activated.state.status, "active");
    assert_eq!(
        activated.state.spawned_enemy_ids,
        vec!["encounter.generated_tunnel_small.wave_1.enemy_001".to_string()]
    );

    bridge
        .apply_fps_primary_fire(FpsPrimaryFireRequest {
            tick: 9,
            origin: [2.5, 1.5, 1.5],
            direction: [0.0, 0.0, 1.0],
        })
        .unwrap();
    let won_lifecycle = FpsEncounterLifecycleInput {
        outcome_kind: "won".to_string(),
        terminal: true,
        enemy_dead: true,
        player_dead: false,
        lifecycle_hash: "fnv1a64:won".to_string(),
    };
    let cleared = bridge
        .apply_fps_encounter_transition(FpsEncounterTransitionRequest {
            preset_id: "generated-tunnel-small-encounter".to_string(),
            action: "sync_lifecycle".to_string(),
            lifecycle: won_lifecycle,
        })
        .unwrap();
    assert!(cleared.accepted);
    assert_eq!(cleared.state.status, "cleared");
    assert_eq!(
        cleared.state.defeated_enemy_ids,
        vec!["encounter.generated_tunnel_small.wave_1.enemy_001".to_string()]
    );
    assert_ne!(cleared.replay_hash, 0);

    let rejected = bridge
        .apply_fps_encounter_transition(FpsEncounterTransitionRequest {
            preset_id: "generated-tunnel-small-encounter".to_string(),
            action: "activate".to_string(),
            lifecycle: FpsEncounterLifecycleInput {
                outcome_kind: "in_progress".to_string(),
                terminal: false,
                enemy_dead: false,
                player_dead: false,
                lifecycle_hash: "fnv1a64:active-again".to_string(),
            },
        })
        .unwrap();
    assert!(!rejected.accepted);
    assert_eq!(
        rejected.rejection_reason.as_deref(),
        Some("encounter_not_pending")
    );

    let restarted = bridge
        .restart_fps_runtime_session(FpsRuntimeSessionRestartRequest { expected_epoch: 1 })
        .unwrap();
    assert_eq!(restarted.session_epoch, 2);
    let reset = bridge
        .read_fps_encounter_director(FpsEncounterLifecycleInput {
            outcome_kind: "in_progress".to_string(),
            terminal: false,
            enemy_dead: false,
            player_dead: false,
            lifecycle_hash: "fnv1a64:reset".to_string(),
        })
        .unwrap();
    assert_eq!(reset.state.status, "pending");
    assert_eq!(reset.state.revision, 0);
}

#[test]
fn fps_runtime_session_restart_is_epoch_guarded_and_authority_owned() {
    let mut bridge = init_bridge();
    bridge
        .load_fps_runtime_session(fps_load_request(75))
        .unwrap();
    bridge
        .apply_fps_primary_fire(FpsPrimaryFireRequest {
            tick: 9,
            origin: [2.5, 1.5, 1.5],
            direction: [0.0, 0.0, 1.0],
        })
        .unwrap();

    let stale = bridge
        .restart_fps_runtime_session(FpsRuntimeSessionRestartRequest { expected_epoch: 0 })
        .unwrap_err();
    assert_eq!(stale.kind, RuntimeBridgeErrorKind::InvalidInput);

    let restarted = bridge
        .restart_fps_runtime_session(FpsRuntimeSessionRestartRequest { expected_epoch: 1 })
        .unwrap();
    assert_eq!(restarted.session_epoch, 2);
    assert_eq!(restarted.lifecycle_status, FpsBridgeLifecycleStatus::Active);
    assert_eq!(
        restarted
            .health
            .iter()
            .find(|health| health.entity == 777)
            .map(|health| (health.current, health.max)),
        Some((75, 75))
    );
    assert_eq!(restarted.replay_records.len(), 1);
}

#[test]
fn invalid_fps_load_fails_closed_without_replacing_prior_session() {
    let mut bridge = init_bridge();
    bridge
        .load_fps_runtime_session(fps_load_request(75))
        .unwrap();
    let before = bridge.read_fps_runtime_session().unwrap();
    let mut invalid = fps_load_request(33);
    invalid.definitions[1].policy_binding = Some(FpsBridgePolicyBinding {
        binding_id: String::new(),
        policy_id: "policy.enemy.custom.v0".to_string(),
        view_kind: "runtime_session.nav_policy_view.v0".to_string(),
        view_version: "v0".to_string(),
        allowed_intents: vec!["runtime.intent.primary_fire.v0".to_string()],
        runtime_moment: "runtime.tick.enemy_policy.v0".to_string(),
    });

    let err = bridge.load_fps_runtime_session(invalid).unwrap_err();
    assert_eq!(err.kind, RuntimeBridgeErrorKind::InvalidInput);
    let after = bridge.read_fps_runtime_session().unwrap();
    assert_eq!(after.session_epoch, before.session_epoch);
    assert_eq!(after.health, before.health);
    assert_eq!(after.replay_hash, before.replay_hash);
}

#[test]
fn voxel_conversion_plan_preview_apply_uses_rust_authority_and_commands() {
    let mut bridge = init_bridge();
    let request = project_voxel_conversion_request(7);
    let plan = bridge.plan_voxel_conversion(request).unwrap();
    assert_eq!(
        plan.authority_version,
        svc_voxel_conversion::AUTHORITY_VERSION
    );
    assert_eq!(plan.source.asset_id, "mesh/import-fixture-a");
    assert_eq!(plan.target.grid, 7);
    assert!(plan.diagnostics.is_empty());
    assert_eq!(plan.estimated_output_voxels, 3);

    let stale = bridge
        .preview_voxel_conversion(VoxelConversionPreviewRequest {
            plan_id: plan.plan_id.clone(),
            expected_plan_hash: "fnv1a64:stale".to_string(),
        })
        .unwrap();
    assert_eq!(
        stale.diagnostics[0].code,
        VoxelConversionDiagnosticCode::StaleAuthoritySnapshot
    );

    let preview = bridge
        .preview_voxel_conversion(VoxelConversionPreviewRequest {
            plan_id: plan.plan_id.clone(),
            expected_plan_hash: svc_voxel_conversion::plan_hash(&plan),
        })
        .unwrap();
    assert!(preview.diagnostics.is_empty());
    assert_eq!(preview.output_voxel_count, 3);

    let receipt = bridge
        .apply_voxel_conversion(VoxelConversionApplyRequest {
            plan_id: plan.plan_id.clone(),
            expected_plan_hash: svc_voxel_conversion::plan_hash(&plan),
            expected_preview_hash: Some(preview.output_hash.clone()),
        })
        .unwrap();
    assert!(receipt.applied);
    assert_eq!(receipt.output_voxel_count, 3);

    let world = bridge.voxel.as_ref().unwrap();
    assert_eq!(world.grid().id(), GridId::new(7));
    let chunk = world.get(ChunkCoord::new(0, 0, 0)).unwrap();
    assert_eq!(
        chunk.get(LocalVoxelCoord::new(0, 0, 0)),
        Some(VoxelValue::solid_raw(3)),
        "conversion output applied through voxel command authority"
    );

    let exported = bridge
        .export_voxel_conversion_evidence(
            plan.evidence
                .iter()
                .chain(preview.evidence.iter())
                .chain(receipt.evidence.iter())
                .cloned()
                .collect(),
        )
        .unwrap();
    assert_eq!(exported.len(), 3);

    let model_info = bridge
        .read_voxel_model_info(VoxelModelInfoRequest {
            grid: 7,
            volume_asset_id: Some("voxel/generated".to_string()),
            include_material_counts: true,
        })
        .unwrap();
    assert!(model_info.resident);
    assert_eq!(
        model_info.model_id,
        "voxel-model:grid:7:volume:voxel/generated"
    );
    assert_eq!(model_info.voxel_count, 3);
    assert_eq!(
        model_info.material_counts,
        vec![VoxelModelMaterialCount {
            material: 3,
            voxel_count: 3
        }]
    );
    assert_eq!(
        model_info.source.as_ref().unwrap().asset_id,
        "mesh/import-fixture-a"
    );
    assert_eq!(
        model_info.latest_plan_id.as_deref(),
        Some(plan.plan_id.as_str())
    );
    assert!(model_info.latest_output_hash.is_some());
    assert!(model_info.session_hash.starts_with("fnv1a64:"));
    assert!(model_info.replay_hash.starts_with("fnv1a64:"));
    assert!(model_info.diagnostics.is_empty());

    let compact_info = bridge
        .read_voxel_model_info(VoxelModelInfoRequest {
            grid: 7,
            volume_asset_id: Some("voxel/generated".to_string()),
            include_material_counts: false,
        })
        .unwrap();
    assert!(compact_info.material_counts.is_empty());

    let exported = bridge
        .export_voxel_volume_asset(VoxelVolumeAssetExportRequest {
            grid: 7,
            volume_asset_id: Some("voxel/generated".to_string()),
            target_asset_id: "voxel-volume/generated-crate".to_string(),
            label: Some("Generated crate".to_string()),
            created_by: Some("runtime-bridge-api-test".to_string()),
            source_tool: Some("svc-voxel-conversion".to_string()),
            max_sparse_runs: 16,
            expected_session_hash: Some(model_info.session_hash.clone()),
        })
        .unwrap();
    assert!(exported.exported);
    assert!(exported.diagnostics.is_empty());
    let asset = exported.asset.as_ref().expect("exported asset");
    assert_eq!(asset.asset_id, "voxel-volume/generated-crate");
    assert_eq!(
        asset.schema_version,
        protocol_voxel_asset::VOXEL_ASSET_SCHEMA_VERSION
    );
    assert_eq!(
        asset.media_type,
        protocol_voxel_asset::VOXEL_ASSET_MEDIA_TYPE
    );
    assert_eq!(
        asset.material_palette[0].material_asset_id,
        "material/surface-a"
    );
    assert_eq!(
        asset
            .representation
            .sparse_runs
            .iter()
            .map(|run| run.length as u64)
            .sum::<u64>(),
        3
    );
    assert_eq!(
        exported.canonical_json_hash.as_deref(),
        Some(asset.content_hashes.canonical_json.as_str())
    );
    assert_eq!(
        exported.voxel_data_hash.as_deref(),
        Some(asset.content_hashes.voxel_data.as_str())
    );
    let canonical_json = exported.canonical_json.as_ref().expect("canonical json");
    let decoded = svc_voxel_asset::decode_asset(canonical_json).expect("canonical asset decodes");
    assert_eq!(decoded, *asset);

    let save = bridge
        .save_voxel_volume_asset(VoxelVolumeAssetSaveRequest {
            export_request: VoxelVolumeAssetExportRequest {
                grid: 7,
                volume_asset_id: Some("voxel/generated".to_string()),
                target_asset_id: "voxel-volume/generated-crate".to_string(),
                label: Some("Generated crate".to_string()),
                created_by: Some("runtime-bridge-api-test".to_string()),
                source_tool: Some("svc-voxel-conversion".to_string()),
                max_sparse_runs: 16,
                expected_session_hash: Some(model_info.session_hash.clone()),
            },
            target_project_bundle: "asha-demo".to_string(),
            target_asset_path: "assets/voxels/generated-crate.avxl.json".to_string(),
            representation_kind: "sparse_runs".to_string(),
            expected_existing_canonical_json_hash: None,
            expected_canonical_json_hash: exported.canonical_json_hash.clone(),
            expected_voxel_data_hash: exported.voxel_data_hash.clone(),
        })
        .unwrap();
    assert!(save.saved);
    assert!(save.diagnostics.is_empty());
    assert_eq!(
        save.canonical_json_hash.as_deref(),
        exported.canonical_json_hash.as_deref()
    );
    assert_eq!(
        save.voxel_data_hash.as_deref(),
        exported.voxel_data_hash.as_deref()
    );
    let diff = save.diff.as_ref().expect("stored diff");
    assert_eq!(diff.project_bundle, "asha-demo");
    assert_eq!(diff.asset_id, "voxel-volume/generated-crate");
    assert_eq!(diff.asset_path, "assets/voxels/generated-crate.avxl.json");
    assert_eq!(diff.operation, "create");
    assert_eq!(
        diff.sparse_run_count,
        asset.representation.sparse_runs.len() as u64
    );
    assert_eq!(diff.voxel_count, 3);
    assert_eq!(diff.material_count, 1);
    assert_eq!(diff.runtime_session_hash, model_info.session_hash);
    assert_eq!(
        save.canonical_json.as_deref(),
        exported.canonical_json.as_deref()
    );

    let invalid_path_save = bridge
        .save_voxel_volume_asset(VoxelVolumeAssetSaveRequest {
            export_request: VoxelVolumeAssetExportRequest {
                grid: 7,
                volume_asset_id: Some("voxel/generated".to_string()),
                target_asset_id: "voxel-volume/generated-crate".to_string(),
                label: None,
                created_by: None,
                source_tool: None,
                max_sparse_runs: 16,
                expected_session_hash: None,
            },
            target_project_bundle: "asha-demo".to_string(),
            target_asset_path: "/tmp/generated-crate.avxl.json".to_string(),
            representation_kind: "sparse_runs".to_string(),
            expected_existing_canonical_json_hash: None,
            expected_canonical_json_hash: None,
            expected_voxel_data_hash: None,
        })
        .unwrap();
    assert!(!invalid_path_save.saved);
    assert_eq!(
        invalid_path_save.diagnostics[0].code,
        protocol_voxel_asset::VoxelAssetDiagnosticCode::InvalidAssetId
    );

    let unsupported_save = bridge
        .save_voxel_volume_asset(VoxelVolumeAssetSaveRequest {
            export_request: VoxelVolumeAssetExportRequest {
                grid: 7,
                volume_asset_id: Some("voxel/generated".to_string()),
                target_asset_id: "voxel-volume/generated-crate".to_string(),
                label: None,
                created_by: None,
                source_tool: None,
                max_sparse_runs: 16,
                expected_session_hash: None,
            },
            target_project_bundle: "asha-demo".to_string(),
            target_asset_path: "assets/voxels/generated-crate.avxl.json".to_string(),
            representation_kind: "dense_grid".to_string(),
            expected_existing_canonical_json_hash: None,
            expected_canonical_json_hash: None,
            expected_voxel_data_hash: None,
        })
        .unwrap();
    assert!(!unsupported_save.saved);
    assert_eq!(
        unsupported_save.diagnostics[0].code,
        protocol_voxel_asset::VoxelAssetDiagnosticCode::UnsupportedRepresentation
    );

    let hash_mismatch_save = bridge
        .save_voxel_volume_asset(VoxelVolumeAssetSaveRequest {
            export_request: VoxelVolumeAssetExportRequest {
                grid: 7,
                volume_asset_id: Some("voxel/generated".to_string()),
                target_asset_id: "voxel-volume/generated-crate".to_string(),
                label: None,
                created_by: None,
                source_tool: None,
                max_sparse_runs: 16,
                expected_session_hash: None,
            },
            target_project_bundle: "asha-demo".to_string(),
            target_asset_path: "assets/voxels/generated-crate.avxl.json".to_string(),
            representation_kind: "sparse_runs".to_string(),
            expected_existing_canonical_json_hash: Some("fnv1a64:previous".to_string()),
            expected_canonical_json_hash: Some("fnv1a64:wrong".to_string()),
            expected_voxel_data_hash: exported.voxel_data_hash.clone(),
        })
        .unwrap();
    assert!(!hash_mismatch_save.saved);
    assert_eq!(
        hash_mismatch_save.diagnostics[0].code,
        protocol_voxel_asset::VoxelAssetDiagnosticCode::ContentHashMismatch
    );

    let stale_export = bridge
        .export_voxel_volume_asset(VoxelVolumeAssetExportRequest {
            grid: 7,
            volume_asset_id: Some("voxel/generated".to_string()),
            target_asset_id: "voxel-volume/stale".to_string(),
            label: None,
            created_by: None,
            source_tool: None,
            max_sparse_runs: 16,
            expected_session_hash: Some("fnv1a64:stale".to_string()),
        })
        .unwrap();
    assert!(!stale_export.exported);
    assert_eq!(
        stale_export.diagnostics[0].code,
        protocol_voxel_asset::VoxelAssetDiagnosticCode::StaleRuntimeSnapshot
    );

    let stale_save = bridge
        .save_voxel_volume_asset(VoxelVolumeAssetSaveRequest {
            export_request: VoxelVolumeAssetExportRequest {
                grid: 7,
                volume_asset_id: Some("voxel/generated".to_string()),
                target_asset_id: "voxel-volume/stale-save".to_string(),
                label: None,
                created_by: None,
                source_tool: None,
                max_sparse_runs: 16,
                expected_session_hash: Some("fnv1a64:stale".to_string()),
            },
            target_project_bundle: "asha-demo".to_string(),
            target_asset_path: "assets/voxels/stale-save.avxl.json".to_string(),
            representation_kind: "sparse_runs".to_string(),
            expected_existing_canonical_json_hash: None,
            expected_canonical_json_hash: None,
            expected_voxel_data_hash: None,
        })
        .unwrap();
    assert!(!stale_save.saved);
    assert_eq!(
        stale_save.diagnostics[0].code,
        protocol_voxel_asset::VoxelAssetDiagnosticCode::StaleRuntimeSnapshot
    );

    let mut load_bridge = init_bridge();
    let load_receipt = load_bridge
        .load_voxel_volume_asset(VoxelVolumeAssetLoadRequest {
            asset: asset.clone(),
            target_grid: 7,
            target_volume_asset_id: Some("voxel/generated".to_string()),
            replace_existing: true,
            include_material_counts: true,
        })
        .unwrap();
    assert!(load_receipt.loaded);
    assert_eq!(load_receipt.request_asset_id, asset.asset_id);
    assert_eq!(load_receipt.voxel_count, 3);
    assert_eq!(
        load_receipt.material_counts,
        vec![VoxelAssetMaterialCount {
            material: 3,
            voxel_count: 3
        }]
    );
    let reloaded_info = load_bridge
        .read_voxel_model_info(VoxelModelInfoRequest {
            grid: 7,
            volume_asset_id: Some("voxel/generated".to_string()),
            include_material_counts: true,
        })
        .unwrap();
    assert!(reloaded_info.resident);
    assert_eq!(reloaded_info.voxel_count, 3);
    assert_eq!(
        reloaded_info.source.as_ref().unwrap().asset_id,
        "voxel-volume/generated-crate"
    );
}

#[test]
fn voxel_volume_asset_load_accepts_hand_authored_asset_and_rejects_invalid_assets() {
    let asset = hand_authored_voxel_volume_asset();
    let mut bridge = init_bridge();
    let receipt = bridge
        .load_voxel_volume_asset(VoxelVolumeAssetLoadRequest {
            asset: asset.clone(),
            target_grid: 7,
            target_volume_asset_id: Some(asset.asset_id.clone()),
            replace_existing: true,
            include_material_counts: true,
        })
        .unwrap();
    assert!(receipt.loaded);
    assert_eq!(receipt.voxel_count, 2);
    assert_eq!(
        receipt.material_counts,
        vec![VoxelAssetMaterialCount {
            material: 1,
            voxel_count: 2
        }]
    );
    let info = bridge
        .read_voxel_model_info(VoxelModelInfoRequest {
            grid: 7,
            volume_asset_id: Some(asset.asset_id.clone()),
            include_material_counts: true,
        })
        .unwrap();
    assert!(info.resident);
    assert_eq!(info.voxel_count, 2);
    assert_eq!(
        info.source.as_ref().unwrap().source_hash,
        asset.content_hashes.voxel_data
    );

    let mut invalid_hash = asset.clone();
    invalid_hash.content_hashes.voxel_data = "fnv1a64:stale".to_string();
    let mut invalid_bridge = init_bridge();
    let rejected = invalid_bridge
        .load_voxel_volume_asset(VoxelVolumeAssetLoadRequest {
            asset: invalid_hash,
            target_grid: 7,
            target_volume_asset_id: Some("voxel/invalid".to_string()),
            replace_existing: true,
            include_material_counts: true,
        })
        .unwrap();
    assert!(!rejected.loaded);
    assert_eq!(
        rejected.diagnostics[0].code,
        protocol_voxel_asset::VoxelAssetDiagnosticCode::ContentHashMismatch
    );
    let missing = invalid_bridge
        .read_voxel_model_info(VoxelModelInfoRequest {
            grid: 7,
            volume_asset_id: Some("voxel/invalid".to_string()),
            include_material_counts: true,
        })
        .unwrap();
    assert!(!missing.resident);

    let mut invalid_material = asset;
    invalid_material.material_palette[0].material_asset_id = "texture/not-material".to_string();
    invalid_material = svc_voxel_asset::with_computed_hashes(&invalid_material);
    let rejected_material = invalid_bridge
        .load_voxel_volume_asset(VoxelVolumeAssetLoadRequest {
            asset: invalid_material,
            target_grid: 7,
            target_volume_asset_id: Some("voxel/invalid-material".to_string()),
            replace_existing: true,
            include_material_counts: true,
        })
        .unwrap();
    assert!(!rejected_material.loaded);
    assert_eq!(
        rejected_material.diagnostics[0].code,
        protocol_voxel_asset::VoxelAssetDiagnosticCode::InvalidMaterialReference
    );
}

#[test]
fn voxel_volume_asset_save_rejects_missing_material_refs_without_storage_diff() {
    let mut bridge = init_bridge();
    let mut request = project_voxel_conversion_request(7);
    request.settings.material_map.entries[0].source_material_id = None;
    let plan = bridge.plan_voxel_conversion(request).unwrap();
    assert!(plan.diagnostics.is_empty());

    let preview = bridge
        .preview_voxel_conversion(VoxelConversionPreviewRequest {
            plan_id: plan.plan_id.clone(),
            expected_plan_hash: svc_voxel_conversion::plan_hash(&plan),
        })
        .unwrap();
    assert!(preview.diagnostics.is_empty());

    let receipt = bridge
        .apply_voxel_conversion(VoxelConversionApplyRequest {
            plan_id: plan.plan_id.clone(),
            expected_plan_hash: svc_voxel_conversion::plan_hash(&plan),
            expected_preview_hash: Some(preview.output_hash),
        })
        .unwrap();
    assert!(receipt.applied);

    let model_info = bridge
        .read_voxel_model_info(VoxelModelInfoRequest {
            grid: 7,
            volume_asset_id: Some("voxel/generated".to_string()),
            include_material_counts: true,
        })
        .unwrap();
    assert!(model_info.resident);

    let save = bridge
        .save_voxel_volume_asset(VoxelVolumeAssetSaveRequest {
            export_request: VoxelVolumeAssetExportRequest {
                grid: 7,
                volume_asset_id: Some("voxel/generated".to_string()),
                target_asset_id: "voxel-volume/missing-material".to_string(),
                label: None,
                created_by: None,
                source_tool: None,
                max_sparse_runs: 16,
                expected_session_hash: Some(model_info.session_hash),
            },
            target_project_bundle: "asha-demo".to_string(),
            target_asset_path: "assets/voxels/missing-material.avxl.json".to_string(),
            representation_kind: "sparse_runs".to_string(),
            expected_existing_canonical_json_hash: None,
            expected_canonical_json_hash: None,
            expected_voxel_data_hash: None,
        })
        .unwrap();
    assert!(!save.saved);
    assert!(save.diff.is_none());
    assert_eq!(
        save.diagnostics[0].code,
        protocol_voxel_asset::VoxelAssetDiagnosticCode::InvalidMaterialReference
    );
}

#[test]
fn voxel_conversion_registers_studio_static_mesh_source_before_plan() {
    let mut bridge = init_bridge();
    let registration_request = studio_registered_source_request();
    let registration = bridge
        .register_voxel_conversion_source(registration_request.clone())
        .unwrap();
    assert!(registration.registered);
    assert!(registration.diagnostics.is_empty());
    assert_eq!(
        registration.source.asset_id,
        "mesh/studio-registered-triangle"
    );
    assert_eq!(registration.source.asset_version, 3);
    assert_eq!(registration.material_slots[0].source_material_slot, 4);
    assert_eq!(
        registration.material_slots[0].source_material_id.as_deref(),
        Some("material/studio-copper")
    );
    assert_eq!(
        registration.evidence[0].kind,
        protocol_voxel_conversion::VoxelConversionEvidenceKind::SourceSnapshot
    );

    let plan = bridge
        .plan_voxel_conversion(registered_source_plan_request(&registration_request))
        .unwrap();
    assert!(plan.diagnostics.is_empty());
    assert_eq!(plan.source.asset_id, "mesh/studio-registered-triangle");
    assert_eq!(
        plan.expected_source_hash,
        "sha256:studio-registered-triangle"
    );
    assert_eq!(
        plan.settings.material_map.entries[0].source_material_slot,
        4
    );
}

#[test]
fn voxel_conversion_registers_project_mesh_asset_before_plan() {
    let mut bridge = init_bridge();
    let registration_request = project_mesh_asset_registration_request();
    let registration = bridge
        .register_voxel_conversion_mesh_asset(registration_request.clone())
        .unwrap();
    assert!(registration.registered);
    assert!(registration.diagnostics.is_empty());
    assert_eq!(registration.source.asset_id, "mesh/project-quad");
    assert_eq!(registration.source.asset_version, 5);
    assert_eq!(registration.material_slots[0].source_material_slot, 2);
    assert_eq!(
        registration.material_slots[0].source_material_id.as_deref(),
        Some("material/project-brick")
    );
    assert_eq!(
        registration.evidence[0].kind,
        protocol_voxel_conversion::VoxelConversionEvidenceKind::SourceSnapshot
    );

    let plan = bridge
        .plan_voxel_conversion(project_mesh_asset_plan_request(&registration_request))
        .unwrap();
    assert!(plan.diagnostics.is_empty());
    assert_eq!(plan.source.asset_id, "mesh/project-quad");
    assert_eq!(plan.expected_source_hash, "sha256:project-quad");
    assert_eq!(
        plan.settings.material_map.entries[0].source_material_slot,
        2
    );
}

#[test]
fn voxel_conversion_larger_registered_source_applies_and_reports_model_info() {
    let mut bridge = init_bridge();
    let registration_request = larger_registered_grid_source_request();
    let registration = bridge
        .register_voxel_conversion_source(registration_request.clone())
        .unwrap();
    assert!(registration.registered);
    assert_eq!(registration_request.positions.len(), 9);
    assert_eq!(registration_request.triangles.len(), 8);

    let plan = bridge
        .plan_voxel_conversion(larger_registered_grid_plan_request(&registration_request))
        .unwrap();
    assert!(plan.diagnostics.is_empty());
    assert_eq!(plan.estimated_output_voxels, 9);
    assert_eq!(plan.estimated_bounds.unwrap().max.x, 2);
    assert_eq!(plan.estimated_bounds.unwrap().max.y, 2);

    let preview = bridge
        .preview_voxel_conversion(VoxelConversionPreviewRequest {
            plan_id: plan.plan_id.clone(),
            expected_plan_hash: svc_voxel_conversion::plan_hash(&plan),
        })
        .unwrap();
    assert!(preview.diagnostics.is_empty());
    assert_eq!(preview.output_voxel_count, 9);

    let receipt = bridge
        .apply_voxel_conversion(VoxelConversionApplyRequest {
            plan_id: plan.plan_id.clone(),
            expected_plan_hash: svc_voxel_conversion::plan_hash(&plan),
            expected_preview_hash: Some(preview.output_hash),
        })
        .unwrap();
    assert!(receipt.applied);
    assert_eq!(receipt.output_voxel_count, 9);

    let model_info = bridge
        .read_voxel_model_info(VoxelModelInfoRequest {
            grid: 7,
            volume_asset_id: Some("voxel/generated".to_string()),
            include_material_counts: true,
        })
        .unwrap();
    assert!(model_info.resident);
    assert_eq!(model_info.voxel_count, 9);
    assert_eq!(
        model_info.material_counts,
        vec![VoxelModelMaterialCount {
            material: 3,
            voxel_count: 9
        }]
    );
    assert_eq!(
        model_info.source.as_ref().unwrap().asset_id,
        "mesh/registered-grid-3x3"
    );
    assert_eq!(
        model_info.latest_plan_id.as_deref(),
        Some(plan.plan_id.as_str())
    );
    assert!(model_info.latest_output_hash.is_some());
    assert!(model_info.session_hash.starts_with("fnv1a64:"));
    assert!(model_info.replay_hash.starts_with("fnv1a64:"));
    assert!(model_info.diagnostics.is_empty());

    let exported = bridge
        .export_voxel_volume_asset(VoxelVolumeAssetExportRequest {
            grid: 7,
            volume_asset_id: Some("voxel/generated".to_string()),
            target_asset_id: "voxel-volume/generated-grid".to_string(),
            label: Some("Generated grid".to_string()),
            created_by: Some("runtime-bridge-api-test".to_string()),
            source_tool: Some("svc-voxel-conversion".to_string()),
            max_sparse_runs: 16,
            expected_session_hash: Some(model_info.session_hash),
        })
        .unwrap();
    assert!(exported.exported);
    let asset = exported.asset.expect("exported larger asset");
    assert_eq!(asset.bounds.max.x, 2);
    assert_eq!(asset.bounds.max.y, 2);
    assert_eq!(asset.representation.sparse_runs.len(), 3);
    assert_eq!(
        asset
            .representation
            .sparse_runs
            .iter()
            .map(|run| run.length)
            .collect::<Vec<_>>(),
        vec![3, 3, 3]
    );
    assert!(svc_voxel_asset::decode_asset(
        exported
            .canonical_json
            .as_ref()
            .expect("larger canonical json")
    )
    .is_ok());

    let limited = bridge
        .export_voxel_volume_asset(VoxelVolumeAssetExportRequest {
            grid: 7,
            volume_asset_id: Some("voxel/generated".to_string()),
            target_asset_id: "voxel-volume/too-large".to_string(),
            label: None,
            created_by: None,
            source_tool: None,
            max_sparse_runs: 2,
            expected_session_hash: None,
        })
        .unwrap();
    assert!(!limited.exported);
    assert_eq!(
        limited.diagnostics[0].code,
        protocol_voxel_asset::VoxelAssetDiagnosticCode::ExportLimitExceeded
    );
}

#[test]
fn voxel_conversion_project_mesh_asset_registration_rejects_invalid_assets() {
    let mut bridge = init_bridge();

    let mut missing_geometry = project_mesh_asset_registration_request();
    missing_geometry.mesh_asset.positions = Vec::new();
    let rejected_missing = bridge
        .register_voxel_conversion_mesh_asset(missing_geometry)
        .unwrap();
    assert!(!rejected_missing.registered);
    assert!(rejected_missing.evidence.is_empty());
    assert_eq!(
        rejected_missing.diagnostics[0].code,
        VoxelConversionDiagnosticCode::UnsupportedSourceAsset
    );

    let mut unsupported_primitive = project_mesh_asset_registration_request();
    unsupported_primitive.source.mesh_primitive = Some("lod1".to_string());
    let rejected_primitive = bridge
        .register_voxel_conversion_mesh_asset(unsupported_primitive)
        .unwrap();
    assert!(!rejected_primitive.registered);
    assert_eq!(
        rejected_primitive.diagnostics[0].code,
        VoxelConversionDiagnosticCode::UnsupportedSourceAsset
    );

    let mut material_slot_mismatch = project_mesh_asset_registration_request();
    material_slot_mismatch.mesh_asset.groups[0].material_slot = 99;
    let rejected_material = bridge
        .register_voxel_conversion_mesh_asset(material_slot_mismatch)
        .unwrap();
    assert!(!rejected_material.registered);
    assert_eq!(
        rejected_material.diagnostics[0].code,
        VoxelConversionDiagnosticCode::UnsupportedSourceAsset
    );
}

#[test]
fn voxel_conversion_project_mesh_asset_stale_source_hash_fails_closed() {
    let mut bridge = init_bridge();
    let registration_request = project_mesh_asset_registration_request();
    let registration = bridge
        .register_voxel_conversion_mesh_asset(registration_request.clone())
        .unwrap();
    assert!(registration.registered);

    let mut plan_request = project_mesh_asset_plan_request(&registration_request);
    plan_request.source.source_hash = "sha256:stale-project-quad".to_string();
    let plan = bridge.plan_voxel_conversion(plan_request).unwrap();
    assert_eq!(
        plan.diagnostics[0].code,
        VoxelConversionDiagnosticCode::SourceHashMismatch
    );
}

#[test]
fn voxel_conversion_source_registration_missing_geometry_fails_closed() {
    let mut bridge = init_bridge();
    let mut registration_request = studio_registered_source_request();
    registration_request.positions = Vec::new();
    let registration = bridge
        .register_voxel_conversion_source(registration_request.clone())
        .unwrap();
    assert!(!registration.registered);
    assert!(registration.evidence.is_empty());
    assert_eq!(
        registration.diagnostics[0].code,
        VoxelConversionDiagnosticCode::UnsupportedSourceAsset
    );

    let plan = bridge
        .plan_voxel_conversion(registered_source_plan_request(&registration_request))
        .unwrap();
    assert_eq!(
        plan.diagnostics[0].code,
        VoxelConversionDiagnosticCode::UnsupportedSourceAsset
    );
}

#[test]
fn voxel_conversion_stale_source_hash_fails_closed() {
    let mut bridge = init_bridge();
    let mut request = project_voxel_conversion_request(7);
    request.source.source_hash = "sha256:stale".to_string();
    let plan = bridge.plan_voxel_conversion(request).unwrap();
    assert_eq!(
        plan.diagnostics[0].code,
        VoxelConversionDiagnosticCode::SourceHashMismatch
    );
}

#[test]
fn voxel_conversion_unsupported_source_fails_closed() {
    let mut bridge = init_bridge();
    let mut request = project_voxel_conversion_request(7);
    request.source.asset_id = "mesh/not-loaded".to_string();
    request.source.source_hash = "sha256:not-loaded".to_string();
    let plan = bridge.plan_voxel_conversion(request).unwrap();
    assert_eq!(
        plan.diagnostics[0].code,
        VoxelConversionDiagnosticCode::UnsupportedSourceAsset
    );
}

#[test]
fn voxel_conversion_apply_to_unregistered_target_returns_diagnostic_receipt() {
    let mut bridge = init_bridge();
    let plan = bridge
        .plan_voxel_conversion(project_voxel_conversion_request(999))
        .unwrap();
    let preview = bridge
        .preview_voxel_conversion(VoxelConversionPreviewRequest {
            plan_id: plan.plan_id.clone(),
            expected_plan_hash: svc_voxel_conversion::plan_hash(&plan),
        })
        .unwrap();
    let receipt = bridge
        .apply_voxel_conversion(VoxelConversionApplyRequest {
            plan_id: plan.plan_id.clone(),
            expected_plan_hash: svc_voxel_conversion::plan_hash(&plan),
            expected_preview_hash: Some(preview.output_hash),
        })
        .unwrap();
    assert!(!receipt.applied);
    assert_eq!(
        receipt.diagnostics[0].code,
        VoxelConversionDiagnosticCode::ConversionReplayMismatch
    );
}

#[test]
fn voxel_model_info_missing_target_fails_closed_with_diagnostic_readout() {
    let bridge = init_bridge();
    let readout = bridge
        .read_voxel_model_info(VoxelModelInfoRequest {
            grid: 999,
            volume_asset_id: Some("voxel/missing".to_string()),
            include_material_counts: true,
        })
        .unwrap();
    assert!(!readout.resident);
    assert_eq!(readout.voxel_count, 0);
    assert!(readout.material_counts.is_empty());
    assert_eq!(
        readout.diagnostics[0].code,
        VoxelConversionDiagnosticCode::VoxelConversionUnavailable
    );
    assert!(readout.session_hash.starts_with("fnv1a64:"));
    assert!(readout.replay_hash.starts_with("fnv1a64:"));
}

fn set_voxel(coord: VoxelCoord, material: u16) -> VoxelCommand {
    VoxelCommand::SetVoxel {
        grid: GridId::new(1),
        coord,
        value: VoxelValue::solid_raw(material),
    }
}

#[test]
fn submit_before_init_fails_closed() {
    let mut bridge = ReferenceBridge::new();
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
    assert_eq!(before.world_hash, "27f89a36b51a8cb7");
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
    assert_ne!(after.world_hash, before.world_hash);
    assert_ne!(after_chunk.mesh_hash.as_ref().unwrap(), &before_hash);
    assert_eq!(after_chunk.material_slots, vec![1, 2]);
    assert!(after_chunk.stats.unwrap().quads > before_chunk.stats.unwrap().quads);
}

#[test]
fn mesh_evidence_fails_closed_before_init_and_unknown_grid() {
    let bridge = ReferenceBridge::new();
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
    let bridge = ReferenceBridge::new();
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
    let mut bridge = ReferenceBridge::new();
    bridge
        .initialize_engine(EngineConfig { seed: 0x01020304 })
        .unwrap();
    let view = bridge.get_buffer(RuntimeBufferHandle::new(0)).unwrap();
    assert_eq!(view.bytes, &0x01020304u64.to_le_bytes());
    let err = bridge.get_buffer(RuntimeBufferHandle::new(99)).unwrap_err();
    assert_eq!(err.kind, RuntimeBridgeErrorKind::UnknownHandle);
}
