//! World-bundle execution over voxel persistence (epic #2310, subtasks #2320/#2321).
//!
//! # Lane
//!
//! `rust-rule` — the orchestration tier that may compose both authority state
//! (`core-scene`) and the voxel persistence rules (`rule-voxel-edit`). The pure
//! manifest/plan *format* lives one layer down in `svc-serialization`; this crate
//! supplies the parts that must execute voxel work:
//!
//! * [`compose`] — compose chunk snapshots / edit logs into world-bundle voxel
//!   sections and perform explicit **save-time compaction** with a reconstruction
//!   guarantee (subtask #2320).
//! * [`regen`] — **fail-closed** generator-mismatch handling plus the development
//!   **regenerate-and-replay** conflict diagnostic (subtask #2321).
//!
//! Both stay diagnostic/inspectable: compaction never runs on ordinary ticks, and
//! the regenerate-and-replay path never silently rewrites a save.

#![forbid(unsafe_code)]

pub mod compose;
pub mod load;
pub mod regen;

pub use compose::{
    compact_voxel_save, reconstruct, voxel_save_plan, ChunkSnapshotArtifact, CompactedVoxelSave,
};
pub use load::{
    execute_load_plan, execute_load_plan_with, ArtifactSource, BundleArtifacts, LoadExecutionError,
    StageOutcome, WorldLoadResult, WorldStage,
};
pub use regen::{
    check_generator, regenerate_and_replay, replay_against, EditConflict, GeneratorMismatch,
    GeneratorPolicy, RegenReplayReport, SuggestedAction,
};

#[cfg(test)]
mod tests {
    use super::*;
    use core_events::VoxelEditEvent;
    use core_space::{ChunkCoord, ChunkDims, GridId, LocalVoxelCoord, VoxelCoord, VoxelGridSpec};
    use core_voxel::VoxelValue;
    use rule_voxel_edit::generate_chunk;
    use rule_voxel_edit::persist::replay_edit_log;
    use svc_spatial::VoxelWorld;
    use svc_volume::VoxelChunk;

    fn spec() -> VoxelGridSpec {
        VoxelGridSpec::new(GridId::new(0), 1.0, ChunkDims::cubic(4).unwrap()).unwrap()
    }

