use super::*;

fn centered_tunnel_fps_load_request(enemy_health: u32) -> FpsRuntimeSessionLoadRequest {
    let mut request = fps_load_request(enemy_health);
    for definition in &mut request.definitions {
        match definition.role {
            FpsBridgeRole::Player => {
                definition.transform = Some(FpsBridgeTransformCapability {
                    translation: [0.0, 1.62, 1.5],
                    rotation: [0.0, 0.0, 0.0, 1.0],
                    scale: [1.0, 1.0, 1.0],
                });
                definition.bounds = Some(FpsBridgeBoundsCapability {
                    min: [-0.25, 0.92, 1.25],
                    max: [0.25, 2.32, 1.75],
                });
            }
            FpsBridgeRole::Enemy => {
                definition.transform = Some(FpsBridgeTransformCapability {
                    translation: [0.0, 0.5, -2.6],
                    rotation: [0.0, 0.0, 0.0, 1.0],
                    scale: [1.0, 1.0, 1.0],
                });
                definition.bounds = Some(FpsBridgeBoundsCapability {
                    min: [-0.25, 0.0, -2.85],
                    max: [0.25, 1.0, -2.35],
                });
            }
            FpsBridgeRole::Neutral => {}
        }
    }
    for node in &mut request.scene_document.nodes {
        let SceneNodeKindDto::EntityInstance { instance } = &node.kind else {
            continue;
        };
        let SceneEntityReferenceDto::EntityDefinition { stable_id } = &instance.reference else {
            continue;
        };
        node.transform.translation = match stable_id.as_str() {
            "actor/custom-player" => [0.0, 1.62, 1.5],
            "actor/custom-enemy" => [0.0, 0.5, -2.6],
            _ => node.transform.translation,
        };
    }
    request
}

fn stored_tunnel_materialization(
    base: &FlatSceneDocumentDto,
) -> svc_environment_authoring::MaterializedEnvironment {
    let scene = EngineBridge::scene_document_from_dto(base.clone()).unwrap();
    svc_environment_authoring::materialize_environment(
        &scene,
        &svc_environment_authoring::EnvironmentMaterializationInput {
            provider_id: svc_levelgen::TUNNEL_GENERATOR_ID.to_owned(),
            preset_id: "tiny-enclosed".to_owned(),
            seed: 42,
            target: svc_environment_authoring::EnvironmentTarget {
                scene_path: "scenes/runtime-saved.scene.json".to_owned(),
                asset_id: "voxel-volume/runtime-saved-tunnel".to_owned(),
                asset_path: "assets/runtime-saved-tunnel.avxl.json".to_owned(),
                voxel_node_id: SceneNodeId::new(9010),
                voxel_parent_id: None,
                voxel_child_order: 3,
                voxel_label: Some("Saved tunnel".to_owned()),
                voxel_transform: core_scene::SceneTransform {
                    translation: Vec3::new(-3.5, -1.0, -5.5),
                    ..core_scene::SceneTransform::IDENTITY
                },
                marker_targets: vec![
                    ProceduralEnvironmentMarkerTargetDto {
                        source_marker_id: "player_start".to_owned(),
                        node_id: SceneNodeId::new(9011),
                        marker_id: "spawn/runtime-player".to_owned(),
                        child_order: 0,
                    },
                    ProceduralEnvironmentMarkerTargetDto {
                        source_marker_id: "exit_hint".to_owned(),
                        node_id: SceneNodeId::new(9012),
                        marker_id: "navigation/runtime-exit".to_owned(),
                        child_order: 1,
                    },
                ],
            },
            material_palette: [1u16, 2, 3]
                .into_iter()
                .map(|material| VoxelAssetMaterialBinding {
                    voxel_material: material,
                    palette_entry_id: format!("voxel-material/runtime-{material}"),
                    display_name: None,
                    material_asset_id: format!("material/runtime-{material}"),
                    material_catalog_binding_id: Some(format!(
                        "catalog-binding/runtime-{material}"
                    )),
                })
                .collect(),
            authoring: VoxelAssetAuthoringMetadata {
                label: Some("Saved runtime tunnel".to_owned()),
                created_by: Some("runtime-test".to_owned()),
                source_tool: Some("Studio".to_owned()),
            },
            limits: ProceduralEnvironmentLimitsDto {
                max_voxels: 10_000,
                max_sparse_runs: 10_000,
                max_markers: 8,
            },
        },
    )
    .unwrap()
}

