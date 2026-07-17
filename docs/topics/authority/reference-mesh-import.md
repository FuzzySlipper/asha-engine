---
status: current
audience: agent
tags: [voxel, mesh, import]
supersedes: []
see-also: []
---

# Reference Mesh Import

Task #5553 adds the first public engine-authorized reference-mesh ingestion path
for voxel conversion. Consumers call
`RuntimeSessionFacade.importVoxelConversionMeshSource` with generated
`VoxelConversionMeshSourceImportRequest` data. Hosts provide bytes and a logical
source path; Rust owns parsing, validation, content hashing, canonical geometry,
registration, and diagnostics.

## Supported GLB Subset

The initial `glb` format accepts:

- GLB 2.0 with an embedded BIN chunk;
- exactly one static mesh;
- one or more indexed `TRIANGLES` primitives;
- finite `POSITION` data and optional matching normals;
- primitive material bindings and material names;
- `KHR_materials_unlit` metadata, which does not change conversion geometry.

The importer rejects external buffers, animations, skins, morph targets,
non-triangle topology, missing indices, malformed accessors, and sources above
the published byte/vertex/index quotas. It performs no filesystem or network
access. Images and texture sampling are outside this import slice; texture-aware
voxel conversion continues to use the existing explicit texture DTOs.

## Resource Bounds

The native transport rejects serialized requests above 268,468,224 bytes before
JSON deserialization. Rust preflight then limits source bytes to 67,108,864,
source asset IDs to 1,024 UTF-8 bytes, source paths to 8,192 UTF-8 bytes, and
primitive selectors to 1,024 UTF-8 bytes before content hashing. GLB accessor
counts are checked cumulatively before canonical vectors are collected: at most
2,000,000 vertices and 6,000,000 indices. Quota rejection does not register a
source or invalidate the current conversion plan.

## Receipt And Provenance

The receipt does not echo source bytes. It returns the Rust-computed `sha256:`
source identity, canonical `VoxelConversionMeshAsset`, source bounds, vertex and
triangle counts, primitive groups, material slots, diagnostics, and a source
snapshot evidence ref. A successful import also registers that exact source for
the existing plan, preview, apply, metadata, and model readout operations.

The committed `kenney-wall-a.glb` fixture is from Kenney's Retro Urban Kit 2.0
under CC0. Its source license is preserved beside the fixture.
