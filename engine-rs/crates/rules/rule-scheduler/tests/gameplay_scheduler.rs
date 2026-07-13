use protocol_game_extension::{
    GameplayCausationRef, GameplayContractRef, GameplayEmitterRef, GameplayEventEnvelope,
    GameplayEventPhase, GameplayEventSchemaDeclaration, GameplayExecutionBudget,
    GameplayHeaderSelector, GameplayModuleManifest, GameplayModuleRef, GameplayOwnerRef,
    GameplayProposalDeclaration, GameplayProposalEnvelope,
};
use rule_gameplay_fabric::{
    gameplay_payload_hash, GameplayFabricCoordinator, GameplayOwnerRoutingCall,
    GameplayOwnerRoutingOutput, GameplayProposalRouter, GameplayRuntimeLimits,
};
use rule_scheduler::{
    EventConditionedActionDraft, GameplayActionScheduler, GameplayEventCondition,
    GameplaySchedulerCommand, GameplaySchedulerError, GameplaySchedulerFact, ScheduledActionId,
    ScheduledActionRejectionReason, ScheduledActionValidity, TickScheduledActionDraft,
};
use std::collections::BTreeSet;
use svc_gameplay_fabric::{
    gameplay_canonical_codec_id, gameplay_contract, stable_identity, GameplayFabricRegistry,
    GameplayFabricRegistryBuilder, GameplayLinkedProvider, GameplayProposalOwnerRegistration,
    TypedGameplayEventCodec,
};

fn contract(namespace: &str, name: &str) -> GameplayContractRef {
    gameplay_contract(namespace, name, 1, &schema_descriptor(namespace, name))
}

fn schema_descriptor(namespace: &str, name: &str) -> String {
    format!("fixture:{namespace}.{name};opaque-bytes-v1")
}

fn declaration(event: GameplayContractRef) -> GameplayEventSchemaDeclaration {
    GameplayEventSchemaDeclaration {
        codec_id: gameplay_canonical_codec_id(&event.schema_hash),
        event,
    }
}

fn bytes_codec(event: GameplayContractRef) -> TypedGameplayEventCodec<Vec<u8>> {
    let descriptor = schema_descriptor(&event.namespace, &event.name);
    TypedGameplayEventCodec::new(
        declaration(event),
        descriptor,
        |payload: &Vec<u8>| Ok(payload.clone()),
        |bytes| Ok(bytes.to_vec()),
    )
}

fn completion_event_contract() -> GameplayContractRef {
    contract("game.factory", "crafting-completed")
}

fn increment_proposal_contract() -> GameplayContractRef {
    contract("game.progression", "increment-production")
}

fn incremented_event_contract() -> GameplayContractRef {
    contract("game.progression", "production-incremented")
}

fn scheduler_owner() -> GameplayOwnerRef {
    GameplayOwnerRef {
        owner_id: "authority.scheduler".to_owned(),
        provider_id: "provider.scheduler".to_owned(),
    }
}

fn progression_owner() -> GameplayOwnerRef {
    GameplayOwnerRef {
        owner_id: "authority.progression".to_owned(),
        provider_id: "provider.progression".to_owned(),
    }
}

fn scheduler() -> GameplayActionScheduler {
    GameplayActionScheduler::with_contracts(
        scheduler_owner(),
        BTreeSet::from([completion_event_contract()]),
        BTreeSet::from([increment_proposal_contract()]),
    )
}

