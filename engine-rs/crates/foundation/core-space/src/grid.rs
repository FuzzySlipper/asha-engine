//! Grid specs and the explicit world↔voxel↔chunk/local conversions.
//!
//! All conversion between continuous world space and integer grid space goes
//! through a [`VoxelGridSpec`]. There is intentionally no free function that
//! converts a [`WorldPos`] to a [`VoxelCoord`] without a spec — the grid scale is
//! never implicit, so terrain/object/local grids of different resolutions coexist.

use crate::voxel::{ChunkCoord, LocalVoxelCoord, VoxelCoord};
use crate::world::{WorldPos, WorldScalar};
use crate::{floor_div, rem_euclid};

/// Identifies which voxel grid a spec describes, so multiple grids (terrain,
/// object, local) can be distinguished at call sites and in storage keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GridId(pub u32);

impl GridId {
    pub const fn new(raw: u32) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u32 {
        self.0
    }
}

/// The voxel dimensions of a chunk, per axis. Each axis is `>= 1`; chunks may be
/// non-cubic (e.g. tall terrain columns) — no global cubic-chunk assumption.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChunkDims {
    x: u32,
    y: u32,
    z: u32,
}

impl ChunkDims {
    /// Construct chunk dimensions. Returns `None` if any axis is zero.
    pub const fn new(x: u32, y: u32, z: u32) -> Option<Self> {
        if x == 0 || y == 0 || z == 0 {
            None
        } else {
            Some(Self { x, y, z })
        }
    }

    /// A cubic chunk `n × n × n`. Returns `None` if `n == 0`.
    pub const fn cubic(n: u32) -> Option<Self> {
        Self::new(n, n, n)
    }

    pub const fn x(self) -> u32 {
        self.x
    }
    pub const fn y(self) -> u32 {
        self.y
    }
    pub const fn z(self) -> u32 {
        self.z
    }

    pub const fn to_array(self) -> [u32; 3] {
        [self.x, self.y, self.z]
    }

    /// Total voxels in one chunk.
    pub const fn volume(self) -> u64 {
        self.x as u64 * self.y as u64 * self.z as u64
    }

    const fn axis(self, index: usize) -> u32 {
        match index {
            0 => self.x,
            1 => self.y,
            _ => self.z,
        }
    }
}

/// Describes a voxel grid: its voxel size, chunk shape, identity, and world
/// origin. The single context object for every world↔grid conversion.
///
/// `origin_world` is the rebasing hook: it is the world position of voxel
/// `(0,0,0)`'s minimum corner. Today it defaults to the world origin; a future
/// rebasing pass can shift it without changing any conversion call site.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VoxelGridSpec {
    id: GridId,
    voxel_size: WorldScalar,
    chunk_dims: ChunkDims,
    origin_world: WorldPos,
}

impl VoxelGridSpec {
    /// Construct a grid spec. Returns `None` if `voxel_size` is not a positive,
    /// finite number.
    pub fn new(id: GridId, voxel_size: WorldScalar, chunk_dims: ChunkDims) -> Option<Self> {
        if !voxel_size.is_finite() || voxel_size <= 0.0 {
            return None;
        }
        Some(Self {
            id,
            voxel_size,
            chunk_dims,
            origin_world: WorldPos::ORIGIN,
        })
    }

    /// Return a copy with an explicit world origin (rebasing hook).
    pub fn with_origin(mut self, origin_world: WorldPos) -> Self {
        self.origin_world = origin_world;
        self
    }

    pub const fn id(self) -> GridId {
        self.id
    }
    pub const fn voxel_size(self) -> WorldScalar {
        self.voxel_size
    }
    pub const fn chunk_dims(self) -> ChunkDims {
        self.chunk_dims
    }
    pub const fn origin_world(self) -> WorldPos {
        self.origin_world
    }

    // ── world ↔ voxel ─────────────────────────────────────────────────────────

    /// World position → the voxel cell that contains it (floor, origin-relative).
    pub fn world_to_voxel(self, pos: WorldPos) -> VoxelCoord {
        let o = self.origin_world;
        VoxelCoord::new(
            self.floor_axis(pos.x - o.x),
            self.floor_axis(pos.y - o.y),
            self.floor_axis(pos.z - o.z),
        )
    }

    #[inline]
    fn floor_axis(self, world_delta: WorldScalar) -> i64 {
        (world_delta / self.voxel_size).floor() as i64
    }

    /// World position of a voxel's minimum corner.
    pub fn voxel_min_world(self, v: VoxelCoord) -> WorldPos {
        let o = self.origin_world;
        WorldPos::new(
            o.x + v.x as WorldScalar * self.voxel_size,
            o.y + v.y as WorldScalar * self.voxel_size,
            o.z + v.z as WorldScalar * self.voxel_size,
        )
    }

