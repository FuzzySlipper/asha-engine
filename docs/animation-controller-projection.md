---
status: current
audience: agent
tags: [animation, projection, render]
supersedes: []
see-also: []
---

# Animation controller G1 projection

Status: implemented foundation for task #5649  
Authority source: `rule-animation-controller`  
Rust projector: `render-animation`  
Browser host: `@asha/renderer-host` `AshaAnimationHost`

## Contract and lifecycle

Animation controller realization is the `animation` domain of the shared G1
`RuntimeProjectionFrame`. It is not another event bus or bridge operation.
Every operation uses G1 ordering, the frame-level
`excludedFromReplayTruth` marker, and ordinary `PresentationOpMeta` origin,
causation, and correlation evidence.

`AnimationProjectionHandle` is branded separately from scene `RenderHandle`.
An animation descriptor references its animated-mesh scene target with a typed
`RenderHandle`, but updates and destroys address only the animation handle.
The Rust projector allocates a stable, monotonic animation handle for an
authority entity and emits one create/update/destroy lifecycle.

Create carries:

- target scene handle and animated asset identity;
- fixed tick duration used only for renderer interpolation;
- graph ID, version, canonical graph hash, state ID, FSM revision, and semantic
  controller-state hash;
- resolved primary/secondary clips, fixed blend weight, and speed;
- optional transition endpoints, fixed-tick progress, and target motion.

Updates replace only the controller projection state. Target and asset identity
remain stable for the lifetime of the animation handle.

## Renderer-local realization

`AshaAnimationHost` consumes only animation-domain G1 operations. It resolves a
state's linear blend and an active transition into at most four coalesced clip
weights. The engine-owned Three.js backend applies those weights to
`AnimationAction`s on the target's existing `AnimationMixer`.

When a lower-rate authority update arrives, the host interpolates from the
currently presented weights to the new authority-resolved weights over one
fixed tick duration. Explicit render-frame `advance(deltaSeconds)` calls update
weights and sample the mixer. This keeps pose motion smooth without promoting
wall-clock pose state, mixer time, bones, or matrices into authority.

The host has no mutation callback, proposal port, or gameplay command surface.
Animation completion and keyframe callbacks cannot cross back into Rust
authority. Gameplay-critical timing is the replayable typed fact documented in
`docs/animation-timing-semantics.md`.

## Failure and diagnostics

Malformed descriptors, duplicate/unknown animation handles, missing scene
targets, missing clips, invalid blend/transition state, stale revisions,
unavailable host capability, and backend failures are typed animation
diagnostics. Diagnostics retain the G1 origin reference.

Scene operations are applied before presentation dispatch. If the animation
host is absent, its operations produce `unavailableHost` diagnostics while the
scene and other presentation domains continue. Missing clips or targets do not
invent fallback authority state.

## Compatibility path

`RenderDiff::SetAnimatedMeshPlayback` remains a named compatibility path for
direct clip callers and older RuntimeSession animation-intent proofs. It is not
used by the new controller projector or `AshaAnimationHost`.

The compatibility path may be deleted when all three conditions hold:

1. downstream gameplay behavior uses authority controller changes and G1 animation
   operations rather than direct playback;
2. provider regressions and visible consumer acceptance use the controller path
   rather than direct playback;
3. no supported engine or downstream caller requires command-selected playback
   for gameplay-driven animation.

Until then, readouts expose `commandSelected` so compatibility use stays
inspectable. Controller-driven playback sets it false and publishes the active
weighted clips separately.

`asha-demo` now satisfies condition 1: it uses the compatibility readout only
to bootstrap the hash-pinned mesh target, filters the direct playback command,
and consumes gameplay-driven controller state through `AshaAnimationHost`.
