# Rust-Owned Voxel Conversion Texture Sampling

Status: design record for Den task #4596, with the first implementation slice
landed by Den task #4912.

## Purpose

ASHA voxel conversion maps source material slots to voxel material ids in Rust.
Task #4912 adds the first Rust-owned texture/UV sampling slice: callers may
provide generated DTOs for authority-visible texture sample assets and per-slot
UV sample bindings. The service validates hashes and deterministic sampling
policy before output generation. This document keeps the authority lane explicit
so later implementation does not drift into Studio, renderer, Three.js, or raw
image-buffer authority.

The durable path remains:

```text
generated voxel-conversion DTOs
  -> RuntimeSessionFacade / runtime bridge
  -> Rust source asset snapshot validation
  -> svc-voxel-conversion or a dedicated Rust sampler service
  -> deterministic material/sample diagnostics and voxel output
```

## Authority Rules

- Rust owns texture sampling, filtering, hashing, and diagnostics.
- Generated protocol DTOs describe source texture refs, UV attributes, sampling
  policy, and accepted outputs.
- Studio and downstream projects may author references and display diagnostics.
  They must not provide renderer-resident pixels as trusted conversion input.
- Renderer buffers, Three.js texture state, browser canvas data, or local image
  memory are projection/tooling data unless Rust has imported and hashed them as
  authority-visible assets.
- A conversion request that asks for texture sampling without authority-visible
  texture snapshots fails closed with typed diagnostics and no partial output.

## Protocol Additions

The generated voxel-conversion protocol now has a small additive texture
sampling family for the first implementation slice:

```rust
pub struct VoxelConversionUvAttributeRef {
    pub attribute_name: String,
    pub source_hash: String,
}

pub struct VoxelConversionTextureSourceRef {
    pub texture_asset_id: String,
    pub asset_version: u64,
    pub content_hash: String,
    pub width: u32,
    pub height: u32,
    pub color_space: String,
    pub channel_layout: String,
}

pub struct VoxelConversionTextureSampleAsset {
    pub texture: VoxelConversionTextureSourceRef,
    pub texel_materials: Vec<u16>,
}

pub struct VoxelConversionTextureBinding {
    pub source_material_slot: u32,
    pub texture: VoxelConversionTextureSourceRef,
    pub uv_attribute: VoxelConversionUvAttributeRef,
    pub sample_uv: [f32; 2],
    pub sampling_policy: String,
    pub wrap_policy: String,
    pub material_mode: String,
}
```

Current accepted values:

- `VoxelConversionTextureColorSpace`: `linear`, `srgb`.
- `VoxelConversionTextureChannelLayout`: `palette_index_u16`.
- `VoxelConversionTextureSamplingPolicy`: `nearest_texel`.
- `VoxelConversionTextureWrapPolicy`: `clamp_to_edge`.
- `VoxelConversionTextureMaterialMode`: `sample_palette_index`.

The fields remain string-valued at the DTO border so Rust authority can return
typed `unsupported_sampling_policy`, `unsupported_texture_format`, or
`invalid_texture_material_rule` diagnostics for unsupported future values rather
than accepting them through TypeScript type pressure.

## Source Snapshot Model

Texture input must be represented as an authority-visible source snapshot:

- texture asset id and version;
- content hash of decoded canonical pixels, not only source file bytes;
- canonical dimensions and channel layout;
- color-space interpretation;
- optional import settings hash when decoding/transcoding affects samples;
- UV attribute hash for the source mesh primitive.

The source snapshot can be produced by an asset-import lane, a dedicated Rust
texture-snapshot service, or the voxel conversion service itself. In every case,
the conversion request references hashes that Rust can validate before sampling.

If a downstream tool has only a browser/renderer texture, it must first route
that texture through the same import/snapshot process. It cannot hand pixels to
conversion as trusted runtime memory.

## Deterministic Sampling Rules

Initial texture sampling should be intentionally boring:

- UV bindings identify a named source mesh UV attribute and hash.
- The first implementation uses an explicit representative `sample_uv` per
  source material slot. Full per-vertex UV interpolation is not implemented yet.
- UV wrapping is explicit per binding. The initial allowed value should be
  `clamp_to_edge`; repeat/mirror can be added later.
- Filtering is explicit per binding. The initial implementation should support
  `nearest_texel`; bilinear support must define exact rounding and color-space
  conversion before acceptance.
- `palette_index_u16` texels map directly to ASHA voxel material ids. Raw
  sampled colors are evidence/projection only unless a generated DTO explicitly
  defines them as authority output.
- All floating-point decisions that affect material assignment must be covered
  by deterministic tests and stable summary hashes.

## Diagnostics

Texture diagnostic codes are part of the generated protocol vocabulary:

- `missing_texture_source`: a requested texture ref is not authority-visible.
- `texture_hash_mismatch`: request hash does not match the validated snapshot.
- `missing_uv_attribute`: source mesh lacks the named UV attribute.
- `unsupported_texture_format`: layout/color-space is not supported.
- `unsupported_sampling_policy`: requested sampling/filter/wrap is unsupported.
- `invalid_texture_material_rule`: palette/threshold/material output mapping is
  malformed or incomplete.

Unsupported texture sampling is an error for requests that require it, not a
best-effort fallback to source material slots. Callers can choose material-slot
mapping explicitly when they want that fallback behavior.

## Evidence And Hashes

Plans, previews, receipts, model-info readouts, and exported evidence should
include texture sampling facts when texture sampling participates in output:

- texture snapshot refs and content hashes;
- UV attribute refs and hashes;
- sampling policy and material rule hashes;
- output material counts;
- stable sample summary hash;
- diagnostics for skipped or rejected texture bindings.

These facts are readouts. They do not grant Studio or TypeScript mutation rights.

## Implementation Status

- Implemented in #4912: generated DTOs, Rust service validation, nearest-texel
  `palette_index_u16` sampling, texture/hash diagnostics, and deterministic
  service tests for sampled output, missing texture, stale hash, and unsupported
  sampling policy.
- Still future work: per-vertex UV buffers, barycentric/cell sampling,
  mipmaps, bilinear filtering, repeat/mirror wrapping, image decode/import,
  color-to-palette rules, average-color material selection, atlas packing, and
  Studio UI affordances.

## Non-Claims

- It does not permit Studio or renderer-owned texture data to become authority.
- It does not define atlas packing, material authoring UI, complex PBR material
  evaluation, mip generation, per-vertex UV interpolation, image import, or
  GPU-assisted sampling.
- It does not require changing current material-slot mapping behavior.
