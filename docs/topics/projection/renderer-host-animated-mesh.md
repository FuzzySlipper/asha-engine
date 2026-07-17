---
status: current
audience: agent
tags: [render, animation, mesh, projection]
supersedes: []
see-also: []
---

# Renderer Host Animated Meshes

`@asha/renderer-host` is the public browser and standalone boundary for animated
mesh projection. ASHA Game Projects do not import `@asha/renderer-three`, Three.js,
or engine fixture paths.

## Public resource contract

The package exports `ASHA_RENDERER_HOST_ANIMATED_MESH_FIXTURE_MANIFEST`. Its
Kenney resource is shipped under the renderer-host package with:

- asset id `mesh-animation/kenney-retro-character-medium`;
- SHA-256 `c71255a41c0373f0d2ef52593369d5fd9d2f6220ae548aff8cd6bf5edb403674`;
- named clips `idle`, `run`, and `jump`;
- a package-relative GLB URL and CC0 license URL.

The default resolver fetches the package URL. Hosts with their own asset serving
layer may provide `resolveAnimatedMeshResource(descriptor)` and return an
`ArrayBuffer`; renderer-host still verifies the declared hash and clip list.

## Browser surface

```ts
import {
  ASHA_RENDERER_HOST_ANIMATED_MESH_FIXTURE_MANIFEST,
  mountAshaRendererAnimatedMeshSurface,
} from '@asha/renderer-host';

const surface = await mountAshaRendererAnimatedMeshSurface(canvas, {
  animatedMeshManifest: ASHA_RENDERER_HOST_ANIMATED_MESH_FIXTURE_MANIFEST,
  frame: roomFrame,
});

const intent = runtimeSession.readAnimationIntent();
const receipt = surface.applyFrame(intent.frame);
const playback = surface.animatedMeshPlayback(intent.instanceHandle);
```

`renderOnce()` and the host animation loop advance projection-only mixers. The
readback reports handle, asset, selected clip, status, mixer/action time,
command-selected state, loop/speed/weight, diagnostics, and a bounded root plus
hierarchy pose sample.

For canvas-free integration tests, `createAshaRendererAnimatedMeshProjection()`
provides the same resource validation, frame application, explicit `advance()`,
playback readback, and structural snapshot without exposing backend types.

## Fail-closed behavior

Manifest/resource/hash/clip load failures throw `AshaRendererHostError` with
typed diagnostics before a surface is mounted. Later frame or handle failures
return rejected receipts/readouts with typed diagnostics. No default clip starts
implicitly: `commandSelected` remains false and status remains `not_started`
until a playback command is applied.

Mixer time and pose samples are visual diagnostics only. They do not mutate or
decide gameplay, lifecycle, collision, health, replay, or RuntimeSession state.
