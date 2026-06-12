//! Deterministic chunk work scheduler (voxel-capability-13).
//!
//! # Lane
//!
//! `rust-rule` — the integration-risk seam where chunk generation, meshing,
//! collision-projection rebuilds, and render upload meet over time. It is
//! **abstract over execution**: it owns the *ordered, budgeted, version-checked
//! queues* keyed by `(ChunkCoord, WorkKind)`; the caller (app/sim runner) drains
//! work, runs it (possibly in parallel), and applies results. It does not call the
//! mesher/generator/collision crates itself.
//!
//! # Determinism invariants
//!
//! - **Deterministic priority order**: drained/queued items sort by
//!   `(priority, kind, chunk)` — lower `priority` value = more urgent (e.g. nearer
//!   the camera). Ties never depend on insertion order or hashing.
//! - **Deterministic result application independent of completion order**: drained
//!   items are totally ordered ([`WorkItem`] is `Ord`), so a caller that runs them
//!   on `rayon` can sort results before applying and get the same authoritative
//!   outcome regardless of which worker finished first.
//! - **Stale work is version-checked**: each item carries the chunk version at
//!   enqueue time; an edit/unload bumps the chunk's current version, and any older
//!   result is [`stale`](ChunkScheduler::is_stale) and must be dropped.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;

use core_space::ChunkCoord;

/// The kind of work scheduled for a chunk. The variant order is the deterministic
/// tie-break within a priority (generation precedes meshing precedes collision
/// precedes upload — a natural data→derived→render dependency order).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WorkKind {
    Generate,
    Mesh,
    CollisionRebuild,
    Upload,
}

impl WorkKind {
    /// A stable label for queue diagnostics / failure routing by lane.
    pub fn label(self) -> &'static str {
        match self {
            WorkKind::Generate => "generate",
            WorkKind::Mesh => "mesh",
            WorkKind::CollisionRebuild => "collision-rebuild",
            WorkKind::Upload => "upload",
        }
    }
}

/// One unit of scheduled work. Ordering is `(priority, kind, chunk)` so a batch of
/// completed items can be sorted into a deterministic apply order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkItem {
    /// Lower = more urgent (distance/visibility/dirtiness, decided by the caller).
    pub priority: i64,
    pub kind: WorkKind,
    pub chunk: ChunkCoord,
    /// The chunk version this work was enqueued against (for staleness checks).
    pub version: u64,
}

impl WorkItem {
    fn order_key(&self) -> (i64, WorkKind, [i64; 3]) {
        (self.priority, self.kind, self.chunk.to_array())
    }
}

impl PartialOrd for WorkItem {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for WorkItem {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.order_key().cmp(&other.order_key())
    }
}

/// Whether a drained result should be accepted or dropped as stale.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResultStatus {
    Accepted,
    /// A newer edit/unload happened since this work was scheduled.
    Stale,
}

#[derive(Debug, Clone, Copy)]
struct Entry {
    priority: i64,
    version: u64,
}

/// A deterministic, budgeted, version-checked scheduler of per-chunk work.
#[derive(Debug, Default)]
pub struct ChunkScheduler {
    /// Pending work, deduplicated by `(chunk, kind)`.
    queue: BTreeMap<(ChunkCoord, WorkKind), Entry>,
    /// Latest known version per chunk (bumped by edits/unloads) for staleness.
    current_version: BTreeMap<ChunkCoord, u64>,
}

impl ChunkScheduler {
    pub fn new() -> Self {
        Self::default()
    }

    /// Schedule `kind` work for `chunk` at `priority`, against chunk `version`.
    /// Deduplicated: re-enqueuing keeps the most urgent priority and the latest
    /// version. Also records `version` as the chunk's current version.
    pub fn enqueue(&mut self, chunk: ChunkCoord, kind: WorkKind, priority: i64, version: u64) {
        self.note_version(chunk, version);
        self.queue
            .entry((chunk, kind))
            .and_modify(|e| {
                e.priority = e.priority.min(priority);
                e.version = e.version.max(version);
            })
            .or_insert(Entry { priority, version });
    }

    /// Convenience policy for an edit: a changed chunk needs a remesh **and** a
    /// collision-projection rebuild (collision rebuild after relevant edits).
    pub fn on_chunk_edited(&mut self, chunk: ChunkCoord, version: u64, priority: i64) {
        self.enqueue(chunk, WorkKind::Mesh, priority, version);
        self.enqueue(chunk, WorkKind::CollisionRebuild, priority, version);
    }

