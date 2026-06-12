//! Voxel diagnostics aggregator (voxel-capability-15).
//!
//! # Lane
//!
//! `rust-tools` — an **observational** read layer that makes storage, meshing,
//! collision, scheduling, and replay state agent-legible. It is more omniscient
//! than runtime crates (it sees across `svc-volume`/`svc-spatial`/`svc-mesh`/
//! `svc-collision`/`rule-scheduler`/`rule-voxel-edit`), but it **never mutates
//! authority** — every entry point takes `&` references and returns reports. Tool
//! omniscience does not leak back into runtime lanes (it is not depended on by them).

#![forbid(unsafe_code)]

use core_space::{ChunkCoord, VoxelGridSpec};
use rule_scheduler::{ChunkScheduler, WorkKind};
use rule_voxel_edit::VoxelEditRejection;
use svc_collision::CollisionProjection;
use svc_mesh::{mesh_chunk_in_world, MeshStats};
use svc_spatial::{ChunkState, VoxelWorld};

/// Per-resident-chunk diagnostics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkReport {
    pub coord: ChunkCoord,
    pub content_hash: u64,
    pub dirty: bool,
    /// Mesh stats from a fresh in-world mesh, or `None` if meshing failed.
    pub mesh: Option<MeshStats>,
    pub has_collider: bool,
}

/// A count of queued work of one kind (scheduler diagnostics by lane).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueueCount {
    pub kind: WorkKind,
    pub count: usize,
}

/// A deterministic snapshot of a voxel scene's diagnostics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoxelSceneReport {
    pub resident: usize,
    pub pending: usize,
    pub unloaded: usize,
    /// Resident chunks, coordinate-ascending.
    pub chunks: Vec<ChunkReport>,
    /// Dirty chunk coordinates, ascending.
    pub dirty_chunks: Vec<ChunkCoord>,
    pub collider_chunks: usize,
    /// Queued work counts by kind (Generate, Mesh, CollisionRebuild, Upload).
    pub queue: Vec<QueueCount>,
}

/// Build a deterministic diagnostics report for a voxel world, optionally enriched
/// with a collision projection and a work scheduler. Read-only.
pub fn report(
    world: &VoxelWorld,
    collision: Option<&CollisionProjection>,
    scheduler: Option<&ChunkScheduler>,
) -> VoxelSceneReport {
    let mut resident = 0;
    let mut pending = 0;
    let mut unloaded = 0;
    for (_, state) in world.tracked() {
        match state {
            ChunkState::Resident => resident += 1,
            ChunkState::Pending => pending += 1,
            ChunkState::Unloaded => unloaded += 1,
            ChunkState::Absent => {}
        }
    }

    let chunks: Vec<ChunkReport> = world
        .resident_chunks()
        .map(|(coord, chunk)| ChunkReport {
            coord,
            content_hash: chunk.content_hash().0,
            dirty: world.is_dirty(coord),
            mesh: mesh_chunk_in_world(world, coord)
                .and_then(|r| r.ok())
                .map(|m| m.stats),
            has_collider: collision.is_some_and(|c| c.has_collider(coord)),
        })
        .collect();

    let dirty_chunks: Vec<ChunkCoord> = world.dirty_chunks().collect();
    let collider_chunks = collision.map_or(0, |c| c.collider_count());

    let queue = scheduler.map_or_else(Vec::new, |s| {
        [
            WorkKind::Generate,
            WorkKind::Mesh,
            WorkKind::CollisionRebuild,
            WorkKind::Upload,
        ]
        .into_iter()
        .map(|kind| QueueCount {
            kind,
            count: s.pending_of(kind),
        })
        .collect()
    });

    VoxelSceneReport {
        resident,
        pending,
        unloaded,
        chunks,
        dirty_chunks,
        collider_chunks,
        queue,
    }
}

impl VoxelSceneReport {
    /// A deterministic, human-readable report for devtools/golden tests.
    pub fn to_report_string(&self) -> String {
        use core::fmt::Write;
        let mut s = String::new();
        let _ = writeln!(
            s,
            "voxel-scene resident={} pending={} unloaded={} colliders={}",
            self.resident, self.pending, self.unloaded, self.collider_chunks
        );
        for c in &self.chunks {
            let mesh = c
                .mesh
                .map(|m| {
                    format!(
                        "quads={} v={} i={} culled={}",
                        m.quads, m.vertices, m.indices, m.faces_culled
                    )
                })
                .unwrap_or_else(|| "mesh=err".to_string());
            let _ = writeln!(
                s,
                "chunk {:?} hash={:#018x} dirty={} collider={} {mesh}",
                c.coord.to_array(),
                c.content_hash,
                c.dirty,
                c.has_collider
            );
        }
        let dirty: Vec<_> = self.dirty_chunks.iter().map(|c| c.to_array()).collect();
        let _ = writeln!(s, "dirty {dirty:?}");
        for q in &self.queue {
            if q.count > 0 {
                let _ = writeln!(s, "queue {} count={}", q.kind.label(), q.count);
            }
        }
        s
    }
}

