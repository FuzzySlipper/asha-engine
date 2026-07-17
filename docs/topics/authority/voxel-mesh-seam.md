---
status: current
audience: agent
tags: [voxel, mesh, render]
supersedes: []
see-also: []
---

# Voxel mesh / render / material seam

> Companion ADR: `governance/adr/0007-voxel-mesh-seam.md`.
> Source designs: Den `voxel-capability-06-voxel-meshing`, `-07-mesh-payload-render-protocol`,
> `-08-threejs-voxel-renderer-path`, `-14-materials-assets-visual-style`; ADR 0006 (runtime bridge).

This is the **coordinated** design for the single seam from Rust meshing output →
generated protocol descriptors / buffer handles → Three.js `BufferGeometry`. Meshing
(#2261), protocol (#2262), and renderer upload (#2263) are implemented from this doc so the
payload layout is uploadable without TS-side per-frame transcoding. It is the cold-start
reference for those tasks.

## 1. Decision in one paragraph

The Rust mesher emits a `MeshPayload` of **separate** `f32` position/normal streams + a `u32`
index stream + material-slot **groups** + bounds/stats, with vertices in **chunk-local** space
(world placement via the node transform). The render protocol carries a **`MeshPayloadDescriptor`**
(layout + groups + bounds metadata) whose vertex/index data is either **inline** (small golden
fixtures) or referenced by a **buffer handle** (runtime, via the runtime bridge). The renderer maps
the descriptor 1:1 onto `THREE.BufferGeometry` (`setAttribute`/`setIndex`/`addGroup`) and material
slots onto an indexed material registry — typed-array/view creation only, no transcoding.

## 2. Layout choices (the invariants all three lanes share)

| Choice | Decision | Why |
|---|---|---|
| Attribute streams | **separate** (`positions`, `normals`; later `uv`, `color`) | maps directly to `BufferGeometry.setAttribute(name, Float32BufferAttribute(data, 3))`; no interleave unpack |
| Position scalar | **`f32`** | render border is f32; `THREE` wants `Float32Array` |
| Vertex space | **chunk-local** (origin = chunk min corner) | keeps f32 precise for large worlds; the chunk child carries an asset-local origin and its retained instance root carries scene TRS |
| Index width | **`u32`** | one width everywhere; supports large chunks. u16 optimisation deferred |
| Material grouping | **groups** `(material_slot, start, count)` over the index range | maps 1:1 to `BufferGeometry.addGroup(start, count, materialIndex)` |
| Winding / normals | CCW front faces, per-face outward normals (axis-aligned) | deterministic, matches `Direction6` |

## 3. `MeshPayload` (Rust, `svc-mesh`, #2261)

```rust
pub struct MeshPayload {
    pub positions: Vec<f32>,   // 3 per vertex, chunk-local
    pub normals:   Vec<f32>,   // 3 per vertex
    pub indices:   Vec<u32>,   // 3 per triangle
    pub groups:    Vec<MeshGroup>,
    pub bounds:    MeshBounds,
    pub stats:     MeshStats,
}
pub struct MeshGroup  { pub material_slot: u16, pub start: u32, pub count: u32 } // count = #indices
pub struct MeshBounds { pub min: [f32; 3], pub max: [f32; 3] }                   // chunk-local
pub struct MeshStats  { pub vertices: u32, pub indices: u32, pub quads: u32,
                        pub faces_emitted: u32, pub faces_culled: u32 }
```

- **Material slot** = the voxel's `VoxelMaterialId` (u16) at emit time. Faces are grouped by slot
  (groups sorted by `material_slot`, deterministic). A material boundary forbids merging faces.
- **Deterministic order**: emit per voxel in `core-space` X-fastest order, faces in `Direction6::ALL`
  order; vertices/indices appended in that order. Two builds of the same chunk produce identical bytes.
- The mesher is **naive visible-face** first (six faces per solid voxel, internal faces culled,
  border faces culled against resident neighbours via `VoxelChunk::border_layer` / neighbour chunks).

## 4. `MeshPayloadDescriptor` (protocol-render, #2262)

Generated into `ts/packages/contracts/src/generated/render.ts`. Structured metadata is inline;
bulk vertex/index data travels by **handle** at runtime, **inline** for fixtures.

```text
MeshAttributeKind = 'f32'                         // (only f32 today; extensible)
MeshAttributeName = 'position' | 'normal' | 'uv' | 'color'   // uv/color reserved, unused initially
MeshAttribute { name: MeshAttributeName, components: u8, kind: MeshAttributeKind }
MeshIndexWidth = 'u32'

MeshBufferLayout {
  vertexCount: u32,
  indexCount: u32,
  indexWidth: MeshIndexWidth,
  attributes: MeshAttribute[],          // e.g. [position×3 f32, normal×3 f32]
}
MeshGroupDescriptor { materialSlot: u16, start: u32, count: u32 }
MeshBoundsDescriptor { min: [f32;3], max: [f32;3] }

MeshPayloadSource =
  | { kind: 'inline'; positions: f32[]; normals: f32[]; indices: u32[] }     // golden fixtures
  | { kind: 'handle'; buffer: number /*RuntimeBufferHandle*/; positionsByteOffset, normalsByteOffset, indicesByteOffset, ... }  // runtime

MeshPayloadDescriptor {
  layout: MeshBufferLayout,
  groups: MeshGroupDescriptor[],
  bounds: MeshBoundsDescriptor,
  source: MeshPayloadSource,
}
```

Render-diff variant (mesh geometry is replaced in place; identity/material/transform stay on the
node, so chunk remeshes don't churn the handle):

```text
Geometry::MeshSlot                       // NEW Copy marker: "geometry is an uploaded mesh payload"
RenderDiff::ReplaceMeshPayload { handle, payload: MeshPayloadDescriptor }
```

A voxel scene node renders as a retained root with validated translation,
quaternion rotation, and non-uniform scale. Chunk nodes are created beneath that
root with `parent: rootHandle` and an asset-local chunk-origin transform, followed
by `ReplaceMeshPayload { handle, payload }`. Multiple roots may share one voxel
asset; each owns distinct retained child handles while a local edit emits the
same remeshed payload for every live instance. Moving one instance is a root-only
`Update`. Removing, replacing, or closing an instance destroys its root and,
by render-protocol convention, all descendants. Handles are not reused.

### Runtime-bridge tie-in (ADR 0006)
The `handle` source references a bridge-owned buffer (`runtime-bridge` `getBuffer`/`releaseBuffer`):
the descriptor's byte offsets + layout let TS wrap the bytes as `Float32Array`/`Uint32Array` **views**
with no copy. Generated contracts stay the semantic border; the buffer bytes are transport.

## 5. Materials (#2263 + future)

- Mesh `material_slot` (u16) is abstract — **not** a terrain atlas index. The renderer holds a
  **material registry**: `slot → THREE.Material`. Initial strategy: **flat/debug** materials
  (flat colour per slot), enough for screenshot/golden + slot plumbing.
- Missing slot → a **debug fallback material** (visible magenta), never a silent skip.
- **Forward-compatible**: the layout's `attributes` list can later add `uv`/`color` streams for the
  terrain-atlas strategy or the per-vertex-colour (object/character) strategy — without changing the
  descriptor shape. No strategy is baked in.

## 6. Failure routing (decision 06§5 / 07§5)

| Symptom | Owner lane / crate |
|---|---|
| wrong voxels feed the mesher | `rust-service` · `svc-volume` (chunk get/set) |
| wrong faces / culling / vertex order / stats | `rust-service` · `svc-mesh` (the mesher) |
| descriptor shape drift, generated TS mismatch | `contract-steward` · `protocol-render` + codegen (`check-contracts`) |
| buffer bytes wrong / stale handle | `ts-shell`/native · `runtime-bridge` + `native-bridge` (buffer transport) |
| geometry/attribute/group/index upload wrong | `ts-shell` · `renderer-three` (`BufferGeometry` upload) |
| material slot maps to wrong appearance | `ts-shell` · `renderer-three` material registry |

## 7. Non-goals (explicit, deferred)

Greedy meshing / face merging (naive visible-face first); texture-atlas packing + UV generation;
complex/non-cubic-shape material/UV assignment; interleaved attribute buffers; `u16` index
optimisation; LOD/simplification; GPU instancing/merged-region batching; `three-mesh-bvh`. The layout
leaves room for each (extra attribute streams, group remapping) without a protocol break.

## 8. Implementation sequencing

1. **#2261** `svc-mesh`: naive visible-face mesher → `MeshPayload` + golden fixtures (deterministic).
2. **#2262** `protocol-render` + codegen: `MeshPayloadDescriptor`/`MeshSlot`/`ReplaceMeshPayload`
   contracts + generated `render.ts` (regenerate, `check-contracts`), construction + malformed tests.
3. **#2263** `renderer-three`: `BufferGeometry` upload from a descriptor + material-slot registry +
   disposal lifecycle; a tiny golden fixture proving payload → geometry with view-only conversion.
Each step keeps existing checks green (`check-contracts`, `check-render-goldens`, depgraph).
