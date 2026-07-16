use super::*;
use crate::tests::{
    bundle, create_spatial, decision_host_input, decision_moment, empty_scheduler_definition,
    scheduled_collision_deactivation, scheduler_host_input, DecisionOwnerFixture,
};
use gameplay_module_sdk::GameplayStaticCompositionBuilder;
use rule_project_bundle::{GameplayBindingEntityTargets, GameplayModuleBindingRegistryBuilder};
use rule_trigger_volume::TriggerOverlapFactKind;

#[test]
fn runtime_reset_checkpoint_clears_decision_evidence_and_reopens_identity_space() {
    let mut host = GameplayRuntimeHost::activate(decision_host_input()).unwrap();
    let baseline = host.readout();
    let reset = host.checkpoint_reset_state();
    let mut owner = DecisionOwnerFixture::default();

    let first = host.decide(
        decision_moment("decision-reused-after-reset", 0),
        &mut owner,
    );
    assert_eq!(first.status, GameplayDecisionStatus::Suspended);
    assert_eq!(host.readout().pending_decision_count, 1);
    assert_eq!(host.readout().decision_receipt_count, 1);

    host.restore_reset_state(reset).unwrap();
    assert_eq!(host.readout(), baseline);

    let repeated = host.decide(
        decision_moment("decision-reused-after-reset", 0),
        &mut owner,
    );
    assert_eq!(repeated.status, GameplayDecisionStatus::Suspended);
    assert_eq!(host.readout().pending_decision_count, 1);
    assert_eq!(host.readout().decision_receipt_count, 1);
}

#[test]
fn public_height_host_binds_actor_pose_to_trigger_authority_and_snapshot() {
    let mut bundle = bundle();
    create_spatial(&mut bundle, EntityId::new(10), 0.0, true);
    create_spatial(&mut bundle, EntityId::new(20), 2.0, false);
    let mut composition = GameplayStaticCompositionBuilder::new();
    composition.include_standard_owner_events();
    let bindings = GameplayModuleBindingRegistryBuilder::new().build();
    let mut host = GameplayRuntimeHost::activate(GameplayRuntimeHostInput {
        bundle,
        composition: composition.build().unwrap(),
        composition_requirement: None,
        bindings,
        entity_targets: GameplayBindingEntityTargets::new(),
        spatial_entities: Vec::new(),
        declared_reads: Vec::new(),
        triggers: vec![GameplayTriggerDefinition {
            schema_version: GAMEPLAY_TRIGGER_DEFINITION_SCHEMA_VERSION,
            entity: 10,
            scope: "zone.host".to_owned(),
            tags: vec!["door".to_owned()],
        }],
        scheduler: empty_scheduler_definition(),
    })
    .unwrap();
    let authority_hash_before = host.readout().authority_state_hash;
    let runtime_hash_before = host.readout().runtime_host_hash;
    assert!(host
        .reconcile_triggers(1, TriggerReconcileCause::Tick)
        .unwrap()
        .collision
        .facts
        .is_empty());
    let reset = host.checkpoint_reset_state();
    let trigger_before = host.readout().trigger_snapshot_hash;
    let moved_without_overlap_change = host
        .set_actor_translation_and_reconcile(EntityId::new(20), [3.0, 0.0, 0.0], 2)
        .unwrap();
    assert!(moved_without_overlap_change.collision.facts.is_empty());
    assert_ne!(host.readout().authority_state_hash, authority_hash_before);
    assert_ne!(host.readout().runtime_host_hash, runtime_hash_before);
    let entered = host
        .set_actor_translation_and_reconcile(EntityId::new(20), [0.0, 0.0, 0.0], 3)
        .unwrap();
    assert_eq!(
        entered.collision.facts[0].kind,
        TriggerOverlapFactKind::Enter
    );
    assert_eq!(host.readout().active_overlap_count, 1);
    assert_eq!(host.readout().reaction_frame_count, 1);
    assert!(host
        .compose_snapshot()
        .unwrap()
        .text
        .contains("triggerSnapshot"));
    host.restore_reset_state(reset).unwrap();
    assert_eq!(host.readout().active_overlap_count, 0);
    assert_eq!(host.readout().reaction_frame_count, 0);
    assert_eq!(host.readout().trigger_snapshot_hash, trigger_before);
}

#[test]
fn runtime_reset_checkpoint_clears_scheduler_state_and_retired_action_ids() {
    let mut host = GameplayRuntimeHost::activate(scheduler_host_input()).unwrap();
    let baseline = host.scheduler_readout();
    let reset = host.checkpoint_reset_state();

    host.scheduler_port()
        .apply(GameplayRuntimeSchedulerCommand::ScheduleTick(
            scheduled_collision_deactivation(),
        ))
        .unwrap();
    assert_eq!(host.scheduler_readout().pending_action_count, 1);

    host.restore_reset_state(reset).unwrap();
    assert_eq!(host.scheduler_readout(), baseline);
    host.scheduler_port()
        .apply(GameplayRuntimeSchedulerCommand::ScheduleTick(
            scheduled_collision_deactivation(),
        ))
        .unwrap();
    assert_eq!(host.scheduler_readout().pending_action_count, 1);
}