fn fabric_registry() -> GameplayFabricRegistry {
    let module = GameplayModuleManifest {
        module_ref: GameplayModuleRef {
            module_id: "game.progression-rules".to_owned(),
            namespace: "game.progression".to_owned(),
            version: "1.0.0".to_owned(),
            sdk_hash: stable_identity(["progression", "sdk"]),
            contract_hash: stable_identity(["progression", "contract"]),
            artifact_hash: stable_identity(["progression", "artifact"]),
            provider_id: "provider.progression".to_owned(),
        },
        published_events: vec![declaration(incremented_event_contract())],
        subscriptions: Vec::new(),
        invocations: Vec::new(),
        read_views: Vec::new(),
        proposal_kinds: vec![GameplayProposalDeclaration {
            proposal: increment_proposal_contract(),
            owner: progression_owner(),
        }],
        state_schemas: Vec::new(),
        fact_schemas: Vec::new(),
        ordering: Vec::new(),
        budget: GameplayExecutionBudget {
            max_waves: 4,
            max_events_per_root: 16,
            max_proposals_per_root: 16,
            max_invocations_per_root: 16,
            max_payload_bytes_per_root: 16_384,
        },
        deterministic_requirements: vec!["canonical-input-order".to_owned()],
        source_hash: stable_identity(["progression", "source"]),
    };
    let mut builder = GameplayFabricRegistryBuilder::new();
    builder
        .register_event_codec(bytes_codec(incremented_event_contract()))
        .register_event_codec(bytes_codec(increment_proposal_contract()))
        .register_linked_provider(GameplayLinkedProvider {
            provider_id: module.module_ref.provider_id.clone(),
            module_id: module.module_ref.module_id.clone(),
            version: module.module_ref.version.clone(),
            contract_hash: module.module_ref.contract_hash.clone(),
            artifact_hash: module.module_ref.artifact_hash.clone(),
            sdk_hash: module.module_ref.sdk_hash.clone(),
            source_hash: module.source_hash.clone(),
        })
        .register_proposal_owner(GameplayProposalOwnerRegistration {
            proposal: increment_proposal_contract(),
            owner: progression_owner(),
        })
        .register_module(module);
    builder.build().expect("closed routing registry")
}

fn runtime_limits() -> GameplayRuntimeLimits {
    GameplayRuntimeLimits {
        max_waves: 4,
        max_events_per_root: 16,
        max_proposals_per_root: 16,
        max_invocations_per_root: 16,
        max_payload_bytes_per_root: 16_384,
    }
}

fn causation() -> GameplayCausationRef {
    GameplayCausationRef {
        root_id: "factory-cycle-1".to_owned(),
        parent_event_id: None,
        decision_id: None,
    }
}

fn proposal() -> GameplayProposalEnvelope {
    let canonical_payload = br#"{"counter":"widgets","amount":1}"#.to_vec();
    GameplayProposalEnvelope {
        proposal_id: "draft".to_owned(),
        proposal: increment_proposal_contract(),
        tick: 0,
        root_sequence: 9,
        wave: 0,
        proposal_sequence: 0,
        emitter: GameplayEmitterRef::Owner {
            owner_id: "draft".to_owned(),
        },
        causation: causation(),
        originating_event_id: None,
        source: None,
        targets: Vec::new(),
        payload_hash: gameplay_payload_hash(&canonical_payload),
        canonical_payload,
    }
}

fn tick_draft(id: &str, execute_at: u64, priority: i32) -> TickScheduledActionDraft {
    TickScheduledActionDraft {
        id: ScheduledActionId::new(id),
        execute_at,
        priority,
        proposal: proposal(),
        source: GameplayEmitterRef::Owner {
            owner_id: "authority.factory".to_owned(),
        },
        causation: causation(),
    }
}

fn conditioned_draft(id: &str, timeout_at: Option<u64>) -> EventConditionedActionDraft {
    EventConditionedActionDraft {
        id: ScheduledActionId::new(id),
        condition: GameplayEventCondition {
            event: completion_event_contract(),
            selector: GameplayHeaderSelector {
                source: None,
                target: None,
                scope: Some("factory.floor-a".to_owned()),
                required_tags: vec!["production".to_owned()],
            },
        },
        priority: 5,
        proposal: proposal(),
        timeout_at,
        source: GameplayEmitterRef::Owner {
            owner_id: "authority.factory".to_owned(),
        },
        causation: causation(),
    }
}

fn completion_event(tick: u64) -> GameplayEventEnvelope {
    let canonical_payload = br#"{"recipe":"widget"}"#.to_vec();
    GameplayEventEnvelope {
        event_id: format!("craft-completed-{tick}"),
        event: completion_event_contract(),
        tick,
        root_sequence: 9,
        wave: 0,
        event_sequence: 0,
        phase: GameplayEventPhase::PostCommit,
        emitter: GameplayEmitterRef::Owner {
            owner_id: "authority.factory".to_owned(),
        },
        causation: causation(),
        source: None,
        subjects: Vec::new(),
        targets: Vec::new(),
        scope: Some("factory.floor-a".to_owned()),
        tags: vec!["production".to_owned()],
        payload_hash: gameplay_payload_hash(&canonical_payload),
        canonical_payload,
    }
}

