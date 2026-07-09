//! Durable voxel edit history and cursor authority.
//!
//! This module owns the undo/revert cursor over accepted
//! [`VoxelEditTransactionReceipt`] event logs. The correctness path is replay
//! from the base world through accepted events; callers do not reconstruct state
//! in TypeScript or from rendered projections.

use core_events::VoxelEditEvent;
use svc_spatial::VoxelWorld;

use crate::{apply_all, voxel_world_hash, VoxelEditRejection, VoxelEditTransactionReceipt};

/// Guardrails for retained history and replay work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VoxelEditHistoryLimits {
    pub max_entries: usize,
    pub max_retained_events: usize,
    pub max_replay_steps: usize,
}

impl VoxelEditHistoryLimits {
    pub const fn new(
        max_entries: usize,
        max_retained_events: usize,
        max_replay_steps: usize,
    ) -> Self {
        Self {
            max_entries,
            max_retained_events,
            max_replay_steps,
        }
    }
}

impl Default for VoxelEditHistoryLimits {
    fn default() -> Self {
        Self::new(10_000, 100_000, 10_000)
    }
}

/// One accepted durable transaction in the history timeline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoxelEditHistoryEntry {
    pub transaction_id: u64,
    pub parent_transaction_id: Option<u64>,
    pub cursor_id: u64,
    pub parent_cursor_id: u64,
    pub receipt: VoxelEditTransactionReceipt,
}

/// Cursor readout for the applied history head.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VoxelEditHistoryCursor {
    pub cursor_id: u64,
    pub applied_transaction_id: Option<u64>,
    pub index: usize,
    pub undo_depth: usize,
    pub redo_depth: usize,
    pub world_hash: u64,
    pub history_hash: u64,
}

/// Bounded diff readout for a replayed target cursor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VoxelEditHistoryDiffSummary {
    pub before_hash: u64,
    pub target_hash: u64,
    pub changed_transaction_count: usize,
    pub replayed_transaction_count: usize,
}

/// Receipt for appending an accepted transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoxelEditHistoryAppendReceipt {
    pub entry: VoxelEditHistoryEntry,
    pub cursor_before: VoxelEditHistoryCursor,
    pub cursor_after: VoxelEditHistoryCursor,
    pub invalidated_redo_count: usize,
}

/// Receipt for previewing or applying a revert-like operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoxelEditHistoryRevertReceipt {
    pub applied: bool,
    pub preview: bool,
    pub cursor_before: VoxelEditHistoryCursor,
    pub cursor_after: VoxelEditHistoryCursor,
    pub diff: VoxelEditHistoryDiffSummary,
    pub replay_hash: u64,
}

/// Why history/cursor authority refused a request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoxelEditHistoryRejection {
    ReceiptWasNotApplied,
    ReceiptHadRejections {
        rejected: u32,
    },
    StaleCursorHash {
        expected: u64,
        actual: u64,
    },
    EntryQuotaExceeded {
        limit: usize,
        actual: usize,
    },
    RetainedEventQuotaExceeded {
        limit: usize,
        actual: usize,
    },
    ReplayQuotaExceeded {
        limit: usize,
        actual: usize,
    },
    UnknownTransaction {
        transaction_id: u64,
    },
    InvalidCursor {
        cursor_index: usize,
        entry_count: usize,
    },
    EmptyUndoStack,
    EmptyRedoStack,
    ReplayFailed(VoxelEditRejection),
}

/// Rust-owned durable edit history over accepted voxel transaction receipts.
#[derive(Debug, Clone)]
pub struct VoxelEditHistory {
    base_world: VoxelWorld,
    current_world: VoxelWorld,
    entries: Vec<VoxelEditHistoryEntry>,
    cursor_index: usize,
    next_transaction_id: u64,
    limits: VoxelEditHistoryLimits,
}

impl VoxelEditHistory {
    /// Start history at a caller-provided base world.
    pub fn new(base_world: VoxelWorld) -> Self {
        Self::with_limits(base_world, VoxelEditHistoryLimits::default())
    }