    fn full_log() -> Vec<VoxelEditEvent> {
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

    // ── #2320 compaction ──────────────────────────────────────────────────────

    #[test]
    fn compacted_save_reconstructs_full_replay_hashes() {
        let full = full_log();
        let save = compact_voxel_save(spec(), &full, 1).expect("compact");
        // Two edits folded (set0, set1); one retained (set2).
        assert_eq!(save.compacted_edits, 2);
        assert_eq!(save.retained_edits.len(), 1);
        assert_eq!(save.snapshots.len(), 1);

        let reconstructed = reconstruct(spec(), &save).expect("reconstruct");
        let direct = replay_edit_log(spec(), &full).expect("replay");
        let chunk = ChunkCoord::new(0, 0, 0);
        assert_eq!(
            reconstructed.get(chunk).unwrap().content_hash(),
            direct.get(chunk).unwrap().content_hash(),
            "compacted snapshot + retained edits must reconstruct the full-replay chunk hash"
        );
    }

    #[test]
    fn full_compaction_folds_everything() {
        let full = full_log();
        let save = compact_voxel_save(spec(), &full, 0).expect("compact");
        assert_eq!(save.retained_edits.len(), 0);
        assert_eq!(save.compacted_edits, 3);
        let reconstructed = reconstruct(spec(), &save).expect("reconstruct");
        let direct = replay_edit_log(spec(), &full).expect("replay");
        let chunk = ChunkCoord::new(0, 0, 0);
        assert_eq!(
            reconstructed.get(chunk).unwrap().content_hash(),
            direct.get(chunk).unwrap().content_hash(),
        );
    }

    #[test]
    fn save_plan_explains_compaction_and_is_replay_separated() {
        let save = compact_voxel_save(spec(), &full_log(), 1).unwrap();
        let plan = voxel_save_plan(&save);
        let desc = plan.describe();
        assert!(desc.contains("fold 2 edits"));
        assert!(desc.contains("retain 1 recent edit"));
        // The snapshot is `generated`, the edit log is `durable`; nothing references
        // a replay record (save and replay are separate concepts).
        assert_eq!(plan.durable_writes().count(), 1);
    }

    // ── #2321 generator mismatch ──────────────────────────────────────────────

    #[test]
    fn matching_generator_loads_under_any_policy() {
        assert_eq!(
            check_generator(3, 3, GeneratorPolicy::FailClosed),
            Ok(false)
        );
        assert_eq!(
            check_generator(3, 3, GeneratorPolicy::RegenerateAndReplay),
            Ok(false)
        );
    }

    #[test]
    fn mismatch_fails_closed_by_default() {
        assert_eq!(
            check_generator(1, 2, GeneratorPolicy::FailClosed),
            Err(GeneratorMismatch {
                saved_version: 1,
                current_version: 2,
            })
        );
        // Dev mode permits the diagnostic instead.
        assert_eq!(
            check_generator(1, 2, GeneratorPolicy::RegenerateAndReplay),
            Ok(true)
        );
    }

    /// A world with chunk (0,0,0) where one local voxel holds `value`.
    fn terrain_with(local: LocalVoxelCoord, value: VoxelValue) -> VoxelWorld {
        let mut world = VoxelWorld::new(spec());
        let mut chunk = VoxelChunk::from_spec(&spec());
        chunk.set(local, value).unwrap();
        world.insert(ChunkCoord::new(0, 0, 0), chunk);
        world
    }

    #[test]
    fn clean_replay_reports_no_conflicts() {
        // Old and new generated bases are identical at the edited coord.
        let old = terrain_with(LocalVoxelCoord::new(1, 1, 0), VoxelValue::solid_raw(1));
        let new = terrain_with(LocalVoxelCoord::new(1, 1, 0), VoxelValue::solid_raw(1));
        let g = GridId::new(0);
        // Edit a coord that is empty in BOTH terrains, so context is unchanged.
        let edits = vec![VoxelEditEvent::VoxelSet {
            grid: g,
            coord: VoxelCoord::new(3, 3, 3),
            value: VoxelValue::solid_raw(9),
        }];
        let report = replay_against(spec(), &old, new, 1, 2, &edits).expect("replay");
        assert!(report.is_clean(), "conflicts: {:?}", report.conflicts);
        assert_eq!(report.replayed_edits, 1);
    }

    #[test]
    fn incompatible_edit_reports_coord_and_material_conflict() {
        // The edited coord was solid material 1 under the old generator, empty
        // under the new one — a changed authored context.
        let old = terrain_with(LocalVoxelCoord::new(1, 1, 0), VoxelValue::solid_raw(1));
        let new = VoxelWorld::new(spec()); // chunk not resident -> generated empty
        let mut staging = new;
        staging.insert(ChunkCoord::new(0, 0, 0), VoxelChunk::from_spec(&spec()));
        let g = GridId::new(0);
        let edits = vec![VoxelEditEvent::VoxelSet {
            grid: g,
            coord: VoxelCoord::new(1, 1, 0),
            value: VoxelValue::solid_raw(5),
        }];
        let report = replay_against(spec(), &old, staging, 1, 2, &edits).expect("replay");
        assert_eq!(report.conflicts.len(), 1);
        let c = report.conflicts[0];
        assert_eq!(c.coord, VoxelCoord::new(1, 1, 0));
        assert_eq!(c.old_generated, VoxelValue::solid_raw(1));
        assert_eq!(c.new_generated, VoxelValue::EMPTY);
        assert_eq!(c.edit_value, VoxelValue::solid_raw(5));
        assert_eq!(c.event_id, 0);
        assert_eq!(c.suggested, SuggestedAction::ReviewConflict);
    }

    #[test]
    fn regenerate_and_replay_is_deterministic() {
        let g = GridId::new(0);
        let chunk = ChunkCoord::new(0, 0, 0);
        let edits = vec![VoxelEditEvent::VoxelSet {
            grid: g,
            coord: VoxelCoord::new(1, 0, 0),
            value: VoxelValue::solid_raw(2),
        }];
        let a = regenerate_and_replay(spec(), 100, 1, 2, &[chunk], &edits).unwrap();
        let b = regenerate_and_replay(spec(), 100, 1, 2, &[chunk], &edits).unwrap();
        assert_eq!(a, b, "diagnostic must be deterministic across runs");
        assert_eq!(a.replayed_edits, 1);
    }
}