fn apply(
    scheduler: &mut GameplayActionScheduler,
    command: GameplaySchedulerCommand,
) -> rule_scheduler::GameplaySchedulerReceipt {
    scheduler
        .apply(&scheduler_owner(), command)
        .expect("scheduler command")
}

#[test]
fn tick_actions_execute_once_at_or_after_the_tick_in_stable_order() {
    let mut scheduler = scheduler();
    for draft in [
        tick_draft("action.z", 20, 5),
        tick_draft("action.b", 20, 1),
        tick_draft("action.a", 20, 1),
        tick_draft("action.early", 10, 9),
    ] {
        apply(
            &mut scheduler,
            GameplaySchedulerCommand::ScheduleTick(draft),
        );
    }

    assert_eq!(
        scheduler
            .due_action_ids(20)
            .iter()
            .map(ScheduledActionId::as_str)
            .collect::<Vec<_>>(),
        vec!["action.early", "action.a", "action.b", "action.z"]
    );
    let receipt = apply(
        &mut scheduler,
        GameplaySchedulerCommand::ExecuteTick {
            action_id: ScheduledActionId::new("action.early"),
            tick: 20,
            validity: ScheduledActionValidity::CURRENT,
        },
    );
    assert!(receipt.dispatch.is_some());
    assert_eq!(scheduler.due_action_ids(20).len(), 3);
    assert_eq!(
        scheduler.apply(
            &scheduler_owner(),
            GameplaySchedulerCommand::ExecuteTick {
                action_id: ScheduledActionId::new("action.early"),
                tick: 20,
                validity: ScheduledActionValidity::CURRENT,
            },
        ),
        Err(GameplaySchedulerError::UnknownAction)
    );
    assert_eq!(
        scheduler.apply(
            &scheduler_owner(),
            GameplaySchedulerCommand::ScheduleTick(tick_draft("action.early", 30, 0)),
        ),
        Err(GameplaySchedulerError::DuplicateAction)
    );
}

struct ProgressionRouter {
    accepted: bool,
}

struct ProgressionEventRouter;

impl GameplayProposalRouter for ProgressionEventRouter {
    fn route(&mut self, _call: &GameplayOwnerRoutingCall) -> GameplayOwnerRoutingOutput {
        let canonical_payload = br#"{"counter":"widgets","value":1}"#.to_vec();
        GameplayOwnerRoutingOutput {
            accepted: true,
            fact_hashes: vec!["fact:production-counter-1".to_owned()],
            events: vec![GameplayEventEnvelope {
                event_id: "owner-candidate".to_owned(),
                event: incremented_event_contract(),
                tick: 999,
                root_sequence: 999,
                wave: 999,
                event_sequence: 999,
                phase: GameplayEventPhase::ScheduledMoment,
                emitter: GameplayEmitterRef::Scheduler {
                    scheduler_id: "untrusted-output".to_owned(),
                },
                causation: GameplayCausationRef {
                    root_id: "untrusted-output".to_owned(),
                    parent_event_id: None,
                    decision_id: None,
                },
                source: None,
                subjects: Vec::new(),
                targets: Vec::new(),
                scope: Some("factory.floor-a".to_owned()),
                tags: vec!["production".to_owned()],
                payload_hash: gameplay_payload_hash(&canonical_payload),
                canonical_payload,
            }],
            diagnostic_codes: Vec::new(),
        }
    }
}

impl GameplayProposalRouter for ProgressionRouter {
    fn route(&mut self, call: &GameplayOwnerRoutingCall) -> GameplayOwnerRoutingOutput {
        assert_eq!(call.owner, progression_owner());
        assert_eq!(call.proposal.proposal, increment_proposal_contract());
        GameplayOwnerRoutingOutput {
            accepted: self.accepted,
            fact_hashes: self
                .accepted
                .then(|| "fact:production-counter-1".to_owned())
                .into_iter()
                .collect(),
            events: Vec::new(),
            diagnostic_codes: (!self.accepted)
                .then(|| "missing-target".to_owned())
                .into_iter()
                .collect(),
        }
    }
}