#[test]
fn fps_runtime_session_loads_project_bundle_through_rust_authority() {
    let mut bridge = init_bridge();
    let snapshot = bridge
        .load_fps_runtime_session(fps_load_request(75))
        .expect("fps session loads");

    assert_eq!(snapshot.backend, "engine_bridge_rust");
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
fn fresh_runtime_derives_voxel_collision_and_projection_from_saved_scene_and_asset() {
    let mut bridge = init_bridge();
    let mut request = fps_load_request(75);
    request.scene_document.schema_version = 4;
    request.scene_document.metadata.authoring_format_version = 4;
    request.scene_document.nodes.push(SceneNodeRecordDto {
        id: SceneNodeId::new(9000),
        parent: None,
        child_order: 2,
        label: Some("Stored recipe".to_owned()),
        tags: Vec::new(),
        transform: SceneTransformDto {
            translation: [0.0; 3],
            rotation: [0.0, 0.0, 0.0, 1.0],
            scale: [1.0; 3],
        },
        kind: SceneNodeKindDto::Bootstrap {
            bindings: SceneBootstrapBindingsDto {
                generator: Some(SceneGeneratorBindingDto {
                    provider_id: svc_levelgen::TUNNEL_GENERATOR_ID.to_owned(),
                    preset_id: "tiny-enclosed".to_owned(),
                    seed: 42,
                }),
                catalogs: Vec::new(),
            },
        },
    });
    assert!(request
        .bootstrap_resolution_registry
        .generator_presets
        .is_empty());
    let materialized = stored_tunnel_materialization(&request.scene_document);
    assert!(materialized.scene.nodes.iter().all(|record| {
        !matches!(
            &record.kind,
            core_scene::SceneNodeKind::Bootstrap(bindings) if bindings.generator.is_some()
        )
    }));
    let decoded_scene = core_scene::decode(&materialized.scene_json).unwrap();
    let decoded_asset = svc_voxel_asset::decode_asset(&materialized.asset_json).unwrap();
    assert_eq!(
        decoded_asset.content_hashes,
        materialized.asset.content_hashes
    );
    assert_eq!(decoded_asset.provenance, materialized.asset.provenance);
    assert_eq!(decoded_asset.provenance.len(), 1);
    assert_eq!(
        decoded_asset.provenance[0].kind,
        VoxelAssetProvenanceKind::Generated
    );
    assert_eq!(
        decoded_asset.provenance[0].uri,
        format!(
            "asha-generator://{}/{}/v{}?seed={}&configHash={}",
            materialized.provenance.provider_id,
            materialized.provenance.preset_id,
            materialized.provenance.provider_version,
            materialized.provenance.seed,
            materialized.provenance.config_hash,
        )
    );
    assert_eq!(
        decoded_asset.provenance[0].content_hash,
        materialized.provenance.output_hash
    );
    assert_eq!(decoded_scene.id, materialized.scene.id);
    assert_eq!(core_scene::encode(&decoded_scene), materialized.scene_json);
    request.scene_document = EngineBridge::scene_document_dto(&decoded_scene);

    bridge.load_fps_runtime_session(request).unwrap();
    let receipt = bridge
        .load_voxel_volume_asset(VoxelVolumeAssetLoadRequest {
            asset: decoded_asset.clone(),
            target_grid: 8,
            target_volume_asset_id: Some(decoded_asset.asset_id.clone()),
            replace_existing: true,
            include_material_counts: true,
        })
        .unwrap();
    assert!(receipt.loaded, "{:?}", receipt.diagnostics);
    assert_eq!(
        receipt.canonical_json_hash,
        Some(decoded_asset.content_hashes.canonical_json.clone())
    );
    assert_eq!(bridge.voxel.collision_world_offset, [-3.5, -1.0, -5.5]);

    let projection = bridge.read_render_diffs(0).unwrap();
    assert!(projection.ops.iter().any(|operation| {
        matches!(
            operation,
            protocol_render::RenderDiff::Create { node, .. }
                if node.transform.translation == [-3.5, -1.0, -5.5]
        )
    }));
    assert!(projection.ops.iter().any(|operation| {
        matches!(
            operation,
            protocol_render::RenderDiff::ReplaceMeshPayload { .. }
        )
    }));

    assert_eq!(
        core_scene::encode(bridge.scene.scene_document.as_ref().unwrap()),
        materialized.scene_json
    );
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
            shooter_role: None,
            target_role: None,
        })
        .expect("primary fire applies");

    assert_eq!(receipt.backend, "engine_bridge_rust");
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

    let projection = bridge
        .read_projection_frame(0)
        .expect("accepted owner fact projects through the shared frame");
    assert_eq!(projection.authority_tick, 9);
    assert!(projection.scene.is_empty());
    assert_eq!(
        projection.presentation.replay_scope,
        ProjectionReplayScope::ExcludedFromReplayTruth
    );
    assert_eq!(projection.presentation.ops.len(), 8);
    let accepted_origin = match &projection.presentation.ops[0] {
        PresentationOp::Audio { meta, .. } => meta
            .origin
            .as_ref()
            .expect("primary-fire audio retains the accepted origin"),
        other => panic!("expected primary-fire audio first, got {other:?}"),
    };
    for operation in &projection.presentation.ops {
        let origin = match operation {
            PresentationOp::Audio { meta, .. }
            | PresentationOp::Billboard { meta, .. }
            | PresentationOp::Particle { meta, .. }
            | PresentationOp::TelemetryOverlay { meta, .. }
            | PresentationOp::Animation { meta, .. } => meta.origin.as_ref(),
        };
        assert_eq!(origin, Some(accepted_origin));
    }
    match &projection.presentation.ops[0] {
        PresentationOp::Audio { meta, op } => {
            assert_eq!(meta.sequence, 0);
            assert_eq!(
                meta.origin.as_ref().map(|origin| origin.kind),
                Some(PresentationOriginKind::OwnerFact)
            );
            assert!(meta
                .origin
                .as_ref()
                .is_some_and(|origin| origin.id.contains(&receipt.replay_hash.to_string())));
            match op {
                AudioProjectionOp::Emit {
                    signal_id,
                    descriptor,
                } => {
                    assert!(signal_id.contains(&receipt.replay_hash.to_string()));
                    assert_eq!(descriptor.clip.asset, "audio/asha-primary-fire-pulse");
                    assert_eq!(descriptor.bus, AudioBus::Sfx);
                    assert!(!descriptor.looping);
                    assert!(matches!(descriptor.emitter, AudioEmitter::World3d { .. }));
                }
                other => panic!("expected one-shot audio projection, got {other:?}"),
            }
        }
        other => panic!("expected audio first under scene-first G1 ordering, got {other:?}"),
    }
    let PresentationOp::Particle { meta, op } = &projection.presentation.ops[1] else {
        panic!("expected particle burst after audio")
    };
    assert_eq!(meta.sequence, 1);
    assert!(matches!(
        op,
        ParticleProjectionOp::Emit {
            descriptor: ParticleEmitterDescriptor {
                anchor: ParticleAnchor::EntityAttached { entity: 777, .. },
                burst_count: 12,
                ..
            },
            ..
        }
    ));
    let PresentationOp::Billboard { meta, op } = &projection.presentation.ops[2] else {
        panic!("expected player billboard after particles")
    };
    assert_eq!(meta.sequence, 2);
    assert!(matches!(
        op,
        BillboardProjectionOp::Create {
            descriptor: BillboardDescriptor {
                anchor: BillboardAnchor::EntityAttached { entity: 101, .. },
                ..
            },
            ..
        }
    ));
    let PresentationOp::Billboard { meta, op } = &projection.presentation.ops[3] else {
        panic!("expected target health billboard after player billboard")
    };
    assert_eq!(meta.sequence, 3);
    assert!(matches!(
        op,
        BillboardProjectionOp::Create {
            descriptor: BillboardDescriptor {
                content: BillboardContent::Value { value, .. },
                visible: true,
                ..
            },
            ..
        } if value == "0/75"
    ));
    let PresentationOp::Animation { meta, op } = &projection.presentation.ops[5] else {
        panic!("expected authoritative controller update after its create")
    };
    assert_eq!(meta.sequence, 5);
    let AnimationProjectionOp::Update { controller, .. } = op else {
        panic!("expected animation controller update")
    };
    let timing_fact = controller
        .timing_fact
        .as_ref()
        .expect("semantic transition retains authority timing evidence");
    assert_eq!(timing_fact.authority_tick, 9);
    assert_eq!(timing_fact.controller_tick, 1);
    assert_eq!(timing_fact.to_state_id, "primary_fire");
    assert_eq!(
        timing_fact.source_fact_id,
        meta.origin.as_ref().expect("animation origin").id
    );
    let PresentationOp::TelemetryOverlay { meta, op } = &projection.presentation.ops[7] else {
        panic!("expected telemetry overlay after gameplay feedback domains")
    };
    assert_eq!(meta.sequence, 7);
    assert_eq!(
        meta.origin.as_ref().expect("telemetry origin").id,
        timing_fact.source_fact_id
    );
    assert!(matches!(
        op,
        TelemetryOverlayProjectionOp::Create {
            descriptor: TelemetryOverlayDescriptor {
                visible: true,
                refresh_interval_ms: 250,
                ..
            },
            ..
        }
    ));
    assert!(bridge.read_projection_frame(10).is_err());
}