    /// World position of a voxel's center.
    pub fn voxel_center_world(self, v: VoxelCoord) -> WorldPos {
        let half = self.voxel_size * 0.5;
        let m = self.voxel_min_world(v);
        WorldPos::new(m.x + half, m.y + half, m.z + half)
    }

    /// World-space `(min, max)` corners of a voxel cell (`max` exclusive extent).
    pub fn voxel_bounds_world(self, v: VoxelCoord) -> (WorldPos, WorldPos) {
        let min = self.voxel_min_world(v);
        let s = self.voxel_size;
        (min, WorldPos::new(min.x + s, min.y + s, min.z + s))
    }

    // ── voxel ↔ chunk / local ──────────────────────────────────────────────────

    /// Which chunk a voxel belongs to (floor division by chunk dims, per axis).
    pub fn voxel_to_chunk(self, v: VoxelCoord) -> ChunkCoord {
        let d = self.chunk_dims;
        ChunkCoord::new(
            floor_div(v.x, d.x() as i64),
            floor_div(v.y, d.y() as i64),
            floor_div(v.z, d.z() as i64),
        )
    }

    /// The voxel's address within its chunk (always in `0..chunk_dims`).
    pub fn voxel_to_local(self, v: VoxelCoord) -> LocalVoxelCoord {
        let d = self.chunk_dims;
        LocalVoxelCoord::new(
            rem_euclid(v.x, d.x() as i64) as u32,
            rem_euclid(v.y, d.y() as i64) as u32,
            rem_euclid(v.z, d.z() as i64) as u32,
        )
    }

    /// Both halves of the split at once.
    pub fn voxel_to_chunk_local(self, v: VoxelCoord) -> (ChunkCoord, LocalVoxelCoord) {
        (self.voxel_to_chunk(v), self.voxel_to_local(v))
    }

    /// Reassemble a voxel coordinate from its chunk + local parts.
    ///
    /// `local` is assumed to be within `chunk_dims`; out-of-range locals simply
    /// address a voxel in an adjacent chunk (the arithmetic stays consistent).
    pub fn chunk_local_to_voxel(self, c: ChunkCoord, local: LocalVoxelCoord) -> VoxelCoord {
        let d = self.chunk_dims;
        VoxelCoord::new(
            c.x * d.x() as i64 + local.x as i64,
            c.y * d.y() as i64 + local.y as i64,
            c.z * d.z() as i64 + local.z as i64,
        )
    }

    /// The minimum (origin) voxel of a chunk.
    pub fn chunk_origin_voxel(self, c: ChunkCoord) -> VoxelCoord {
        self.chunk_local_to_voxel(c, LocalVoxelCoord::ORIGIN)
    }

