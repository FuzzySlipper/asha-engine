use super::tests::{fps_load_request, init_bridge};
use super::*;

fn fps_load_request_with_authority_game_rule(enemy_health: u32) -> FpsRuntimeSessionLoadRequest {
    FpsRuntimeSessionLoadRequest {
        game_rule_modules: vec![built_in_game_rule_declared_manifest()],
        ..fps_load_request(enemy_health)
    }
}

fn downstream_game_rule_module_ref() -> GameRuleModuleRef {
    GameRuleModuleRef {
        module_id: "demo.primary_fire_effect".to_string(),
        version: "0.2.0".to_string(),
        contract_hash: "sha256:demo-primary-fire-effect-contract-v0".to_string(),
    }
}

fn downstream_game_rule_manifest() -> GameRuleModuleManifest {
    GameRuleModuleManifest {
        module_ref: downstream_game_rule_module_ref(),
        declared_hooks: vec![GameRuleHookDeclaration {
            hook_id: "demo.weapon.primary_fire_effect".to_string(),
            kind: GameExtensionHookKind::WeaponEffect,
            input_contract: WEAPON_EFFECT_INPUT_CONTRACT.to_string(),
            output_contract: GAME_EXTENSION_PROPOSAL_CONTRACT.to_string(),
            required_capabilities: vec!["health".to_string(), "weaponMount".to_string()],
        }],
        deterministic_requirements: GAME_RULE_DETERMINISTIC_REQUIREMENTS
            .iter()
            .map(|requirement| (*requirement).to_string())
            .collect(),
        source_hash: "sha256:demo-primary-fire-effect-source".to_string(),
    }
}

fn fps_load_request_with_downstream_game_rule(enemy_health: u32) -> FpsRuntimeSessionLoadRequest {
    FpsRuntimeSessionLoadRequest {
        game_rule_modules: vec![downstream_game_rule_manifest()],
        ..fps_load_request(enemy_health)
    }
}

fn weapon_effect_request(tick: u64) -> WeaponEffectHookRequest {
    weapon_effect_request_for(
        tick,
        built_in_game_rule_module_ref(),
        BUILT_IN_GAME_RULE_HOOK_ID.to_string(),
    )
}

fn downstream_weapon_effect_request(tick: u64) -> WeaponEffectHookRequest {
    weapon_effect_request_for(
        tick,
        downstream_game_rule_module_ref(),
        "demo.weapon.primary_fire_effect".to_string(),
    )
}

fn weapon_effect_request_for(
    tick: u64,
    module_ref: GameRuleModuleRef,
    hook_id: String,
) -> WeaponEffectHookRequest {
    WeaponEffectHookRequest {
        module_ref,
        hook_id,
        request_id: format!("request.primary-fire.{tick}"),
        tick,
        source: EntityId::new(101),
        target: Some(EntityId::new(777)),
        base_damage: 25,
        range_millimeters: 16_000,
        tags: vec!["primary-fire".to_string()],
        input_hash: format!("fnv1a64:{}", EngineBridge::fnv1a64(&format!("hook|{tick}"))),
    }
}

fn generic_game_rule_catalog() -> GameRuleCatalog {
    GameRuleCatalog {
        catalog: protocol_game_rules::GameRuleCatalogRef {
            catalog_id: "catalog.conformance.game-rules".to_string(),
            version: "1.0.0".to_string(),
            content_hash: "fnv1a64:catalog-conformance".to_string(),
        },
        value_channels: vec![protocol_game_rules::GameRuleValueChannelRef {
            channel_id: "value.health".to_string(),
            display_name: Some("Health".to_string()),
        }],
        bundles: vec![protocol_game_rules::GameRuleEffectBundle {
            bundle_id: "bundle.conformance.damage".to_string(),
            effect_ops: vec![protocol_game_rules::GameRuleEffectOp::ApplyDelta {
                op_id: "op.conformance.damage".to_string(),
                channel_id: "value.health".to_string(),
                amount: -7,
                tags: vec!["conformance".to_string()],
            }],
            modifiers: Vec::new(),
            tags: vec!["conformance".to_string()],
            source_hash: "fnv1a64:bundle-conformance".to_string(),
        }],
    }
}

#[test]
fn generic_game_rule_operations_validate_resolve_and_publish_runtime_readout() {
    let mut bridge = init_bridge();
    let catalog = generic_game_rule_catalog();

    let validation = bridge.validate_game_rule_catalog(catalog.clone()).unwrap();
    assert!(validation.accepted, "{:?}", validation.diagnostics);
    assert_eq!(validation.evidence.len(), 1);

    let resolution = bridge
        .submit_game_rule_effect_intent(GameRuleEffectIntentRequest {
            request: GameRuleResolutionRequest {
                catalog: catalog.catalog.clone(),
                bundle_id: "bundle.conformance.damage".to_string(),
                source: EntityId::new(101),
                target: EntityId::new(777),
                values: vec![protocol_game_rules::GameRuleBoundedValue {
                    channel_id: "value.health".to_string(),
                    min: 0,
                    current: 50,
                    max: 100,
                }],
                tick: 9,
            },
            catalog,
        })
        .unwrap();
    assert!(resolution.accepted, "{:?}", resolution.diagnostics);
    assert_eq!(resolution.pending_value_deltas[0].amount, -7);

    let readout = bridge.read_game_rule_runtime_readout().unwrap();
    assert_eq!(readout.backend, "engine_bridge_rust");
    assert_eq!(readout.latest_replay_hash, Some(resolution.replay_hash));
    assert!(!readout.recent_trace.is_empty());
}