#[test]
fn generated_tunnel_preserves_explicit_role_targeted_enemy_damage() {
    let mut bridge = init_bridge();
    bridge
        .load_fps_runtime_session(centered_tunnel_fps_load_request(75))
        .unwrap();
    bridge
        .apply_generated_tunnel_to_runtime_world(GeneratedTunnelRuntimeApplyRequest {
            preset: GeneratedTunnelPreset::TinyEnclosed,
            seed: 17,
        })
        .unwrap();

    let receipt = bridge
        .apply_fps_primary_fire(FpsPrimaryFireRequest {
            tick: 6,
            origin: [0.0, 0.5, -2.6],
            direction: [0.0, 1.12, 4.1],
            shooter_role: Some(FpsBridgeRole::Enemy),
            target_role: Some(FpsBridgeRole::Player),
        })
        .expect("role-targeted enemy fire remains authoritative after tunnel apply");

    assert_eq!(receipt.shooter, 777);
    assert_eq!(receipt.target, Some(101));
    assert_eq!(
        receipt.target_health_before,
        Some(FpsBridgeHealth {
            current: 88,
            max: 88,
        })
    );
    assert_eq!(
        receipt.target_health_after,
        Some(FpsBridgeHealth {
            current: 78,
            max: 88,
        })
    );
    let snapshot = bridge.read_fps_runtime_session().unwrap();
    assert_eq!(
        snapshot
            .health
            .iter()
            .find(|health| health.entity == 101)
            .map(|health| health.current),
        Some(78)
    );

    let enemy_projection = bridge.read_projection_frame(0).unwrap();
    assert!(enemy_projection
        .presentation
        .ops
        .iter()
        .all(|operation| !matches!(operation, PresentationOp::Animation { .. })));
    assert!(enemy_projection.presentation.ops.iter().any(|operation| {
        matches!(
            operation,
            PresentationOp::Billboard {
                op: BillboardProjectionOp::Create {
                    descriptor: BillboardDescriptor {
                        content: BillboardContent::Text { fallback_text, .. },
                        ..
                    },
                    ..
                },
                ..
            } if fallback_text == "Enemy"
        )
    }));
    assert!(enemy_projection.presentation.ops.iter().any(|operation| {
        matches!(
            operation,
            PresentationOp::Billboard {
                op: BillboardProjectionOp::Create {
                    descriptor: BillboardDescriptor {
                        content: BillboardContent::Value { fallback_label, .. },
                        ..
                    },
                    ..
                },
                ..
            } if fallback_label == "Player health"
        )
    }));

    let player_receipt = bridge
        .apply_fps_primary_fire(FpsPrimaryFireRequest {
            tick: 7,
            origin: [0.0, 1.62, 1.5],
            direction: [0.0, -1.12, -4.1],
            shooter_role: Some(FpsBridgeRole::Player),
            target_role: Some(FpsBridgeRole::Enemy),
        })
        .expect("enemy feedback cannot claim the player animation controller");
    assert_eq!(player_receipt.shooter, 101);
    let player_projection = bridge.read_projection_frame(0).unwrap();
    assert!(player_projection
        .presentation
        .ops
        .iter()
        .any(|operation| matches!(operation, PresentationOp::Animation { .. })));
}

