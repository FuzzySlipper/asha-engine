//! Canonical accepted voxel edit/generation events.
//!
//! The authoritative record of a validated voxel change (voxel-capability-05).
//! Owned here (and mirrored into generated `protocol` TS contracts), produced by
//! `rule-voxel-edit` from a [`crate::DomainEvent`]-sibling voxel command, and
//! applied to voxel storage. Services consume these; they do not redefine them.

use core_space::{ChunkCoord, GridId, VoxelCoord};
use core_voxel::VoxelValue;

/// An accepted, authoritative voxel change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoxelEditEvent {
    /// A single cell was set.
    VoxelSet {
        grid: GridId,
        coord: VoxelCoord,
        value: VoxelValue,
    },
    /// A half-open region `[min, max)` was filled with one value.
    VoxelRegionFilled {
        grid: GridId,
        min: VoxelCoord,
        max: VoxelCoord,
        value: VoxelValue,
    },
    /// A chunk was deterministically generated; `hash` is the resulting chunk
    /// content hash, recorded so replay/divergence can compare generation output.
    ChunkGenerated {
        grid: GridId,
        chunk: ChunkCoord,
        seed: u64,
        generator_version: u32,
        hash: u64,
    },
}

impl VoxelEditEvent {
    pub fn grid(self) -> GridId {
        match self {
            VoxelEditEvent::VoxelSet { grid, .. }
            | VoxelEditEvent::VoxelRegionFilled { grid, .. }
            | VoxelEditEvent::ChunkGenerated { grid, .. } => grid,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn voxel_event_carries_its_grid() {
        let e = VoxelEditEvent::VoxelSet {
            grid: GridId::new(7),
            coord: VoxelCoord::new(0, 0, 0),
            value: VoxelValue::EMPTY,
        };
        assert_eq!(e.grid(), GridId::new(7));
    }
}