#[test]
fn crafting_completion_triggers_a_next_boundary_owner_routed_progression_change() {
    let mut scheduler = scheduler();
    apply(
        &mut scheduler,
        GameplaySchedulerCommand::ScheduleEventConditioned(conditioned_draft(
            "action.increment-widget-count",
            Some(100),
        )),
    );
    let mut event = completion_event(42);
    event.wave = 2;
    assert_eq!(
        scheduler.matching_action_ids(&event),
        vec![ScheduledActionId::new("action.increment-widget-count")]
    );

    let triggered = apply(
        &mut scheduler,
        GameplaySchedulerCommand::TriggerEvent {
            action_id: ScheduledActionId::new("action.increment-widget-count"),
            event: event.clone(),
            validity: ScheduledActionValidity::CURRENT,
        },
    );
    let dispatch = triggered.dispatch.expect("next-boundary proposal");
    assert_eq!(dispatch.proposal.tick, 42);
    assert_eq!(dispatch.proposal.wave, 3);
    assert_eq!(
        dispatch.proposal.originating_event_id.as_deref(),
        Some(event.event_id.as_str())
    );
    assert!(matches!(
        dispatch.proposal.emitter,
        GameplayEmitterRef::Scheduler { .. }
    ));

    let registry = fabric_registry();
    let mut router = ProgressionRouter { accepted: true };
    let routing_receipt = GameplayFabricCoordinator::new(&registry, runtime_limits())
        .route_proposal(dispatch.proposal.clone(), &mut router)
        .expect("closed-registry route");
    assert_eq!(
        routing_receipt.evidence().proposal_hash,
        dispatch.proposal_hash
    );
    let routed = apply(
        &mut scheduler,
        GameplaySchedulerCommand::RecordRouting {
            action_id: dispatch.action_id,
            receipt: routing_receipt,
        },
    );
    assert!(matches!(
        routed.fact,
        GameplaySchedulerFact::RoutingAccepted { .. }
    ));
    assert!(scheduler.matching_action_ids(&event).is_empty());
}

#[test]
fn timeout_missing_target_stale_causation_and_owner_rejection_are_typed_facts() {
    let mut scheduler = scheduler();
    apply(
        &mut scheduler,
        GameplaySchedulerCommand::ScheduleEventConditioned(conditioned_draft(
            "action.timeout",
            Some(50),
        )),
    );
    assert_eq!(
        scheduler.timed_out_action_ids(50),
        vec![ScheduledActionId::new("action.timeout")]
    );
    let timeout = apply(
        &mut scheduler,
        GameplaySchedulerCommand::Timeout {
            action_id: ScheduledActionId::new("action.timeout"),
            tick: 50,
        },
    );
    assert!(matches!(
        timeout.fact,
        GameplaySchedulerFact::TimedOut { .. }
    ));

    for (id, validity, expected) in [
        (
            "action.missing",
            ScheduledActionValidity {
                targets_present: false,
                causation_current: true,
            },
            ScheduledActionRejectionReason::MissingTarget,
        ),
        (
            "action.stale",
            ScheduledActionValidity {
                targets_present: true,
                causation_current: false,
            },
            ScheduledActionRejectionReason::StaleCausation,
        ),
    ] {
        apply(
            &mut scheduler,
            GameplaySchedulerCommand::ScheduleTick(tick_draft(id, 10, 0)),
        );
        let rejected = apply(
            &mut scheduler,
            GameplaySchedulerCommand::ExecuteTick {
                action_id: ScheduledActionId::new(id),
                tick: 10,
                validity,
            },
        );
        assert_eq!(
            rejected.fact,
            GameplaySchedulerFact::Rejected {
                action_id: ScheduledActionId::new(id),
                reason: expected,
            }
        );
        assert!(rejected.dispatch.is_none());
    }

    apply(
        &mut scheduler,
        GameplaySchedulerCommand::ScheduleTick(tick_draft("action.owner-reject", 10, 0)),
    );
    let triggered = apply(
        &mut scheduler,
        GameplaySchedulerCommand::ExecuteTick {
            action_id: ScheduledActionId::new("action.owner-reject"),
            tick: 10,
            validity: ScheduledActionValidity::CURRENT,
        },
    );
    let dispatch = triggered.dispatch.expect("outstanding dispatch");
    let registry = fabric_registry();
    let mut router = ProgressionRouter { accepted: false };
    let routing_receipt = GameplayFabricCoordinator::new(&registry, runtime_limits())
        .route_proposal(dispatch.proposal, &mut router)
        .expect("closed-registry rejection");
    let routed = apply(
        &mut scheduler,
        GameplaySchedulerCommand::RecordRouting {
            action_id: ScheduledActionId::new("action.owner-reject"),
            receipt: routing_receipt,
        },
    );
    assert!(matches!(
        routed.fact,
        GameplaySchedulerFact::RoutingRejected { .. }
    ));
}

