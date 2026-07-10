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
    request
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
