//! Replay record/step/hash/snapshot border shapes for the ASHA boundary.
//!
//! # Lane
//!
//! `contract-steward` — owns the shape of the deterministic replay record. May
//! depend on `core-ids`, `core-error`, `core-events`, and `core-commands`.
//!
//! # Border ownership
//!
//! ASHA's correctness story is replay: a recorded run is a sequence of
//! `(command in, events out, resulting state hash)` steps, and re-applying the
//! same commands to the same initial state must reproduce every hash. This
//! crate defines the *shape* of that record so it can be serialized, diffed, and
//! generated into TypeScript for tooling — it is the schema, not the engine.
//!
//! - [`StepIndex`] / [`ReplayHash`] are border-owned scalar wrappers.
//! - [`ReplayStep`] pairs an input [`CommandEnvelope`] with its [`StepOutcome`]
//!   (accepted [`DomainEvent`]s or a rejection summary) and the post-step hash.
//! - [`SnapshotMeta`] marks a point where a full state snapshot was taken.
//! - [`ReplayRecord`] is the whole run: an initial hash plus ordered steps and
//!   snapshot markers.
//!
//! These shapes are *sufficient for later Phase 4 scaffolding* (record format,
//! divergence reporting, replay tool); Phase 2 only fixes the border, it does
//! not record or replay anything.
//!
//! # Forbidden convenience logic
//!
//! No recording, no replaying, no hashing, no divergence detection. The
//! [`ReplayHash`] here is an opaque carrier of a hash computed elsewhere
//! (`core-snapshot`), not a hasher. Anything that *runs* the simulation lives in
//! the sim lane.

#![forbid(unsafe_code)]

use core_commands::CommandEnvelope;
use core_events::DomainEvent;

/// Compatibility marker for the replay record wire format.
///
/// Increment when the meaning or layout of [`ReplayRecord`] changes so old
/// records can be detected and migrated rather than silently misread.
pub const REPLAY_FORMAT_VERSION: u32 = 1;

// ── Scalar wrappers ───────────────────────────────────────────────────────────

/// Zero-based position of a step within a [`ReplayRecord`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StepIndex(pub u64);

impl StepIndex {
    #[inline]
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    #[inline]
    pub const fn raw(self) -> u64 {
        self.0
    }
}

/// A deterministic state fingerprint at a point in a replay.
///
/// This is the border carrier for the `u64` produced by `core-snapshot`'s
/// FNV-1a state hash. It is opaque here; the border records hashes, it does not
/// compute or interpret them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ReplayHash(pub u64);

impl ReplayHash {
    #[inline]
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    #[inline]
    pub const fn raw(self) -> u64 {
        self.0
    }
}

// ── Record shapes ─────────────────────────────────────────────────────────────

/// What the authority core decided about a proposed command.
///
/// A sum type rather than an "events + maybe-rejection" struct: a rejected
/// proposal *structurally* carries no accepted events, so the record can never
/// claim a rejected command also produced events.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepOutcome {
    /// The command was accepted; these domain events were applied, in order.
    Accepted { events: Vec<DomainEvent> },
    /// The command was rejected; no events were applied. `summary` is a
    /// human-readable reason — the authority validator owns the precise reason,
    /// the record keeps an inspectable summary.
    Rejected { summary: String },
}

impl StepOutcome {
    /// The accepted events, or an empty slice for a rejected step.
    pub fn events(&self) -> &[DomainEvent] {
        match self {
            StepOutcome::Accepted { events } => events,
            StepOutcome::Rejected { .. } => &[],
        }
    }

    pub fn is_accepted(&self) -> bool {
        matches!(self, StepOutcome::Accepted { .. })
    }
}

/// One recorded step: an input command, the authority core's [`StepOutcome`],
/// and the state hash immediately after the step.
///
/// Replaying re-applies `command` to the prior state and checks that the result
/// reproduces `outcome` and `post_hash`. A mismatch is a divergence (Phase 4).
/// For a rejected step `post_hash` equals the pre-step hash (state is unchanged).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayStep {
    pub index: StepIndex,
    pub command: CommandEnvelope,
    pub outcome: StepOutcome,
    pub post_hash: ReplayHash,
}

