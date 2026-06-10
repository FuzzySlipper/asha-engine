//! Headless tick execution for the ASHA authority core.
//!
//! # Lane
//!
//! `rust-state` — may depend on `core-ids`, `core-state`, `core-commands`,
//! `core-events`, `sim-kernel`, `sim-validator`, `sim-applier`. Must not
//! reference render, protocol, UI, or TypeScript packages.
//!
//! # Design
//!
//! [`run_tick`] wires the five kernel phases into a single function call:
//!
//! ```text
//! TickInput → validate each command → accumulate EventBatches
//!           → apply batches to StateStore → return TickOutcome
//! ```
//!
//! Rejected commands produce a [`RejectedEntry`] with the validator's
//! `Debug` reason; they do not touch the store. Accepted commands are applied
//! in submission order. The function returns a [`TickOutcome`] that callers
//! can inspect or forward to snapshot/telemetry layers (Phase 4/5).

#![forbid(unsafe_code)]

use core_commands::CommandEnvelope;
use core_state::StateStore;
use sim_applier::apply_batch;
use sim_kernel::{AcceptedEntry, RejectedEntry, TickInput, TickOutcome};
use sim_replay::{
    diff, CheckpointInterval, Divergence, ReplayHash, ReplayRecord, ReplayStep, SnapshotMeta,
    StepIndex, StepOutcome,
};
use sim_validator::validate;

/// Execute one authority tick: validate all proposed commands, apply accepted
/// event batches to `store` in order, and return the tick summary.
///
/// Rejected commands are recorded in [`TickOutcome::rejected`] and do not
/// mutate the store. Accepted commands are applied in submission order.
pub fn run_tick(store: &mut StateStore, input: TickInput) -> TickOutcome {
    let tick = input.tick;
    let mut accepted = Vec::new();
    let mut rejected = Vec::new();

    // Phase: Validate + AccumulateEvents
    for envelope in input.commands {
        match validate(store, &envelope) {
            Ok(batch) => accepted.push(AcceptedEntry { envelope, batch }),
            Err(err) => rejected.push(RejectedEntry {
                envelope,
                reason: format!("{err:?}"),
            }),
        }
    }

    // Phase: ApplyEvents
    let mut events_applied = 0;
    for entry in &accepted {
        // apply_batch errors here would indicate a bug (validator already
        // checked the store); propagate as a panic to keep the path loud.
        apply_batch(store, &entry.batch)
            .expect("applier must not fail for validator-accepted events");
        events_applied += entry.batch.len();
    }

    TickOutcome {
        tick,
        accepted,
        rejected,
        events_applied,
    }
}

// ── Replay recording ──────────────────────────────────────────────────────────

/// Records proposed commands, the authority [`StepOutcome`], and post-step state
/// hashes into a [`ReplayRecord`] as a store is driven forward.
///
/// Recording is explicit and opt-in: the normal [`run_tick`] path does no
/// recording and keeps no hidden global state. To record, construct a
/// `Recorder`, feed it commands, and call [`Recorder::finish`].
///
/// Rejected commands are recorded as [`StepOutcome::Rejected`] proposals — they
/// never appear as accepted events, and their `post_hash` equals the prior hash
/// because a rejection does not mutate the store.
pub struct Recorder {
    record: ReplayRecord,
    next_index: u64,
    /// When set, checkpoints are captured automatically per this interval and a
    /// final checkpoint is appended on [`Recorder::finish`]. `None` means only
    /// explicit [`Recorder::checkpoint`] calls capture checkpoints.
    interval: Option<CheckpointInterval>,
}

impl Recorder {
    /// Begin recording from `store`'s current state (captured as the initial
    /// hash). Checkpoints are captured only via explicit [`Recorder::checkpoint`].
    pub fn new(store: &StateStore) -> Self {
        Self {
            record: ReplayRecord::new(state_hash(store)),
            next_index: 0,
            interval: None,
        }
    }

    /// Begin recording with automatic state-hash checkpoints at `interval`
    /// (plus a final checkpoint on [`Recorder::finish`]).
    pub fn with_interval(store: &StateStore, interval: CheckpointInterval) -> Self {
        Self {
            record: ReplayRecord::new(state_hash(store)),
            next_index: 0,
            interval: Some(interval),
        }
    }

