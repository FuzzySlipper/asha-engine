//! Generator-mismatch handling: fail-closed by default, plus an explicit
//! development regenerate-and-replay diagnostic (subtask #2321).
//!
//! When a save's terrain generator version/params differ from the current build,
//! the **default** load posture fails closed ([`GeneratorPolicy::FailClosed`]) —
//! authority is never loaded against terrain it was not authored over. Development
//! tooling may opt into [`GeneratorPolicy::RegenerateAndReplay`]: regenerate
//! terrain at the new version in a staging world, replay the saved edit log, and
//! report every edit whose authored context changed (coordinate, old/new
//! generated value, edit event id, suggested action). This is a **diagnostic** —
//! it never silently rewrites the save.

use core_events::VoxelEditEvent;
use core_space::{VoxelCoord, VoxelGridSpec};
use core_voxel::VoxelValue;
use rule_voxel_edit::{apply_all, generate_chunk, VoxelEditRejection};
use svc_serialization::BundleHash;
use svc_spatial::VoxelWorld;

/// How a generator version mismatch is handled at load.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GeneratorPolicy {
    /// Production/default: any mismatch is a hard load failure.
    #[default]
    FailClosed,
    /// Development/tooling: run the regenerate-and-replay diagnostic instead.
    RegenerateAndReplay,
}

/// A fail-closed generator version mismatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GeneratorMismatch {
    pub saved_version: u32,
    pub current_version: u32,
}

impl core::fmt::Display for GeneratorMismatch {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "generator version mismatch: save={} current={} (fail closed)",
            self.saved_version, self.current_version
        )
    }
}

impl std::error::Error for GeneratorMismatch {}

/// Apply the generator policy. Under [`GeneratorPolicy::FailClosed`], a version
/// mismatch is an error; matching versions always load. Under
/// [`GeneratorPolicy::RegenerateAndReplay`], a mismatch is permitted (the caller
/// then runs [`regenerate_and_replay`]); the returned bool is whether a
/// regenerate-and-replay diagnostic is warranted.
pub fn check_generator(
    saved_version: u32,
    current_version: u32,
    policy: GeneratorPolicy,
) -> Result<bool, GeneratorMismatch> {
    if saved_version == current_version {
        return Ok(false);
    }
    match policy {
        GeneratorPolicy::FailClosed => Err(GeneratorMismatch {
            saved_version,
            current_version,
        }),
        GeneratorPolicy::RegenerateAndReplay => Ok(true),
    }
}

/// What a developer might do about a conflicting edit. Structured for future
/// protocol diagnostics/tooling rather than free text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuggestedAction {
    /// New terrain matches the edit's intent; keep the edit as-is.
    KeepEdit,
    /// The edit now lands on different generated material/state — review whether
    /// to reapply, drop, or pin the old generator.
    ReviewConflict,
}

impl SuggestedAction {
    pub fn label(self) -> &'static str {
        match self {
            SuggestedAction::KeepEdit => "keepEdit",
            SuggestedAction::ReviewConflict => "reviewConflict",
        }
    }
}

/// One edit whose authored generated context changed under the new generator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EditConflict {
    /// Ordinal of the edit in the replayed log (the edit event id).
    pub event_id: u64,
    pub coord: VoxelCoord,
    /// Generated value at `coord` under the saved generator.
    pub old_generated: VoxelValue,
    /// Generated value at `coord` under the new generator.
    pub new_generated: VoxelValue,
    /// The value the edit writes (the authored delta).
    pub edit_value: VoxelValue,
    pub suggested: SuggestedAction,
}

/// The outcome of a regenerate-and-replay diagnostic.
#[derive(Debug, Clone, PartialEq)]
pub struct RegenReplayReport {
    pub saved_version: u32,
    pub new_version: u32,
    /// Edits whose generated context changed. Empty == clean replay.
    pub conflicts: Vec<EditConflict>,
    /// Number of edit events examined (non-generation events).
    pub replayed_edits: u32,
    /// Deterministic fingerprint of the regenerated+replayed staging world.
    pub staging_world_hash: BundleHash,
}

impl RegenReplayReport {
    /// Whether every edit still applies over identical generated context.
    pub fn is_clean(&self) -> bool {
        self.conflicts.is_empty()
    }
}

