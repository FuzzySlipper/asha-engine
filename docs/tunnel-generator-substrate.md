# Deterministic Tunnel Generator Substrate

Task #4038 adds the first generic generated-level substrate for enclosed voxel
tunnel spaces. The public Rust import path is:

```rust
use svc_levelgen::{generate_tunnel, TunnelGeneratorConfig};
```

The generator is upstream ASHA engine infrastructure, not demo code. It validates
a small `TunnelGeneratorConfig`, consumes explicit seeded randomness from
`svc-rng`, emits authoritative voxel data as a `svc_spatial::VoxelWorld`, and
records stable replay/hash metadata in `TunnelGenerationRecord`.

## Tiny Preset

`TunnelGeneratorConfig::tiny_enclosed(seed)` produces a single-chunk enclosed
tunnel fixture:

- grid id `0`, voxel size `1.0`, chunk dims `8 x 6 x 12`
- tunnel dims `5 x 4 x 9`
- wall material `1`, floor material `2`, accent material `3`
- two generic spawn markers: `player_start` and `exit_hint`
- one `ChunkGenerated` event-equivalent record for replay/hash checks

The seed changes deterministic metadata/material placement while preserving the
validated shape. The generator rejects degenerate voxel sizes, dimensions too
small to enclose walkable space, dimensions exceeding the chunk, and duplicate
material-role IDs.

## Projection Evidence

The generator itself does not render and does not own collision queries. Its
output is consumed by the existing projection lanes:

- collision: `svc_collision::CollisionProjection::build(&generated.world)`
- render: `render_bridge::VoxelChunkProjector::project_coords(&generated.world, coords)`

Committed evidence:

- `harness/fixtures/generated-levels/tiny-tunnel.snapshot.txt`
- `harness/goldens/render-diffs/generated-tunnel.snapshot`

These fixtures prove deterministic generation, stable replay/hash metadata,
collision shell availability, and render projection availability without adding
demo-specific behavior.
