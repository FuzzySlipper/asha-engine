use core_entity::{
    ActivatableCapabilityKind, CapabilityActivationEvent, CapabilityActivationState,
    EntityLifecycleEvent, EntitySource,
};
use core_events::DomainEvent;
use core_ids::{EntityId, ModeId, ProcessId, TagId};
use protocol_game_rules::{
    GameRuleCatalogRef, GameRuleModifierState, GameRuleResolutionReceipt, GameRuleValueDelta,
};
use rule_gameplay_fabric::{
    adapt_capability_activation_event, adapt_combat_readout, adapt_entity_lifecycle_event,
    adapt_game_rule_resolution, adapt_process_domain_event, adapt_session_tick,
    adapt_state_machine_event, adapt_trigger_overlap_fact, register_standard_owner_events,
    CombatGameplayPayload, EntityLifecycleGameplayPayload, GameplayOwnerEventContext,
    ModifierGameplayPayload, StandardGameplayEventKind, StateMachineGameplayPayload,
    TriggerOverlapGameplayPayload, ValueDeltaGameplayPayload,
};
use rule_state_machine::StateMachineEvent;
use svc_combat::{CombatEvent, CombatFireOutcome, CombatReadout, FireControlState};
use svc_game_rules::EffectResolutionRequest;
use svc_gameplay_fabric::GameplayFabricRegistryBuilder;

fn context(owner: &str) -> GameplayOwnerEventContext {
    GameplayOwnerEventContext {
        owner_id: owner.to_owned(),
        tick: 14,
        root_id: "root.owner-fixture".to_owned(),
        root_sequence: 9,
        first_event_sequence: 3,
        parent_event_id: Some("event.accepted-owner-fact".to_owned()),
    }
}

#[test]
fn standard_owner_module_registers_typed_codecs_and_stable_topology() {
    let mut first_builder = GameplayFabricRegistryBuilder::new();
    register_standard_owner_events(&mut first_builder);
    let first = first_builder.build().unwrap();

    let mut second_builder = GameplayFabricRegistryBuilder::new();
    register_standard_owner_events(&mut second_builder);
    let second = second_builder.build().unwrap();

    assert_eq!(first.registry_digest(), second.registry_digest());
    assert_eq!(first.readout().event_kinds.len(), 20);
    assert!(first
        .readout()
        .event_kinds
        .contains(&"asha.combat.damage-applied.v1".to_owned()));
    assert!(first
        .readout()
        .event_kinds
        .contains(&"asha.prefab.part-interacted.v1".to_owned()));
}

#[test]
fn trigger_owner_facts_preserve_semantic_identity_scope_tags_and_causation() {
    use rule_trigger_volume::{TriggerOverlapFact, TriggerOverlapFactKind, TriggerReconcileCause};

    let fact = TriggerOverlapFact {
        kind: TriggerOverlapFactKind::Enter,
        trigger: 10,
        subject: 20,
        scope: "zone.exit".to_owned(),
        tags: vec!["exit".to_owned(), "door".to_owned()],
        tick: 14,
        cause: TriggerReconcileCause::Teleport,
        pair_hash: "fnv1a64:pair".to_owned(),
    };
    let envelope = adapt_trigger_overlap_fact(&context("rule-trigger-volume"), &fact).unwrap();
    assert_eq!(
        envelope.event,
        StandardGameplayEventKind::TriggerEntered.contract()
    );
    assert_eq!(envelope.source.unwrap().entity, EntityId::new(10));
    assert_eq!(envelope.subjects[0].entity, EntityId::new(20));
    assert_eq!(envelope.scope.as_deref(), Some("zone.exit"));
    assert_eq!(envelope.tags, vec!["door", "enter", "exit"]);
    assert_eq!(
        envelope.causation.parent_event_id.as_deref(),
        Some("event.accepted-owner-fact")
    );
    let payload: TriggerOverlapGameplayPayload =
        serde_json::from_slice(&envelope.canonical_payload).unwrap();
    assert_eq!(payload.cause, "teleport");
    assert_eq!(payload.pair_hash, "fnv1a64:pair");
}

#[test]
fn lifecycle_and_activation_facts_adapt_at_their_semantic_origin() {
    let event = EntityLifecycleEvent::Created {
        id: EntityId::new(7),
        source: EntitySource::RuntimeCreated {
            by: Some(ProcessId::new(2)),
        },
        labels: vec![TagId::new(9), TagId::new(3)],
    };
    let envelope = adapt_entity_lifecycle_event(&context("core-entity"), &event).unwrap();
    assert_eq!(
        envelope.event,
        StandardGameplayEventKind::EntityCreated.contract()
    );
    assert_eq!(envelope.subjects[0].entity, EntityId::new(7));
    let payload: EntityLifecycleGameplayPayload =
        serde_json::from_slice(&envelope.canonical_payload).unwrap();
    assert_eq!(payload.source_kind.as_deref(), Some("runtimeCreated"));
    assert_eq!(payload.labels, vec![3, 9]);

    let activation = adapt_capability_activation_event(
        &context("core-entity.activation"),
        CapabilityActivationEvent {
            entity: EntityId::new(7),
            capability: ActivatableCapabilityKind::Collision,
            from: CapabilityActivationState::Active,
            to: CapabilityActivationState::Inactive,
        },
    )
    .unwrap();
    assert_eq!(
        activation.event,
        StandardGameplayEventKind::CapabilityActivationChanged.contract()
    );
    assert_eq!(activation.tags, vec!["collision", "inactive"]);
}

