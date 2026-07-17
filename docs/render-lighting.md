---
status: current
audience: agent
tags: [render, lighting, projection]
supersedes: []
see-also: []
---

# Renderer-neutral lighting and uploaded mesh presentation

ASHA represents ordinary lights as retained render projection, not as Three.js
objects and not as gameplay authority. `LightDescriptor` supports ambient,
directional, point, and spot lights. `CreateLight`, `UpdateLight`, and the common
`Destroy` operation give lights stable handles and optional scene-node parents.

The contract uses linear RGB, non-negative intensity, scene-unit range, a
non-negative decay exponent, radians for spot outer angle, and a `0..=1`
penumbra. Direction vectors point from a light toward the scene. Rust and the
renderer-neutral TypeScript projection reject non-finite values, zero directions,
invalid ranges, and invalid cone/penumbra values before backend application.

## Generic contract to Three.js

`@asha/renderer-three` is the adapter boundary:

- ambient maps to `THREE.AmbientLight`;
- directional maps to `THREE.DirectionalLight` with a normalized local target;
- point maps range and decay to `THREE.PointLight`;
- spot maps range, decay, angle, penumbra, and normalized target to
  `THREE.SpotLight`.

The protocol contains no `THREE.Light`, `Object3D`, shader name, or arbitrary
renderer property bag. A backend may run without shadow-map support. Requested
shadows then remain visible in `RendererLightReadout` as
`requested_unsupported`; they are never silently reported as active.

Editor viewport channels no longer inject private Three.js work lights. Stored
scene lights and Studio-local work lights must enter through the same generic
diff vocabulary, keeping downstream rendering aligned with upstream behavior.

## Stored SceneDocument lights

Scene schema/authoring format version 2 adds `SceneNodeKind.light` with a typed
`SceneLight` payload. Ambient, directional, point, and spot settings are stored
without a renderer object or a second pose. The containing node transform owns
position; its local `-Z` axis owns directional/spot orientation. Parent
translation, rotation, and scale compose into the projected world pose.
Light-node scale itself must remain `[1, 1, 1]` because scaling a light has no
portable meaning.

Rust canonical decode rejects unknown fields and validates colors, intensity,
range, decay, cone, penumbra, shadow intent, transform, and hierarchy. A typed
`updateLight` scene-object command changes light properties under the guarded
document hash. Transform or property changes retain the same render handle and
emit `UpdateLight`; removal emits `Destroy`.

Version-1 documents remain accepted and encode byte-for-byte at version 1. They
are not silently rewritten. A document containing a light must explicitly use
schema and authoring format version 2. The inspectable canonical fixture is
`harness/fixtures/scenes/lights-v2.json`.

An authored scene light is durable project content. A Studio work light is an
editor preference and must not be inserted into the saved SceneDocument unless
the user deliberately creates a light node.

## Uploaded voxel meshes

`ReplaceMeshPayload` now realizes uploaded groups with lighting-aware standard
materials, so the payload's normal stream affects visible faces. Each group keeps
its registered material-slot colour while the retained node `Material` supplies
projection view style: RGB tint, opacity, and wireframe. A later `Update` changes
existing uploaded materials, and a later remesh rebuilds them from the retained
style instead of reverting to an unlit default.

This is presentation only. Voxel contents, edit authority, collision, and scene
transforms remain Rust-owned. Renderer and host readouts expose light descriptors
and mesh material style for diagnostics without creating an authority backchannel.

## Visible artifact

The browser source at `harness/fixtures/browser/lit-voxel-showcase.html` applies
ambient, directional, point, and spot diffs to uploaded box meshes. The committed
capture `harness/goldens/screenshots/lit-voxel-showcase.png` visibly demonstrates
differentiated faces, point-light range, and the spot cone. Reproduction commands
live beside the image in `harness/goldens/screenshots/README.md`.
