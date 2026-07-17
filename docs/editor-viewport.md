---
status: current
audience: agent
tags: [editor, viewport, render]
supersedes: []
see-also: []
---

# Backend-neutral editor viewport

`@asha/renderer-host` owns the concrete browser renderer used by product and
editor consumers. ASHA Studio may author renderer-neutral `RenderFrameDiff`
content, camera inputs, and pick filters, but it does not import `three`,
`@asha/renderer-three`, renderer backend subpaths, scene objects, shaders, or GPU
resource handles.

The public entrypoint is:

```ts
import { mountAshaRendererEditorViewport } from '@asha/renderer-host';

const viewport = await mountAshaRendererEditorViewport(canvas, {
  autoStart: true,
});
```

The viewport is projection fabric, not gameplay authority. It owns canvas
realization, resize, rendering, resource disposal, retained channel composition,
camera realization, and renderer-side picking. Studio owns tool modes, orbit and
pan intent, drag state, selection policy, and the mapping from a pick hint to a
typed authoring or RuntimeSession proposal. Rust revalidates any runtime anchor
before mutation.

## Retained channels

The surface has exactly three fixed channels:

| Channel | Order | Layer policy | Intended content |
|---|---:|---|---|
| `runtime` | 0 | `scene` or `debug` | Current RuntimeSession projection |
| `authored` | 1 | `scene` or `debug` | Stored scene and unsaved authoring preview |
| `overlay` | 2 | `debug` only, rendered after a depth clear | Selection, gizmo, and debug evidence |

Each channel supports bounded `apply`, atomic `replace`, `clear`, `snapshot`, and
`dispose`. Equal downstream `RenderHandle` values are mapped to distinct
engine-owned handles per channel. A rejected frame or missing resource leaves the
last accepted channel projection intact and does not block the other channels.
Frame receipts carry typed diagnostics and stable logical snapshot hashes.

Use existing `RenderFrameDiff` primitives first. Cubes and quads cover selection
bounds and planes; points and lines cover pivots, axes, and pick markers.
Static meshes, animated meshes, sprites, material previews, and handle-backed
voxel meshes continue through the same engine-owned renderer realization and
classified resource paths. The editor API does not add a renderer plugin or
callback registry.

## Spatial grid

The viewport exposes the procedural editor grid separately from the retained
scene channels:

```ts
viewport.setGrid(descriptor); // add or update
viewport.setGrid(null);       // remove
const readout = viewport.grid();
```

`EditorGridDescriptor` is generated from the Rust protocol. Its
`coordinateSystem` accepts only `rightHandedYUp`; the usual ground grid uses the
`xz` plane and an explicit Y origin. `SpatialGridSpec` defines the exact origin,
per-axis spacing, minimum-corner cell bounds, and boundary or cell-center snap
anchor shared with `core-space` and `@asha/editor-tools`. Committed conformance
fixtures cover non-unit spacing, shifted origins, and negative coordinates in
both languages.

The renderer derives visible bounds, fade, and minor-line level of detail from
the current camera. It may omit minor lines only in exact multiples of the base
spacing; camera movement never changes the authored origin or spacing. The grid
is renderer-local projection and does not consume an authored/overlay handle or
mutate Rust authority. Invalid descriptors are rejected while the last valid
grid remains intact.

## Camera and picking

`setCamera` accepts generated `CameraPose`, `CameraBasis`, and
`PerspectiveProjection` values with a source classification:

- `stored_editor` for Studio-owned orbit/pan/zoom camera state;
- `runtime_authority` for a current authoritative camera snapshot.

The basis must be finite and orthonormal and the perspective range must be valid.
The host owns concrete camera realization and viewport resizing.

Stored orbit/target camera policy can use the root-exported
`resolveAshaStoredEditorCamera` helper. It accepts renderer-neutral position,
target, up, and perspective values and returns either one complete canonical
`stored_editor` pose/basis camera or a typed diagnostic. Coordinate conversion,
yaw/pitch convention, and orthonormal-basis derivation therefore remain in the
engine-owned host rather than downstream Studio code.

`pick` accepts canvas-relative pixel coordinates plus bounded channel, handle,
layer, and tag filters. It returns disposable projection evidence containing the
logical channel and handle, source trace when present, world position, surface
normal, and distance. Picking changes no retained projection, camera, Session,
or authority state.

## Lifecycle and failure behavior

The viewport owns `start`, `stop`, `renderOnce`, `resize`, and idempotent
`dispose`. Channel and viewport operation limits fail closed. Missing or malformed
resources reject only the affected channel transaction; the previous accepted
projection and healthy channels remain usable. The readout includes lifecycle,
camera, size, channel policies, bounded diagnostics, and a stable viewport hash.

The compatibility markers are `renderer-host.v1` for the package and
`editor-viewport.v0` for this additive surface.
