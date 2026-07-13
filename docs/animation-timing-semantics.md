# Animation timing semantics and gameplay-origin proof

Status: implemented for task #5650  
Authority: `rule-animation-controller`  
Projection: `render-animation` and the G1 `animation` domain  
Live consumer: `asha-demo` primary-fire path

## The boundary

Gameplay-critical time is a Rust fact. A clip keyframe, mixer completion, or
renderer callback is never allowed to decide that gameplay happened.

`AnimationInputOrigin` identifies the already-accepted gameplay fact supplied
to a controller evaluation:

- `sourceFactId` names the owner fact;
- `authorityTick` is the source authority tick;
- `causationId` preserves the accepted causal edge;
- `correlationId` groups all realizations of the same gameplay interaction.

When a semantic controller transition starts or completes,
`AnimationTransitionTimingFact` records that origin together with the
controller input sequence, controller tick, entity, graph, transition,
from/to states, duration, resulting FSM revision, and a canonical fact hash.
The fact is retained in controller state, included in the controller state
hash, stored in snapshots, and reconstructed from serialized input records by
verification replay.

Controller ticks without a caller-supplied gameplay fact receive a deterministic
controller-local origin. Product composition should use `tick_from_fact` when
the evaluation is caused by accepted gameplay authority.

## One fact, several disposable realizations

The public FPS proof uses an accepted `combat.primary-fire` owner fact. The
game-rule proposal first passes the gameplay-fabric Transform coordinator and
combat owner route. The resulting owner fact then drives the animation
controller and all presentation operations:

```text
resolved browser input
  -> gameplay fabric / combat owner
  -> combat.primary-fire.accepted:<replay-hash>
       -> animation controller transition fact (replay truth)
       -> G1 animation controller state (disposable)
       -> G1 audio cue (disposable)
       -> G1 particle cue (disposable)
       -> G1 world-space billboard update (disposable)
       -> G1 telemetry overlay update (disposable)
```

The animation operation metadata must match the timing fact's source ID, tick,
causation, and correlation. `render-animation` rejects mismatched metadata.
Other presentation domains carry the same source ID and correlation through
`PresentationOriginRef`; they do not create additional gameplay events merely
to request rendering.

Dropping the entire presentation frame changes no combat result, controller
snapshot, or replay hash. Re-reading/rebuilding presentation realizes current
controller state again. Renderer-local interpolation may be skipped or sampled
at a different rate without changing authority.

## Downstream consumption

The browser path deliberately separates target setup from gameplay selection:

1. `buildRuntimeSessionAnimationControllerTargetFrame` creates the hash-pinned
   animated-mesh target while removing the compatibility
   `setAnimatedMeshPlayback` command.
2. `AshaAnimationHost` consumes only G1 animation operations through the
   renderer surface's typed `animationProjection` port.
3. The mounted surface owns its render clock. The animation host interpolates
   weights; it cannot send a command or callback to authority.
4. `animationProjectionEvidence` exposes the semantic transition and timing
   fact. `animationPlayback` exposes renderer-local weighted clips and reports
   `commandSelected: false`.

The legacy direct playback command remains available for compatibility callers,
but it is no longer the gameplay-driven path demonstrated by `asha-demo`.

## Current proof and limits

The live proof uses one accepted primary-fire action to move the controller from
`ready` toward `primary_fire`. The target motion is a fixed 350/650 blend of the
fixture's `run` and `jump` clips while the state transition crossfades from
`idle`. Chromium verifies the semantic transition, origin equality with the
audio cue, three active weighted clips, and `commandSelected: false`.

This slice provides one composed proof, not a universal character-controller
policy. Downstream games should author their own graphs, parameters, assets,
and mapping from gameplay facts to controller inputs.

Clip/keyframe-authored audio and VFX cues are renderer-only presentation
derivations. `AshaAnimationHostOptions.cues` supplies typed asset, clip, marker,
and cosmetic-signal definitions. As the public host samples realized local
clip time, it emits a one-shot `AshaAnimationSampledCue` in the frame receipt.
The cue retains the controller operation's `PresentationOriginRef` and states
both `replayScope: excludedFromReplayTruth` and
`authorityMutation: forbidden`.

The current live proof samples the `jump` clip at 0.05 seconds, returns a typed
particle signal, and realizes that signal through the public particle host.
The HUD and `animationSampledCueEvidence` readout expose the sampled marker and
receipt. The cue cannot become a gameplay fact, advance the controller, or
cross back into Rust authority. Replaying authority therefore does not depend
on mixer time or on whether a renderer emitted the cue.