    /// Record a chunk's current version (e.g. after an edit or unload) without
    /// scheduling work — so already-queued/in-flight older work becomes stale.
    pub fn note_version(&mut self, chunk: ChunkCoord, version: u64) {
        let v = self.current_version.entry(chunk).or_insert(version);
        *v = (*v).max(version);
    }

    /// Drain up to `budget` items in deterministic priority order and remove them
    /// from the queue. The remainder stays queued for the next [`step`](Self::step)
    /// (budget exhaustion/resume). Returns the drained items (already sorted, so a
    /// caller may run them in any order and re-sort results for a deterministic apply).
    pub fn step(&mut self, budget: usize) -> Vec<WorkItem> {
        let mut items: Vec<WorkItem> = self
            .queue
            .iter()
            .map(|(&(chunk, kind), e)| WorkItem {
                priority: e.priority,
                kind,
                chunk,
                version: e.version,
            })
            .collect();
        items.sort_unstable();
        items.truncate(budget);
        for it in &items {
            self.queue.remove(&(it.chunk, it.kind));
        }
        items
    }

    /// Whether `item`'s result is stale — a newer version of its chunk exists than
    /// the one the work was scheduled against. Drop stale results rather than
    /// applying them.
    pub fn is_stale(&self, item: &WorkItem) -> bool {
        self.current_version
            .get(&item.chunk)
            .is_some_and(|&v| v > item.version)
    }

    /// Classify a drained result for application.
    pub fn classify(&self, item: &WorkItem) -> ResultStatus {
        if self.is_stale(item) {
            ResultStatus::Stale
        } else {
            ResultStatus::Accepted
        }
    }

    // ── diagnostics ────────────────────────────────────────────────────────────

    /// Total queued items.
    pub fn pending_len(&self) -> usize {
        self.queue.len()
    }

    /// Whether `(chunk, kind)` work is queued.
    pub fn is_queued(&self, chunk: ChunkCoord, kind: WorkKind) -> bool {
        self.queue.contains_key(&(chunk, kind))
    }

    /// Count of queued items of `kind` (queue diagnostics by lane).
    pub fn pending_of(&self, kind: WorkKind) -> usize {
        self.queue.keys().filter(|(_, k)| *k == kind).count()
    }