#[test]
fn explicit_role_pair_does_not_disable_generated_tunnel_occlusion() {
    let mut bridge = init_bridge();
    bridge
        .load_fps_runtime_session(centered_tunnel_fps_load_request(75))
        .unwrap();
    bridge
        .apply_generated_tunnel_to_runtime_world(GeneratedTunnelRuntimeApplyRequest {
            preset: GeneratedTunnelPreset::TinyEnclosed,
            seed: 17,
        })
        .unwrap();

    let receipt = bridge
        .apply_fps_primary_fire(FpsPrimaryFireRequest {
            tick: 6,
            origin: [0.0, 0.0, 0.0],
            direction: [0.0, 1.62, 1.5],
            shooter_role: Some(FpsBridgeRole::Enemy),
            target_role: Some(FpsBridgeRole::Player),
        })
        .expect("explicit roles select identities without bypassing tunnel geometry");

    assert_eq!(receipt.shooter, 777);
    assert_eq!(receipt.target, None);
    assert_eq!(receipt.target_health_before, receipt.target_health_after);
    assert_eq!(
        receipt.target_health_after,
        Some(FpsBridgeHealth {
            current: 88,
            max: 88,
        })
    );
}

#[test]
fn autonomous_enemy_movement_moves_authoritative_combat_bounds() {
    let mut bridge = init_bridge();
    bridge
        .load_fps_runtime_session(centered_tunnel_fps_load_request(75))
        .unwrap();
    bridge
        .apply_generated_tunnel_to_runtime_world(GeneratedTunnelRuntimeApplyRequest {
            preset: GeneratedTunnelPreset::TinyEnclosed,
            seed: 17,
        })
        .unwrap();

    let authored_pose_receipt = bridge
        .apply_fps_primary_fire(FpsPrimaryFireRequest {
            tick: 5,
            origin: [0.0, 1.62, 1.5],
            direction: [0.0, 0.0, -1.0],
            shooter_role: None,
            target_role: None,
        })
        .expect("ray above the authored enemy bounds remains a miss");
    assert_eq!(authored_pose_receipt.target, None);
    assert_eq!(
        authored_pose_receipt.target_health_after,
        Some(FpsBridgeHealth {
            current: 75,
            max: 75,
        })
    );
    let before_movement = bridge.read_fps_runtime_session().unwrap();

    let moved = bridge
        .apply_enemy_direct_nav_movement(EnemyDirectNavMovementRequest {
            entity: 777,
            seed_position: Vec3::new(0.0, 0.5, -2.6),
            target: Vec3::new(0.0, 1.62, 1.15),
            max_step_units: 8.0,
        })
        .expect("autonomous movement applies through the loaded FPS entity store");
    assert_eq!(
        moved.authority_source,
        EnemyDirectNavAuthoritySource::RustEntityStore
    );
    assert_eq!(moved.next_waypoint, Vec3::new(0.0, 1.62, 1.15));
    assert!(moved.reached);
    let after_movement = bridge.read_fps_runtime_session().unwrap();
    assert_ne!(after_movement.entity_hash, before_movement.entity_hash);
    assert_eq!(after_movement.health_hash, before_movement.health_hash);
    assert_eq!(
        after_movement.replay_records.len(),
        before_movement.replay_records.len() + 1
    );
    assert_eq!(
        after_movement.replay_records.last().unwrap().replay_unit,
        "runtime_session.fps.autonomous_movement.v0"
    );
    assert_eq!(
        after_movement.replay_records.last().unwrap().entity_hash,
        after_movement.entity_hash
    );
    assert_ne!(after_movement.replay_hash, before_movement.replay_hash);

    let receipt = bridge
        .apply_fps_primary_fire(FpsPrimaryFireRequest {
            tick: 6,
            origin: [0.0, 1.62, 1.5],
            direction: [0.0, 0.0, -1.0],
            shooter_role: None,
            target_role: None,
        })
        .expect("primary fire raycasts against the moved enemy bounds");

    assert_eq!(receipt.target, Some(777));
    assert_eq!(
        receipt.target_health_after,
        Some(FpsBridgeHealth {
            current: 0,
            max: 75,
        })
    );
}