    /// Validate one proposed command, apply it if accepted, and record the step.
    pub fn record_command(&mut self, store: &mut StateStore, envelope: CommandEnvelope) {
        let index = self.next_index;
        self.next_index += 1;

        let outcome = match validate(store, &envelope) {
            Ok(batch) => {
                let events = batch.events().to_vec();
                apply_batch(store, &batch)
                    .expect("applier must not fail for validator-accepted events");
                StepOutcome::Accepted { events }
            }
            Err(err) => StepOutcome::Rejected {
                summary: format!("{err:?}"),
            },
        };

        let post_hash = state_hash(store);
        self.record.steps.push(ReplayStep {
            index: StepIndex::new(index),
            command: envelope,
            outcome,
            post_hash,
        });

        if let Some(interval) = self.interval {
            if interval.captures_after(index) {
                self.push_checkpoint(StepIndex::new(index), post_hash);
            }
        }
    }

    /// Record every command of a tick, in submission order.
    pub fn record_tick(&mut self, store: &mut StateStore, input: TickInput) {
        for envelope in input.commands {
            self.record_command(store, envelope);
        }
    }

    /// Mark a full-snapshot checkpoint for the most recently recorded step.
    ///
    /// Records the step index and state hash; the snapshot payload itself is
    /// owned by `core-snapshot`. Call after at least one command.
    pub fn checkpoint(&mut self, store: &StateStore) {
        let step = StepIndex::new(self.next_index.saturating_sub(1));
        self.push_checkpoint(step, state_hash(store));
    }

    /// Number of steps recorded so far.
    pub fn step_count(&self) -> usize {
        self.record.step_count()
    }

    /// Consume the recorder and return the finished record. If an interval was
    /// configured, ensures a final checkpoint exists for the last step.
    pub fn finish(mut self) -> ReplayRecord {
        if self.interval.is_some() {
            if let Some(last) = self.record.steps.last() {
                let step = last.index;
                let hash = last.post_hash;
                if self.record.snapshots.last().map(|s| s.step) != Some(step) {
                    self.push_checkpoint(step, hash);
                }
            }
        }
        self.record
    }

    /// Append a checkpoint, de-duplicating a repeat of the most recent step.
    fn push_checkpoint(&mut self, step: StepIndex, hash: ReplayHash) {
        if self.record.snapshots.last().map(|s| s.step) == Some(step) {
            return;
        }
        self.record.snapshots.push(SnapshotMeta {
            step,
            hash,
            snapshot_version: core_snapshot::SNAPSHOT_VERSION,
        });
    }
}

/// The deterministic state hash, carried as a border [`ReplayHash`].
fn state_hash(store: &StateStore) -> ReplayHash {
    ReplayHash::new(core_snapshot::hash_store(store).0)
}

// ── Golden replay playback ────────────────────────────────────────────────────

/// Replay a recorded golden against the *current* authority logic, starting from
/// an empty store, and verify it reproduces the recorded outcomes and hashes.
///
/// It re-validates and re-applies each recorded command to build a fresh record,
/// then [`diff`]s that against the golden. The first divergence is returned as a
/// structured [`Divergence`] with a class and likely-owner routing (see
/// [`Divergence::report`]). `Ok(())` means the golden reproduces exactly.
///
/// The golden is assumed to start from the empty world (`StateStore::new()`);
/// its `initial_hash` is verified by the diff.
pub fn playback(golden: &ReplayRecord) -> Result<(), Divergence> {
    let produced = reproduce(golden);
    match diff(golden, &produced) {
        Some(divergence) => Err(divergence),
        None => Ok(()),
    }
}