#[test]
fn game_extension_weapon_effect_requires_declared_module() {
    let mut bridge = init_bridge();
    bridge
        .load_fps_runtime_session(fps_load_request(75))
        .expect("fps session loads");

    let err = bridge
        .invoke_game_extension_weapon_effect(GameExtensionWeaponEffectInvocationRequest {
            hook: weapon_effect_request(9),
            primary_fire: FpsPrimaryFireRequest {
                tick: 9,
                origin: [2.5, 1.5, 1.5],
                direction: [0.0, 0.0, 1.0],
                shooter_role: None,
                target_role: None,
            },
        })
        .expect_err("missing module declaration rejects");

    assert_eq!(err.kind, RuntimeBridgeErrorKind::InvalidInput);
    assert!(err.message.contains("is not declared"));
}

#[test]
fn game_extension_weapon_effect_applies_validated_proposal_through_combat_authority() {
    let mut bridge = init_bridge();
    let mut request = fps_load_request_with_authority_game_rule(75);
    request.definitions[0]
        .weapon
        .as_mut()
        .expect("player weapon")
        .damage = 25;
    bridge
        .load_fps_runtime_session(request)
        .expect("fps session loads with module declaration");

    let result = bridge
        .invoke_game_extension_weapon_effect(GameExtensionWeaponEffectInvocationRequest {
            hook: weapon_effect_request(9),
            primary_fire: FpsPrimaryFireRequest {
                tick: 9,
                origin: [2.5, 1.5, 1.5],
                direction: [0.0, 0.0, 1.0],
                shooter_role: None,
                target_role: None,
            },
        })
        .expect("extension hook invokes and applies");
    let primary_fire = result.primary_fire.expect("accepted primary fire");

    assert_eq!(
        result.hook_receipt.status,
        GameExtensionReceiptStatus::Proposed
    );
    assert_eq!(
        result.hook_receipt.proposal_hash,
        result.replay_evidence.proposal_hash
    );
    assert_eq!(result.replay_evidence.validation_status, "accepted");
    assert_eq!(
        result.hook_receipt.trace[1].code,
        "gameplayFabric.transformAccepted"
    );
    assert_eq!(result.hook_receipt.trace[1].refs.len(), 3);
    assert!(result.hook_receipt.trace[1].refs[0].starts_with("fnv1a64:"));
    assert_eq!(
        result.replay_evidence.event_hashes,
        vec![format!("fnv1a64:{:016x}", primary_fire.replay_hash)]
    );
    assert_eq!(primary_fire.target, Some(777));
    assert_eq!(
        result.hook_receipt.module_ref.module_id,
        BUILT_IN_GAME_RULE_MODULE_ID
    );
    assert_eq!(
        primary_fire.target_health_after,
        Some(FpsBridgeHealth {
            current: 45,
            max: 75
        })
    );
    let projection = bridge
        .read_projection_frame(0)
        .expect("accepted extension fire publishes a projection frame");
    assert_eq!(projection.authority_tick, 9);
    assert_eq!(projection.presentation.ops.len(), 8);
    let PresentationOp::Audio { meta, .. } = &projection.presentation.ops[0] else {
        panic!("audio remains the first presentation operation")
    };
    assert_eq!(
        meta.origin
            .as_ref()
            .expect("audio projection retains its authority origin")
            .id,
        format!("combat.primary-fire.accepted:{}", primary_fire.replay_hash)
    );
    let animation = projection
        .presentation
        .ops
        .iter()
        .find_map(|operation| match operation {
            PresentationOp::Animation {
                meta,
                op: AnimationProjectionOp::Update { controller, .. },
            } => Some((meta, controller)),
            _ => None,
        })
        .expect("accepted extension fire publishes controller transition state");
    assert_eq!(
        animation.0.origin.as_ref().expect("origin").id,
        format!("combat.primary-fire.accepted:{}", primary_fire.replay_hash)
    );
    let timing = animation
        .1
        .timing_fact
        .as_ref()
        .expect("controller state retains the durable transition fact");
    assert_eq!(
        timing.source_fact_id,
        animation.0.origin.as_ref().unwrap().id
    );
    assert_eq!(timing.authority_tick, 9);
    assert_eq!(timing.to_state_id, "primary_fire");
    assert_eq!(
        animation
            .1
            .transition
            .as_ref()
            .unwrap()
            .target_motion
            .blend_weight_milli,
        650
    );
}