/// Regenerate terrain for `chunks` at `version` and replay `edits` on top,
/// reporting conflicts against the `saved_version` terrain. This is the full
/// diagnostic: it generates both the saved-version and new-version base terrain
/// from `seed`, so it is self-contained.
pub fn regenerate_and_replay(
    spec: VoxelGridSpec,
    seed: u64,
    saved_version: u32,
    new_version: u32,
    chunks: &[core_space::ChunkCoord],
    edits: &[VoxelEditEvent],
) -> Result<RegenReplayReport, VoxelEditRejection> {
    let mut old_terrain = VoxelWorld::new(spec);
    let mut new_terrain = VoxelWorld::new(spec);
    for &c in chunks {
        old_terrain.insert(c, generate_chunk(&spec, c, seed, saved_version));
        new_terrain.insert(c, generate_chunk(&spec, c, seed, new_version));
    }
    replay_against(
        spec,
        &old_terrain,
        new_terrain,
        saved_version,
        new_version,
        edits,
    )
}

/// Core diagnostic, terrain-source-agnostic: compare each edit's target against
/// the `old_terrain` vs `staging` (new) generated base, then replay the edits onto
/// `staging`. Lets callers supply explicit terrains (tests) or regenerated ones.
pub fn replay_against(
    spec: VoxelGridSpec,
    old_terrain: &VoxelWorld,
    mut staging: VoxelWorld,
    saved_version: u32,
    new_version: u32,
    edits: &[VoxelEditEvent],
) -> Result<RegenReplayReport, VoxelEditRejection> {
    let mut conflicts = Vec::new();
    let mut replayed_edits = 0u32;

    for (i, event) in edits.iter().enumerate() {
        match *event {
            VoxelEditEvent::VoxelSet { coord, value, .. } => {
                replayed_edits += 1;
                record_conflict(
                    spec,
                    old_terrain,
                    &staging,
                    i as u64,
                    coord,
                    value,
                    &mut conflicts,
                );
            }
            VoxelEditEvent::VoxelRegionFilled {
                min, max, value, ..
            } => {
                replayed_edits += 1;
                for z in min.z..max.z {
                    for y in min.y..max.y {
                        for x in min.x..max.x {
                            record_conflict(
                                spec,
                                old_terrain,
                                &staging,
                                i as u64,
                                VoxelCoord::new(x, y, z),
                                value,
                                &mut conflicts,
                            );
                        }
                    }
                }
            }
            VoxelEditEvent::ChunkGenerated { .. } => {}
        }
    }

    // Replay the edits onto the new (staging) terrain so the caller can inspect
    // the resulting world. Snapshot a deterministic fingerprint of it.
    apply_all(&mut staging, edits)?;

    Ok(RegenReplayReport {
        saved_version,
        new_version,
        conflicts,
        replayed_edits,
        staging_world_hash: world_fingerprint(&staging),
    })
}

fn record_conflict(
    spec: VoxelGridSpec,
    old_terrain: &VoxelWorld,
    staging: &VoxelWorld,
    event_id: u64,
    coord: VoxelCoord,
    edit_value: VoxelValue,
    out: &mut Vec<EditConflict>,
) {
    let old = value_at(spec, old_terrain, coord);
    let new = value_at(spec, staging, coord);
    if old != new {
        out.push(EditConflict {
            event_id,
            coord,
            old_generated: old,
            new_generated: new,
            edit_value,
            suggested: SuggestedAction::ReviewConflict,
        });
    }
}

/// The generated value at a world voxel coordinate (Empty when the chunk is not
/// resident — treated as "nothing generated there").
fn value_at(spec: VoxelGridSpec, world: &VoxelWorld, coord: VoxelCoord) -> VoxelValue {
    let (chunk, local) = spec.voxel_to_chunk_local(coord);
    world
        .get(chunk)
        .and_then(|c| c.get(local))
        .unwrap_or(VoxelValue::EMPTY)
}

/// Deterministic FNV fingerprint of a world's resident chunks (sorted by coord,
/// each by content hash).
fn world_fingerprint(world: &VoxelWorld) -> BundleHash {
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
