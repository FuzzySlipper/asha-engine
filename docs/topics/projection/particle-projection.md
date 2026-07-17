---
status: current
audience: agent
tags: [projection, particles, render]
supersedes: []
see-also: []
---

# Particle projection and renderer-owned simulation

Status: implemented Wave 1 surface  
Task: #5603  
Envelope decision: [Shared non-scene projection channel](non-scene-projection-channel.md)

Particles are disposable realizations of accepted gameplay meaning. Rust owns
the typed emitter descriptor, catalog identity, retained lifecycle, budgets,
and origin evidence. The renderer host owns individual particle positions,
lifetimes, flipbook frames, and billboards. Those per-particle values never
become Session state or replay truth.

## End-to-end path

```text
accepted owner fact / gameplay event
  -> render-particle validates catalog identity, descriptor, lifecycle, and budgets
  -> protocol-presentation ParticleProjectionOp in RuntimeProjectionFrame
  -> RuntimeSessionFacade.readProjection().runtimeFrame
  -> applyAshaRuntimeProjectionFrame dispatches the closed particle domain
  -> AshaParticleHost simulates and sends neutral billboards to an injected sink
```

The first live route is accepted primary fire. Rust emits one entity-attached
spark burst with a `gameplayEvent` origin. `asha-demo` resolves the projected
sprite by content hash and renders it over the engine surface without importing
a concrete renderer package.

## Generated descriptor and lifecycle

`protocol-presentation` owns:

- `ParticleEmitterHandle`, separate from scene, audio, and billboard handles;
- world or entity-attached anchors;
- catalog-hash-bound Sprite or SpriteSheet identity;
- rate and burst count, lifetime and velocity ranges, acceleration, ordered
  size/color curves, flipbook rate, seed, visibility, and emitter budget;
- `emit` for one-shot bursts and `create`/`update`/`destroy` for retained
  continuous emitters;
- typed diagnostics and a bounded aggregate readout.

The descriptor contains no DOM, canvas, Three.js, GPU-buffer, callback, or
per-particle state. A seed makes renderer behavior stable enough for debugging
and approximate visual comparison; it does not promote the simulation into
authority.

## Rust validation and budgets

`render-particle::ParticleProjector` validates complete descriptors before they
cross the public border. It rejects wrong asset kinds, missing catalog entries,
hash drift, non-finite or reversed numeric ranges, unordered curves, unsafe
JSON seeds, duplicate signals/handles, unknown updates/destroys, and configured
emitter or reserved-particle budget exhaustion.

Updates are atomic: the patch is applied to a copy and the full descriptor is
revalidated before retained state changes. Restart clears retained handles,
signal identity, and the latest disposable readout.

Rust's reservation budget bounds projected intent. The browser host separately
bounds live emitters and particles so a valid but visually dense frame cannot
exhaust the renderer. Partial drops produce `budgetExceeded` diagnostics and a
dropped-particle count without blocking scene, audio, or billboard projection.

## Renderer-owned realization

`AshaParticleHost` is exported from `@asha/renderer-host`. It uses a deterministic
xorshift stream per emitter to realize ranges, integrates velocity and
acceleration, interpolates size/color curves, chooses flipbook frames, and
cleans up expired particles. Entity anchors resolve through an injected
read-only position adapter. An injected `AshaParticleBillboardSink` keeps the
host independent of a specific renderer.

Resolved sprite bytes are SHA-256 checked against the projected catalog hash
and cached by immutable sprite identity. A missing entity anchor is diagnosed
without consuming the burst signal, so the same frame can succeed when the
projection becomes available. Once a burst is realized or intentionally
dropped by a live budget, its stable signal id prevents duplicate realization
when a frame is reread.

## Cosmetic adapter and authority boundary

`@asha/cosmetic` has a one-way
`adaptParticleBurstToHitSparkDescriptor` adapter for callers that still consume
the existing `hit_spark` view model. It accepts only projected particle bursts,
retains origin/signal identity, and creates no command, event, or reverse path.
Local UI-only screen flashes and view kicks remain valid outside the Rust
border; this task does not force wholesale package migration.

`PresentationFrameDiff.replayScope` remains `excludedFromReplayTruth`. Replay
may compare the authority outcome and projector diagnostics, but it does not
record or restore every particle position.

## Downstream visible acceptance

`asha-demo` visibly triggers twelve sparks from an accepted primary-fire
interaction. The live Chromium check observes the native particle operation,
its gameplay-event origin, the aggregate host readout, and a visible downstream
particle element. Demo owns that visible result. Engine regressions cover
`read_projection_frame`, the generated contract, and the public renderer-host
root without duplicating the product verdict.

## Wave 1 limits

GPU compute/transform feedback, mesh particles, collision, sub-emitters,
ribbons, lights, graph authoring, and a general-purpose VFX graph are deferred.
Continuous emitters are supported by the public contract and host, but the
first downstream product proof is a burst. Renderer hosts may replace the
billboard sink without changing Rust authority or generated descriptors.
