---
status: current
audience: agent
tags: [projection, renderer, downstream, authoring]
supersedes: []
see-also: [editor-viewport.md, ../consumer/consumer-compatibility.md]
---

# Interactive renderer inspection surface

`@asha/renderer-host` exposes `mountAshaRendererInspectionSurface` for tools
that need to inspect generated or authored `RenderFrameDiff` content in a real
engine-owned browser renderer without becoming a game runtime or importing a
renderer backend.

The surface is explicitly `projection_only_inspection`. Its orbit camera,
focused WASD movement, drag state, resize state, and picks are disposable
renderer state. They do not configure a `RuntimeSession`, submit gameplay input,
authorize edits, or create replay truth. A tool that needs an authoritative game
camera must use the RuntimeSession input and camera operations instead.

## Public mount and projection channels

```ts
import { mountAshaRendererInspectionSurface } from '@asha/renderer-host';

const viewer = await mountAshaRendererInspectionSurface(canvas, {
  autoStart: true,
  initialGrid: projectGrid,
  controls: {
    initialPosition: [6, 5, 9],
    initialTarget: [0, 1, 0],
  },
});

const receipt = viewer.replaceFrame(generationResult.renderFrame);
if (!receipt.applied) {
  showDiagnostics(receipt.diagnostics);
}

const runtimeReceipt = viewer.applyRuntimeFrame(runtimeSessionFrame);
if (!runtimeReceipt.applied) {
  showDiagnostics(runtimeReceipt.diagnostics);
}

// Run restart, project switch, or explicit runtime teardown:
viewer.clearRuntimeProjection();
```

`replaceFrame` treats its argument as the complete retained inspection result.
Replacement is atomic: malformed frames, invalid handles, policy violations, or
backend resource failures return a rejected receipt while the last accepted
frame remains visible. A malformed `frame` passed during mount rejects the mount
and disposes the prepared renderer.

`applyRuntimeFrame` incrementally retains engine-produced runtime projection on
the viewport's dedicated `runtime` channel. It does not replace or mutate the
complete authored result supplied through `replaceFrame`. Runtime frames use the
same bounded frame/history validation, handle namespacing, atomic backend swap,
picking, and optional `bufferSource` upload path as the editor viewport.
`clearRuntimeProjection` resets only that runtime channel for run restarts,
project switches, or teardown.

The readout reports `runtimeGeneration`, `runtimeFrameHash`, and
`runtimeRetainedOpCount` separately from the authored `retainedFrameHash` and
`retainedOpCount`. These are bounded projection diagnostics, not gameplay state,
replay evidence, or authority receipts. A rejected malformed, over-limit, or
resource-invalid runtime frame leaves the previous runtime generation, hash, and
retained operations intact.

The helper reuses the engine-owned editor viewport for retained frame
validation, backend realization, picking, buffer-backed resources, resize, and
disposal. It reuses the renderer-host stored-camera resolver for the Y-up orbit
camera. No Studio camera implementation or downstream Three.js code is copied.

Primary-button drag orbits the stored inspection target. WASD moves the camera
and target together while the canvas has keyboard focus. Focused arrow keys
orbit independently of pointer input; focused `+`/`-` keys and the mouse wheel
dolly between configured minimum and maximum distances. Pitch is clamped before
the camera reaches its Y-up poles. Only mapped controls consumed by the focused
canvas prevent browser scrolling.

Drag uses one pointer-event stream with pointer identity and capture, so movement
continues coherently across the canvas boundary until pointer release. Pointer
cancellation, capture loss, canvas/window blur, page visibility loss, `stop`, and
`dispose` clear retained input state.

`initialGrid`, `setGrid(descriptor)`, `setGrid(null)`, and `grid()` expose the
same generated `EditorGridDescriptor` and procedural grid realization used by
the editor viewport. Grid intent never enters retained scene channels and does
not become authored or runtime authority. The inspection readout includes the
current grid plus grid revision, camera distance/revision, last camera-change
kind, drag state, and focused key sets so downstream smoke checks can distinguish
pointer orbit, keyboard orbit, movement, zoom, and grid application.

The editor viewport establishes shared runtime/authored scene depth before drawing
the grid with depth testing enabled and depth writes disabled. A grid plane placed
slightly above a floor therefore remains visible across that surface, while walls
and other geometry nearer the camera occlude it. The explicit overlay channel is
still drawn last after its depth clear; the grid is not an X-ray overlay.

`resizeToCanvas` samples the canvas dimensions, `resize` accepts an explicit
bounded size, and a browser `ResizeObserver` keeps the surface synchronized when
available. `start`, `stop`, `renderOnce`, and idempotent `dispose` own the
complete render and input lifecycle.

## Consumer boundary

The reusable `downstream-authoring` role remains renderer-free. Tools needing
this surface use the narrower `downstream-visual-authoring` role. That role may
consume these package roots:

- `@asha/contracts`
- `@asha/game-workspace`
- `@asha/renderer-host`
- `@asha/runtime-bridge`
- `@asha/runtime-session`

It may import `@asha/renderer-host` only from the package root. It remains
forbidden from `@asha/render-projection`, `@asha/renderer-three`, bare `three`,
renderer-host private paths, raw native/WASM transports, generated internals,
and Rust crate paths. Backend selection and renderer object ownership therefore
remain upstream even when a downstream tool owns the surrounding tabs and UI.

## Asha Procgen handback

For Procgen task #5980:

1. Classify the browser workbench as `downstream-visual-authoring` and validate
   its imports against the engine public-surface manifest.
2. Keep the current evidence/result view. Add the interactive renderer as a
   separate 3D viewer tab rather than replacing diagnostic evidence.
3. Import only `mountAshaRendererInspectionSurface` and public types from the
   `@asha/renderer-host` root. Do not add `three`, `@asha/renderer-three`, or an
   engine package private path.
4. Convert an accepted authored Procgen result into one complete
   `RenderFrameDiff`, then pass it to `replaceFrame`. When the workbench attaches
   a real RuntimeSession, forward each emitted `RenderFrameDiff` through
   `applyRuntimeFrame` and call `clearRuntimeProjection` on run restart or
   project switch. Do not merge those two channels downstream.
5. Preserve the runtime buffer source when mounting so handle-backed voxel mesh
   frames use the engine upload path. Display rejected receipts instead of
   falling back to a downstream renderer or reference runtime.
6. Dispose the surface when the tab/workbench is destroyed and resize it when
   the viewer panel changes dimensions.
7. Use `initialGrid` or `setGrid` for the project grid; do not draw a Procgen-local
   grid. Exercise primary drag, arrow orbit, focused zoom, and grid replacement
   in the live workbench and record the inspection readout as task evidence.
8. For the Voxel 3D and planned CA trace views, place the XZ grid plane at the
   intended cell boundary (for example, just above a floor surface when the grid
   should remain legible). Runtime and authored voxel meshes share depth, walls
   occlude the grid normally, and debug overlays retain their final-pass precedence.

The surface proves visible realization of Procgen output. It does not claim that
camera interaction changes the generated artifact or that the inspection frame
has entered runtime authority.
