//! World-bundle save & compaction plan model (scene-capability-02, subtask #2320).
//!
//! A [`SavePlan`] declares **what a save will write** and **what it compacts**,
//! without performing any voxel work itself — the actual snapshot/edit-log
//! composition and reconstruction lives in the `rule-world-bundle` crate (which
//! can reach the `rule-voxel-edit` persistence layer). This keeps the plan a
//! pure, inspectable description usable below the rules layer.
//!
//! Compaction is **explicit**: a save may fold old edit history into current
//! snapshots, but ordinary simulation ticks must not silently compact. Save and
//! replay stay separate concepts — replay records are their own classified
//! artifacts and are never required to load current authority.

use crate::artifact::{ArtifactClass, ArtifactEntry};

/// How save-time compaction folds edit history into snapshots.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompactionPlan {
    /// Edits folded into the written snapshots (dropped from the retained log).
    pub compacted_edits: u32,
    /// Edits kept after the latest snapshot (replayed on next load).
    pub retained_edits: u32,
    /// Chunk labels whose snapshots absorb the compacted edits, in stable order.
    pub snapshot_chunks: Vec<String>,
}

impl CompactionPlan {
    /// A no-op compaction (full edit log retained, no snapshots folded).
    pub fn none(retained_edits: u32) -> Self {
        CompactionPlan {
            compacted_edits: 0,
            retained_edits,
            snapshot_chunks: Vec::new(),
        }
    }

    /// Whether this plan actually compacts anything.
    pub fn compacts(&self) -> bool {
        self.compacted_edits > 0
    }
}

/// A declarative save plan: the artifacts a save will write plus its compaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavePlan {
    /// Artifacts to write, classified durable/generated/cache. Canonicalized
    /// (sorted by path) by [`SavePlan::new`].
    pub writes: Vec<ArtifactEntry>,
    pub compaction: CompactionPlan,
}

impl SavePlan {
    /// Build a save plan, sorting writes by path for determinism.
    pub fn new(mut writes: Vec<ArtifactEntry>, compaction: CompactionPlan) -> Self {
        writes.sort_by(|a, b| a.path.cmp(&b.path));
        SavePlan { writes, compaction }
    }

    /// Count of writes in a given class.
    pub fn count(&self, class: ArtifactClass) -> usize {
        self.writes.iter().filter(|a| a.class == class).count()
    }

    /// The durable writes only — the minimum a load needs (cache excluded).
    pub fn durable_writes(&self) -> impl Iterator<Item = &ArtifactEntry> {
        self.writes
            .iter()
            .filter(|a| a.class == ArtifactClass::Durable)
    }

    /// A human/agent-legible, deterministic explanation of the save.
    pub fn describe(&self) -> String {
        use core::fmt::Write;
        let mut s = String::new();
        let _ = writeln!(
            s,
            "writes: {} durable, {} generated, {} cache",
            self.count(ArtifactClass::Durable),
            self.count(ArtifactClass::Generated),
            self.count(ArtifactClass::Cache),
        );
        if self.compaction.compacts() {
            let _ = writeln!(
                s,
                "compaction: fold {} edits into {} chunk snapshot(s) [{}], retain {} recent edit(s)",
                self.compaction.compacted_edits,
                self.compaction.snapshot_chunks.len(),
                self.compaction.snapshot_chunks.join(","),
                self.compaction.retained_edits,
            );
        } else {
            let _ = writeln!(
                s,
                "compaction: none, retain {} edit(s)",
                self.compaction.retained_edits
            );
        }
        for a in &self.writes {
            let _ = writeln!(s, "  {} [{}] {}", a.path, a.class.tag(), a.role.tag());
        }
        s
    }
}