    /// Start history with explicit guardrails.
    pub fn with_limits(base_world: VoxelWorld, limits: VoxelEditHistoryLimits) -> Self {
        Self {
            current_world: base_world.clone(),
            base_world,
            entries: Vec::new(),
            cursor_index: 0,
            next_transaction_id: 1,
            limits,
        }
    }

    pub fn entries(&self) -> &[VoxelEditHistoryEntry] {
        &self.entries
    }

    pub fn cursor(&self) -> VoxelEditHistoryCursor {
        self.cursor_at(self.cursor_index, voxel_world_hash(&self.current_world))
    }

    pub fn current_world(&self) -> &VoxelWorld {
        &self.current_world
    }

    pub fn current_world_hash(&self) -> u64 {
        voxel_world_hash(&self.current_world)
    }

    /// Append one accepted applied receipt at the current cursor.
    ///
    /// If the cursor is not at the end, the redo tail is invalidated before the
    /// new entry is retained.
    pub fn append_accepted(
        &mut self,
        receipt: VoxelEditTransactionReceipt,
    ) -> Result<VoxelEditHistoryAppendReceipt, VoxelEditHistoryRejection> {
        if !receipt.applied {
            return Err(VoxelEditHistoryRejection::ReceiptWasNotApplied);
        }
        if receipt.rejected != 0 {
            return Err(VoxelEditHistoryRejection::ReceiptHadRejections {
                rejected: receipt.rejected,
            });
        }

        let current_hash = voxel_world_hash(&self.current_world);
        if receipt.before_hash != current_hash {
            return Err(VoxelEditHistoryRejection::StaleCursorHash {
                expected: current_hash,
                actual: receipt.before_hash,
            });
        }

        let retained_prefix_events = self.retained_event_count_for_prefix(self.cursor_index);
        let actual_entries = self.cursor_index.saturating_add(1);
        let actual_events = retained_prefix_events.saturating_add(receipt.events.len());
        if actual_entries > self.limits.max_entries {
            return Err(VoxelEditHistoryRejection::EntryQuotaExceeded {
                limit: self.limits.max_entries,
                actual: actual_entries,
            });
        }
        if actual_events > self.limits.max_retained_events {
            return Err(VoxelEditHistoryRejection::RetainedEventQuotaExceeded {
                limit: self.limits.max_retained_events,
                actual: actual_events,
            });
        }

        let cursor_before = self.cursor();
        let invalidated_redo_count = self.entries.len().saturating_sub(self.cursor_index);
        self.entries.truncate(self.cursor_index);

        apply_all(&mut self.current_world, &receipt.events)
            .map_err(VoxelEditHistoryRejection::ReplayFailed)?;
        let after_hash = voxel_world_hash(&self.current_world);
        if receipt.after_hash != after_hash {
            return Err(VoxelEditHistoryRejection::StaleCursorHash {
                expected: receipt.after_hash,
                actual: after_hash,
            });
        }

        let parent_transaction_id = self.entries.last().map(|entry| entry.transaction_id);
        let parent_cursor_id = cursor_id_for_index(self.cursor_index);
        let transaction_id = self.next_transaction_id;
        self.next_transaction_id = self.next_transaction_id.saturating_add(1);
        self.cursor_index = self.cursor_index.saturating_add(1);
        let entry = VoxelEditHistoryEntry {
            transaction_id,
            parent_transaction_id,
            cursor_id: cursor_id_for_index(self.cursor_index),
            parent_cursor_id,
            receipt,
        };
        self.entries.push(entry.clone());
        let cursor_after = self.cursor();

        Ok(VoxelEditHistoryAppendReceipt {
            entry,
            cursor_before,
            cursor_after,
            invalidated_redo_count,
        })
    }

    /// Preview the cursor/world that would result from reverting to `cursor_index`.
    pub fn preview_revert_to_cursor(
        &self,
        cursor_index: usize,
    ) -> Result<VoxelEditHistoryRevertReceipt, VoxelEditHistoryRejection> {
        self.revert_receipt_for_cursor(cursor_index, false)
            .map(|(receipt, _)| receipt)
    }

