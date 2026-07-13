use core_ids::EntityId;
use rule_animation_controller::{
    validate_animation_catalog, AnimationCatalog, AnimationCatalogDiagnosticCode,
    AnimationClipAsset, AnimationCondition, AnimationControllerAuthority, AnimationGraphDefinition,
    AnimationInputOrigin, AnimationMotionDefinition, AnimationParameterDefinition,
    AnimationParameterKind, AnimationParameterValue, AnimationStateDefinition,
    AnimationTransitionDefinition, AnimationTransitionFactMoment,
};

fn catalog() -> AnimationCatalog {
    AnimationCatalog {
        schema_version: 1,
        catalog_id: "demo.animation".to_string(),
        assets: vec![AnimationClipAsset {
            asset_id: "character".to_string(),
            clips: vec![
                "idle".to_string(),
                "walk".to_string(),
                "run".to_string(),
                "jump".to_string(),
            ],
        }],
        graphs: vec![AnimationGraphDefinition {
            graph_id: "player".to_string(),
            version: 1,
            asset_id: "character".to_string(),
            initial_state_id: "idle".to_string(),
            parameters: vec![
                AnimationParameterDefinition {
                    parameter_id: "speed".to_string(),
                    kind: AnimationParameterKind::Float,
                    default_value: AnimationParameterValue::Float(0),
                },
                AnimationParameterDefinition {
                    parameter_id: "moving".to_string(),
                    kind: AnimationParameterKind::Bool,
                    default_value: AnimationParameterValue::Bool(false),
                },
                AnimationParameterDefinition {
                    parameter_id: "jump".to_string(),
                    kind: AnimationParameterKind::Trigger,
                    default_value: AnimationParameterValue::Trigger(false),
                },
            ],
            states: vec![
                AnimationStateDefinition {
                    state_id: "idle".to_string(),
                    motion: AnimationMotionDefinition::Clip {
                        clip_id: "idle".to_string(),
                        speed_milli: 1_000,
                    },
                },
                AnimationStateDefinition {
                    state_id: "locomotion".to_string(),
                    motion: AnimationMotionDefinition::LinearBlend {
                        parameter_id: "speed".to_string(),
                        low_clip_id: "walk".to_string(),
                        high_clip_id: "run".to_string(),
                        minimum_milli: 0,
                        maximum_milli: 1_000,
                        speed_milli: 1_000,
                    },
                },
                AnimationStateDefinition {
                    state_id: "airborne".to_string(),
                    motion: AnimationMotionDefinition::Clip {
                        clip_id: "jump".to_string(),
                        speed_milli: 1_000,
                    },
                },
            ],
            transitions: vec![
                AnimationTransitionDefinition {
                    transition_id: "idle.jump".to_string(),
                    from_state_id: "idle".to_string(),
                    to_state_id: "airborne".to_string(),
                    priority: 0,
                    duration_ticks: 0,
                    conditions: vec![AnimationCondition::TriggerSet {
                        parameter_id: "jump".to_string(),
                    }],
                },
                AnimationTransitionDefinition {
                    transition_id: "idle.move".to_string(),
                    from_state_id: "idle".to_string(),
                    to_state_id: "locomotion".to_string(),
                    priority: 1,
                    duration_ticks: 2,
                    conditions: vec![AnimationCondition::BoolEquals {
                        parameter_id: "moving".to_string(),
                        value: true,
                    }],
                },
                AnimationTransitionDefinition {
                    transition_id: "move.idle".to_string(),
                    from_state_id: "locomotion".to_string(),
                    to_state_id: "idle".to_string(),
                    priority: 1,
                    duration_ticks: 1,
                    conditions: vec![AnimationCondition::BoolEquals {
                        parameter_id: "moving".to_string(),
                        value: false,
                    }],
                },
                AnimationTransitionDefinition {
                    transition_id: "move.jump".to_string(),
                    from_state_id: "locomotion".to_string(),
                    to_state_id: "airborne".to_string(),
                    priority: 0,
                    duration_ticks: 0,
                    conditions: vec![AnimationCondition::TriggerSet {
                        parameter_id: "jump".to_string(),
                    }],
                },
                AnimationTransitionDefinition {
                    transition_id: "air.idle".to_string(),
                    from_state_id: "airborne".to_string(),
                    to_state_id: "idle".to_string(),
                    priority: 0,
                    duration_ticks: 1,
                    conditions: vec![AnimationCondition::BoolEquals {
                        parameter_id: "moving".to_string(),
                        value: false,
                    }],
                },
            ],
        }],
    }
}

