---
status: current
audience: agent
tags: [camera, input, authority]
supersedes: []
see-also: []
---

# Camera Modes, Navigation, and Transitions

Status: current public engine boundary for Den task #5604.

This surface makes first-person, orbit, and top-down cameras three expressions
of one camera controller, not three unrelated downstream systems. It is meant
to support the ordinary messy work of gameplay and level editing: selecting a
thing, orbiting it, panning across a room, zooming through constrained space,
and returning to embodied play without losing the authoritative camera or
leaving two controllers active.

## Public path

Consumers use `RuntimeSessionFacade`:

- `createCamera(request)` creates the camera and its initial first-person
  controller state.
- `applyCameraModeCommand(command)` proposes a revision-guarded first-person,
  orbit, or top-down target.
- `applyCameraNavigationInput(envelope)` proposes orbit/top-down pan, rotation,
  and zoom against an expected controller revision.
- `readCameraControllerState(request)` reads the accepted mode, pivot,
  distance/height limits, snapshot, revision, and state hash.

The same generated contracts cross the Rust trait, native addon, TypeScript
bridge, and RuntimeSession facade. There is no generic JSON method dispatcher
or renderer-specific controls escape hatch.

## Authority and expression

Rust owns the accepted mode, pivot, distance or height, angle limits, pose,
revision, and hashes. Commands that are stale, malformed, incompatible with the
current mode, or blocked by terrain return typed atomic rejection receipts.
Orbit and top-down rays are checked against the runtime voxel collision
projection. A camera can be shortened to preserve clearance; if even its
declared minimum cannot fit, the command rejects without changing state.

This authority is a support for expressive camera work, not a claim that every
visual frame belongs in replay. A mode receipt can carry two accepted endpoint
snapshots plus duration and easing. `@asha/renderer-host` may use
`sampleCameraTransition` to draw disposable in-between poses. Those samples are
never fed back into RuntimeSession state or replay evidence. Replay records the
accepted endpoint transaction; the renderer owns how it looks on the way
there.

## Named input and controller exclusion

`BrowserInputHost` normalizes keyboard, pointer, and wheel events into the same
Session input resolver used by gameplay and editor actions. The default catalog
adds a high-priority `cameraNavigation` context. It consumes lower gameplay
bindings while orbit or top-down navigation is active, so `KeyW` cannot drive
both an FPS body and an orbit pivot.

`ResolvedCameraNavigationConsumer` is downstream composition over public
verbs. It:

1. Requires a consumer-selected pivot before entering orbit or top-down.
2. Pushes `cameraNavigation`, applies the revision-guarded mode command, and
   compensates the context if authority rejects.
3. Converts only resolved `camera.navigation.*` actions into bounded pan,
   rotate, and zoom proposals.
4. Pops the context when returning to first person and restores it if the
   authority transition rejects.

`@asha/editor-tools` exports the pure `editorCameraPivot` projection so the
shared editor selection store can provide the pivot without owning a camera or
importing RuntimeSession. The app-level proof composes that selection with the
public input consumer, exercises orbit/pan/zoom, and returns to FPS through the
same facade.

The default bindings are intentionally replaceable catalog data. Their useful
semantic vocabulary is `camera.mode.firstPerson`, `camera.mode.orbit`,
`camera.mode.topDown`, and `camera.navigation.*`; downstream gameplay code does
not need DOM key codes.

## Determinism and diagnostics

Controller state and receipts are field-hashed and revision guarded. Repeating
the same accepted command sequence produces the same authority evidence.
Renderer interpolation timing is deliberately excluded from that claim. Mode
receipts explicitly report whether terrain shortened the requested camera, and
rejections distinguish stale revision, invalid target, incompatible mode,
invalid input, and terrain blockage.

The engine tests cover deterministic mode changes, stale and invalid atomic
rejection, terrain clamping, terrain blockage, orbit navigation, top-down
switching, return to FPS, native transport, named-input context exclusion, and
renderer-only interpolation. A later integrated downstream proof may choose
product-specific selection and camera UI without reopening the engine
authority boundary.

## Current limits

This slice does not define cinematic rails, camera shake, multiple active
viewports, target-follow smoothing, gesture input, gamepad bindings, editor
selection policy, or product-specific mode UI. Those can compose over the
typed controller and named-input fabric without creating a parallel camera
authority.