#[test]
fn save_reload_and_fact_replay_preserve_pending_queue_and_later_outcome() {
    let mut original = scheduler();
    apply(
        &mut original,
        GameplaySchedulerCommand::ScheduleTick(tick_draft("action.saved", 80, 0)),
    );
    apply(
        &mut original,
        GameplaySchedulerCommand::ScheduleEventConditioned(conditioned_draft(
            "action.conditioned",
            Some(120),
        )),
    );
    let snapshot = original.encode_snapshot().expect("snapshot");
    let mut restored = GameplayActionScheduler::decode_snapshot(&snapshot).expect("restore");
    assert_eq!(restored.state_hash(), original.state_hash());
    assert_eq!(restored.encode_snapshot().expect("fixed point"), snapshot);

    let command = GameplaySchedulerCommand::ExecuteTick {
        action_id: ScheduledActionId::new("action.saved"),
        tick: 80,
        validity: ScheduledActionValidity::CURRENT,
    };
    let original_receipt = apply(&mut original, command.clone());
    let restored_receipt = apply(&mut restored, command);
    assert_eq!(original_receipt.fact, restored_receipt.fact);
    assert_eq!(
        original_receipt.state_hash_after,
        restored_receipt.state_hash_after
    );

    let replayed = GameplayActionScheduler::replay(
        scheduler_owner(),
        BTreeSet::from([completion_event_contract()]),
        BTreeSet::from([increment_proposal_contract()]),
        original.facts(),
    )
    .expect("fact replay");
    assert_eq!(replayed.state_hash(), original.state_hash());
}

#[test]
fn triggered_dispatch_survives_reload_and_fact_replay_until_routing_acceptance() {
    let mut original = scheduler();
    apply(
        &mut original,
        GameplaySchedulerCommand::ScheduleTick(tick_draft("action.recover", 25, 0)),
    );
    let triggered = apply(
        &mut original,
        GameplaySchedulerCommand::ExecuteTick {
            action_id: ScheduledActionId::new("action.recover"),
            tick: 25,
            validity: ScheduledActionValidity::CURRENT,
        },
    );
    let expected = triggered.dispatch.expect("triggered dispatch");
    assert_eq!(original.outstanding_dispatches(), vec![&expected]);

    let snapshot = original.encode_snapshot().expect("snapshot with dispatch");
    let mut restored = GameplayActionScheduler::decode_snapshot(&snapshot).expect("restore");
    assert_eq!(restored.outstanding_dispatches(), vec![&expected]);

    let replayed = GameplayActionScheduler::replay(
        scheduler_owner(),
        BTreeSet::from([completion_event_contract()]),
        BTreeSet::from([increment_proposal_contract()]),
        original.facts(),
    )
    .expect("fact replay");
    assert_eq!(replayed.outstanding_dispatches(), vec![&expected]);

    let registry = fabric_registry();
    let mut wrong_proposal = expected.proposal.clone();
    wrong_proposal.proposal_id = "scheduler/action.other/0".to_owned();
    let mut wrong_router = ProgressionRouter { accepted: true };
    let wrong_receipt = GameplayFabricCoordinator::new(&registry, runtime_limits())
        .route_proposal(wrong_proposal, &mut wrong_router)
        .expect("other proposal routes through the same closed registry");
    assert_eq!(
        restored.apply(
            &scheduler_owner(),
            GameplaySchedulerCommand::RecordRouting {
                action_id: expected.action_id.clone(),
                receipt: wrong_receipt,
            },
        ),
        Err(GameplaySchedulerError::RoutingMismatch)
    );
    assert_eq!(restored.outstanding_dispatches(), vec![&expected]);

    let mut router = ProgressionRouter { accepted: true };
    let routing_receipt = GameplayFabricCoordinator::new(&registry, runtime_limits())
        .route_proposal(expected.proposal.clone(), &mut router)
        .expect("recoverable dispatch routes");
    apply(
        &mut restored,
        GameplaySchedulerCommand::RecordRouting {
            action_id: expected.action_id,
            receipt: routing_receipt,
        },
    );
    assert!(restored.outstanding_dispatches().is_empty());
}

