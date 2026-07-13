# Audio projection: catalog identity to Web Audio

Status: implemented Wave 1 surface  
Task: #5595  
Envelope decision: [Shared non-scene projection channel](non-scene-projection-channel.md)

Audio is a disposable presentation of accepted gameplay meaning. Rust validates
which catalog clip and descriptor may be projected; the browser realizes that
projection with Web Audio. Neither an `AudioNode` nor playback timing becomes
Session authority or replay truth.

## End-to-end path

```text
accepted owner fact / gameplay outcome
  -> render-audio validates AudioClip catalog identity, hash, descriptor, handle
  -> protocol-presentation AudioProjectionOp in RuntimeProjectionFrame
  -> stable RuntimeBridge.readProjectionFrame(cursor)
  -> RuntimeSessionFacade.readProjection().runtimeFrame
  -> applyAshaRuntimeProjectionFrame (scene first, then presentation sequence)
  -> AshaAudioHost verifies bytes, decodes/caches the clip, and realizes Web Audio nodes
```

The first live route is accepted primary fire. Its audio `emit` operation keeps
an `ownerFact` origin whose id includes the combat replay hash. This makes the
audible cue inspectably attributable without making the cue itself authority.

## Contract

`protocol-presentation` owns the generated border:

- `AudioClipRef`: typed `audio/...` asset identity plus catalog content hash;
- `AudioSourceDescriptor`: SFX/ambient/UI bus, volume, pitch, looping, spatial
  blend, attenuation, pan, and emitter;
- `AudioEmitter`: `global2d`, `world3d`, or `entityAttached`;
- `AudioProjectionOp`: `emit`, `create`, `update`, and `destroy`;
- `AudioHandle`: retained audio identity, separate from `RenderHandle`;
- typed diagnostic and bounded readout shapes.

Use `emit` for a one-shot cue. Use `create` plus `update`/`destroy` for looping or
retained sources. A retained handle cannot be created twice or updated after it
is destroyed. Rust validates the full patched descriptor before replacing the
retained source state. One-shot signal ids are unique within a Session
generation, and the browser host remembers realized ids so re-reading the same
frame cannot play a cue twice. A new FPS Session generation resets Rust audio
projection state and gives new signals a generation-qualified identity.

## Validation boundary

`render-audio::AudioProjector` rejects an operation before it enters the public
frame when:

- the asset id is not an `AudioClip`;
- the clip is absent from the closed catalog;
- the projected hash differs from the catalog hash;
- numeric parameters are non-finite or outside their bounded ranges;
- a retained handle transition is invalid.

`AshaAudioHost` independently SHA-256 hashes resolved bytes before decoding.
The resource resolver's declared hash, projected hash, and actual bytes must all
agree. Decoded buffers are cached by that hash, not by a mutable URL or display
name.

## Browser host behavior

`AshaAudioHost` is exported from the `@asha/renderer-host` package root. It owns
one `AudioContext`, one gain node for each SFX/ambient/UI bus, decoded-buffer
cache entries, retained source graphs, and one-shot cleanup. `global2d` uses a
stereo panner. Spatial sources use an equal-power `PannerNode` with typed
attenuation and position.

The downstream shell updates the Web Audio listener with
`audioHost.updateListener({ position, forward, up })` from its current camera
projection. Entity-attached emitters resolve through the host's injected
read-only entity-position resolver; they do not read authority stores. After
scene application on every runtime projection frame, the host refreshes all
retained entity-attached panners from that resolver, so looping sources follow
projected entity motion without requiring redundant audio descriptor updates.

Browsers normally require `AudioContext.resume()` during a user gesture. A
blocked or unavailable context yields `audioContextBlocked` or
`unavailableHost`; it does not reject the authority action that originated the
cue.

## Failure isolation and replay

`applyAshaRuntimeProjectionFrame` validates the shared frame and contiguous
operation sequence, then applies the scene before each closed presentation domain. After outer framing is
valid, missing resources, byte-hash drift, decode failures, missing entity
positions, and host failures are domain diagnostics. They retain origin data
and do not block scene projection or mutate Session state.

`PresentationFrameDiff.replayScope` is
`excludedFromReplayTruth`. Verification can compare the accepted owner evidence
and projector diagnostics. DAC timing, browser scheduling, listener state, and
live `AudioNode` graphs are not replay inputs.

## Public consumer proof

`asha-demo` consumes only package roots. Its primary-fire interaction resolves a
consumer-owned WAV resource, reads the combined runtime frame, updates the
listener, applies the frame through `@asha/renderer-host`, and exposes a bounded
audio readout plus the originating owner-fact id. The live browser test checks
that one signal was decoded/emitted and that its origin matches the accepted
combat result.

The consumer need is `asha-demo.audio-projection`; joined reachability evidence
binds it to `read_projection_frame`, the generated presentation contract, the
renderer host, and the live downstream delivery.

## Wave 1 limits

The three buses are fixed routing groups, not a Unity-style mixer graph. There
are no reverb zones, occlusion, mixer snapshots, automation, custom HRTF,
streaming clips, or procedural-synthesis API. Source/listener realization is a
browser presentation concern; semantic cue choice stays in gameplay rules or
owner projections.
