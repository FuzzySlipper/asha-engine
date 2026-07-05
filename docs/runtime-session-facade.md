# RuntimeSession Facade Status

Status: initial public semantic facade for task #4028.

## Public Import Path

Consumers import from the package root:

```ts
import { createMockRuntimeSession, type RuntimeSessionFacade } from '@asha/runtime-bridge';
```

No consumer should import package internals, raw native transports, generated file paths, or Rust crate paths.

The cross-surface consumer proof for #4053 lives in
`ts/packages/smoke/src/public-consumer-compat.test.ts`. It intentionally imports
only package roots and is the current evidence that `asha-demo` can consume the
RuntimeSession readout/HUD surfaces without private ASHA paths.

## Current API

`RuntimeSessionFacade` exposes:

- `initialize(input)`: validates semantic session/project input, initializes the bridge, and loads a ProjectBundle-shaped request.
- `loadEcrpProject(input)`: validates and loads ProjectBundle-shaped ECRP content (`ProjectBundle`, `EntityDefinition[]`, and `SceneDocument` placements) into the reference RuntimeSession ECRP project state. It returns a typed load receipt with diagnostics and bootstrap hash; rejected loads mutate nothing.
- `submitCommands(batch)`: submits generated `CommandBatch` values only.
- `tick(input?)`: advances deterministic runtime ticks through the bridge.
- `createCamera(request)`: creates a typed bridge-owned camera.
- `applyFirstPersonCameraInput(envelope)`: applies unconstrained first-person camera motion/look input.
- `applyCollisionConstrainedCameraInput(envelope)`: applies first-person camera motion/look input through the typed collision bridge surface and returns a receipt with collided, blocked axes, world/collision projection hashes, movement hash, and the generated before/attempted/after `CameraCollisionSnapshot`.
- `submitRuntimeActionIntent(envelope)`: accepts a typed `RuntimeActionIntentEnvelope` proposal. The reference slice accepts `primary_fire` pressed intents and returns combat/fire/health readout evidence; unsupported action intents still fail closed with typed receipts.
- `runAutonomousPolicyTick(input)`: advances a narrow generated-tunnel enemy policy loop, validates typed movement/fire proposals, routes primary fire through runtime action authority, and reports proposal counts, nav/replay hashes, movement/combat summaries, and deterministic tick hash.
- `readLifecycleStatus(request?)`: reads player/enemy lifecycle status, win/loss/in-progress outcome, restart eligibility, fixture reset hash, lifecycle/replay hashes, and terminal death events.
- `requestSessionRestart(intent)`: validates a typed `runtime.restart_session_intent`, rejects stale/non-terminal requests with typed receipts, or resets the session deterministically through the existing restart path.
- `readCombatReadout(request?)`: reads the #4040 generated-tunnel combat fixture readouts for hit/death and geometry-blocked miss evidence.
- `readGeneratedTunnelReadout(request?)`: reads the #4038 tiny generated tunnel fixture evidence, including seed, config/output/replay hashes, spawn markers, material roles, and render/collision projection hashes.
- `requestGeneratedTunnelOperation(request)`: returns typed fail-closed receipts for unsupported generated tunnel regenerate/apply operations.
- `readNavProjection()`: reads #4041 generated-tunnel nav projection availability/hash evidence.
- `queryNavPath(request?)`: returns reachable or no-path generated-tunnel path readouts.
- `readNavPolicyView()`: returns a read-only/proposal-only policy-facing nav view shape with no mutation/apply authority.
- `readCameraProjection(request)`: reads typed camera projection matrices and projection hash.
- `readProjection()`: returns a render/projection summary from public render diff contracts.
- `readEcrpRuntimeReadout()`: returns live Entity/CapabilityState/event readouts derived from the loaded ECRP project state.
- `readTelemetry()`: returns sequence/tick/composition/command/replay/hash summary.
- `restart()`: unloads/reinitializes/reloads the same ProjectBundle input and resets tick/command counters and lifecycle state.

Lifecycle fixture hashes in the current reference slice:

- initial reset hash: `fnv1a64:d0c05bd05488e8a5`
- enemy defeated lifecycle hash: `fnv1a64:5fbf190733451da1`
- player defeated fixture lifecycle hash: `fnv1a64:32322a108d4f2767`

The first implementation is `createMockRuntimeSession`, a reference/mock facade over the existing public `RuntimeBridge` mock. It is sufficient for downstream skeleton boot/readout tests and Studio contract work. For collision-constrained camera input, the reference mock hosts the upstream static-room collision fixture so consumers can prove wall-stop/open-space behavior without importing demo-local physics. For ECRP content, the reference RuntimeSession now owns a loaded project-state projection seeded by `loadEcrpProject`; primary-fire lifecycle updates apply to that loaded enemy entity. It does not claim native runtime attach or renderer ownership.

## Runtime Vocabulary

The public facade uses `RuntimeSession` and `ProjectBundle` vocabulary. Internally, the current bridge still wraps older WorldBundle-shaped DTOs for compatibility (`WorldLoadRequest`), as documented in `docs/vocabulary-compatibility.md`.

## Non-Claims

The reference RuntimeSession reports these non-claims:

- `not_native_runtime`
- `not_raw_state_store`
- `not_arbitrary_json_bridge`
- `not_gameplay_loop`
- `not_renderer`

These non-claims are intentional until native runtime/session attach and renderer/gameplay tasks land.
