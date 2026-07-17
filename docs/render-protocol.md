---
status: current
audience: agent
tags: [render, protocol, projection]
supersedes: []
see-also: []
---

# Render protocol

## Model: retained-mode diffs

Rust does not send "everything to draw this frame."
It emits a compact diff of what changed:

```
create handle
update transform
replace mesh payload
set visibility
set material
destroy handle
emit debug overlay
```

The renderer maintains a scene graph keyed by handles. It applies each diff operation
to the relevant handle and lets Three.js manage the actual draw calls.

## Why retained-mode

- Traffic stays small — only changes travel across the boundary.
- Fixtures are simple — a diff batch is a short list of operations.
- Renderer tests are fixture-friendly — apply a diff, assert handle registry state.
- Agents have a bounded vocabulary to reason about.

## Protocol types

Defined in `engine-rs/crates/protocol/protocol-render`. Generated TypeScript lives in
`ts/packages/contracts/src/generated/render.ts`.

Key types:
- `RenderHandle` — opaque stable ID for a renderable object
- `RenderDiff` — one diff operation (`create`/`update`/`destroy`)
- `RenderFrameDiff` — ordered list of diffs for one tick
- `RenderNode` — a node's full description at create time (geometry, material, transform, visibility, layer, metadata)
- `Geometry` — abstract primitive (`cube`/`sphere`/`quad`/`point`/`line`)
- `Material` — placeholder appearance (flat RGBA colour + wireframe flag)
- `RenderLayer` — `scene` vs `debug` overlay

## Large payloads

Large geometry or buffer data travels through bridge-owned memory views, not structured messages.

Rules:
- Structured `RenderDiff` carries small metadata only.
- Large buffers use stable bridge memory views referenced by pointer+length or handle.
- Renderer upload behavior is isolated inside `@asha/runtime-bridge` and `renderer-three`.
- No policy package may access raw WASM memory.

## Renderer boundary rule

The renderer consumes diffs. It does not:
- Inspect `StateStore`.
- Import policy packages.
- Submit authority commands except through approved UI/app paths.

## Debug overlays

`render-debug` emits debug overlay diffs (point/line markers, labels) on the
`debug` `RenderLayer`. These are non-authoritative. They reuse the same retained
diff protocol, so the renderer can route them into a separate layer group and
toggle them without changing the core diff stream.

## Non-scene presentation

Audio, world-space UI, particles, animation-controller realization, and the
live telemetry overlay do not add unrelated variants to scene `RenderDiff`.
Their shared typed envelope, lifecycle, ordering, replay stance, handle
namespaces, and compatibility posture are fixed by
[`non-scene-projection-channel.md`](non-scene-projection-channel.md).

The implementation contract keeps scene diffs intact and places them beside an
ordered, generated `PresentationFrameDiff` for the same authority tick.
The implemented domains are the catalog-validated Web Audio path described in
[`audio-projection.md`](audio-projection.md) and retained world-space status/name
projection described in [`billboard-projection.md`](billboard-projection.md),
plus typed burst/retained emitters with renderer-owned particle simulation
described in [`particle-projection.md`](particle-projection.md), and the
machine-readable live snapshot plus disposable overlay described in
[`live-telemetry-overlay.md`](live-telemetry-overlay.md).

## Phase 5 dataflow and failure routing

The implemented dataflow, end to end:

```
Rust render-bridge (RenderProjector::project)
  → RenderFrameDiff  → render-bridge::json::encode_sequence
  → harness/fixtures/render-diffs/*.json   (shared, inspectable artifact)
  → @asha/runtime-bridge decodeRenderFrameDiff (TS: validate into contract types; backs readRenderDiffs)
  → renderer-three ThreeRenderer.applyFrame (TS: apply to handle registry + Three.js scene)
```

The renderer's scene changes **only** through applied diffs — never by reading
`StateStore` or importing policy/core packages (enforced by the depgraph).

When the cross-language fixture path breaks, the failing layer routes the repair:

| Symptom | Likely lane / crate |
|---|---|
| `render-bridge` golden test `bridge_emits_the_committed_render_fixture` fails | `rust-render` (bridge projection) or `contract-steward` (render protocol shape) — regenerate the fixture |
| generated `render.ts` drift (`check-contracts`) | `contract-steward` (`protocol-render` + codegen) |
| `@asha/runtime-bridge` `RenderDecodeError` | `ts-shell` (`runtime-bridge` render decoder) or a fixture/contract mismatch |
| `renderer-three` `RenderApplyError` (duplicate/unknown/stale handle) | `ts-shell` (`renderer-three` handle registry) |
