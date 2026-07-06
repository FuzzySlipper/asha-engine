# ASHA Demo Upstream Gap Audit

Status: task #4389 audit after the renderer-host / renderer-three split.

## Current Path

The playable browser demo now mounts through `@asha/renderer-host`. The demo no
longer imports `@asha/renderer-three` or bare Three.js. `@asha/renderer-three`
owns concrete WebGL rendering, center picking by render label, and visual object
projection. `@asha/renderer-host` owns the current browser control loop and
adapts movement/action requests to the public RuntimeSession facade.

Runtime authority for the demo is still intended to be Rust-backed
RuntimeSession state:

- `loadEcrpProject` loads authored ProjectBundle/entity/scene content.
- `applyCollisionConstrainedCameraInput` validates first-person camera movement.
- `submitRuntimeActionIntent` applies primary-fire combat.
- `readLifecycleStatus`, `requestSessionRestart`, telemetry, combat, nav, and
  autonomous policy readouts describe runtime state.

## Remaining Upstream Gaps

These are the demo-visible behaviors still owned by TypeScript shell/demo glue
because ASHA does not yet expose the durable upstream surface.

1. Render target identity is label-based.
   The demo derives enemy/player render targets from authored entity data and
   local labels, then asks renderer-host to update objects by label. Runtime and
   render projections should expose stable entity-to-renderable identity so
   consumers do not invent label conventions.

2. Playable-loop HUD counters are local.
   `asha-demo` tracks shots fired, hits, action ticks, restart count, paused
   state, and some command gating locally. RuntimeSession owns combat,
   lifecycle, restart, and telemetry, but there is not yet a single current
   epoch readout for demo HUD counters and command availability.

3. Native runtime bootstrap is local.
   `asha-demo` defines `globalThis.ashaDemoRuntimeBridge`, checks
   `native_rust`, rejects reference authority, and builds unavailable-backend
   diagnostics locally. That fail-closed posture should become an upstream
   launcher/provider helper so browser and standalone hosts do not repeat it.

4. Generated room frame composition still sits behind the Three backend.
   The generated-tunnel room helper emits renderer-neutral render diffs, but it
   currently lives in `@asha/renderer-three/backend`. That data should move to a
   renderer-neutral or Rust/runtime projection owner.

5. Enemy encounter ticking is still demo-scheduled.
   The demo owns `setInterval`, passes target/enemy positions, chooses
   line-of-sight/range inputs, and gates terminal/paused states before calling
   autonomous policy. RuntimeSession should expose a cohesive encounter tick
   receipt that derives state from the loaded session.

6. Browser FPS input ownership is transitional.
   #4388 moved controls out of the Three backend, but renderer-host now owns DOM
   pointer lock, key state, camera-relative movement math, and movement
   authority envelopes. The durable lane should align renderer-host,
   `BrowserFpsInputCollector`, and `@asha/ui-dom` so input can be reused without
   becoming render backend behavior.

## Follow-Up Tasks

- #4399: expose authoritative render-target identity in runtime render
  projections.
- #4400: add RuntimeSession playable-loop state readout for HUD counters and
  gating.
- #4401: promote demo native RuntimeBridge provider bootstrap into an upstream
  launcher surface.
- #4402: move generated tunnel room render-frame composition upstream of
  renderer-three.
- #4403: provide an upstream autonomous encounter tick surface for demo enemy
  loop scheduling.
- #4404: define durable browser FPS input ownership outside renderer-host
  backend composition.

## Non-Goals

- Do not move gameplay authority into `asha-demo`, renderer-host, or
  renderer-three.
- Do not add new proof-only pages to close these gaps.
- Do not allow downstream consumers to import `@asha/renderer-three`, bare
  Three.js, native transports, generated internals, or Rust crate paths.
