use super::*;
use core_commands::{Command, CommandKind, EntityCommand, TagCommand};
use core_ids::TagId;

fn queue_command(bridge: &mut EngineBridge, tick: u64, command: Command) {
    super::super::time_control::queue_simulation_command(
        bridge,
        tick,
        CommandEnvelope::new(CommandKind::System, command),
    );
}

#[test]
fn time_control_requires_an_initialized_session() {
    let mut bridge = EngineBridge::new();
    assert_eq!(
        bridge.read_time_control_state().unwrap_err().kind,
        RuntimeBridgeErrorKind::NotInitialized
    );
    assert_eq!(
        bridge
            .apply_time_control_command(TimeControlCommand::Pause)
            .unwrap_err()
            .kind,
        RuntimeBridgeErrorKind::NotInitialized
    );
}

#[test]
fn pause_blocks_cadence_ticks_while_projection_reads_remain_live() {
    let mut bridge = init_bridge();
    let initial = bridge.read_time_control_state().unwrap();
    assert_eq!(initial.mode, TimeControlMode::Running);
    assert_eq!(initial.speed_multiplier, 1);
    assert_eq!(initial.authority_tick, 0);

    let pause = bridge
        .apply_time_control_command(TimeControlCommand::Pause)
        .unwrap();
    assert!(pause.accepted);
    assert_eq!(pause.after.mode, TimeControlMode::Paused);
    assert_eq!(pause.exact_ticks_advanced, 0);

    assert_eq!(
        bridge
            .step_simulation(StepInputEnvelope { tick: 9 })
            .unwrap(),
        StepResult {
            tick: 0,
            diff_count: 0,
        }
    );
    assert_eq!(bridge.read_projection_frame(0).unwrap().authority_tick, 0);
    assert_eq!(bridge.read_time_control_state().unwrap(), pause.after);
}

#[test]
fn exact_steps_advance_the_requested_count_and_remain_paused() {
    let mut bridge = init_bridge();
    bridge
        .apply_time_control_command(TimeControlCommand::Pause)
        .unwrap();

    let receipt = bridge
        .apply_time_control_command(TimeControlCommand::StepTicks { ticks: 3 })
        .unwrap();
    assert!(receipt.accepted);
    assert_eq!(receipt.before.authority_tick, 0);
    assert_eq!(receipt.after.authority_tick, 3);
    assert_eq!(receipt.after.mode, TimeControlMode::Paused);
    assert_eq!(receipt.exact_ticks_advanced, 3);
    assert_ne!(receipt.before.state_hash, receipt.after.state_hash);

    assert_eq!(
        bridge
            .step_simulation(StepInputEnvelope { tick: 20 })
            .unwrap()
            .tick,
        3
    );
    bridge
        .apply_time_control_command(TimeControlCommand::Resume)
        .unwrap();
    assert_eq!(
        bridge
            .step_simulation(StepInputEnvelope { tick: 4 })
            .unwrap()
            .tick,
        4
    );
}

#[test]
fn exact_steps_execute_each_fixed_tick_in_sequence() {
    let mut bridge = init_bridge();
    assert_eq!(
        bridge
            .step_simulation(StepInputEnvelope { tick: 6 })
            .unwrap(),
        StepResult {
            tick: 6,
            diff_count: 0,
        }
    );
    bridge
        .apply_time_control_command(TimeControlCommand::Pause)
        .unwrap();
    queue_command(
        &mut bridge,
        7,
        Command::Tag(TagCommand::Define { id: TagId::new(3) }),
    );
    queue_command(
        &mut bridge,
        8,
        Command::Entity(EntityCommand::Create {
            id: EntityId::new(12),
        }),
    );
    queue_command(
        &mut bridge,
        9,
        Command::Entity(EntityCommand::AddTag {
            id: EntityId::new(12),
            tag: TagId::new(3),
        }),
    );

    let receipt = bridge
        .apply_time_control_command(TimeControlCommand::StepTicks { ticks: 3 })
        .unwrap();
    assert_eq!(receipt.before.authority_tick, 6);
    assert_eq!(receipt.after.authority_tick, 9);
    assert_eq!(receipt.exact_ticks_advanced, 3);
    assert!(bridge
        .time
        .simulation
        .state()
        .entity(EntityId::new(12))
        .expect("exact stepping created the scheduled entity")
        .tags
        .contains(&TagId::new(3)));
    assert_eq!(bridge.time.simulation.queued_tick_count(), 0);

    bridge
        .apply_time_control_command(TimeControlCommand::Resume)
        .unwrap();
    assert_eq!(
        bridge
            .step_simulation(StepInputEnvelope { tick: 10 })
            .unwrap(),
        StepResult {
            tick: 10,
            diff_count: 0,
        },
        "the next cadence tick follows the exact-step sequence"
    );
}

