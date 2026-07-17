---
status: current
audience: agent
tags: [adr, voxel, mesh]
supersedes: []
see-also: []
---

# ADR 0007 — Voxel mesh / render / material seam

**Status:** Accepted

Meshing (06), mesh-payload protocol (07), and Three.js upload (08) are one seam, designed
together so the payload uploads without TypeScript per-frame transcoding.

- Rust `svc-mesh` emits a `MeshPayload`: **separate** `f32` position/normal streams, a `u32` index
  stream, material-slot **groups** `(slot, start, count)`, bounds, stats. Vertices are
  **chunk-local**; world placement is the render node `Transform`. Deterministic order (X-fastest
  voxels, `Direction6` faces). Naive visible-face first.
- `protocol-render` gains `Geometry::MeshSlot` (Copy marker), `RenderDiff::ReplaceMeshPayload`, and a
  generated `MeshPayloadDescriptor` (layout + groups + bounds; data **inline** for fixtures or by
  **buffer handle** for runtime per ADR 0006). Generated contracts stay the semantic border.
- `renderer-three` maps the descriptor 1:1 to `THREE.BufferGeometry` (`setAttribute`/`setIndex`/
  `addGroup`) and material slots to an indexed material registry (flat/debug initially; missing slot →
  debug fallback). Typed-array views only.

Material strategy is **not** baked to terrain atlases: the layout's attribute list can later add
`uv`/`color` streams (atlas terrain, or per-vertex-colour objects/characters) without a protocol break.

Non-goals (deferred): greedy meshing, atlas/UV packing, complex-shape UVs, interleaved buffers, u16
indices, LOD, instancing.

Implementation note: shipped as **`RenderDiff::ReplaceMeshPayload`-only** (no `Geometry::MeshSlot`
marker) — a mesh node is created as an ordinary node and has its geometry replaced, keeping
`Geometry` a closed/`Copy` enum and avoiding renderer/decoder exhaustive-switch churn.

Full design + failure routing: `docs/voxel-mesh-seam.md`.
