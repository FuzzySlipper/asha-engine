//! Save → load round-trip equivalence verification (scene-capability-06,
//! subtask #2333).
//!
//! The critical end-to-end diagnostic: prove that world-bundle serialization
//! preserves authority-equivalent state. The flow follows the source doc:
//!
//! 1. Load a world (an initial edit log) → authority state A.
//! 2. Run N deterministic operations (more edits) → authority state B.
//! 3. Save a bundle from B (explicit compaction).
//! 4. Load the saved bundle → authority state C.
//! 5. Assert B and C are equivalent: per-chunk voxel hashes and the overall
//!    state hash (plus a scene-document round-trip for transforms/asset refs).
//!
//! Everything here is observational and generic: the equivalence and any
//! mismatch are reported as stable [`protocol_diagnostics`] artifacts with no
//! Den-specific fields or imports.

use core::fmt::Write;

use core_events::VoxelEditEvent;
use core_scene::document::FlatSceneDocument;
use core_space::VoxelGridSpec;
use protocol_diagnostics::{
    DiagnosticCode, DiagnosticReport, DiagnosticReportSet, DiagnosticSourceRef, RemedyAction,
    SuggestedRemedy,
};
use rule_voxel_edit::persist::replay_edit_log;
use rule_voxel_edit::VoxelEditRejection;
use rule_world_bundle::{compact_voxel_save, reconstruct, CompactedVoxelSave};
use svc_serialization::BundleHash;
use svc_spatial::VoxelWorld;

/// A deterministic fingerprint of a voxel world's resident chunks (coordinate
/// order, each by content hash). State B and state C must share this.
pub fn world_fingerprint(world: &VoxelWorld) -> BundleHash {
    let mut rows: Vec<(i64, i64, i64, u64)> = world
        .resident_chunks()
        .map(|(c, chunk)| (c.x, c.y, c.z, chunk.content_hash().0))
        .collect();
    rows.sort_unstable();
    let mut s = String::new();
    for (x, y, z, h) in rows {
        let _ = writeln!(s, "{x},{y},{z}:{h}");
    }
    BundleHash::of_str(&s)
}

/// The outcome of a save → load round-trip.
#[derive(Debug, Clone, PartialEq)]
pub struct RoundTripReport {
    /// State hash after the N deterministic operations, before saving.
    pub state_b_hash: BundleHash,
    /// State hash after loading the saved bundle.
    pub state_c_hash: BundleHash,
    /// Edits folded into snapshots by save-time compaction.
    pub compacted_edits: u32,
    /// Edits retained in the saved log (replayed on load).
    pub retained_edits: usize,
    /// Any equivalence-failure diagnostics (empty == clean round-trip).
    pub diagnostics: DiagnosticReportSet,
}

impl RoundTripReport {
    /// True when B and C state hashes match and no diagnostics were raised.
    pub fn is_equivalent(&self) -> bool {
        self.state_b_hash == self.state_c_hash && self.diagnostics.is_empty()
    }

    /// Deterministic text rendering for goldens / readback.
    pub fn to_report_text(&self) -> String {
        let mut s = String::new();
        let _ = writeln!(
            s,
            "roundtrip equivalent={} stateB={} stateC={} compacted={} retained={}",
            self.is_equivalent(),
            self.state_b_hash.to_hex(),
            self.state_c_hash.to_hex(),
            self.compacted_edits,
            self.retained_edits
        );
        s.push_str(&crate::text::report_set_to_text(&self.diagnostics));
        s
    }
}

/// Run the full voxel save → load round-trip.
///
/// `initial_log` is the loaded world (state A); `tick_edits` are the N
/// deterministic operations applied to reach state B; `retain_recent` controls
/// how much of the tail the save keeps as an edit log (the rest is compacted into
/// snapshots). Returns the equivalence report. A hash mismatch — which only
/// happens if the save/compaction path loses state — is reported as an `Error`
/// [`DiagnosticCode::RoundTripMismatch`] (a genuinely tampered on-disk artifact
/// is `CorruptBundleArtifact`; see [`check_saved_bundle`]).
pub fn voxel_round_trip(
    spec: VoxelGridSpec,
    initial_log: &[VoxelEditEvent],
    tick_edits: &[VoxelEditEvent],
    retain_recent: usize,
) -> Result<RoundTripReport, VoxelEditRejection> {
    let mut full_log = initial_log.to_vec();
    full_log.extend_from_slice(tick_edits);

    // State B: the world after the deterministic operations.
    let world_b = replay_edit_log(spec, &full_log)?;
    let state_b_hash = world_fingerprint(&world_b);

    // Save B, then load it back into state C.
    let save = compact_voxel_save(spec, &full_log, retain_recent)?;
    let world_c = reconstruct(spec, &save)?;
    let state_c_hash = world_fingerprint(&world_c);

    let mut diagnostics = DiagnosticReportSet::new();
    if state_b_hash != state_c_hash {
        // A clean replay→save→reconstruct that loses state is a round-trip
        // equivalence failure, not a corrupt artifact (#2368 taxonomy).
        diagnostics.push(round_trip_mismatch_report(state_b_hash, state_c_hash));
    }

    Ok(RoundTripReport {
        state_b_hash,
        state_c_hash,
        compacted_edits: save.compacted_edits,
        retained_edits: save.retained_edits.len(),
        diagnostics,
    })
}

