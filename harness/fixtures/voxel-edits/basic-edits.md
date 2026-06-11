# Voxel edit replay fixture — basic-edits

Canonical voxel-edit event sequence used by `rule-voxel-edit`'s replay
reconstruction test (`replay_reconstructs_chunk_and_detects_generation_divergence`).
Replaying it on a fresh `VoxelWorld` reproduces the chunk byte-for-byte
(`content_hash` match); a wrong recorded generation hash surfaces a
`GenerationDivergence` rather than silently reconstructing different terrain.

Grid: `GridId(0)`, voxel_size 1.0, chunk dims 8³. Target chunk: `(0,0,0)`.

Event sequence (canonical `VoxelEditEvent`s):

1. `ChunkGenerated { grid: 0, chunk: (0,0,0), seed: 100, generator_version: 1, hash: <content_hash of generate_chunk(seed=100, v=1)> }`
2. `VoxelSet { grid: 0, coord: (0,7,0), value: Solid(2) }`
3. `VoxelRegionFilled { grid: 0, min: (1,0,1), max: (3,2,3), value: Solid(1) }`

Determinism contract: `generate_chunk(seed, chunk, version)` is a pure FNV-1a-derived
heightfield (no noise library); identical inputs always produce identical voxels, so
the recorded generation `hash` is a stable golden. Changing the generator is a
`generator_version` bump and a deliberate fixture-hash update.