/// Marks that a full state snapshot was captured at a given step.
///
/// Snapshots let a replay resume or verify from an interior point instead of
/// re-running from step zero. The metadata records *where* and *what hash*; the
/// snapshot payload itself is owned by `core-snapshot`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SnapshotMeta {
    /// The step index whose post-state this snapshot captures.
    pub step: StepIndex,
    pub hash: ReplayHash,
    /// `core-snapshot::SNAPSHOT_VERSION` in effect when captured.
    pub snapshot_version: u32,
}

/// A complete recorded run: an initial state plus the ordered steps that
/// evolved it, with any snapshot markers taken along the way.
///
/// Determinism contract: re-applying `steps` in order to a world whose initial
/// state hashes to `initial_hash` must reproduce every `post_hash`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayRecord {
    pub format_version: u32,
    pub initial_hash: ReplayHash,
    pub steps: Vec<ReplayStep>,
    pub snapshots: Vec<SnapshotMeta>,
}

impl ReplayRecord {
    /// A fresh record at [`REPLAY_FORMAT_VERSION`] with the given initial hash
    /// and no steps or snapshots yet.
    pub fn new(initial_hash: ReplayHash) -> Self {
        Self {
            format_version: REPLAY_FORMAT_VERSION,
            initial_hash,
            steps: Vec::new(),
            snapshots: Vec::new(),
        }
    }

    pub fn step_count(&self) -> usize {
        self.steps.len()
    }

    /// The hash after the last step, or the initial hash if there are no steps.
    pub fn latest_hash(&self) -> ReplayHash {
        self.steps
            .last()
            .map(|s| s.post_hash)
            .unwrap_or(self.initial_hash)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use core_commands::{Command, CommandKind, EntityCommand};
    use core_ids::EntityId;

    fn create_step(index: u64, eid: u64, hash: u64) -> ReplayStep {
        ReplayStep {
            index: StepIndex::new(index),
            command: CommandEnvelope::new(
                CommandKind::Input,
                Command::Entity(EntityCommand::Create {
                    id: EntityId::new(eid),
                }),
            ),
            outcome: StepOutcome::Accepted {
                events: vec![DomainEvent::EntityCreated {
                    id: EntityId::new(eid),
                }],
            },
            post_hash: ReplayHash::new(hash),
        }
    }

    #[test]
    fn fresh_record_latest_hash_is_initial() {
        let rec = ReplayRecord::new(ReplayHash::new(0xABCD));
        assert_eq!(rec.format_version, REPLAY_FORMAT_VERSION);
        assert_eq!(rec.step_count(), 0);
        assert_eq!(rec.latest_hash(), ReplayHash::new(0xABCD));
    }

    #[test]
    fn record_accumulates_ordered_steps() {
        let mut rec = ReplayRecord::new(ReplayHash::new(1));
        rec.steps.push(create_step(0, 1, 100));
        rec.steps.push(create_step(1, 2, 200));

        assert_eq!(rec.step_count(), 2);
        assert_eq!(rec.steps[0].index, StepIndex::new(0));
        assert_eq!(rec.steps[1].index, StepIndex::new(1));
        assert_eq!(rec.latest_hash(), ReplayHash::new(200));
        assert!(rec.steps[0].outcome.is_accepted());
        assert!(matches!(
            rec.steps[0].outcome.events()[0],
            DomainEvent::EntityCreated { .. }
        ));
    }

    #[test]
    fn rejected_outcome_has_no_events() {
        let rejected = StepOutcome::Rejected {
            summary: "EntityNotFound".to_string(),
        };
        assert!(!rejected.is_accepted());
        assert!(rejected.events().is_empty());
    }

    #[test]
    fn snapshot_meta_marks_a_step() {
        let meta = SnapshotMeta {
            step: StepIndex::new(5),
            hash: ReplayHash::new(0xDEAD),
            snapshot_version: 1,
        };
        assert_eq!(meta.step.raw(), 5);
        assert_eq!(meta.hash.raw(), 0xDEAD);
    }

    #[test]
    fn step_and_hash_wrappers_are_distinct_types() {
        // Same raw value, different border meaning — cannot be confused.
        let i = StepIndex::new(7);
        let h = ReplayHash::new(7);
        assert_eq!(i.raw(), h.raw());
        // `assert_eq!(i, h)` would not compile: distinct newtypes.
    }
}
