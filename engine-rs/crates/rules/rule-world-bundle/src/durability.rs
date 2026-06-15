//! Save/reload/replay **durability evidence** for a canonical voxel edit sequence
//! (launchable-voxel-09 / task #2440).
//!
//! This composes the existing persistence primitives ([`replay_edit_log`],
//! [`compact_voxel_save`], [`reconstruct`]) into three committed checkpoints that
//! prove a canonical edited world survives a save → compaction → reload cycle with an
//! identical content fingerprint:
//!
//! - **post-load** — the base fixture replayed (generation only, before user edits);
//! - **post-edit** — after the canonical edit sequence is applied on top;
//! - **post-reload** — after the full log is compacted to chunk snapshots + a retained
//!   edit tail and then reconstructed.
//!
//! Durability holds iff `post_edit == post_reload`. A mismatch (e.g. a tampered
//! snapshot or edit log) fails **closed** with a classified [`DurabilityError`] rather
//! than silently loading a divergent world.
//!
//! # Deferred debt: voxel durability vs. the generic `ReplayRecord`
//!
//! This is a *dedicated* voxel save/reload fingerprint path, deliberately parallel to
//! the engine's generic `protocol-replay` / `sim-replay` `ReplayRecord` (which records
//! tick-stepped input/checkpoint streams). Unifying the two — so a voxel edit sequence
//! is just another replay stream — is recorded as deferred debt for the first
//! launchable loop (Den task #2440); see `docs/replay-model.md`. The fingerprint here
//! is the same FNV-1a [`BundleHash`] world fingerprint the regenerate-and-replay
//! diagnostic already uses, so the two paths stay comparable when they are unified.

use core_events::VoxelEditEvent;
use core_space::VoxelGridSpec;
use rule_voxel_edit::persist::replay_edit_log;
use rule_voxel_edit::VoxelEditRejection;
use svc_serialization::BundleHash;
use svc_spatial::VoxelWorld;

use crate::compose::{compact_voxel_save, reconstruct};

/// Deterministic content fingerprint over a voxel world's resident chunks: the FNV-1a
/// [`BundleHash`] of each resident `(chunk coord, chunk content hash)` in chunk order.
/// Stable across runs and platforms; legible in diagnostics. This is the single world
/// fingerprint shared by the durability checkpoints and the regenerate-and-replay
/// staging hash, so the two paths remain directly comparable.
pub fn world_fingerprint(world: &VoxelWorld) -> BundleHash {
    let mut rows: Vec<(i64, i64, i64, u64)> = world
        .resident_chunks()
        .map(|(c, chunk)| (c.x, c.y, c.z, chunk.content_hash().0))
        .collect();
    rows.sort_unstable();
    let mut s = String::new();
    for (x, y, z, h) in rows {
        s.push_str(&format!("{x},{y},{z}:{h}\n"));
    }
    BundleHash::of_str(&s)
}

/// The three durability checkpoints for a canonical edit sequence, plus the compaction
/// shape that produced the reload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DurabilityEvidence {
    /// Fingerprint after replaying the base/generation prefix (the loaded fixture).
    pub post_load: BundleHash,
    /// Fingerprint after applying the canonical edit sequence on top of the base.
    pub post_edit: BundleHash,
    /// Fingerprint after compaction + reconstruction (the reloaded world).
    pub post_reload: BundleHash,
    /// Edit events folded into chunk snapshots by compaction.
    pub compacted_edits: u32,
    /// Edit events retained as a replayed tail after compaction.
    pub retained_edits: u32,
}

impl DurabilityEvidence {
    /// Durability holds iff a compacted reload reproduces the post-edit world exactly.
    pub fn is_durable(&self) -> bool {
        self.post_edit == self.post_reload
    }
}

/// A classified failure while building/verifying durability evidence — always
/// fail-closed (no divergent world is ever returned as if it loaded cleanly).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DurabilityError {
    /// Replaying the base log, the full log, or the retained tail was rejected
    /// (e.g. a stale generation hash → [`VoxelEditRejection::GenerationDivergence`]).
    Replay(VoxelEditRejection),
    /// The reloaded world fingerprint disagreed with the post-edit world — a tampered
    /// or corrupt save. Surfaced instead of loading the divergent world.
    ReloadDivergence {
        post_edit: BundleHash,
        post_reload: BundleHash,
    },
}

impl core::fmt::Display for DurabilityError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            DurabilityError::Replay(r) => write!(f, "durability replay rejected: {r:?}"),
            DurabilityError::ReloadDivergence {
                post_edit,
                post_reload,
            } => write!(
                f,
                "reload diverged: post_edit={} post_reload={}",
                post_edit.to_hex(),
                post_reload.to_hex()
            ),
        }
    }
}

impl std::error::Error for DurabilityError {}

