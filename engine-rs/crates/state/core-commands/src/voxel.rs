//! Canonical proposed voxel edit/generation commands.
//!
//! Per voxel-capability-05's ownership rule, voxel edit commands are **border /
//! authority surfaces owned here** (and mirrored into generated `protocol-script`
//! TS contracts), not service-local types. Terrain/generation/edit services
//! *consume* these; they must not define parallel local command types.
//!
//! These are a standalone command surface (not folded into the entity [`Command`]
//! union) because voxel edits apply to voxel storage (`svc-volume`/`svc-spatial`),
//! not the entity `StateStore` — so they have their own validate/apply path in
//! `rule-voxel-edit`. Folding them into the unified submission `Command` is a
//! deliberate later step, not done here.

use core_space::{ChunkCoord, GridId, VoxelCoord};
use core_voxel::VoxelValue;

/// A proposed change to voxel data, awaiting validation by `rule-voxel-edit`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoxelCommand {
    /// Set a single voxel cell.
    SetVoxel {
        grid: GridId,
        coord: VoxelCoord,
        value: VoxelValue,
    },
    /// Fill the half-open voxel region `[min, max)` with one value.
    FillRegion {
        grid: GridId,
        min: VoxelCoord,
        max: VoxelCoord,
        value: VoxelValue,
    },
    /// Deterministically (re)generate a chunk from `seed` + chunk coord +
    /// `generator_version`. Carries no noise-library commitment.
    GenerateChunk {
        grid: GridId,
        chunk: ChunkCoord,
        seed: u64,
        generator_version: u32,
    },
}

impl VoxelCommand {
    /// The grid this command targets.
    pub fn grid(self) -> GridId {
        match self {
            VoxelCommand::SetVoxel { grid, .. }
            | VoxelCommand::FillRegion { grid, .. }
            | VoxelCommand::GenerateChunk { grid, .. } => grid,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn voxel_command_carries_its_grid() {
        let c = VoxelCommand::SetVoxel {
            grid: GridId::new(2),
            coord: VoxelCoord::new(1, 2, 3),
            value: VoxelValue::solid_raw(4),
        };
        assert_eq!(c.grid(), GridId::new(2));
    }
}