/// Re-run the golden's commands under current authority logic to produce a fresh
/// record with the same step indices and (golden-mirrored) checkpoint positions,
/// but re-derived outcomes and hashes.
fn reproduce(golden: &ReplayRecord) -> ReplayRecord {
    let mut store = StateStore::new();
    let mut produced = ReplayRecord::new(state_hash(&store));

    for step in &golden.steps {
        let outcome = match validate(&store, &step.command) {
            Ok(batch) => {
                let events = batch.events().to_vec();
                apply_batch(&mut store, &batch)
                    .expect("applier must not fail for validator-accepted events");
                StepOutcome::Accepted { events }
            }
            Err(err) => StepOutcome::Rejected {
                summary: format!("{err:?}"),
            },
        };
        produced.steps.push(ReplayStep {
            index: step.index,
            command: step.command.clone(),
            outcome,
            post_hash: state_hash(&store),
        });
    }

    // Mirror the golden's checkpoint positions with re-derived hashes so the
    // diff compares checkpoint hashes at the same steps.
    for snap in &golden.snapshots {
        let hash = produced
            .steps
            .iter()
            .find(|s| s.index == snap.step)
            .map(|s| s.post_hash)
            .unwrap_or(snap.hash);
        produced.snapshots.push(SnapshotMeta {
            step: snap.step,
            hash,
            snapshot_version: snap.snapshot_version,
        });
    }

    produced
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use core_commands::{
        Command, CommandKind, EntityCommand, ModeCommand, ProcessCommand, TagCommand,
    };
    use core_ids::{EntityId, ModeId, ProcessId, TagId};
    use sim_kernel::TickInput;

    fn sys(cmd: Command) -> core_commands::CommandEnvelope {
        core_commands::CommandEnvelope::new(CommandKind::System, cmd)
    }

    // ── Headless tick test ────────────────────────────────────────────────

    /// Phase 1 epic exit criterion: headless tick exercising the full
    /// authority path — propose → validate → apply → inspect state + hash.
    #[test]
    fn headless_tick_test_authority_core_flow() {
        use core_snapshot::{hash_store, snapshot};

        let mut store = StateStore::new();

        // -- Tick 1: define a tag, create an entity, start a process
        let mut input = TickInput::new(1);
        input.push(sys(Command::Tag(TagCommand::Define { id: TagId::new(1) })));
        input.push(sys(Command::Entity(EntityCommand::Create {
            id: EntityId::new(10),
        })));
        input.push(sys(Command::Mode(ModeCommand::Define {
            id: ModeId::new(1),
        })));
        input.push(sys(Command::Process(ProcessCommand::Start {
            id: ProcessId::new(1),
        })));

        let hash_before = hash_store(&store);
        let outcome = run_tick(&mut store, input);

        assert_eq!(outcome.tick, 1);
        assert_eq!(outcome.accepted_count(), 4);
        assert_eq!(outcome.rejected_count(), 0);
        assert_eq!(outcome.events_applied, 4);

        // State must have changed.
        let hash_after = hash_store(&store);
        assert_ne!(hash_before, hash_after, "tick must change state hash");

        assert!(store.tag(TagId::new(1)).is_some());
        assert!(store.entity(EntityId::new(10)).is_some());
        assert!(store.mode(ModeId::new(1)).is_some());
        assert!(store.process(ProcessId::new(1)).is_some());

        // -- Tick 2: add tag to entity, set process mode
        let mut input2 = TickInput::new(2);
        input2.push(sys(Command::Entity(EntityCommand::AddTag {
            id: EntityId::new(10),
            tag: TagId::new(1),
        })));
        input2.push(sys(Command::Process(ProcessCommand::SetMode {
            id: ProcessId::new(1),
            mode: ModeId::new(1),
        })));

        let outcome2 = run_tick(&mut store, input2);
        assert_eq!(outcome2.accepted_count(), 2);
        assert_eq!(outcome2.rejected_count(), 0);
        assert!(store
            .entity(EntityId::new(10))
            .unwrap()
            .tags
            .contains(&TagId::new(1)));
        assert_eq!(
            store.process(ProcessId::new(1)).unwrap().mode,
            Some(ModeId::new(1))
        );

        // -- Tick 3: delete entity
        let mut input3 = TickInput::new(3);
        input3.push(sys(Command::Entity(EntityCommand::Delete {
            id: EntityId::new(10),
        })));
        let outcome3 = run_tick(&mut store, input3);
        assert_eq!(outcome3.accepted_count(), 1);
        assert!(store.entity(EntityId::new(10)).is_none());

        // Snapshot the final state for inspectability.
        let snap = snapshot(&store);
        assert_eq!(snap.version, core_snapshot::SNAPSHOT_VERSION);
        assert_eq!(snap.hash, hash_store(&store));
    }

    // ── Rejected command does not mutate state ────────────────────────────

    #[test]
    fn rejected_command_does_not_mutate_store() {
        let mut store = StateStore::new();
        // Entity 99 does not exist — Delete should be rejected.
        let mut input = TickInput::new(1);
        input.push(sys(Command::Entity(EntityCommand::Delete {
            id: EntityId::new(99),
        })));

        let outcome = run_tick(&mut store, input);
        assert_eq!(outcome.accepted_count(), 0);
        assert_eq!(outcome.rejected_count(), 1);
        assert_eq!(outcome.events_applied, 0);
        assert!(store.entity(EntityId::new(99)).is_none()); // store unchanged
    }

    #[test]
    fn mixed_tick_accepted_and_rejected() {
        let mut store = StateStore::new();
        store.insert_entity(EntityId::new(1)); // already exists

        let mut input = TickInput::new(1);
        // This will be rejected (duplicate).
        input.push(sys(Command::Entity(EntityCommand::Create {
            id: EntityId::new(1),
        })));
        // This will be accepted (new entity).
        input.push(sys(Command::Entity(EntityCommand::Create {
            id: EntityId::new(2),
        })));

        let outcome = run_tick(&mut store, input);
        assert_eq!(outcome.accepted_count(), 1);
        assert_eq!(outcome.rejected_count(), 1);
        assert!(!outcome.rejected[0].reason.is_empty());
        // Entity 2 was created; entity 1 still exists unchanged.
        assert!(store.entity(EntityId::new(2)).is_some());
        assert_eq!(store.entity_count(), 2);
    }

    // ── Phase 1 epic exit criteria represented as tests ───────────────────

    /// create/update/delete entity fixture
    #[test]
    fn epic_exit_create_update_delete_entity() {
        let mut store = StateStore::new();

        // Create
        let mut i = TickInput::new(1);
        i.push(sys(Command::Tag(TagCommand::Define { id: TagId::new(1) })));
        i.push(sys(Command::Entity(EntityCommand::Create {
            id: EntityId::new(1),
        })));
        let o = run_tick(&mut store, i);
        assert_eq!(o.rejected_count(), 0);

        // Update (add tag)
        let mut i2 = TickInput::new(2);
        i2.push(sys(Command::Entity(EntityCommand::AddTag {
            id: EntityId::new(1),
            tag: TagId::new(1),
        })));
        let o2 = run_tick(&mut store, i2);
        assert_eq!(o2.rejected_count(), 0);
        assert!(store
            .entity(EntityId::new(1))
            .unwrap()
            .tags
            .contains(&TagId::new(1)));

        // Delete
        let mut i3 = TickInput::new(3);
        i3.push(sys(Command::Entity(EntityCommand::Delete {
            id: EntityId::new(1),
        })));
        let o3 = run_tick(&mut store, i3);
        assert_eq!(o3.rejected_count(), 0);
        assert!(store.entity(EntityId::new(1)).is_none());
    }

    /// command validation fixture
    #[test]
    fn epic_exit_command_validation_fixture() {
        let mut store = StateStore::new();
        store.insert_entity(EntityId::new(1));

        // Valid command.
        let mut i = TickInput::new(1);
        i.push(sys(Command::Entity(EntityCommand::Delete {
            id: EntityId::new(1),
        })));
        let o = run_tick(&mut store, i);
        assert_eq!(o.accepted_count(), 1);

        // Invalid command (already deleted).
        let mut i2 = TickInput::new(2);
        i2.push(sys(Command::Entity(EntityCommand::Delete {
            id: EntityId::new(1),
        })));
        let o2 = run_tick(&mut store, i2);
        assert_eq!(o2.rejected_count(), 1);
    }

    /// event application fixture (via full tick path)
    #[test]
    fn epic_exit_event_application_fixture() {
        let mut store = StateStore::new();
        let mut i = TickInput::new(1);
        i.push(sys(Command::Entity(EntityCommand::Create {
            id: EntityId::new(5),
        })));
        let o = run_tick(&mut store, i);
        assert_eq!(o.events_applied, 1);
        assert!(store.entity(EntityId::new(5)).is_some());
    }

    /// state hash fixture
    #[test]
    fn epic_exit_state_hash_fixture() {
        use core_snapshot::hash_store;

        let mut s1 = StateStore::new();
        let mut s2 = StateStore::new();

        let input_a = {
            let mut i = TickInput::new(1);
            i.push(sys(Command::Entity(EntityCommand::Create {
                id: EntityId::new(1),
            })));
            i
        };
        let input_b = {
            let mut i = TickInput::new(1);
            i.push(sys(Command::Entity(EntityCommand::Create {
                id: EntityId::new(1),
            })));
            i
        };

        run_tick(&mut s1, input_a);
        run_tick(&mut s2, input_b);

        assert_eq!(
            hash_store(&s1),
            hash_store(&s2),
            "same sequence → same hash"
        );

        // Different sequence → different hash.
        let mut i3 = TickInput::new(2);
        i3.push(sys(Command::Entity(EntityCommand::Create {
            id: EntityId::new(2),
        })));
        run_tick(&mut s1, i3);
        assert_ne!(hash_store(&s1), hash_store(&s2));
    }

    // ── Replay recording ──────────────────────────────────────────────────

    #[test]
    fn recording_accepts_command_with_events_and_hash_change() {
        let mut store = StateStore::new();
        let mut rec = Recorder::new(&store);
        let initial = rec.record.initial_hash;

        rec.record_command(
            &mut store,
            sys(Command::Entity(EntityCommand::Create {
                id: EntityId::new(5),
            })),
        );

        let record = rec.finish();
        assert_eq!(record.steps.len(), 1);
        let step = &record.steps[0];
        assert!(step.outcome.is_accepted());
        assert!(matches!(
            step.outcome.events()[0],
            core_events::DomainEvent::EntityCreated { .. }
        ));
        // An accepted command mutates state, so the hash advances.
        assert_ne!(step.post_hash, initial);
    }

    #[test]
    fn recording_marks_rejection_without_events_and_unchanged_hash() {
        let mut store = StateStore::new();
        let mut rec = Recorder::new(&store);
        let initial = rec.record.initial_hash;

        // Entity 99 does not exist — Delete is rejected.
        rec.record_command(
            &mut store,
            sys(Command::Entity(EntityCommand::Delete {
                id: EntityId::new(99),
            })),
        );

        let record = rec.finish();
        let step = &record.steps[0];
        match &step.outcome {
            StepOutcome::Rejected { summary } => assert!(summary.contains("EntityNotFound")),
            other => panic!("expected rejection, got {other:?}"),
        }
        assert!(step.outcome.events().is_empty());
        // A rejection does not mutate state, so the hash is unchanged.
        assert_eq!(step.post_hash, initial);
    }

    #[test]
    fn recording_preserves_command_order_for_mixed_tick() {
        let mut store = StateStore::new();
        store.insert_entity(EntityId::new(1)); // pre-existing

        let mut input = TickInput::new(1);
        // rejected (duplicate), accepted (new), accepted (tag define)
        input.push(sys(Command::Entity(EntityCommand::Create {
            id: EntityId::new(1),
        })));
        input.push(sys(Command::Entity(EntityCommand::Create {
            id: EntityId::new(2),
        })));
        input.push(sys(Command::Tag(TagCommand::Define { id: TagId::new(9) })));

        let mut rec = Recorder::new(&store);
        rec.record_tick(&mut store, input);
        rec.checkpoint(&store);
        let record = rec.finish();

        assert_eq!(record.steps.len(), 3);
        assert_eq!(record.steps[0].index, StepIndex::new(0));
        assert!(!record.steps[0].outcome.is_accepted()); // rejected duplicate
        assert!(record.steps[1].outcome.is_accepted());
        assert!(record.steps[2].outcome.is_accepted());
        // The checkpoint marks the last recorded step.
        assert_eq!(record.snapshots.len(), 1);
        assert_eq!(record.snapshots[0].step, StepIndex::new(2));

        // The recorded run encodes to a stable, re-decodable artifact.
        let text = sim_replay::encode(&record);
        assert_eq!(sim_replay::decode(&text).unwrap(), record);
    }

    // ── State hash checkpoint intervals ───────────────────────────────────

    fn record_three_tag_defines(interval: CheckpointInterval) -> ReplayRecord {
        let mut store = StateStore::new();
        let mut rec = Recorder::with_interval(&store, interval);
        let mut input = TickInput::new(1);
        input.push(sys(Command::Tag(TagCommand::Define { id: TagId::new(1) })));
        input.push(sys(Command::Tag(TagCommand::Define { id: TagId::new(2) })));
        input.push(sys(Command::Tag(TagCommand::Define { id: TagId::new(3) })));
        rec.record_tick(&mut store, input);
        rec.finish()
    }

    #[test]
    fn identical_sequences_produce_identical_checkpoint_series() {
        let a = record_three_tag_defines(CheckpointInterval::EveryStep);
        let b = record_three_tag_defines(CheckpointInterval::EveryStep);
        assert_eq!(a, b, "same sequence must produce an identical record");

        let series_a: Vec<_> = a.snapshots.iter().map(|s| (s.step, s.hash)).collect();
        let series_b: Vec<_> = b.snapshots.iter().map(|s| (s.step, s.hash)).collect();
        assert_eq!(series_a, series_b);
        assert_eq!(a.snapshots.len(), 3, "EveryStep checkpoints every step");
    }

    #[test]
    fn accepted_event_changes_checkpoint_hash() {
        let mut store = StateStore::new();
        let mut rec = Recorder::with_interval(&store, CheckpointInterval::EveryStep);
        let initial = rec.record.initial_hash;

        rec.record_command(
            &mut store,
            sys(Command::Entity(EntityCommand::Create {
                id: EntityId::new(5),
            })),
        );
        let record = rec.finish();

        assert_eq!(record.snapshots.len(), 1);
        assert_ne!(
            record.snapshots[0].hash, initial,
            "an accepted event must change the checkpoint hash"
        );
    }

    #[test]
    fn rejected_command_does_not_advance_checkpoint_hash() {
        let mut store = StateStore::new();
        let mut rec = Recorder::with_interval(&store, CheckpointInterval::EveryStep);
        let initial = rec.record.initial_hash;

        rec.record_command(
            &mut store,
            sys(Command::Entity(EntityCommand::Delete {
                id: EntityId::new(99),
            })),
        );
        let record = rec.finish();

        assert_eq!(record.snapshots.len(), 1);
        assert_eq!(
            record.snapshots[0].hash, initial,
            "a rejected command must not advance the hash as if events applied"
        );
        assert!(!record.steps[0].outcome.is_accepted());
    }

    #[test]
    fn interval_every_two_steps_captures_at_interval_and_final() {
        let record = record_three_tag_defines(CheckpointInterval::EverySteps(2));
        let steps: Vec<u64> = record.snapshots.iter().map(|s| s.step.raw()).collect();
        // Capture after the 2nd step (index 1) and a final at index 2.
        assert_eq!(steps, vec![1, 2]);
    }

    #[test]
    fn final_only_interval_captures_just_the_last_step() {
        let record = record_three_tag_defines(CheckpointInterval::FinalOnly);
        let steps: Vec<u64> = record.snapshots.iter().map(|s| s.step.raw()).collect();
        assert_eq!(steps, vec![2]);
    }

    fn golden_path(name: &str) -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(4)
            .expect("repo root")
            .join("harness/goldens/replays")
            .join(format!("{name}.replay"))
    }

    /// Golden replay check: the committed golden must (1) still be exactly what
    /// the recorder produces for the scenario, and (2) play back cleanly against
    /// current authority logic — reproducing every event and hash checkpoint.
    #[test]
    fn golden_replay_tagged_entity_run() {
        let golden =
            std::fs::read_to_string(golden_path("tagged-entity-run")).expect("read golden replay");

        // (1) The decoded golden plays back without divergence — checked first so
        // a drift surfaces as a routed divergence report rather than a byte diff.
        let decoded = sim_replay::decode(&golden).expect("golden decodes");
        playback(&decoded).unwrap_or_else(|d| panic!("\n{}", d.report("tagged-entity-run")));

        // (2) The recorder still produces exactly the golden bytes.
        let mut store = StateStore::new();
        let mut rec = Recorder::with_interval(&store, CheckpointInterval::EverySteps(2));
        rec.record_tick(&mut store, tagged_entity_run_input());
        let record = rec.finish();
        assert_eq!(
            sim_replay::encode(&record),
            golden,
            "recorder output drifted from golden 'tagged-entity-run'"
        );
    }

    /// A tampered golden must be caught by playback (the audit actually bites).
    #[test]
    fn playback_detects_a_tampered_event() {
        let golden =
            std::fs::read_to_string(golden_path("tagged-entity-run")).expect("read golden replay");
        // Flip an accepted event id so the recorded outcome no longer matches.
        let tampered = golden.replacen("event entity.created 10", "event entity.created 12", 1);
        let record = sim_replay::decode(&tampered).expect("still decodes");
        let err = playback(&record).expect_err("tampered event must diverge");
        assert_eq!(err.step, Some(1));
        assert_eq!(
            err.class,
            sim_replay::DivergenceClass::AcceptedEventMismatch
        );
        // The report routes a repair agent to the validate/apply path.
        assert!(err.report("tagged-entity-run").contains("sim-validator"));
    }

    fn tagged_entity_run_input() -> TickInput {
        let mut input = TickInput::new(1);
        input.push(sys(Command::Tag(TagCommand::Define { id: TagId::new(1) })));
        input.push(sys(Command::Entity(EntityCommand::Create {
            id: EntityId::new(10),
        })));
        input.push(sys(Command::Entity(EntityCommand::AddTag {
            id: EntityId::new(10),
            tag: TagId::new(1),
        })));
        // Stale delete — rejected, state unchanged.
        input.push(sys(Command::Entity(EntityCommand::Delete {
            id: EntityId::new(99),
        })));
        input.push(sys(Command::Entity(EntityCommand::Create {
            id: EntityId::new(11),
        })));
        input
    }
}