#[test]
fn game_extension_weapon_effect_invokes_downstream_registered_module_ref() {
    let mut bridge = init_bridge();
    let mut request = fps_load_request_with_downstream_game_rule(75);
    request.definitions[0]
        .weapon
        .as_mut()
        .expect("player weapon")
        .damage = 25;
    bridge
        .load_fps_runtime_session(request)
        .expect("fps session loads with downstream module declaration");

    let result = bridge
        .invoke_game_extension_weapon_effect(GameExtensionWeaponEffectInvocationRequest {
            hook: downstream_weapon_effect_request(10),
            primary_fire: FpsPrimaryFireRequest {
                tick: 10,
                origin: [2.5, 1.5, 1.5],
                direction: [0.0, 0.0, 1.0],
                shooter_role: None,
                target_role: None,
            },
        })
        .expect("downstream extension hook invokes and applies");
    let primary_fire = result.primary_fire.expect("accepted primary fire");

    assert_eq!(
        result.hook_receipt.status,
        GameExtensionReceiptStatus::Proposed
    );
    assert_eq!(
        result.hook_receipt.module_ref,
        downstream_game_rule_module_ref()
    );
    assert_eq!(
        result.replay_evidence.module_ref,
        downstream_game_rule_module_ref()
    );
    assert_ne!(
        result.hook_receipt.module_ref.module_id,
        BUILT_IN_GAME_RULE_MODULE_ID
    );
    assert_eq!(
        result.hook_receipt.trace[0].refs,
        vec![
            "demo.primary_fire_effect".to_string(),
            "0.2.0".to_string(),
            "sha256:demo-primary-fire-effect-contract-v0".to_string()
        ]
    );
    let GameExtensionProposal::DamageModifier { tags, .. } =
        result.hook_receipt.proposal.as_ref().expect("proposal")
    else {
        panic!("downstream module should propose a damage modifier");
    };
    assert!(tags.contains(&"registered-rust-module".to_string()));
    assert!(tags.contains(&"demo.primary_fire_effect".to_string()));
    assert_eq!(primary_fire.target, Some(777));
    assert_eq!(
        primary_fire.target_health_after,
        Some(FpsBridgeHealth {
            current: 45,
            max: 75
        })
    );
}

#[test]
fn game_extension_weapon_effect_commits_lethal_demo_damage_to_session_readout() {
    let mut bridge = init_bridge();
    let mut request = fps_load_request_with_downstream_game_rule(40);
    request.definitions[0]
        .weapon
        .as_mut()
        .expect("player weapon")
        .damage = 40;
    bridge
        .load_fps_runtime_session(request)
        .expect("fps session loads with downstream module declaration");

    let result = bridge
        .invoke_game_extension_weapon_effect(GameExtensionWeaponEffectInvocationRequest {
            hook: downstream_weapon_effect_request(11),
            primary_fire: FpsPrimaryFireRequest {
                tick: 11,
                origin: [2.5, 1.5, 1.5],
                direction: [0.0, 0.0, 1.0],
                shooter_role: Some(FpsBridgeRole::Player),
                target_role: Some(FpsBridgeRole::Enemy),
            },
        })
        .expect("downstream extension hook invokes and applies");
    let primary_fire = result.primary_fire.expect("accepted primary fire");

    assert_eq!(primary_fire.target, Some(777));
    assert_eq!(
        primary_fire.target_health_before,
        Some(FpsBridgeHealth {
            current: 40,
            max: 40
        })
    );
    assert_eq!(
        primary_fire.target_health_after,
        Some(FpsBridgeHealth {
            current: 0,
            max: 40
        })
    );
    let snapshot = bridge
        .read_fps_runtime_session()
        .expect("session readout reflects committed damage");
    assert_eq!(
        snapshot
            .health
            .iter()
            .find(|entry| entry.entity == 777)
            .map(|entry| (entry.current, entry.max)),
        Some((0, 40))
    );
    assert!(matches!(
        snapshot.lifecycle_status,
        FpsBridgeLifecycleStatus::EnemyDefeated {
            entity: 777,
            tick: 11
        }
    ));
}

#[test]
fn game_extension_weapon_effect_rejects_incompatible_downstream_manifest() {
    let mut bridge = init_bridge();
    let mut request = fps_load_request(75);
    let mut manifest = downstream_game_rule_manifest();
    manifest.declared_hooks[0].input_contract = "WeaponEffectHookRequest.future".to_string();
    request.game_rule_modules = vec![manifest];

    let err = bridge
        .load_fps_runtime_session(request)
        .expect_err("incompatible downstream manifest rejects");

    assert_eq!(err.kind, RuntimeBridgeErrorKind::InvalidInput);
    assert!(err.message.contains("incompatible contract"));
}
