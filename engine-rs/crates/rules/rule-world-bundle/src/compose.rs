//! Voxel save composition + explicit compaction (subtask #2320).
//!
//! Composes the existing `rule-voxel-edit` chunk-snapshot / edit-log persistence
//! into world-bundle voxel sections and declares the save via a
//! `svc-serialization` [`SavePlan`]. Compaction is **explicit and save-time**:
//! [`compact_voxel_save`] folds the older edit history into chunk snapshots and
//! retains only the recent edits, and [`reconstruct`] proves a compacted snapshot
//! plus the retained edit log reconstructs the exact same chunk hashes as a full
//! replay. Ordinary simulation ticks never call this — compaction is a deliberate
//! save operation.

use core_events::VoxelEditEvent;
use core_space::{ChunkCoord, VoxelGridSpec};
use rule_voxel_edit::persist::{
    decode_chunk_snapshot, encode_chunk_snapshot, encode_edit_log, replay_edit_log,
};
use rule_voxel_edit::{apply_all, VoxelEditRejection};
use svc_serialization::{ArtifactEntry, ArtifactRole, CompactionPlan, SavePlan};
use svc_spatial::VoxelWorld;

/// One compacted chunk snapshot artifact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkSnapshotArtifact {
    pub chunk: ChunkCoord,
    /// Bundle-relative path (`voxel/chunk_x_y_z.snapshot`).
    pub path: String,
    /// The encoded snapshot text (reconstructs the chunk's `content_hash`).
    pub text: String,
}

/// The result of compacting a voxel world save: snapshots that absorb the folded
/// edits, plus the retained recent edit log.
#[derive(Debug, Clone, PartialEq)]
pub struct CompactedVoxelSave {
    pub snapshots: Vec<ChunkSnapshotArtifact>,
    /// Edits retained after the compaction point (replayed on load).
    pub retained_edits: Vec<VoxelEditEvent>,
    /// The encoded retained edit log text.
    pub retained_log_text: String,
    /// Count of edit events folded into the snapshots.
    pub compacted_edits: u32,
}

/// Compact a full edit log into chunk snapshots plus a retained recent-edit tail.
///
/// `retain_recent` is the number of trailing edit events kept in the edit log; all
/// earlier events (including the `ChunkGenerated` base) are folded by replaying
/// them and snapshotting the resulting resident chunks. The fold point is clamped
/// to the log length.
pub fn compact_voxel_save(
    spec: VoxelGridSpec,
    full_log: &[VoxelEditEvent],
    retain_recent: usize,
) -> Result<CompactedVoxelSave, VoxelEditRejection> {
    let retain = retain_recent.min(full_log.len());
    let split = full_log.len() - retain;
    let (prefix, retained) = full_log.split_at(split);

    // Replay the folded prefix and snapshot every resident chunk, in chunk order.
    let folded = replay_edit_log(spec, prefix)?;
    let mut residents: Vec<(ChunkCoord, String)> = folded
        .resident_chunks()
        .map(|(coord, chunk)| (coord, encode_chunk_snapshot(chunk)))
        .collect();
    residents.sort_by_key(|(c, _)| (c.x, c.y, c.z));

    let snapshots = residents
        .into_iter()
        .map(|(chunk, text)| ChunkSnapshotArtifact {
            path: format!("voxel/chunk_{}_{}_{}.snapshot", chunk.x, chunk.y, chunk.z),
            chunk,
            text,
        })
        .collect();

    // The folded prefix's *edit* events (non-generation) are what compaction removes.
    let compacted_edits = prefix
        .iter()
        .filter(|e| !matches!(e, VoxelEditEvent::ChunkGenerated { .. }))
        .count() as u32;

    Ok(CompactedVoxelSave {
        snapshots,
        retained_edits: retained.to_vec(),
        retained_log_text: encode_edit_log(retained),
        compacted_edits,
    })
}

/// Reconstruct a world from a compacted save: load each chunk snapshot, then
/// replay the retained edits on top. Produces the same chunk content hashes as a
/// full replay of the original log.
pub fn reconstruct(
    spec: VoxelGridSpec,
    save: &CompactedVoxelSave,
) -> Result<VoxelWorld, VoxelEditRejection> {
    let mut world = VoxelWorld::new(spec);
    for snap in &save.snapshots {
        // The snapshot decode is infallible for our own encoder output; a malformed
        // snapshot would surface as a reconstruction mismatch in the caller's hash
        // check rather than corrupt authority silently.
        if let Ok(chunk) = decode_chunk_snapshot(&snap.text) {
            world.insert(snap.chunk, chunk);
        }
    }
    apply_all(&mut world, &save.retained_edits)?;
    Ok(world)
}

/// Build a declarative [`SavePlan`] for a compacted voxel save. Snapshots are
/// classified `generated` (reproducible) and the retained edit log `durable`. A
/// disposable cache artifact may be appended by the caller; it never affects load.
pub fn voxel_save_plan(save: &CompactedVoxelSave) -> SavePlan {
    let mut writes: Vec<ArtifactEntry> = save
        .snapshots
        .iter()
        .map(|s| {
            ArtifactEntry::generated(
                s.path.clone(),
                ArtifactRole::VoxelChunkSnapshot,
                s.text.as_bytes(),
            )
        })
        .collect();
    writes.push(ArtifactEntry::durable(
        "voxel/recent.log",
        ArtifactRole::VoxelEditLog,
        save.retained_log_text.as_bytes(),
    ));

    let snapshot_chunks = save
        .snapshots
        .iter()
        .map(|s| format!("{},{},{}", s.chunk.x, s.chunk.y, s.chunk.z))
        .collect();

    SavePlan::new(
        writes,
        CompactionPlan {
            compacted_edits: save.compacted_edits,
            retained_edits: save.retained_edits.len() as u32,
            snapshot_chunks,
        },
    )
}
