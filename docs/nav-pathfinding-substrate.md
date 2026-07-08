# Navigation Pathfinding Substrate

Task #4041 activates the upstream `svc-pathfinding` lane with a read-only
navigation projection and deterministic path query over voxel authority.
Task #5028 adds a separate opt-in 3D/volumetric query for bounded procgen
connectivity experiments. The default navigation substrate remains planar
walkable-surface pathfinding.

The public Rust import path is:

```rust
use svc_pathfinding::{
    build_nav_projection, find_path, find_volumetric_path,
    propose_projected_direct_nav_movement, NavPathQuery, NavProjectionConfig,
    ProjectedDirectNavMovementRequest, VolumetricNavConfig, VolumetricNavQuery,
};
```

This is projection/query infrastructure only. It does not implement enemy AI,
policy behavior, demo wiring, or movement authority.

## Named Surface

- Projection config: `NavProjectionConfig`
- Read-only projection: `NavProjection`
- Query: `NavPathQuery`
- Readout: `NavPathReadout`
- Outcome: `NavPathOutcome::{Reached, NoPath}`
- Rejections: `NavError::{InvalidAgentHeight, InvalidQueryBudget,
  StartNotWalkable, GoalNotWalkable}`
- Projection-backed direct navigation: `ProjectedDirectNavMovementRequest`,
  `propose_projected_direct_nav_movement`, `ProjectedDirectNavMovementReadout`,
  `ProjectedDirectNavMovementError`
- Optional volumetric navigation: `VolumetricNavQuery`,
  `VolumetricNavConfig`, `VolumetricAgentVolume`, `VolumetricNeighborSet`,
  `VolumetricVerticalPolicy`, `VolumetricTraversalRule`,
  `find_volumetric_path`, `VolumetricNavReadout`,
  `VolumetricNavOutcome::{Reached, NoPath, BudgetExhausted}`,
  `VolumetricNavError::{InvalidAgentVolume, InvalidQueryBudget,
  StartNotTraversable, GoalNotTraversable}`

`build_nav_projection` reads a `svc_spatial::VoxelWorld` and marks walkable
cells where the agent has empty vertical clearance and, by default, a solid
floor. `find_path` runs deterministic shortest-path search over the projection
using fixed X/Z neighbor order. The projection is read-only evidence suitable
for future policy views to inspect before proposing movement.

`propose_projected_direct_nav_movement` converts live positions into the
projection grid, queries `find_path`, and proposes one bounded waypoint toward
the next path cell (or the final target when the goal cell is next). The service
keeps no internal cache. Callers that cache externally must invalidate on the
readout/projection `projection_hash`; each movement readout also carries the
`path_hash` and a deterministic `movement_hash`.

## Planar Default Versus Volumetric Opt-In

`NavProjection`, `NavPathQuery`, and `find_path` are the default 2D/2.5D
surface-navigation substrate. They derive walkable cells from resident voxel
authority, require empty agent clearance, require a solid floor by default, and
expand only fixed X/Z neighbors. Existing runtime movement and tunnel fixtures
use this path.

`VolumetricNavQuery` is a separate opt-in 3D query surface. It does not build a
walkable-surface projection and it does not replace planar movement. It checks a
bounded resident voxel volume directly with explicit semantics:

- `VolumetricAgentVolume` declares the whole-voxel X/Y/Z occupied volume.
- `VolumetricNeighborSet` declares planar four-neighbor or six-face expansion.
- `VolumetricVerticalPolicy` independently allows or rejects vertical neighbors.
- `VolumetricTraversalRule` declares whether traversable resident cells are
  empty or solid.
- `max_visited` is required and returns `BudgetExhausted` when the query would
  exceed its search budget.

Missing or unloaded chunks are non-traversable for volumetric mode. This is
intentional: treating absent data as empty would let a procgen check leak into
unbounded implicit space. Callers should choose small budgets and use this
surface for generation/conformance checks, not as an always-on runtime default.

Both planar and volumetric readouts expose deterministic cost/audit data:
visited count, path length (`path.len()` for planar, `path_len` for volumetric),
and stable path hashes. The volumetric readout also carries
`NavQueryMode::Volumetric3d` so downstream evidence cannot confuse it with the
planar surface path.

## Evidence

The committed fixture uses the #4038 generated tunnel:

- `harness/fixtures/nav/generated-tunnel-path.snapshot.txt`

The focused tests cover:

- reachable path from `exit_hint`-style marker to `player_start`-style marker
- blocked/no-path projection
- deterministic path hash
- invalid query rejection for an unwalkable start
- projection-backed direct navigation obstacle/path following, no-path,
  same-cell reached behavior, invalid inputs/endpoints, and deterministic
  projection/path/movement hashes
- opt-in volumetric navigation over vertical connected spaces, separated
  unreachable volumes, budget exhaustion, invalid/non-traversable endpoints,
  agent-volume clearance, disabled vertical policy, deterministic output, and
  preservation of existing planar default hashes

Fixture values:

- walkable cells `66`
- projection hash `d1f6ac3e051d6b6e`
- path hash `e8e1ea7a09811ced`