    /// `true` if `local` is within this grid's chunk dimensions.
    pub fn local_in_bounds(self, local: LocalVoxelCoord) -> bool {
        let [lx, ly, lz] = local.to_array();
        lx < self.chunk_dims.axis(0) && ly < self.chunk_dims.axis(1) && lz < self.chunk_dims.axis(2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::region::VoxelRegion;

    fn terrain() -> VoxelGridSpec {
        // 1 world-unit voxels, 16×16×16 chunks.
        VoxelGridSpec::new(GridId::new(0), 1.0, ChunkDims::cubic(16).unwrap()).unwrap()
    }

    #[test]
    fn rejects_degenerate_specs() {
        assert!(VoxelGridSpec::new(GridId::new(0), 0.0, ChunkDims::cubic(8).unwrap()).is_none());
        assert!(VoxelGridSpec::new(GridId::new(0), -1.0, ChunkDims::cubic(8).unwrap()).is_none());
        assert!(
            VoxelGridSpec::new(GridId::new(0), f64::NAN, ChunkDims::cubic(8).unwrap()).is_none()
        );
        assert!(ChunkDims::new(0, 4, 4).is_none());
    }

    #[test]
    fn world_to_voxel_floors_including_near_boundaries_and_negatives() {
        let g = terrain();
        assert_eq!(
            g.world_to_voxel(WorldPos::new(0.0, 0.0, 0.0)),
            VoxelCoord::new(0, 0, 0)
        );
        assert_eq!(
            g.world_to_voxel(WorldPos::new(0.999, 0.5, 0.0)),
            VoxelCoord::new(0, 0, 0)
        );
        assert_eq!(
            g.world_to_voxel(WorldPos::new(1.0, 2.5, 0.0)),
            VoxelCoord::new(1, 2, 0)
        );
        // Negative: -0.001 is in voxel -1, not 0 (floor, not truncate).
        assert_eq!(
            g.world_to_voxel(WorldPos::new(-0.001, -1.0, -16.0)),
            VoxelCoord::new(-1, -1, -16)
        );
        assert_eq!(
            g.world_to_voxel(WorldPos::new(-0.001, -0.999, -0.5)),
            VoxelCoord::new(-1, -1, -1)
        );
    }

    #[test]
    fn voxel_center_and_bounds_use_the_unit_occupancy_convention() {
        let g = terrain();
        let v = VoxelCoord::new(2, 0, -1);
        assert_eq!(g.voxel_min_world(v), WorldPos::new(2.0, 0.0, -1.0));
        assert_eq!(g.voxel_center_world(v), WorldPos::new(2.5, 0.5, -0.5));
        let (min, max) = g.voxel_bounds_world(v);
        assert_eq!(min, WorldPos::new(2.0, 0.0, -1.0));
        assert_eq!(max, WorldPos::new(3.0, 1.0, 0.0));
        // Center of a cell maps back to that cell.
        assert_eq!(g.world_to_voxel(g.voxel_center_world(v)), v);
    }

    #[test]
    fn voxel_chunk_local_roundtrips_including_negatives() {
        let g = terrain();
        for v in [
            VoxelCoord::new(0, 0, 0),
            VoxelCoord::new(15, 15, 15),
            VoxelCoord::new(16, 0, 0),
            VoxelCoord::new(-1, -1, -1),
            VoxelCoord::new(-16, -17, 33),
        ] {
            let (c, l) = g.voxel_to_chunk_local(v);
            assert!(
                g.local_in_bounds(l),
                "local {l:?} must be within chunk for {v:?}"
            );
            assert_eq!(
                g.chunk_local_to_voxel(c, l),
                v,
                "roundtrip failed for {v:?}"
            );
        }
    }

    #[test]
    fn negative_voxel_maps_to_expected_chunk_and_local() {
        let g = terrain();
        // voxel -1 is in chunk -1, local 15 (floor div / euclid rem).
        let (c, l) = g.voxel_to_chunk_local(VoxelCoord::new(-1, -16, -17));
        assert_eq!(c, ChunkCoord::new(-1, -1, -2));
        assert_eq!(l, LocalVoxelCoord::new(15, 0, 15));
    }

    #[test]
    fn chunk_origin_is_the_minimum_voxel_of_the_chunk() {
        let g = terrain();
        assert_eq!(
            g.chunk_origin_voxel(ChunkCoord::new(-1, 0, 2)),
            VoxelCoord::new(-16, 0, 32)
        );
        // Every voxel in a chunk reports that chunk.
        let c = ChunkCoord::new(-1, 0, 2);
        let origin = g.chunk_origin_voxel(c);
        let region = VoxelRegion::new(
            origin,
            VoxelCoord::new(origin.x + 16, origin.y + 16, origin.z + 16),
        );
        assert!(region.iter().all(|v| g.voxel_to_chunk(v) == c));
    }

    #[test]
    fn same_world_position_resolves_differently_under_two_grid_specs() {
        // No single universal voxel size: a coarse 4-unit grid and a fine
        // 0.25-unit grid disagree about which cell a world point falls in.
        let coarse = VoxelGridSpec::new(GridId::new(1), 4.0, ChunkDims::cubic(8).unwrap()).unwrap();
        let fine = VoxelGridSpec::new(GridId::new(2), 0.25, ChunkDims::cubic(8).unwrap()).unwrap();
        let p = WorldPos::new(10.0, 10.0, 10.0);
        assert_eq!(coarse.world_to_voxel(p), VoxelCoord::new(2, 2, 2));
        assert_eq!(fine.world_to_voxel(p), VoxelCoord::new(40, 40, 40));
        assert_ne!(coarse.world_to_voxel(p), fine.world_to_voxel(p));
    }

    #[test]
    fn non_cubic_chunks_split_per_axis() {
        // Tall terrain columns: 16 × 256 × 16.
        let g =
            VoxelGridSpec::new(GridId::new(3), 1.0, ChunkDims::new(16, 256, 16).unwrap()).unwrap();
        let (c, l) = g.voxel_to_chunk_local(VoxelCoord::new(20, 300, -1));
        assert_eq!(c, ChunkCoord::new(1, 1, -1));
        assert_eq!(l, LocalVoxelCoord::new(4, 44, 15));
        assert_eq!(g.chunk_local_to_voxel(c, l), VoxelCoord::new(20, 300, -1));
    }

    #[test]
    fn origin_rebasing_shifts_world_mapping_without_changing_grid_logic() {
        let base = terrain();
        let shifted = terrain().with_origin(WorldPos::new(100.0, 0.0, 0.0));
        // The same world point is a different voxel under a shifted origin...
        let p = WorldPos::new(100.0, 0.0, 0.0);
        assert_eq!(base.world_to_voxel(p), VoxelCoord::new(100, 0, 0));
        assert_eq!(shifted.world_to_voxel(p), VoxelCoord::new(0, 0, 0));
        // ...but voxel→chunk/local math is origin-independent.
        let v = VoxelCoord::new(-5, 0, 33);
        assert_eq!(
            base.voxel_to_chunk_local(v),
            shifted.voxel_to_chunk_local(v)
        );
    }
}