/// Build durability evidence for a canonical sequence.
///
/// `base_events` is the generation/load prefix (replayed to produce the loaded
/// fixture); `edit_events` is the user edit sequence applied on top. `retain_recent`
/// controls compaction (trailing edits kept as a replayed tail). Returns
/// [`DurabilityError::ReloadDivergence`] if the compacted reload does not reproduce the
/// post-edit world — the durability guarantee, enforced rather than assumed.
pub fn build_durability_evidence(
    spec: VoxelGridSpec,
    base_events: &[VoxelEditEvent],
    edit_events: &[VoxelEditEvent],
    retain_recent: usize,
) -> Result<DurabilityEvidence, DurabilityError> {
    let loaded = replay_edit_log(spec, base_events).map_err(DurabilityError::Replay)?;
    let post_load = world_fingerprint(&loaded);

    let mut full: Vec<VoxelEditEvent> = base_events.to_vec();
    full.extend_from_slice(edit_events);
    let edited = replay_edit_log(spec, &full).map_err(DurabilityError::Replay)?;
    let post_edit = world_fingerprint(&edited);

    let save = compact_voxel_save(spec, &full, retain_recent).map_err(DurabilityError::Replay)?;
    let reloaded = reconstruct(spec, &save).map_err(DurabilityError::Replay)?;
    let post_reload = world_fingerprint(&reloaded);

    if post_edit != post_reload {
        return Err(DurabilityError::ReloadDivergence {
            post_edit,
            post_reload,
        });
    }

    Ok(DurabilityEvidence {
        post_load,
        post_edit,
        post_reload,
        compacted_edits: save.compacted_edits,
        retained_edits: save.retained_edits.len() as u32,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compose::{ChunkSnapshotArtifact, CompactedVoxelSave};
    use core_space::{ChunkCoord, ChunkDims, GridId, VoxelCoord};
    use core_voxel::VoxelValue;
    use rule_voxel_edit::generate_chunk;
    use rule_voxel_edit::persist::{
        decode_edit_log, encode_chunk_snapshot, encode_edit_log, SnapshotError,
    };
    use svc_volume::VoxelChunk;

    fn spec() -> VoxelGridSpec {
        VoxelGridSpec::new(GridId::new(0), 1.0, ChunkDims::cubic(4).unwrap()).unwrap()
    }

    fn base() -> Vec<VoxelEditEvent> {
        let chunk = ChunkCoord::new(0, 0, 0);
        let gen = generate_chunk(&spec(), chunk, 7, 1);
        vec![VoxelEditEvent::ChunkGenerated {
            grid: GridId::new(0),
            chunk,
            seed: 7,
            generator_version: 1,
            hash: gen.content_hash().0,
        }]
    }

    fn edits() -> Vec<VoxelEditEvent> {
        let g = GridId::new(0);
        vec![
            VoxelEditEvent::VoxelSet {
                grid: g,
                coord: VoxelCoord::new(0, 3, 0),
                value: VoxelValue::solid_raw(2),
            },
            VoxelEditEvent::VoxelSet {
                grid: g,
                coord: VoxelCoord::new(1, 3, 0),
                value: VoxelValue::solid_raw(3),
            },
        ]
    }

    #[test]
    fn canonical_sequence_is_durable_and_load_differs_from_edit() {
        let ev = build_durability_evidence(spec(), &base(), &edits(), 1).expect("evidence");
        assert!(ev.is_durable(), "post_edit must equal post_reload");
        assert_ne!(
            ev.post_load, ev.post_edit,
            "the edit sequence must actually change the world"
        );
        assert_eq!(ev.compacted_edits, 1);
        assert_eq!(ev.retained_edits, 1);
    }

    #[test]
    fn fingerprint_is_deterministic() {
        let a = build_durability_evidence(spec(), &base(), &edits(), 1).unwrap();
        let b = build_durability_evidence(spec(), &base(), &edits(), 1).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn stale_generation_hash_fails_closed_classified() {
        // A base whose recorded generation hash is stale (old generator) is rejected on
        // replay — a classified GenerationDivergence, surfaced as a Replay error.
        let stale = vec![VoxelEditEvent::ChunkGenerated {
            grid: GridId::new(0),
            chunk: ChunkCoord::new(0, 0, 0),
            seed: 7,
            generator_version: 1,
            hash: 0xdead_beef,
        }];
        let err = build_durability_evidence(spec(), &stale, &edits(), 1).unwrap_err();
        assert!(matches!(
            err,
            DurabilityError::Replay(VoxelEditRejection::GenerationDivergence { .. })
        ));
    }

    #[test]
    fn tampered_edit_log_text_fails_closed_classified() {
        // A durable edit log artifact that has been corrupted on disk fails to decode
        // with a classified SnapshotError — never a silent partial load.
        let mut full = base();
        full.extend(edits());
        let text = encode_edit_log(&full);
        let tampered = text.replace("set 0 0 3", "set 0 notanumber 3");
        assert!(matches!(
            decode_edit_log(&tampered),
            Err(SnapshotError::BadToken { .. })
        ));
    }

    #[test]
    fn tampered_snapshot_reload_is_caught_as_divergence() {
        // Tamper a compacted snapshot's bytes: reconstruct yields a different world, and
        // the durability fingerprint comparison catches it (fail closed) rather than
        // accepting the corrupted reload.
        let mut full = base();
        full.extend(edits());
        let mut save = crate::compose::compact_voxel_save(spec(), &full, 1).expect("compact");
        let post_edit = {
            let edited = replay_edit_log(spec(), &full).unwrap();
            world_fingerprint(&edited)
        };

        // Replace the snapshot's bytes with a valid-but-wrong (all-empty) chunk for the
        // same coord — a plausible on-disk corruption that still decodes.
        let snap = &save.snapshots[0];
        let empty_text = encode_chunk_snapshot(&VoxelChunk::from_spec(&spec()));
        save = CompactedVoxelSave {
            snapshots: vec![ChunkSnapshotArtifact {
                chunk: snap.chunk,
                path: snap.path.clone(),
                text: empty_text,
            }],
            ..save
        };

        // The reloaded fingerprint must differ from the authoritative post-edit one —
        // the durability check refuses to accept the corrupted reload as equivalent.
        let reloaded = reconstruct(spec(), &save).expect("decodes, but wrong content");
        assert_ne!(
            world_fingerprint(&reloaded),
            post_edit,
            "a tampered snapshot must not reproduce the authoritative world"
        );
    }
}