    /// A deterministic snapshot of the queue, in apply order — for devtools/diagnostics.
    pub fn diagnostics(&self) -> Vec<WorkItem> {
        let mut items: Vec<WorkItem> = self
            .queue
            .iter()
            .map(|(&(chunk, kind), e)| WorkItem {
                priority: e.priority,
                kind,
                chunk,
                version: e.version,
            })
            .collect();
        items.sort_unstable();
        items
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cc(x: i64, y: i64, z: i64) -> ChunkCoord {
        ChunkCoord::new(x, y, z)
    }

    #[test]
    fn drains_in_deterministic_priority_order() {
        let mut s = ChunkScheduler::new();
        s.enqueue(cc(2, 0, 0), WorkKind::Mesh, 10, 1);
        s.enqueue(cc(0, 0, 0), WorkKind::Generate, 10, 1);
        s.enqueue(cc(1, 0, 0), WorkKind::Mesh, 5, 1); // most urgent
        s.enqueue(cc(0, 0, 0), WorkKind::Mesh, 10, 1);

        let drained = s.step(10);
        assert_eq!(
            drained
                .iter()
                .map(|i| (i.priority, i.kind, i.chunk))
                .collect::<Vec<_>>(),
            vec![
                (5, WorkKind::Mesh, cc(1, 0, 0)),      // lowest priority value first
                (10, WorkKind::Generate, cc(0, 0, 0)), // then kind order (Generate < Mesh)
                (10, WorkKind::Mesh, cc(0, 0, 0)),     // then chunk coord
                (10, WorkKind::Mesh, cc(2, 0, 0)),
            ],
        );
        assert_eq!(s.pending_len(), 0);
    }

    #[test]
    fn enqueue_dedups_keeping_most_urgent_priority_and_latest_version() {
        let mut s = ChunkScheduler::new();
        s.enqueue(cc(0, 0, 0), WorkKind::Mesh, 10, 1);
        s.enqueue(cc(0, 0, 0), WorkKind::Mesh, 3, 2); // more urgent, newer version
        assert_eq!(s.pending_len(), 1);
        let item = &s.diagnostics()[0];
        assert_eq!(item.priority, 3);
        assert_eq!(item.version, 2);
    }

    #[test]
    fn budget_exhaustion_then_resume_reaches_the_same_set() {
        let mut s = ChunkScheduler::new();
        for x in 0..5 {
            s.enqueue(cc(x, 0, 0), WorkKind::Generate, 10, 1);
        }
        let first = s.step(2);
        assert_eq!(first.len(), 2);
        assert_eq!(s.pending_len(), 3);
        let rest = s.step(10);
        assert_eq!(rest.len(), 3);
        let mut all: Vec<_> = first.iter().chain(&rest).map(|i| i.chunk).collect();
        all.sort();
        assert_eq!(all, (0..5).map(|x| cc(x, 0, 0)).collect::<Vec<_>>());
    }

    #[test]
    fn result_is_stale_after_a_newer_edit() {
        let mut s = ChunkScheduler::new();
        s.enqueue(cc(0, 0, 0), WorkKind::Mesh, 10, 1);
        let drained = s.step(10);
        let item = drained[0];
        assert_eq!(s.classify(&item), ResultStatus::Accepted);
        s.note_version(cc(0, 0, 0), 2); // an edit bumps the version
        assert_eq!(s.classify(&item), ResultStatus::Stale);
    }

    #[test]
    fn editing_a_chunk_schedules_mesh_and_collision_rebuild() {
        let mut s = ChunkScheduler::new();
        s.on_chunk_edited(cc(1, 0, 0), 5, 0);
        assert!(s.is_queued(cc(1, 0, 0), WorkKind::Mesh));
        assert!(s.is_queued(cc(1, 0, 0), WorkKind::CollisionRebuild));
        assert_eq!(s.pending_of(WorkKind::CollisionRebuild), 1);
    }

    #[test]
    fn apply_order_is_independent_of_completion_order() {
        let mut s = ChunkScheduler::new();
        s.enqueue(cc(0, 0, 0), WorkKind::Mesh, 5, 1);
        s.enqueue(cc(1, 0, 0), WorkKind::Mesh, 1, 1);
        let drained = s.step(10);
        // Workers finish in reverse order; re-sorting reproduces the apply order.
        let mut completed: Vec<WorkItem> = drained.iter().copied().rev().collect();
        completed.sort_unstable();
        assert_eq!(completed, drained);
    }

    #[test]
    fn integration_scenario_edit_pan_generate_query() {
        // Player edits chunk A while the camera pans, 5 chunks enter the generate
        // queue, and a terrain/collision query runs (reads only — no scheduling).
        let mut s = ChunkScheduler::new();
        let a = cc(0, 0, 0);
        // The edit: high urgency (priority 0) remesh + collision rebuild for A.
        s.on_chunk_edited(a, 1, 0);
        // Camera pan brings 5 distant chunks into the generate queue at low urgency.
        for x in 1..=5 {
            s.enqueue(cc(x, 0, 10), WorkKind::Generate, 100, 1);
        }
        // (The terrain/collision query is a read; it schedules nothing.)
        assert_eq!(s.pending_len(), 7);

        // With a tight budget, edit-relevant work must NOT starve behind bulk
        // generation: A's mesh + collision come first.
        let batch = s.step(2);
        assert_eq!(
            batch.iter().map(|i| (i.chunk, i.kind)).collect::<Vec<_>>(),
            vec![(a, WorkKind::Mesh), (a, WorkKind::CollisionRebuild)],
        );

        // A second edit to A arrives before that batch is applied → those drained
        // results are now stale and must be dropped.
        s.on_chunk_edited(a, 2, 0); // bumps A's version to 2
        assert_eq!(s.classify(&batch[0]), ResultStatus::Stale);

        // Bulk generation still drains deterministically afterwards.
        let rest = s.step(100);
        let generated: Vec<_> = rest
            .iter()
            .filter(|i| i.kind == WorkKind::Generate)
            .map(|i| i.chunk)
            .collect();
        assert_eq!(generated, (1..=5).map(|x| cc(x, 0, 10)).collect::<Vec<_>>());
    }
}