    /// Apply a revert to `cursor_index` by replaying from the base world.
    pub fn apply_revert_to_cursor(
        &mut self,
        cursor_index: usize,
    ) -> Result<VoxelEditHistoryRevertReceipt, VoxelEditHistoryRejection> {
        let (receipt, target_world) = self.revert_receipt_for_cursor(cursor_index, true)?;
        self.current_world = target_world;
        self.cursor_index = cursor_index;
        Ok(receipt)
    }

    /// Preview reverting to the cursor immediately after `transaction_id`.
    pub fn preview_revert_to_transaction(
        &self,
        transaction_id: u64,
    ) -> Result<VoxelEditHistoryRevertReceipt, VoxelEditHistoryRejection> {
        let cursor_index = self.cursor_index_after_transaction(transaction_id)?;
        self.preview_revert_to_cursor(cursor_index)
    }

    /// Apply a revert to the cursor immediately after `transaction_id`.
    pub fn apply_revert_to_transaction(
        &mut self,
        transaction_id: u64,
    ) -> Result<VoxelEditHistoryRevertReceipt, VoxelEditHistoryRejection> {
        let cursor_index = self.cursor_index_after_transaction(transaction_id)?;
        self.apply_revert_to_cursor(cursor_index)
    }

    /// Undo one accepted transaction.
    pub fn undo_one(&mut self) -> Result<VoxelEditHistoryRevertReceipt, VoxelEditHistoryRejection> {
        if self.cursor_index == 0 {
            return Err(VoxelEditHistoryRejection::EmptyUndoStack);
        }
        self.apply_revert_to_cursor(self.cursor_index - 1)
    }

    /// Redo one retained transaction from the redo tail.
    pub fn redo_one(&mut self) -> Result<VoxelEditHistoryRevertReceipt, VoxelEditHistoryRejection> {
        if self.cursor_index >= self.entries.len() {
            return Err(VoxelEditHistoryRejection::EmptyRedoStack);
        }
        self.apply_revert_to_cursor(self.cursor_index + 1)
    }

    fn revert_receipt_for_cursor(
        &self,
        cursor_index: usize,
        apply: bool,
    ) -> Result<(VoxelEditHistoryRevertReceipt, VoxelWorld), VoxelEditHistoryRejection> {
        let cursor_before = self.cursor();
        let before_hash = cursor_before.world_hash;
        let target_world = self.replay_to_cursor(cursor_index)?;
        let target_hash = voxel_world_hash(&target_world);
        let cursor_after = self.cursor_at(cursor_index, target_hash);
        let changed_transaction_count = cursor_before.index.abs_diff(cursor_index);
        let diff = VoxelEditHistoryDiffSummary {
            before_hash,
            target_hash,
            changed_transaction_count,
            replayed_transaction_count: cursor_index,
        };
        let receipt = VoxelEditHistoryRevertReceipt {
            applied: apply,
            preview: !apply,
            cursor_before,
            cursor_after,
            diff,
            replay_hash: replay_hash(cursor_index, target_hash, &self.entries),
        };
        Ok((receipt, target_world))
    }

    fn replay_to_cursor(
        &self,
        cursor_index: usize,
    ) -> Result<VoxelWorld, VoxelEditHistoryRejection> {
        if cursor_index > self.entries.len() {
            return Err(VoxelEditHistoryRejection::InvalidCursor {
                cursor_index,
                entry_count: self.entries.len(),
            });
        }
        if cursor_index > self.limits.max_replay_steps {
            return Err(VoxelEditHistoryRejection::ReplayQuotaExceeded {
                limit: self.limits.max_replay_steps,
                actual: cursor_index,
            });
        }

        let mut world = self.base_world.clone();
        for entry in &self.entries[..cursor_index] {
            apply_all(&mut world, &entry.receipt.events)
                .map_err(VoxelEditHistoryRejection::ReplayFailed)?;
        }
        Ok(world)
    }

    fn cursor_index_after_transaction(
        &self,
        transaction_id: u64,
    ) -> Result<usize, VoxelEditHistoryRejection> {
        self.entries
            .iter()
            .position(|entry| entry.transaction_id == transaction_id)
            .map(|index| index + 1)
            .ok_or(VoxelEditHistoryRejection::UnknownTransaction { transaction_id })
    }