fn exercise() -> AnimationControllerAuthority {
    let validated = validate_animation_catalog(catalog()).expect("valid catalog");
    let entity = EntityId::new(42);
    let mut authority = AnimationControllerAuthority::new(validated);
    assert!(authority
        .attach(entity, "player")
        .expect("attach")
        .change
        .is_some());
    assert!(authority
        .tick(entity, 1)
        .expect("idle tick")
        .change
        .is_none());
    authority
        .set_float(entity, "speed", 500)
        .expect("set float");
    authority
        .set_bool(entity, "moving", true)
        .expect("set bool");
    let started = authority.tick(entity, 2).expect("start transition");
    assert!(started.change.is_some());
    assert_eq!(
        authority
            .state(entity)
            .expect("transition state")
            .transition
            .expect("active transition")
            .target_motion
            .blend_weight_milli,
        500
    );
    authority.tick(entity, 3).expect("transition tick");
    authority.tick(entity, 4).expect("complete transition");
    authority
}

#[test]
fn float_bool_trigger_and_linear_blend_are_authoritative() {
    let entity = EntityId::new(42);
    let mut authority = exercise();
    let locomotion = authority.state(entity).expect("locomotion state");
    assert_eq!(locomotion.current_state_id, "locomotion");
    assert_eq!(locomotion.revision, 1);
    assert_eq!(locomotion.motion.clip_a, "walk");
    assert_eq!(locomotion.motion.clip_b.as_deref(), Some("run"));
    assert_eq!(locomotion.motion.blend_weight_milli, 500);

    authority.fire_trigger(entity, "jump").expect("trigger");
    authority.tick(entity, 5).expect("jump transition");
    let airborne = authority.state(entity).expect("airborne state");
    assert_eq!(airborne.current_state_id, "airborne");
    assert_eq!(airborne.revision, 2);
    assert_eq!(
        airborne.parameters.get("jump"),
        Some(&AnimationParameterValue::Trigger(false))
    );
}

#[test]
fn identical_inputs_produce_identical_state_and_replay_hashes() {
    let left = exercise();
    let right = exercise();
    let entity = EntityId::new(42);
    assert_eq!(
        left.state(entity).expect("left").state_hash,
        right.state(entity).expect("right").state_hash
    );
    assert_eq!(left.records(), right.records());

    let replayed = AnimationControllerAuthority::replay(
        validate_animation_catalog(catalog()).expect("catalog"),
        left.records(),
    )
    .expect("replay");
    assert_eq!(
        left.state(entity).expect("source").state_hash,
        replayed.state(entity).expect("replayed").state_hash
    );
    assert_eq!(
        left.snapshot_hash().expect("source snapshot"),
        replayed.snapshot_hash().expect("replay snapshot")
    );
}

#[test]
fn accepted_gameplay_fact_is_retained_in_transition_state_and_verification_replay() {
    let validated = validate_animation_catalog(catalog()).expect("valid catalog");
    let entity = EntityId::new(42);
    let mut authority = AnimationControllerAuthority::new(validated);
    authority.attach(entity, "player").expect("attach");
    authority
        .set_float(entity, "speed", 650)
        .expect("set blend input");
    authority
        .set_bool(entity, "moving", true)
        .expect("set semantic input");
    let origin = AnimationInputOrigin {
        source_fact_id: "combat.primary-fire.accepted:91".to_string(),
        authority_tick: 9,
        causation_id: "combat.primary-fire:91".to_string(),
        correlation_id: "fps.session:3".to_string(),
    };
    let receipt = authority
        .tick_from_fact(entity, 1, origin.clone())
        .expect("fact-driven tick");
    let fact = receipt
        .change
        .as_ref()
        .and_then(|change| change.state.timing_fact.as_ref())
        .expect("transition timing fact");
    assert_eq!(fact.source, origin);
    assert_eq!(fact.controller_input_sequence, 3);
    assert_eq!(fact.controller_tick, 1);
    assert_eq!(fact.moment, AnimationTransitionFactMoment::Started);
    assert_eq!(fact.transition_id, "idle.move");
    assert_eq!(fact.to_state_id, "locomotion");
    assert_eq!(
        receipt.change.as_ref().expect("change").state.motion.clip_a,
        "idle"
    );
    assert_eq!(
        receipt
            .change
            .as_ref()
            .expect("change")
            .state
            .transition
            .as_ref()
            .expect("active transition")
            .target_motion
            .blend_weight_milli,
        650
    );

    let replayed = AnimationControllerAuthority::replay(
        validate_animation_catalog(catalog()).expect("catalog"),
        authority.records(),
    )
    .expect("verification replay");
    assert_eq!(
        authority.state(entity).unwrap(),
        replayed.state(entity).unwrap()
    );
    assert_eq!(authority.records(), replayed.records());
}

