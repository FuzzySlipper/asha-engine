---
status: current
audience: agent
tags: [voxel, authority, rust, typescript]
supersedes: []
see-also: [runtime-session-facade.md, workspace-authoring-facade.md]
---

# Voxel Authority

This topic covers ASHA's voxel data model, coordinate system, meshing, editing, conversion, annotation, and asset lifecycle. Voxel authority is Rust-owned; TypeScript projects and proposes.

## Coordinate Foundation

`core-space` (rust-foundation, std-only, zero deps) defines the canonical voxel coordinate system. Right-handed Y-up. Grid/chunk/cell addressing is explicit and typed. See `docs/voxel-coordinates.md` for the full foundation.

## Mesh / Render / Material Seam

Rust owns mesh generation through `svc-mesh`. The render protocol carries mesh payload descriptors, not raw geometry. The material seam maps `VoxelMaterialId` to catalog material assets. See `docs/voxel-mesh-seam.md` for the ADR and design.

## UI / Editor Architecture

The voxel editor uses a composition root with explicit ports. Editor state is pure TypeScript; voxel authority is Rust. See `docs/voxel-ui-architecture.md` for the ADR and architecture.

## Bulk Edit Transactions

Large authored voxel edits go through `rule-voxel-edit` transactions wrapping the canonical `VoxelCommand` union (`SetVoxel`, `FillRegion`, `GenerateChunk`). Transactions are atomic, validated, and replayable. See `docs/bulk-voxel-edit-transactions.md`.

## Edit History, Undo, and Revert

Rust owns a timeline of voxel edits with ProjectBundle persistence. Generated protocol and RuntimeBridge surfaces expose read/revert/undo/redo. Studio is a projection and intent client; it does not own the undo stack. See `docs/voxel-edit-history.md`.

## Annotation and Semantic Regions

Voxel annotation layers provide semantic region markup on top of voxel volumes. Draft and finalized layers have distinct validation paths. Runtime annotation authority is Rust-owned. See `docs/voxel-annotation-regions.md`.

## Volume Asset Format

`VoxelVolumeAsset` is the durable stored format for converted voxel models. It carries sparse runs, material palette bindings, provenance refs, and canonical JSON. `svc-voxel-asset` validates and hashes. See `docs/voxel-volume-asset-format.md`.

## Conversion and Texture Sampling

Mesh-to-voxel conversion uses typed plans, previews, and authority application. Texture sampling uses nearest-texel `palette_index_u16` for the first slice. See `docs/voxel-conversion-texture-sampling.md` and `docs/reference-mesh-import.md`.

## Launchable Voxel Loop

The end-to-end "first launchable" path: boot runtime, load canonical voxel world, project to renderer, pick/select, preview/commit edit, see render update, save/reload/replay. See `docs/launchable-voxel.md` for commands, fixtures, and known limitations.
