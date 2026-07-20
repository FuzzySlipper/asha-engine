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

## Public mount and frame replacement

```ts
import { mountAshaRendererInspectionSurface } from '@asha/renderer-host';

const viewer = await mountAshaRendererInspectionSurface(canvas, {
  autoStart: true,
  controls: {
    initialPosition: [6, 5, 9],
    initialTarget: [0, 1, 0],
  },
});

const receipt = viewer.replaceFrame(generationResult.renderFrame);
if (!receipt.applied) {
  showDiagnostics(receipt.diagnostics);
}
```

`replaceFrame` treats its argument as the complete retained inspection result.
Replacement is atomic: malformed frames, invalid handles, policy violations, or
backend resource failures return a rejected receipt while the last accepted
frame remains visible. A malformed `frame` passed during mount rejects the mount
and disposes the prepared renderer.

The helper reuses the engine-owned editor viewport for retained frame
validation, backend realization, picking, buffer-backed resources, resize, and
disposal. It reuses the renderer-host stored-camera resolver for the Y-up orbit
camera. No Studio camera implementation or downstream Three.js code is copied.

Primary-button drag orbits the stored inspection target. WASD moves the camera
and target together while the canvas has keyboard focus. `resizeToCanvas`
samples the canvas dimensions, `resize` accepts an explicit bounded size, and a
browser `ResizeObserver` keeps the surface synchronized when available. `start`,
`stop`, `renderOnce`, and idempotent `dispose` own the complete render and input
lifecycle.

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
4. Convert the accepted Procgen result into one complete `RenderFrameDiff`, then
   pass it to `replaceFrame`. Display rejected receipts instead of falling back
   to a downstream renderer or reference runtime.
5. Dispose the surface when the tab/workbench is destroyed and resize it when
   the viewer panel changes dimensions.

The surface proves visible realization of Procgen output. It does not claim that
camera interaction changes the generated artifact or that the inspection frame
has entered runtime authority.
