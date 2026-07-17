---
status: current
audience: agent
tags: [voxel, asset, format]
supersedes: []
see-also: []
---

# Asha Voxel Volume Asset Format

Status: accepted first engine format for task #4816.

## Decision

Asha stores durable voxel creations, edited voxel volumes, and converted voxel
volumes as an Asha-native JSON envelope:

- Extension: `.avxl.json`
- Media type: `application/vnd.asha.voxel-volume+json;version=1`
- Schema version: `1`
- Public DTO source: `protocol-voxel-asset`
- Rust authority service: `svc-voxel-asset`
- Generated TypeScript surface: `@asha/contracts` root export, module
  `voxelAsset`

`.voxel` was only a planning shorthand. `.vforge` is explicitly rejected for this
phase: Asha does not promise VoxelForge import/export compatibility, does not
inherit VoxelForge's project lifecycle, and does not preserve VoxelForge's MCP or
sidecar control plane.

## Format Shape

The top-level `VoxelVolumeAsset` records:

- `assetId`: a `voxel-volume/...` asset id validated by Rust.
- `schemaVersion` and `mediaType`: fail-closed version/media gates.
- `grid`: origin, cell size, and coordinate-system tag.
- `bounds`: inclusive stored voxel-space bounds.
- `representation`: schema-v1 uses `sparse_runs`.
- `materialPalette`: compact `u16` voxel material ids mapped to named palette
  entries, validated catalog `material/...` asset ids, and optional durable
  material catalog binding ids.
- `provenance`: authored, converted, runtime-export, or imported-reference
  evidence refs.
- `authoring`: label/source-tool/editor metadata with no authority rights.
- `validationDiagnostics`: persisted observational diagnostics.
- `contentHashes`: authority-computed canonical JSON and voxel-data hashes.

The sparse-run payload stores solid voxels only. Empty space is absence. Each run
starts at `start` and extends `length` cells along +X with one compact material
id. Bounds, material palette membership, duplicate palette/binding identifiers,
duplicate occupied coordinates, and hash consistency are Rust-validated.

## Authority Rules

Rust owns validation, serialization/deserialization, canonical hashing,
diagnostics, and stored/runtime transition rules.

TypeScript may author JSON-shaped data and display diagnostics through generated
DTOs. TypeScript must not decide whether a voxel asset is valid, mutate runtime
SessionState directly, or read engine private crates/transports.

Stored `VoxelVolumeAsset` data is durable ProjectBundle/catalog content. Named
voxel material palette entries live on the stored asset because they are
authoring/catalog bindings, not live runtime authority. Runtime voxel
SessionState consumes compact material ids through Rust-validated load/edit
operations. Moving runtime state into a stored asset is an explicit export
operation:

1. Select a runtime volume/session source.
2. Produce a proposed `VoxelVolumeAsset` with `runtime_export` provenance.
3. Compute a diff against any existing stored asset.
4. Run Rust validation and canonical hashing.
5. Save only through an explicit ProjectBundle/catalog write.

There is no silent promotion from SessionState to stored ProjectBundle data.

Durable palette editing uses the separate
`updateVoxelVolumeAssetPalette` transaction. The caller supplies the current
stored asset, a complete bounded replacement palette, required expected
canonical and voxel-data hashes, and a ProjectBundle target path. Rust validates
both the current asset and replacement, preserves the voxel-data hash, and
returns a canonical updated asset plus stored diff. The operation has an
immutable runtime bridge receiver and cannot update resident SessionState.

## Versioning And Migration

Schema version `1` supports JSON plus `sparse_runs`. Unknown newer schema versions
fail closed. A future schema can add a binary payload or manifest-plus-payload
layout, but it must introduce a new media type/version and a Rust migration plan.

The current JSON envelope is chosen because small authored and converted volumes
need inspectability, reviewability, deterministic hashing, and direct Studio
round-trip work before optimizing for large binary payloads.

## Studio-Facing Surface

Studio should consume only public generated DTOs from `@asha/contracts` and the
eventual public runtime/session facade verbs that wrap `svc-voxel-asset`.

The follow-up Studio save/load/export workflow can now depend on:

- `VoxelVolumeAsset`
- `VoxelAssetRepresentation`
- `VoxelAssetSparseRun`
- `VoxelAssetMaterialBinding`
- `VoxelVolumeAssetPaletteUpdateRequest`
- `VoxelVolumeAssetPaletteUpdateReceipt`
- `VoxelVolumeAssetPaletteStoredDiff`
- `VoxelAssetProvenanceRef`
- `VoxelAssetDiagnostic`
- `VOXEL_ASSET_SCHEMA_VERSION`
- `VOXEL_ASSET_MEDIA_TYPE`
- `VOXEL_ASSET_EXTENSION`

Studio material choosing and named palette editing should project and mutate
these public fields through generated contracts plus the public
export/save/load/palette-update facade. Studio must not keep its own hidden material-binding
model for saved voxel assets.

It must not import `svc-voxel-asset`, protocol crate internals, private bridge
transports, or any VoxelForge `.vforge` parser.