/// Reload an already-saved bundle and check it still reproduces `expected_b_hash`.
///
/// This is the intentional-data-loss surface: hand it a tampered
/// [`CompactedVoxelSave`] (a corrupted snapshot, a dropped retained edit) and the
/// resulting hash mismatch yields a structured `Fatal` diagnostic rather than a
/// silently wrong load.
pub fn check_saved_bundle(
    spec: VoxelGridSpec,
    expected_b_hash: BundleHash,
    save: &CompactedVoxelSave,
) -> Result<RoundTripReport, VoxelEditRejection> {
    let world_c = reconstruct(spec, save)?;
    let state_c_hash = world_fingerprint(&world_c);
    let mut diagnostics = DiagnosticReportSet::new();
    if expected_b_hash != state_c_hash {
        diagnostics.push(mismatch_report(expected_b_hash, state_c_hash));
    }
    Ok(RoundTripReport {
        state_b_hash: expected_b_hash,
        state_c_hash,
        compacted_edits: save.compacted_edits,
        retained_edits: save.retained_edits.len(),
        diagnostics,
    })
}

/// A genuinely tampered/corrupt saved bundle: it no longer reproduces the
/// authority state it claimed. `Fatal`.
fn mismatch_report(b: BundleHash, c: BundleHash) -> DiagnosticReport {
    DiagnosticReport::new(
        DiagnosticCode::CorruptBundleArtifact,
        "round-trip",
        DiagnosticSourceRef::empty(),
        format!(
            "save/load round-trip lost state: pre-save hash {} != reloaded hash {}",
            b.to_hex(),
            c.to_hex()
        ),
    )
    .with_remedy(SuggestedRemedy::new(
        RemedyAction::RestoreArtifact,
        "the saved bundle does not reproduce authority state; restore from a known-good save",
    ))
}

/// A clean round-trip that lost equivalence: the save path itself does not
/// preserve authority state. `Error` (a correctness bug to fix, not a corrupt
/// on-disk artifact). See `protocol_diagnostics::DiagnosticCode::RoundTripMismatch`.
fn round_trip_mismatch_report(b: BundleHash, c: BundleHash) -> DiagnosticReport {
    DiagnosticReport::new(
        DiagnosticCode::RoundTripMismatch,
        "round-trip",
        DiagnosticSourceRef::empty(),
        format!(
            "save/load round-trip lost state: pre-save hash {} != reloaded hash {}",
            b.to_hex(),
            c.to_hex()
        ),
    )
    .with_remedy(SuggestedRemedy::new(
        RemedyAction::Inspect,
        "the save/compaction path is not equivalence-preserving; inspect the lost state",
    ))
}

/// A scene-document round-trip: encode to canonical JSON, decode, and confirm the
/// canonical forms match (transforms, asset refs, node ids preserved). A
/// non-equivalent decode is an `Error` [`DiagnosticCode::RoundTripMismatch`]; a
/// decode *failure* is a `Fatal` [`DiagnosticCode::CorruptBundleArtifact`]. Read-only.
pub fn scene_round_trip(doc: &FlatSceneDocument) -> DiagnosticReportSet {
    let mut set = DiagnosticReportSet::new();
    let encoded = core_scene::encode(doc);
    match core_scene::decode(&encoded) {
        Ok(decoded) if decoded.canonical() == doc.canonical() => {}
        Ok(_) => set.push(
            DiagnosticReport::new(
                DiagnosticCode::RoundTripMismatch,
                "scene-document",
                DiagnosticSourceRef::empty(),
                "scene document did not survive an encode/decode round-trip".to_string(),
            )
            .with_remedy(SuggestedRemedy::new(
                RemedyAction::Inspect,
                "the serialized scene document is not equivalence-preserving",
            )),
        ),
        Err(e) => set.push(
            DiagnosticReport::new(
                DiagnosticCode::CorruptBundleArtifact,
                "scene-document",
                DiagnosticSourceRef::empty(),
                format!("scene document failed to decode on round-trip: {e:?}"),
            )
            .with_remedy(SuggestedRemedy::new(
                RemedyAction::RestoreArtifact,
                "restore the scene-document artifact from a known-good copy",
            )),
        ),
    }
    set
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_space::{ChunkCoord, ChunkDims, GridId, VoxelCoord, VoxelGridSpec};
    use core_voxel::VoxelValue;
    use rule_voxel_edit::generate_chunk;

    fn spec() -> VoxelGridSpec {
        VoxelGridSpec::new(GridId::new(0), 1.0, ChunkDims::cubic(4).unwrap()).unwrap()
    }

    fn initial() -> Vec<VoxelEditEvent> {
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
        ]
    }

    fn ticks() -> Vec<VoxelEditEvent> {
        let g = GridId::new(0);
        vec![
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

    #[test]
    fn clean_round_trip_is_equivalent() {
        let report = voxel_round_trip(spec(), &initial(), &ticks(), 1).unwrap();
        assert!(report.is_equivalent(), "{}", report.to_report_text());
        assert_eq!(report.state_b_hash, report.state_c_hash);
        assert!(report.diagnostics.is_empty());
    }

    #[test]
    fn round_trip_is_deterministic() {
        let a = voxel_round_trip(spec(), &initial(), &ticks(), 1).unwrap();
        let b = voxel_round_trip(spec(), &initial(), &ticks(), 1).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn tampered_save_produces_fatal_diagnostic() {
        let mut full = initial();
        full.extend(ticks());
        let world_b = replay_edit_log(spec(), &full).unwrap();
        let expected = world_fingerprint(&world_b);

        // Corrupt the save: drop a retained edit (intentional data loss).
        let mut save = compact_voxel_save(spec(), &full, 2).unwrap();
        assert!(!save.retained_edits.is_empty());
        save.retained_edits.pop();

        let report = check_saved_bundle(spec(), expected, &save).unwrap();
        assert!(!report.is_equivalent());
        assert!(report.diagnostics.blocks_load());
        assert_eq!(
            report.diagnostics.reports[0].code,
            DiagnosticCode::CorruptBundleArtifact
        );
    }
}