#[test]
fn combat_state_machine_and_process_outcomes_produce_rich_stable_events() {
    let combat = CombatReadout {
        outcome: CombatFireOutcome::Hit {
            target: EntityId::new(22),
            distance: 3.5,
            defeated: true,
        },
        events: vec![
            CombatEvent::FireHit {
                shooter: EntityId::new(11),
                target: EntityId::new(22),
                distance: 3.5,
                tick: 14,
            },
            CombatEvent::DamageApplied {
                target: EntityId::new(22),
                amount: 8,
                before: 8,
                after: 0,
            },
            CombatEvent::EntityDefeated {
                target: EntityId::new(22),
            },
        ],
        next_fire_control: FireControlState {
            ammo: 2,
            cooldown_ticks_remaining: 4,
            cooldown_ticks_after_fire: 4,
        },
        health_hash: 77,
        replay_hash: 88,
    };
    let first = adapt_combat_readout(&context("svc-combat"), &combat).unwrap();
    let second = adapt_combat_readout(&context("svc-combat"), &combat).unwrap();
    assert_eq!(first, second);
    assert_eq!(first.len(), 3);
    let damage: CombatGameplayPayload =
        serde_json::from_slice(&first[1].canonical_payload).unwrap();
    assert_eq!(damage.shooter, Some(11));
    assert_eq!(damage.target, Some(22));
    assert_eq!(damage.damage, Some(8));
    assert!(damage.defeated);

    let state_machine = adapt_state_machine_event(
        &context("rule-state-machine"),
        StateMachineEvent::StateTransitioned {
            entity: EntityId::new(22),
            machine: ProcessId::new(5),
            from: ModeId::new(1),
            to: ModeId::new(2),
            revision: 6,
        },
    )
    .unwrap();
    let state_payload: StateMachineGameplayPayload =
        serde_json::from_slice(&state_machine.canonical_payload).unwrap();
    assert_eq!(state_payload.from, Some(1));
    assert_eq!(state_payload.to, 2);

    let process = adapt_process_domain_event(
        &context("rule-process"),
        &DomainEvent::ProcessModeSet {
            id: ProcessId::new(5),
            mode: ModeId::new(2),
        },
    )
    .unwrap()
    .unwrap();
    assert_eq!(
        process.event,
        StandardGameplayEventKind::ProcessModeSet.contract()
    );
}

#[test]
fn combat_owner_payloads_are_admitted_for_runtime_distances() {
    let combat = CombatReadout {
        outcome: CombatFireOutcome::Hit {
            target: EntityId::new(10),
            distance: 0.060_139_368_429_633_036,
            defeated: false,
        },
        events: vec![CombatEvent::FireHit {
            shooter: EntityId::new(20),
            target: EntityId::new(10),
            distance: 0.060_139_368_429_633_036,
            tick: 9,
        }],
        next_fire_control: FireControlState {
            ammo: 2,
            cooldown_ticks_remaining: 4,
            cooldown_ticks_after_fire: 4,
        },
        health_hash: 77,
        replay_hash: 9_050_254_006_610_280_778,
    };
    let envelopes = adapt_combat_readout(&context("svc-combat"), &combat).unwrap();
    let mut builder = GameplayFabricRegistryBuilder::new();
    register_standard_owner_events(&mut builder);
    let registry = builder.build().unwrap();

    for envelope in &envelopes {
        registry.admit_event(envelope).unwrap();
    }
}

#[test]
fn accepted_modifier_facts_adapt_and_rejected_resolution_emits_nothing() {
    let request = EffectResolutionRequest {
        catalog: GameRuleCatalogRef {
            catalog_id: "catalog.fixture".to_owned(),
            version: "1.0.0".to_owned(),
            content_hash: "fnv1a64:catalog".to_owned(),
        },
        bundle_id: "bundle.fixture".to_owned(),
        source: EntityId::new(1),
        target: EntityId::new(2),
        values: Vec::new(),
        incoming_tags: Vec::new(),
        tick: 14,
    };
    let accepted = GameRuleResolutionReceipt {
        accepted: true,
        request_hash: "fnv1a64:request".to_owned(),
        pending_value_deltas: vec![GameRuleValueDelta {
            channel_id: "value.health".to_owned(),
            amount: -4,
        }],
        applied_modifiers: vec![GameRuleModifierState {
            modifier_id: "modifier.slow".to_owned(),
            source: EntityId::new(1),
            target: EntityId::new(2),
            stacks: 1,
            applied_tick: 14,
            expires_tick: Some(20),
            next_tick: None,
            source_hash: "fnv1a64:slow".to_owned(),
        }],
        diagnostics: Vec::new(),
        trace: Vec::new(),
        evidence: Vec::new(),
        replay_hash: "fnv1a64:replay".to_owned(),
    };
    let events =
        adapt_game_rule_resolution(&context("svc-game-rules"), &request, &accepted).unwrap();
    assert_eq!(events.len(), 2);
    let delta: ValueDeltaGameplayPayload =
        serde_json::from_slice(&events[0].canonical_payload).unwrap();
    assert_eq!(delta.channel_id, "value.health");
    let modifier: ModifierGameplayPayload =
        serde_json::from_slice(&events[1].canonical_payload).unwrap();
    assert_eq!(modifier.modifier_id, "modifier.slow");

    let mut rejected = accepted;
    rejected.accepted = false;
    assert!(
        adapt_game_rule_resolution(&context("svc-game-rules"), &request, &rejected)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn named_tick_moment_is_scheduled_not_a_hidden_update_callback() {
    let event = adapt_session_tick(&context("runtime-session.time")).unwrap();
    assert_eq!(event.phase.as_str(), "scheduledMoment");
    assert_eq!(
        event.event,
        StandardGameplayEventKind::SessionTick.contract()
    );
}