#[test]
fn invalid_time_commands_are_atomic_and_classified() {
    let mut bridge = init_bridge();
    let running = bridge.read_time_control_state().unwrap();
    let running_step = bridge
        .apply_time_control_command(TimeControlCommand::StepTicks { ticks: 1 })
        .unwrap();
    assert!(!running_step.accepted);
    assert_eq!(
        running_step.rejection,
        Some(TimeControlRejection::NotPausedForExactStep)
    );
    assert_eq!(running_step.before, running);
    assert_eq!(running_step.after, running);

    let speed = bridge
        .apply_time_control_command(TimeControlCommand::SetSpeedMultiplier { multiplier: 0 })
        .unwrap();
    assert!(!speed.accepted);
    assert_eq!(
        speed.rejection,
        Some(TimeControlRejection::InvalidSpeedMultiplier)
    );
    assert_eq!(speed.before, speed.after);

    bridge
        .apply_time_control_command(TimeControlCommand::Pause)
        .unwrap();
    for ticks in [0, sim_runner::MAX_EXACT_STEP_TICKS + 1] {
        let rejected = bridge
            .apply_time_control_command(TimeControlCommand::StepTicks { ticks })
            .unwrap();
        assert!(!rejected.accepted);
        assert_eq!(
            rejected.rejection,
            Some(TimeControlRejection::InvalidStepCount)
        );
        assert_eq!(rejected.before, rejected.after);
    }
}

#[test]
fn speed_multiplier_executes_multiple_fixed_ticks_per_cadence_pulse() {
    let mut normal = init_bridge();
    let mut faster = init_bridge();
    for bridge in [&mut normal, &mut faster] {
        queue_command(
            bridge,
            7,
            Command::Tag(TagCommand::Define { id: TagId::new(3) }),
        );
        queue_command(
            bridge,
            8,
            Command::Entity(EntityCommand::Create {
                id: EntityId::new(12),
            }),
        );
        queue_command(
            bridge,
            9,
            Command::Entity(EntityCommand::AddTag {
                id: EntityId::new(12),
                tag: TagId::new(3),
            }),
        );
        queue_command(
            bridge,
            10,
            Command::Entity(EntityCommand::Create {
                id: EntityId::new(13),
            }),
        );
    }
    let speed = faster
        .apply_time_control_command(TimeControlCommand::SetSpeedMultiplier { multiplier: 4 })
        .unwrap();
    assert!(speed.accepted);
    assert_eq!(speed.after.speed_multiplier, 4);

    let normal_step = normal
        .step_simulation(StepInputEnvelope { tick: 7 })
        .unwrap();
    let faster_step = faster
        .step_simulation(StepInputEnvelope { tick: 7 })
        .unwrap();
    assert_eq!(normal_step.diff_count, 1);
    assert_eq!(normal_step.tick, 7);
    assert_eq!(faster_step.tick, 10);
    assert_eq!(faster_step.diff_count, 4);
    assert_eq!(faster.read_time_control_state().unwrap().authority_tick, 10);
    assert!(normal
        .time
        .simulation
        .state()
        .entity(EntityId::new(12))
        .is_none());
    assert!(faster
        .time
        .simulation
        .state()
        .entity(EntityId::new(12))
        .expect("four fixed ticks created and updated the entity")
        .tags
        .contains(&TagId::new(3)));
    assert!(faster
        .time
        .simulation
        .state()
        .entity(EntityId::new(13))
        .is_some());
}

#[test]
fn equivalent_time_commands_produce_deterministic_receipt_hashes() {
    let mut first = init_bridge();
    let mut second = init_bridge();
    let first_receipt = first
        .apply_time_control_command(TimeControlCommand::Pause)
        .unwrap();
    let second_receipt = second
        .apply_time_control_command(TimeControlCommand::Pause)
        .unwrap();
    assert_eq!(first_receipt, second_receipt);
}