    fn cursor_at(&self, cursor_index: usize, world_hash: u64) -> VoxelEditHistoryCursor {
        let applied_transaction_id = cursor_index
            .checked_sub(1)
            .and_then(|index| self.entries.get(index))
            .map(|entry| entry.transaction_id);
        VoxelEditHistoryCursor {
            cursor_id: cursor_id_for_index(cursor_index),
            applied_transaction_id,
            index: cursor_index,
            undo_depth: cursor_index,
            redo_depth: self.entries.len().saturating_sub(cursor_index),
            world_hash,
            history_hash: self.history_hash_for_cursor(cursor_index, world_hash),
        }
    }

    fn retained_event_count_for_prefix(&self, prefix_len: usize) -> usize {
        self.entries
            .iter()
            .take(prefix_len)
            .map(|entry| entry.receipt.events.len())
            .sum()
    }

    fn history_hash_for_cursor(&self, cursor_index: usize, world_hash: u64) -> u64 {
        let mut hasher = Fnv1a::new();
        hasher.feed_usize(cursor_index);
        hasher.feed_u64(world_hash);
        for entry in &self.entries {
            hasher.feed_u64(entry.transaction_id);
            hasher.feed_u64(entry.parent_transaction_id.unwrap_or(0));
            hasher.feed_u64(entry.receipt.transaction_hash);
            hasher.feed_u64(entry.receipt.before_hash);
            hasher.feed_u64(entry.receipt.after_hash);
            hasher.feed_usize(entry.receipt.events.len());
        }
        hasher.finish()
    }
}

fn cursor_id_for_index(index: usize) -> u64 {
    0x4849_5354_4355_5253u64 ^ index as u64
}

fn replay_hash(cursor_index: usize, target_hash: u64, entries: &[VoxelEditHistoryEntry]) -> u64 {
    let mut hasher = Fnv1a::new();
    hasher.feed_usize(cursor_index);
    hasher.feed_u64(target_hash);
    for entry in entries.iter().take(cursor_index) {
        hasher.feed_u64(entry.transaction_id);
        hasher.feed_u64(entry.receipt.transaction_hash);
        for event in &entry.receipt.events {
            feed_event(&mut hasher, event);
        }
    }
    hasher.finish()
}

fn feed_event(hasher: &mut Fnv1a, event: &VoxelEditEvent) {
    match *event {
        VoxelEditEvent::VoxelSet { grid, coord, value } => {
            hasher.feed_u8(0);
            hasher.feed_u32(grid.raw());
            hasher.feed_i64(coord.x);
            hasher.feed_i64(coord.y);
            hasher.feed_i64(coord.z);
            hasher.feed_u32(value.to_encoded());
        }
        VoxelEditEvent::VoxelRegionFilled {
            grid,
            min,
            max,
            value,
        } => {
            hasher.feed_u8(1);
            hasher.feed_u32(grid.raw());
            hasher.feed_i64(min.x);
            hasher.feed_i64(min.y);
            hasher.feed_i64(min.z);
            hasher.feed_i64(max.x);
            hasher.feed_i64(max.y);
            hasher.feed_i64(max.z);
            hasher.feed_u32(value.to_encoded());
        }
        VoxelEditEvent::ChunkGenerated {
            grid,
            chunk,
            seed,
            generator_version,
            hash,
        } => {
            hasher.feed_u8(2);
            hasher.feed_u32(grid.raw());
            hasher.feed_i64(chunk.x);
            hasher.feed_i64(chunk.y);
            hasher.feed_i64(chunk.z);
            hasher.feed_u64(seed);
            hasher.feed_u32(generator_version);
            hasher.feed_u64(hash);
        }
    }
}

struct Fnv1a {
    value: u64,
}

impl Fnv1a {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;

    fn new() -> Self {
        Self {
            value: Self::OFFSET,
        }
    }

    fn feed(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.value ^= u64::from(*byte);
            self.value = self.value.wrapping_mul(Self::PRIME);
        }
    }

    fn feed_u8(&mut self, value: u8) {
        self.feed(&value.to_le_bytes());
    }

    fn feed_u32(&mut self, value: u32) {
        self.feed(&value.to_le_bytes());
    }

    fn feed_u64(&mut self, value: u64) {
        self.feed(&value.to_le_bytes());
    }

    fn feed_i64(&mut self, value: i64) {
        self.feed(&value.to_le_bytes());
    }

    fn feed_usize(&mut self, value: usize) {
        self.feed_u64(value as u64);
    }

    fn finish(self) -> u64 {
        self.value
    }
}