#[test]
fn public_enemy_nav_rejects_non_enemy_fps_entities_without_mutation() {
    let mut bridge = init_bridge();
    let mut load = fps_load_request(75);
    let mut neutral = load.definitions[0].clone();
    neutral.entity = 303;
    neutral.stable_id = "actor/custom-neutral".to_string();
    neutral.display_name = "Custom Neutral".to_string();
    neutral.source_path = "catalogs/actors/neutral.entity.json".to_string();
    neutral.tags = vec!["neutral".to_string()];
    neutral.role = FpsBridgeRole::Neutral;
    neutral.health = None;
    neutral.weapon = None;
    neutral.policy_binding = None;
    load.definitions.push(neutral);
    bridge.load_fps_runtime_session(load).unwrap();
    let before = bridge.read_fps_runtime_session().unwrap();

    for entity in [101, 303, 999] {
        let error = bridge
            .apply_enemy_direct_nav_movement(EnemyDirectNavMovementRequest {
                entity,
                seed_position: Vec3::new(0.0, 1.5, 0.0),
                target: Vec3::new(3.0, 1.62, 1.5),
                max_step_units: 8.0,
            })
            .expect_err("loaded FPS session rejects non-enemy movement");
        assert_eq!(error.kind, RuntimeBridgeErrorKind::InvalidInput);
        assert!(error.message.contains("UnauthorizedAutonomousMovement"));
        let after = bridge.read_fps_runtime_session().unwrap();
        assert_eq!(after.entity_hash, before.entity_hash);
        assert_eq!(after.health_hash, before.health_hash);
        assert_eq!(after.replay_hash, before.replay_hash);
        assert_eq!(after.replay_records, before.replay_records);
    }
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
    assert_eq!(pending.backend, "engine_bridge_rust");
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
            shooter_role: None,
            target_role: None,
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
            shooter_role: None,
            target_role: None,
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
    let projection = bridge.read_projection_frame(0).unwrap();
    assert_eq!(projection.authority_tick, 0);
    assert!(projection.presentation.ops.is_empty());
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
fn scene_reference_drift_from_bootstrap_registry_fails_before_session_replacement() {
    let mut bridge = init_bridge();
    let loaded = bridge
        .load_fps_runtime_session(fps_load_request(75))
        .unwrap();
    let mut invalid = fps_load_request(33);
    let SceneNodeKindDto::EntityInstance { instance } = &mut invalid.scene_document.nodes[1].kind
    else {
        panic!("FPS fixture enemy must be an entity instance");
    };
    instance.spawn_marker_id = Some("spawn.scene-invented".to_string());

    let err = bridge.load_fps_runtime_session(invalid).unwrap_err();
    assert_eq!(err.kind, RuntimeBridgeErrorKind::InvalidInput);
    assert!(err.message.contains("UnknownSpawnMarker"));

    let after = bridge.read_fps_runtime_session().unwrap();
    assert_eq!(after.session_epoch, loaded.session_epoch);
    assert_eq!(after.entity_hash, loaded.entity_hash);
    assert_eq!(after.health_hash, loaded.health_hash);
    assert_eq!(after.replay_hash, loaded.replay_hash);
}