#[test]
fn semantic_state_hash_does_not_depend_on_controller_entity_identity() {
    let validated = validate_animation_catalog(catalog()).expect("catalog");
    let mut authority = AnimationControllerAuthority::new(validated);
    let first = EntityId::new(41);
    let second = EntityId::new(42);
    authority.attach(first, "player").expect("first attach");
    authority.attach(second, "player").expect("second attach");

    assert_eq!(
        authority.state(first).expect("first").state_hash,
        authority.state(second).expect("second").state_hash
    );
}

#[test]
fn snapshot_round_trip_preserves_authority_without_pose_data() {
    let authority = exercise();
    let encoded = authority.encode_snapshot().expect("snapshot");
    assert!(!encoded.contains("bone"));
    assert!(!encoded.contains("joint"));
    assert!(!encoded.contains("matrix"));

    let restored = AnimationControllerAuthority::decode_snapshot(
        validate_animation_catalog(catalog()).expect("catalog"),
        &encoded,
    )
    .expect("restore");
    assert_eq!(
        authority.snapshot_hash().expect("before"),
        restored.snapshot_hash().expect("after")
    );
}

#[test]
fn snapshot_decode_rejects_state_that_is_not_derived_from_its_replay_log() {
    let authority = exercise();
    let encoded = authority.encode_snapshot().expect("snapshot");
    let mut value: serde_json::Value = serde_json::from_str(&encoded).expect("json");
    let records = value["records"].as_array_mut().expect("records");
    let speed_record = records
        .iter_mut()
        .find(|record| record["input"]["kind"] == "setFloat")
        .expect("speed record");
    speed_record["input"]["valueMilli"] = serde_json::json!(900);
    let tampered = serde_json::to_string(&value).expect("tampered snapshot");

    assert!(AnimationControllerAuthority::decode_snapshot(
        validate_animation_catalog(catalog()).expect("catalog"),
        &tampered,
    )
    .is_err());
}

#[test]
fn invalid_graphs_fail_at_catalog_load_with_specific_diagnostics() {
    let mut invalid = catalog();
    let graph = &mut invalid.graphs[0];
    graph.states[0].motion = AnimationMotionDefinition::Clip {
        clip_id: "missing".to_string(),
        speed_milli: 1_000,
    };
    graph.states.push(AnimationStateDefinition {
        state_id: "orphan".to_string(),
        motion: AnimationMotionDefinition::Clip {
            clip_id: "idle".to_string(),
            speed_milli: 1_000,
        },
    });
    graph.transitions.push(AnimationTransitionDefinition {
        transition_id: "idle.ambiguous".to_string(),
        from_state_id: "idle".to_string(),
        to_state_id: "locomotion".to_string(),
        priority: 1,
        duration_ticks: 1,
        conditions: vec![AnimationCondition::FloatGreaterThan {
            parameter_id: "moving".to_string(),
            threshold_milli: 0,
        }],
    });

    let error = validate_animation_catalog(invalid).expect_err("invalid catalog rejected");
    let codes = error
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code)
        .collect::<Vec<_>>();
    assert!(codes.contains(&AnimationCatalogDiagnosticCode::MissingClip));
    assert!(codes.contains(&AnimationCatalogDiagnosticCode::UnreachableState));
    assert!(codes.contains(&AnimationCatalogDiagnosticCode::AmbiguousTransition));
    assert!(codes.contains(&AnimationCatalogDiagnosticCode::ParameterTypeMismatch));
}

#[test]
fn non_positive_clip_and_blend_speeds_fail_at_catalog_load() {
    let cases = [
        (
            AnimationMotionDefinition::Clip {
                clip_id: "idle".to_string(),
                speed_milli: 0,
            },
            "graphs[0].states[0].motion.speedMilli",
        ),
        (
            AnimationMotionDefinition::LinearBlend {
                parameter_id: "speed".to_string(),
                low_clip_id: "walk".to_string(),
                high_clip_id: "run".to_string(),
                minimum_milli: 0,
                maximum_milli: 1_000,
                speed_milli: -1,
            },
            "graphs[0].states[0].motion.speedMilli",
        ),
    ];

    for (motion, expected_path) in cases {
        let mut invalid = catalog();
        invalid.graphs[0].states[0].motion = motion;
        let error = validate_animation_catalog(invalid)
            .expect_err("non-positive playback speed must fail authority validation");
        assert!(error.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == AnimationCatalogDiagnosticCode::InvalidPlaybackSpeed
                && diagnostic.path == expected_path
        }));
    }
}

#[test]
fn catalog_hash_is_canonical_over_authored_collection_order() {
    let first = validate_animation_catalog(catalog()).expect("first");
    let mut reordered = catalog();
    reordered.assets[0].clips.reverse();
    reordered.graphs[0].parameters.reverse();
    reordered.graphs[0].states.reverse();
    reordered.graphs[0].transitions.reverse();
    let second = validate_animation_catalog(reordered).expect("second");
    assert_eq!(first.catalog_hash(), second.catalog_hash());
}