#[test]
fn accepted_owner_events_survive_interruption_and_complete_delivery_exactly_once() {
    let mut scheduler = scheduler();
    apply(
        &mut scheduler,
        GameplaySchedulerCommand::ScheduleTick(tick_draft("action.event-recovery", 25, 0)),
    );
    let dispatch = apply(
        &mut scheduler,
        GameplaySchedulerCommand::ExecuteTick {
            action_id: ScheduledActionId::new("action.event-recovery"),
            tick: 25,
            validity: ScheduledActionValidity::CURRENT,
        },
    )
    .dispatch
    .expect("triggered dispatch");
    let registry = fabric_registry();
    let mut router = ProgressionEventRouter;
    let route = GameplayFabricCoordinator::new(&registry, runtime_limits())
        .route_proposal(dispatch.proposal, &mut router)
        .expect("accepted owner event route");
    apply(
        &mut scheduler,
        GameplaySchedulerCommand::RecordRouting {
            action_id: dispatch.action_id.clone(),
            receipt: route,
        },
    );
    assert!(scheduler.outstanding_dispatches().is_empty());
    let delivery = scheduler.outstanding_event_deliveries()[0].clone();
    assert_eq!(delivery.events.len(), 1);
    assert_eq!(delivery.events[0].wave, 1);
    assert_eq!(
        delivery.events[0].emitter,
        GameplayEmitterRef::Owner {
            owner_id: progression_owner().owner_id,
        }
    );

    let snapshot = scheduler
        .encode_snapshot()
        .expect("pending delivery snapshot");
    let mut restored = GameplayActionScheduler::decode_snapshot(&snapshot).expect("restore");
    assert_eq!(restored.outstanding_event_deliveries(), vec![&delivery]);
    let replayed = GameplayActionScheduler::replay(
        scheduler_owner(),
        BTreeSet::from([completion_event_contract()]),
        BTreeSet::from([increment_proposal_contract()]),
        scheduler.facts(),
    )
    .expect("verification replay");
    assert_eq!(replayed.outstanding_event_deliveries(), vec![&delivery]);

    let before_wrong = restored.state_hash();
    assert_eq!(
        restored.apply(
            &scheduler_owner(),
            GameplaySchedulerCommand::CompleteEventDelivery {
                action_id: dispatch.action_id.clone(),
                routing_hash: "wrong-routing-hash".to_owned(),
            },
        ),
        Err(GameplaySchedulerError::RoutingMismatch)
    );
    assert_eq!(restored.state_hash(), before_wrong);

    apply(
        &mut restored,
        GameplaySchedulerCommand::CompleteEventDelivery {
            action_id: dispatch.action_id.clone(),
            routing_hash: delivery.routing.routing_hash.clone(),
        },
    );
    assert!(restored.outstanding_event_deliveries().is_empty());
    assert_eq!(
        restored.apply(
            &scheduler_owner(),
            GameplaySchedulerCommand::CompleteEventDelivery {
                action_id: dispatch.action_id,
                routing_hash: delivery.routing.routing_hash,
            },
        ),
        Err(GameplaySchedulerError::UnknownAction)
    );
    GameplayActionScheduler::replay(
        scheduler_owner(),
        BTreeSet::from([completion_event_contract()]),
        BTreeSet::from([increment_proposal_contract()]),
        restored.facts(),
    )
    .expect("completed delivery verification replay");
}

#[test]
fn foreign_owner_and_undeclared_contracts_fail_without_mutation() {
    let mut scheduler = scheduler();
    let before = scheduler.state_hash();
    assert_eq!(
        scheduler.apply(
            &progression_owner(),
            GameplaySchedulerCommand::ScheduleTick(tick_draft("action.foreign", 1, 0)),
        ),
        Err(GameplaySchedulerError::ForeignOwner)
    );
    let mut undeclared = tick_draft("action.undeclared", 1, 0);
    undeclared.proposal.proposal = contract("game.unknown", "proposal");
    assert_eq!(
        scheduler.apply(
            &scheduler_owner(),
            GameplaySchedulerCommand::ScheduleTick(undeclared),
        ),
        Err(GameplaySchedulerError::UndeclaredProposal)
    );
    assert_eq!(scheduler.state_hash(), before);
}
