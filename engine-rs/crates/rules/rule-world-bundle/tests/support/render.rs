//! Deterministic renderers + shared fixtures for the world-bundle golden tests.
//! Shared between the golden drift tests and the regenerator examples; each
//! consumer uses a subset, so unused-in-one-binary helpers are expected.
#![allow(dead_code)]

use core_events::VoxelEditEvent;
use core_space::{ChunkCoord, ChunkDims, GridId, VoxelCoord, VoxelGridSpec};
use core_voxel::VoxelValue;
use rule_voxel_edit::generate_chunk;
use rule_world_bundle::{
    build_durability_evidence, compact_voxel_save, regenerate_and_replay, voxel_save_plan,
    CompactedVoxelSave, DurabilityEvidence, RegenReplayReport,
};

pub fn spec() -> VoxelGridSpec {
    VoxelGridSpec::new(GridId::new(0), 1.0, ChunkDims::cubic(4).unwrap()).unwrap()
}

/// The full edit log used for the compacted-save fixture: generate one chunk, then
/// three abstract voxel edits.
pub fn full_log() -> Vec<VoxelEditEvent> {
    let g = GridId::new(0);
    let chunk = ChunkCoord::new(0, 0, 0);
    let gen = generate_chunk(&spec(), chunk, 7, 1);
    vec![
        VoxelEditEvent::ChunkGenerated {
            grid: g,
            chunk,
            seed: 7,
            generator_version: 1,
            hash: gen.content_hash().0,
        },
        VoxelEditEvent::VoxelSet {
            grid: g,
            coord: VoxelCoord::new(0, 3, 0),
            value: VoxelValue::solid_raw(2),
        },
        VoxelEditEvent::VoxelSet {
            grid: g,
            coord: VoxelCoord::new(1, 3, 0),
            value: VoxelValue::solid_raw(2),
        },
        VoxelEditEvent::VoxelSet {
            grid: g,
            coord: VoxelCoord::new(2, 3, 0),
            value: VoxelValue::solid_raw(3),
        },
    ]
}

pub fn sample_compacted_save() -> CompactedVoxelSave {
    compact_voxel_save(spec(), &full_log(), 1).expect("compact")
}

// ── #2440 durability evidence ──────────────────────────────────────────────────

/// The canonical durability sequence reuses the compacted-save fixture log: the
/// `ChunkGenerated` prefix is the loaded base; the three voxel edits are the user
/// edit sequence applied on top.
pub fn canonical_durability_base() -> Vec<VoxelEditEvent> {
    full_log()[..1].to_vec()
}

pub fn canonical_durability_edits() -> Vec<VoxelEditEvent> {
    full_log()[1..].to_vec()
}

/// Build the committed durability evidence for the canonical sequence (retain 1).
pub fn sample_durability_evidence() -> DurabilityEvidence {
    build_durability_evidence(
        spec(),
        &canonical_durability_base(),
        &canonical_durability_edits(),
        1,
    )
    .expect("durability evidence")
}

/// Render the durability checkpoint evidence deterministically (the golden form).
pub fn render_durability(ev: &DurabilityEvidence) -> String {
    use core::fmt::Write;
    let mut s = String::new();
    let _ = writeln!(s, "voxeldurability 1");
    let _ = writeln!(s, "fixture launch-sequence");
    let _ = writeln!(s, "postLoad {}", ev.post_load.to_hex());
    let _ = writeln!(s, "postEdit {}", ev.post_edit.to_hex());
    let _ = writeln!(s, "postReload {}", ev.post_reload.to_hex());
    let _ = writeln!(s, "durable {}", ev.is_durable());
    let _ = writeln!(s, "compactedEdits {}", ev.compacted_edits);
    let _ = writeln!(s, "retainedEdits {}", ev.retained_edits);
    s
}

/// Render the compacted save bundle section: the save plan summary plus each
/// snapshot's encoded text. Deterministic.
pub fn render_compacted_save(save: &CompactedVoxelSave) -> String {
    use core::fmt::Write;
    let mut s = String::new();
    s.push_str(&voxel_save_plan(save).describe());
    s.push_str("--- snapshots ---\n");
    for snap in &save.snapshots {
        let _ = writeln!(s, "# {}", snap.path);
        s.push_str(&snap.text);
    }
    s.push_str("--- retained edit log ---\n");
    s.push_str(&save.retained_log_text);
    s
}

/// The conflicting-edit regenerate-and-replay scenario: a single voxel edit whose
/// generated base differs between version 1 and version 2 of the generator. We
/// search a small coordinate window for a coord that actually diverges so the
/// fixture is a genuine conflict regardless of generator internals.
pub fn conflict_report() -> RegenReplayReport {
    let g = GridId::new(0);
    let chunk = ChunkCoord::new(0, 0, 0);
    let old = generate_chunk(&spec(), chunk, 100, 1);
    let new = generate_chunk(&spec(), chunk, 100, 2);
    let dims = spec().chunk_dims();
    // Find the first local coord whose generated value differs between versions.
    let mut target = VoxelCoord::new(0, 0, 0);
    'search: for z in 0..dims.z() {
        for y in 0..dims.y() {
            for x in 0..dims.x() {
                let l = core_space::LocalVoxelCoord::new(x, y, z);
                if old.get(l) != new.get(l) {
                    target = spec().chunk_local_to_voxel(chunk, l);
                    break 'search;
                }
            }
        }
    }
    let edits = vec![VoxelEditEvent::VoxelSet {
        grid: g,
        coord: target,
        value: VoxelValue::solid_raw(9),
    }];
    regenerate_and_replay(spec(), 100, 1, 2, &[chunk], &edits).expect("replay")
}

/// Render a regenerate-and-replay report deterministically (JSON-ish text).
pub fn render_report(r: &RegenReplayReport) -> String {
    use core::fmt::Write;
    let mut s = String::new();
    let _ = writeln!(s, "savedVersion {}", r.saved_version);
    let _ = writeln!(s, "newVersion {}", r.new_version);
    let _ = writeln!(s, "replayedEdits {}", r.replayed_edits);
    let _ = writeln!(s, "clean {}", r.is_clean());
    let _ = writeln!(
        s,
        "stagingSpatialSessionHash {}",
        r.staging_spatial_session_hash.to_hex()
    );
    let _ = writeln!(s, "conflicts {}", r.conflicts.len());
    for c in &r.conflicts {
        let _ = writeln!(
            s,
            "  event {} coord {},{},{} old {} new {} edit {} action {}",
            c.event_id,
            c.coord.x,
            c.coord.y,
            c.coord.z,
            encode_value(c.old_generated),
            encode_value(c.new_generated),
            encode_value(c.edit_value),
            c.suggested.label(),
        );
    }
    s
}

fn encode_value(v: VoxelValue) -> String {
    match v.material() {
        Some(m) => format!("solid:{}", m.raw()),
        None => "empty".to_string(),
    }
}