#[cfg(test)]
mod tests {
    use core_commands::VoxelCommand;
    use core_space::{ChunkCoord, ChunkDims, GridId, LocalVoxelCoord, VoxelCoord, VoxelGridSpec};
    use core_voxel::{MaterialCatalog, VoxelMaterialId, VoxelValue};
    use svc_spatial::VoxelWorld;
    use svc_volume::VoxelChunk;

    use super::*;
    use crate::{
        execute_transaction, VoxelEditTransaction, VoxelEditTransactionLimits,
        VoxelEditTransactionMode,
    };

    fn spec() -> VoxelGridSpec {
        VoxelGridSpec::new(GridId::new(0), 1.0, ChunkDims::cubic(8).unwrap()).unwrap()
    }

    fn materials() -> MaterialCatalog {
        MaterialCatalog::new([VoxelMaterialId::new(1), VoxelMaterialId::new(2)])
    }

    fn resident_world() -> VoxelWorld {
        let mut world = VoxelWorld::new(spec());
        world.insert(ChunkCoord::new(0, 0, 0), VoxelChunk::from_spec(&spec()));
        world.drain_dirty();
        world
    }

    fn set_command(x: i64, material: u16) -> VoxelCommand {
        VoxelCommand::SetVoxel {
            grid: GridId::new(0),
            coord: VoxelCoord::new(x, 0, 0),
            value: VoxelValue::solid_raw(material),
        }
    }

    fn applied_receipt(
        world: &mut VoxelWorld,
        command: VoxelCommand,
    ) -> VoxelEditTransactionReceipt {
        let receipt = execute_transaction(
            world,
            &materials(),
            &VoxelEditTransaction {
                mode: VoxelEditTransactionMode::Apply,
                commands: &[command],
                limits: VoxelEditTransactionLimits::default(),
            },
        );
        assert!(receipt.applied);
        receipt
    }

    #[test]
    fn append_accepted_transaction_advances_cursor_and_replays_internal_world() {
        let mut external = resident_world();
        let mut history = VoxelEditHistory::new(resident_world());
        let receipt = applied_receipt(&mut external, set_command(1, 1));

        let append = history.append_accepted(receipt).unwrap();

        assert_eq!(append.entry.transaction_id, 1);
        assert_eq!(append.entry.parent_transaction_id, None);
        assert_eq!(append.cursor_before.index, 0);
        assert_eq!(append.cursor_after.index, 1);
        assert_eq!(history.entries().len(), 1);
        assert_eq!(history.current_world_hash(), voxel_world_hash(&external));
        assert_eq!(
            history
                .current_world()
                .get(ChunkCoord::new(0, 0, 0))
                .unwrap()
                .get(LocalVoxelCoord::new(1, 0, 0)),
            Some(VoxelValue::solid_raw(1))
        );
    }

    #[test]
    fn append_after_undo_forks_and_invalidates_redo_tail() {
        let mut external = resident_world();
        let mut history = VoxelEditHistory::new(resident_world());
        let first = applied_receipt(&mut external, set_command(1, 1));
        let second = applied_receipt(&mut external, set_command(2, 1));
        history.append_accepted(first).unwrap();
        history.append_accepted(second).unwrap();

        history.undo_one().unwrap();
        assert_eq!(history.cursor().redo_depth, 1);

        let mut fork_external = history.current_world().clone();
        let fork = applied_receipt(&mut fork_external, set_command(3, 2));
        let append = history.append_accepted(fork).unwrap();

        assert_eq!(append.invalidated_redo_count, 1);
        assert_eq!(history.entries().len(), 2);
        assert_eq!(history.cursor().redo_depth, 0);
        assert_eq!(
            history.current_world_hash(),
            voxel_world_hash(&fork_external)
        );
    }

