# Named Input Actions and Session Contexts

Status: #5641 contracts/resolver, #5642 browser/FPS/editor integration, and the
#5643 replay/pause engine boundary are implemented.

## Purpose

ASHA resolves normalized platform input into named actions before gameplay,
editor, camera, menu, or dialog consumers see it. A consumer asks for
`game.move.forward`, `camera.look`, or `menu.close`; it does not own `KeyW`,
pointer deltas, DOM listeners, or context priority.

This boundary is intentionally separate from the gameplay fabric:

- raw input is host evidence, not a gameplay event;
- a resolved action is a Session-level intent source;
- an accepted owner fact produced because of that intent may later become a
  gameplay event.

## Ownership

`protocol-input` owns generated border DTOs. `rule-input` owns catalog
validation, active-context state, and deterministic resolution. Context state
belongs to the RuntimeSession, not to an Entity or CapabilityState.

TypeScript collects and normalizes platform events, but it does not choose the
winning context or mutate the active stack. The public RuntimeSession surface
submits `RawInputSample` and typed `InputContextCommand` values to this Rust
rule through five bounded operations: configure, context command, raw submit,
resolved-action replay, and read context state.

## Catalog and context model

An `InputBindingCatalog` declares:

- named actions with button, one-axis, or two-axis value kinds and accepted
  phases;
- named contexts with bounded priority and an explicit
  `consumesLowerPriority` rule;
- platform bindings that join one normalized control to one action in one
  context.

Catalog activation fails atomically for unsupported schemas, malformed or
duplicate ids, unknown references, invalid priorities, conflicting controls,
value-kind mismatches, invalid scales, or executable modifier/chord data.

`InputBindingExtension` is the versioned modifier/chord seam. Schema v1 carries
the optional field but rejects it when present. This keeps future catalog
evolution explicit without quietly claiming v1 chord behavior.

The active context stack is typed Session state. Push, pop, and replace
commands either produce a new complete `InputContextStackState` or leave the
previous state unchanged with classified diagnostics. Pop is expectation-bound
to the current top context, preventing stale UI owners from removing a newer
modal context.

## Deterministic resolution

Resolution is a pure function of the validated canonical catalog, complete
context state, and one normalized raw sample.

Active contexts are considered by:

1. higher authored priority;
2. later stack position;
3. stable context id as the final deterministic tie-break.

The first matching binding resolves one named action. A higher context with
`consumesLowerPriority` set consumes an unmatched input before gameplay below
it can receive it. Conflicting bindings inside one context are rejected at
catalog activation, so resolution never makes an arbitrary winner choice.

Every receipt carries catalog, context, input, and resolution hashes plus the
winning action or classified reason. An accepted receipt also carries an
authority-issued `RecordedInputAction`. That record contains the semantic
`ResolvedInputAction` plus catalog/context/record hashes; it contains no
platform kind, control code, or DOM event. Identical inputs produce byte-stable
evidence.

## Direct action replay

`RuntimeSessionFacade.replayResolvedInputAction` injects an authority-issued
record directly into the active input Session. Replay does not synthesize a
keyboard, mouse, or browser event and does not resolve a platform binding a
second time.

Rust validates the record schema, canonical hash, active catalog hash, active
context hash, action/value/phase declaration, binding lineage, and active
context before delivery. A record hash can be delivered only once per
configured input Session. Missing or tampered evidence, catalog/context drift,
invalid phases, and repeated delivery return deterministic classified replay
receipts without an action.

This makes a replay tape a sequence of semantic outcomes. Context-changing
consumers still submit the same typed context commands while replaying, so the
next record must meet the resulting context hash. The tape does not depend on
the browser binding that originally produced an action.

## Save, restore, and readout posture

`InputSessionSnapshot` stores the validated catalog hash and complete
`InputContextStackState`. The state contains schema version, monotonic revision,
canonical zero-based stack order, active context ids, and a field-wise stable
hash. Restore rejects catalog drift, unknown or duplicate contexts,
non-canonical order, unsupported schema, and hash mismatch before activation.

The same context state is the operator/readout shape; there is no separate
hidden stack. Raw platform state such as browser key-down sets is host-local and
is not part of this snapshot.

## Browser host and consumers

`BrowserInputHost` is the one DOM keyboard/mouse normalization point. It owns
listener attachment, monotonic raw-sample sequence numbers, pointer-lock
intents, and a bounded diagnostic history. Each diagnostic delivery shows the
normalized sample, a delivery-time snapshot of active Session context ids,
resolution receipt, chosen named consumer, and the first rejection or
consumption reason. Later context pushes/pops do not rewrite older delivery
records.

The default catalog carries gameplay, editor, camera-navigation, menu, and dialog contexts. Menu
and dialog have higher priority and consume unmatched controls, so `KeyW`
cannot leak into gameplay while a modal surface is open. Their own bindings,
such as menu navigation, close, dialog confirm, and dialog cancel, still
resolve normally. `Escape` resolves to `runtime.time.pause` in gameplay and
`runtime.time.resume` in the menu context. `ResolvedPauseContextConsumer`
sequences the public context and time-control commands, including compensating
the context change if time authority rejects the paired transition.

While paused, the menu context consumes movement and camera actions but menu
navigation continues to resolve. Authority cadence ticks remain stopped by the
Rust time controller, while input context, projection, UI, and inspection reads
remain live. Resuming pops the expectation-bound menu context and restores
gameplay delivery without a second raw-input path. Context changes reset
transient FPS state so held movement cannot remain latched across a modal
transition.

The `cameraNavigation` context sits above gameplay and consumes lower bindings
while orbit or top-down is active. `ResolvedCameraNavigationConsumer` requires
a selected pivot, sequences context and revision-guarded camera authority with
compensation on rejection, and converts only resolved `camera.navigation.*`
actions into pan, rotate, and zoom proposals. Returning to
`camera.mode.firstPerson` pops that context, so the same physical `KeyW` sample
resolves to camera pan in orbit and gameplay movement in FPS, never both. Wheel
input is normalized as an `axis1d` sample; downstream camera code does not read
the DOM event.

`BrowserFpsResolvedActionConsumer` turns only `gameplay.*` resolved actions into
camera movement/look state. The renderer surface accepts an initialized public
RuntimeSession input port; without one, interactive controls remain inactive
rather than opening a private authority path. `EditorResolvedInputConsumer`
similarly accepts only `editor.*` actions. The production `@asha/app`
composition connects `BrowserInputHost` to that consumer, drains camera frames
through an injected editor-camera port, and routes primary/cancel frames to the
shell's one `VoxelEditController`. Editor tools contain no DOM codes or binding
table.

The former `BrowserFpsKeyCode` union and `BrowserFpsInputCollector` were removed
in #5642. There is no compatibility export or production fallback. A downstream
consumer that still imports either name must migrate to `BrowserInputHost` and
the RuntimeSession input surface rather than extending the retired five-key
path.

## Current non-claims

The current input surface does not execute modifiers or chords, support
gamepads, touch, gesture navigation, IME/text composition, accessibility switch devices, or
rebinding UI. It records accepted resolved actions only; rejected/consumed raw
samples remain resolution evidence rather than replay deliveries. Downstream
live-browser adoption is tracked by Den task #5686 in the `asha-demo` repository
so the reference game cannot quietly keep a parallel pause/input authority path.
