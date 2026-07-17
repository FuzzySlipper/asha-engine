---
status: current
audience: agent
tags: [voxel, coordinates, foundation]
supersedes: []
see-also: []
---

# Voxel coordinate foundation (`core-space`)

Source design: Den `voxel-capability-01-spatial-coordinate-foundation`. Crate:
`engine-rs/crates/foundation/core-space` (`rust-foundation`, std-only, zero deps).

This is the typed spatial vocabulary every later voxel system shares (storage,
partitioning, meshing, collision, picking, rendering). The goal is that **mixing
spaces is a compile error** and **there is no single universal voxel size**.

## Spaces (distinct newtypes)

| Type | Space | Backing |
|---|---|---|
| `WorldPos` / `WorldVec` | continuous world (Y-up, right-handed) | `WorldScalar` (= `f64`) |
| `VoxelCoord` | integer voxel cell in *some* grid | `i64` |
| `ChunkCoord` | integer chunk coordinate | `i64` |
| `LocalVoxelCoord` | voxel address inside a chunk (`0..chunk_dims`) | `u32` |

`WorldScalar` is an alias so the scalar backing can change without touching call
sites. World positions are `f64` (precision / large worlds); the render border
down-converts to `f32`.

## Grid context

`SpatialGridSpec` is the generic, grid-ID-independent world lattice used by
editor snapping and other spatial tools. It carries an explicit world origin
and positive finite spacing for each axis. Its cell convention is minimum-corner
anchored: cell `n` occupies `[origin + n*spacing, origin + (n+1)*spacing)`, and
negative coordinates use floor semantics. Boundary and cell-center snapping are
defined by this same spec rather than by renderer pixels or an editor-only
formula.

All world↔grid conversion goes through an explicit **`VoxelGridSpec`** — there is
no spec-less `WorldPos → VoxelCoord`. A spec carries:

- `GridId` — distinguishes coexisting grids (terrain / object / local).
- `voxel_size: f64` — world units per voxel edge (must be finite, `> 0`).
- `ChunkDims` — voxels per chunk per axis (`>= 1`, may be non-cubic).
- `origin_world: WorldPos` — **rebasing hook**: world position of voxel `(0,0,0)`'s
  min corner. Defaults to the world origin; an origin shift can be introduced later
  without changing conversion call sites.

## Conventions

- ASHA world and stored project data are always right-handed Y-up. The common
  ground grid is therefore the XZ plane at an explicit Y origin. Importers
  convert source coordinate systems at the border; voxel/runtime consumers do
  not select an alternate up-axis.
- Voxel `(0,0,0)` occupies `[0,1)³` in grid units; center `(0.5,0.5,0.5)`; world
  size of a cell is `voxel_size`.
- **Floor division** for negatives (`floor_div`/`rem_euclid`), so the grid is
  uniform across the origin: voxel `-1` is in chunk `-1`, local `dim-1`.

## Conversions (on `VoxelGridSpec`)

```
world_to_voxel(WorldPos) -> VoxelCoord          // floor, origin-relative
voxel_min_world / voxel_center_world / voxel_bounds_world(VoxelCoord) -> WorldPos
voxel_to_chunk / voxel_to_local / voxel_to_chunk_local(VoxelCoord)
chunk_local_to_voxel(ChunkCoord, LocalVoxelCoord) -> VoxelCoord
chunk_origin_voxel(ChunkCoord) -> VoxelCoord
local_in_bounds(LocalVoxelCoord) -> bool
```

## Directions & regions

- `Axis {X,Y,Z}`, `Direction6` (6 face normals) with `offset`/`normal`/`axis`/
  `opposite`/`is_positive`; `Face` is an alias of `Direction6` (cube faces = the
  six directions). `VoxelCoord::neighbor(Direction6)` / `ChunkCoord::neighbor`.
- `VoxelRegion` / `ChunkRegion` — half-open `[min, max)` boxes with **deterministic
  iteration: X-fastest, then Y, then Z** (fixed so meshing/generation/hashing/golden
  fixtures are reproducible). Exact `size_hint`.

## Deferred (hooks left, not implemented)

Full origin/world rebasing (only the `origin_world` hook exists); per-grid voxel
*occupancy* shapes beyond unit cubes; `f32` vs `f64` is settled as `f64` but kept
behind `WorldScalar`. Multi-grid coexistence is supported by passing different
specs (verified: one `WorldPos` resolves to different `VoxelCoord`s under coarse vs
fine specs).