    #[test]
    fn revert_to_transaction_receipt_uses_replay_without_mutating_preview() {
        let mut external = resident_world();
        let mut history = VoxelEditHistory::new(resident_world());
        let first = history
            .append_accepted(applied_receipt(&mut external, set_command(1, 1)))
            .unwrap()
            .entry
            .transaction_id;
        history
            .append_accepted(applied_receipt(&mut external, set_command(2, 1)))
            .unwrap();
        let current = history.current_world_hash();

        let preview = history.preview_revert_to_transaction(first).unwrap();
        assert!(preview.preview);
        assert!(!preview.applied);
        assert_eq!(preview.cursor_before.index, 2);
        assert_eq!(preview.cursor_after.index, 1);
        assert_eq!(history.current_world_hash(), current);

        let applied = history.apply_revert_to_transaction(first).unwrap();
        assert!(applied.applied);
        assert_eq!(history.cursor().index, 1);
        assert_eq!(
            history.current_world_hash(),
            applied.cursor_after.world_hash
        );
        assert_ne!(history.current_world_hash(), current);
    }

    #[test]
    fn undo_and_redo_move_cursor_over_retained_entries() {
        let mut external = resident_world();
        let mut history = VoxelEditHistory::new(resident_world());
        history
            .append_accepted(applied_receipt(&mut external, set_command(1, 1)))
            .unwrap();
        history
            .append_accepted(applied_receipt(&mut external, set_command(2, 2)))
            .unwrap();
        let after_two = history.current_world_hash();

        let undo = history.undo_one().unwrap();
        assert_eq!(undo.cursor_after.index, 1);
        assert_eq!(history.cursor().redo_depth, 1);
        assert_ne!(history.current_world_hash(), after_two);

        let redo = history.redo_one().unwrap();
        assert_eq!(redo.cursor_after.index, 2);
        assert_eq!(history.cursor().redo_depth, 0);
        assert_eq!(history.current_world_hash(), after_two);
    }

    #[test]
    fn stale_hash_rejects_out_of_order_receipt() {
        let mut external = resident_world();
        let mut history = VoxelEditHistory::new(resident_world());
        let first = applied_receipt(&mut external, set_command(1, 1));
        let stale_second = applied_receipt(&mut external, set_command(2, 1));

        assert!(matches!(
            history.append_accepted(stale_second),
            Err(VoxelEditHistoryRejection::StaleCursorHash { .. })
        ));

        history.append_accepted(first).unwrap();
        assert_eq!(history.entries().len(), 1);
    }

    #[test]
    fn rejects_unapplied_or_rejected_transaction_receipts() {
        let mut world = resident_world();
        let mut history = VoxelEditHistory::new(resident_world());
        let preview = execute_transaction(
            &mut world,
            &materials(),
            &VoxelEditTransaction::preview(&[set_command(1, 1)]),
        );
        assert!(matches!(
            history.append_accepted(preview),
            Err(VoxelEditHistoryRejection::ReceiptWasNotApplied)
        ));

        let rejected = execute_transaction(
            &mut world,
            &materials(),
            &VoxelEditTransaction::apply(&[set_command(1, 9)]),
        );
        assert!(matches!(
            history.append_accepted(rejected),
            Err(VoxelEditHistoryRejection::ReceiptWasNotApplied)
                | Err(VoxelEditHistoryRejection::ReceiptHadRejections { .. })
        ));
    }

    #[test]
    fn large_history_and_replay_quotas_fail_closed() {
        let mut external = resident_world();
        let mut history =
            VoxelEditHistory::with_limits(resident_world(), VoxelEditHistoryLimits::new(1, 10, 0));
        let first = applied_receipt(&mut external, set_command(1, 1));
        let second = applied_receipt(&mut external, set_command(2, 1));
        history.append_accepted(first).unwrap();

        assert!(matches!(
            history.append_accepted(second),
            Err(VoxelEditHistoryRejection::EntryQuotaExceeded {
                limit: 1,
                actual: 2
            })
        ));
        assert!(matches!(
            history.preview_revert_to_cursor(1),
            Err(VoxelEditHistoryRejection::ReplayQuotaExceeded {
                limit: 0,
                actual: 1
            })
        ));
    }
}