/// Format a voxel edit/replay rejection with chunk + voxel coordinate context, so a
/// replay divergence is agent-legible (which chunk, where in voxel space).
pub fn describe_rejection(spec: &VoxelGridSpec, rejection: &VoxelEditRejection) -> String {
    match rejection {
        VoxelEditRejection::GenerationDivergence {
            chunk,
            expected,
            actual,
        } => {
            let origin = spec.chunk_origin_voxel(*chunk);
            format!(
                "generation divergence: chunk {:?} (voxel origin {:?}) expected hash {expected:#018x}, got {actual:#018x} \
                 — terrain generator changed; see migration options (regenerate+replay / snapshot / pin)",
                chunk.to_array(),
                origin.to_array()
            )
        }
        VoxelEditRejection::ChunkNotResident { chunk } => {
            format!("edit rejected: chunk {:?} not resident", chunk.to_array())
        }
        VoxelEditRejection::UnknownMaterial(id) => {
            format!("edit rejected: unknown material {}", id.raw())
        }
        VoxelEditRejection::EmptyRegion { min, max } => {
            format!(
                "edit rejected: empty fill region {:?}..{:?}",
                min.to_array(),
                max.to_array()
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_space::{ChunkDims, GridId, LocalVoxelCoord};
    use core_voxel::VoxelValue;
    use rule_voxel_edit::generate_chunk;
    use svc_volume::VoxelChunk;

    fn spec() -> VoxelGridSpec {
        VoxelGridSpec::new(GridId::new(0), 1.0, ChunkDims::cubic(4).unwrap()).unwrap()
    }

    fn scene() -> VoxelWorld {
        let mut w = VoxelWorld::new(spec());
        let mut c = VoxelChunk::from_spec(&spec());
        c.set(LocalVoxelCoord::new(0, 0, 0), VoxelValue::solid_raw(1))
            .unwrap();
        c.set(LocalVoxelCoord::new(1, 0, 0), VoxelValue::solid_raw(1))
            .unwrap();
        w.insert(ChunkCoord::new(0, 0, 0), c); // resident + dirty (insert marks dirty)
        w.insert(ChunkCoord::new(1, 0, 0), VoxelChunk::from_spec(&spec())); // empty resident
        w.request(ChunkCoord::new(2, 0, 0)).unwrap(); // pending
        w
    }

    #[test]
    fn report_aggregates_lifecycle_mesh_and_collision_deterministically() {
        let world = scene();
        let collision = CollisionProjection::build(&world);
        let report = report(&world, Some(&collision), None);
        assert_eq!(report.resident, 2);
        assert_eq!(report.pending, 1);
        assert_eq!(report.collider_chunks, 1); // only the solid chunk has a collider
                                               // Chunk 0 has 2 solid voxels → mesh emits 10 quads (shared face culled).
        let c0 = report
            .chunks
            .iter()
            .find(|c| c.coord == ChunkCoord::new(0, 0, 0))
            .unwrap();
        assert_eq!(c0.mesh.unwrap().quads, 10);
        assert!(c0.has_collider);
        // Empty chunk 1 → 0 quads, no collider.
        let c1 = report
            .chunks
            .iter()
            .find(|c| c.coord == ChunkCoord::new(1, 0, 0))
            .unwrap();
        assert_eq!(c1.mesh.unwrap().quads, 0);
        assert!(!c1.has_collider);

        // Deterministic.
        assert_eq!(report, super::report(&world, Some(&collision), None));
    }

    #[test]
    fn report_string_matches_golden() {
        let mut world = scene();
        world.drain_dirty(); // clean state for a stable golden
        let collision = CollisionProjection::build(&world);
        let mut sched = ChunkScheduler::new();
        sched.on_chunk_edited(ChunkCoord::new(0, 0, 0), 1, 0);
        let report = report(&world, Some(&collision), Some(&sched));
        assert_eq!(
            report.to_report_string(),
            include_str!(
                "../../../../../harness/fixtures/voxel-diagnostics/sample-scene.report.txt"
            ),
        );
    }

    #[test]
    fn describe_rejection_includes_chunk_and_voxel_context() {
        let chunk = ChunkCoord::new(-1, 0, 2);
        let r = VoxelEditRejection::GenerationDivergence {
            chunk,
            expected: 1,
            actual: 2,
        };
        let text = describe_rejection(&spec(), &r);
        assert!(text.contains("chunk [-1, 0, 2]"));
        // The chunk's voxel origin (-4,0,8) for 4³ chunks.
        assert!(text.contains("voxel origin [-4, 0, 8]"), "{text}");
        assert!(text.contains("migration options"));
    }

    #[test]
    fn diagnostics_never_mutate_the_world() {
        let world = scene();
        let before = world.get(ChunkCoord::new(0, 0, 0)).unwrap().content_hash();
        let _ = report(&world, None, None);
        let _ = generate_chunk(&spec(), ChunkCoord::new(0, 0, 0), 1, 1); // read-only helper
        assert_eq!(
            world.get(ChunkCoord::new(0, 0, 0)).unwrap().content_hash(),
            before
        );
    }
}
