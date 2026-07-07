# RuntimeSession Facade Status

Status: current public semantic facade. Rust-backed RuntimeSession authority is the product/live path; the explicit reference helper remains a fixture and compatibility surface only.

## Public Import Path

Consumers import facade types and product/live launcher types from the package root. Reference/mock helpers live behind the explicit reference entrypoint and carry a fixture-only backend profile:

```ts
import { type RuntimeSessionFacade } from '@asha/runtime-bridge';
import {
  REFERENCE_RUNTIME_BACKEND_PROFILE,
  createMockRuntimeSession,
} from '@asha/runtime-bridge/reference';
```

No consumer should import package internals, raw native transports, generated file paths, or Rust crate paths.
Demo and Studio live/default flows should not treat `@asha/runtime-bridge/reference` as product authority. Its `REFERENCE_RUNTIME_BACKEND_PROFILE.productAuthority` value is `false`, and reference RuntimeSession identities include `not_product_authority`.

The cross-surface consumer proof for #4053 lives in
`ts/packages/smoke/src/public-consumer-compat.test.ts`. It intentionally imports
only package roots and is the current evidence that `asha-demo` can consume the
RuntimeSession readout/HUD surfaces without private ASHA paths.

## Current API

`RuntimeSessionFacade` exposes the same semantic methods in explicit backend
modes. Product/live consumers create the facade from the package root with a
Rust-capable bridge and `mode: 'rust'`; reference fixtures use
`createMockRuntimeSession()` from `@asha/runtime-bridge/reference`.

`RuntimeSessionFacade` exposes:

- `initialize(input)`: validates semantic session/project input, initializes the bridge, and loads a ProjectBundle-shaped request.
- `loadEcrpProject(input)`: validates and loads ProjectBundle-shaped ECRP content (`ProjectBundle`, `EntityDefinition[]`, and `SceneDocument` placements). Rust-backed sessions route bootstrap through the bridge authority surface and return Rust provenance/read sets; reference sessions keep fixture/project-state compatibility. Rejected loads mutate nothing.
- `submitCommands(batch)`: submits generated `CommandBatch` values only.
- `tick(input?)`: advances deterministic runtime ticks through the bridge.
- `createCamera(request)`: creates a typed bridge-owned camera.
- `applyFirstPersonCameraInput(envelope)`: applies unconstrained first-person camera motion/look input.
- `applyCollisionConstrainedCameraInput(envelope)`: applies first-person camera motion/look input through the typed collision bridge surface and returns a receipt with collided, blocked axes, world/collision projection hashes, movement hash, and the generated before/attempted/after `CameraCollisionSnapshot`.
- `submitRuntimeActionIntent(envelope)`: accepts a typed `RuntimeActionIntentEnvelope` proposal. Rust-backed sessions route accepted `primary_fire` pressed intents through the Rust bridge authority surface and return combat/fire/health provenance; reference sessions return labelled fixture/reference receipts. Unsupported action intents fail closed with typed receipts.
- `runAutonomousPolicyTick(input)`: advances a narrow generated-tunnel enemy policy loop, validates typed movement/fire proposals, routes primary fire through runtime action authority, and reports proposal counts, nav/replay hashes, movement/combat summaries, backend provenance, and deterministic tick hash.
- `readLifecycleStatus(request?)`: reads player/enemy lifecycle status, win/loss/in-progress outcome, restart eligibility, fixture reset hash, lifecycle/replay hashes, and terminal death events.
- `requestSessionRestart(intent)`: validates a typed `runtime.restart_session_intent`, rejects stale/non-terminal requests with typed receipts, or resets the session deterministically through the existing restart path.
- `readCombatReadout(request?)`: reads the committed #4040 generated-tunnel combat fixture readouts for compatibility/golden evidence. Runtime action receipts use the loaded RuntimeSession state when a project has been loaded.
- `readGeneratedTunnelReadout(request?)`: reads the #4038 tiny generated tunnel fixture evidence, including seed, config/output/replay hashes, spawn markers, material roles, and render/collision projection hashes.
- `requestGeneratedTunnelOperation(request)`: returns typed fail-closed receipts for unsupported generated tunnel regenerate/apply operations.
- `readNavProjection()`: reads #4041 generated-tunnel nav projection availability/hash evidence.
- `queryNavPath(request?)`: returns reachable or no-path generated-tunnel path readouts.
- `readNavPolicyView()`: returns a read-only/proposal-only policy-facing nav view shape with no mutation/apply authority.
- `readCameraProjection(request)`: reads typed camera projection matrices and projection hash.
- `planVoxelConversion(request)`: requests a Rust-owned mesh-to-voxel conversion plan using generated voxel conversion DTOs. Rust-backed sessions route through the native/runtime bridge authority surface; reference sessions fail closed with `operation_unimplemented`.
- `previewVoxelConversion(request)`: requests bounded conversion preview output for a previously planned conversion, guarded by the expected plan hash. Rust-backed sessions return typed diagnostics for stale plan hashes; reference sessions fail closed.
- `applyVoxelConversion(request)`: requests authority application of a validated conversion plan/preview pair. Rust-backed sessions preserve plan/preview hash guards and apply accepted output through the upstream voxel command path; mismatched previews or unsupported authority target grids return classified diagnostics.
- `exportVoxelConversionEvidence(evidence)`: requests export of selected generated voxel conversion evidence refs from the current Rust authority conversion state. Unknown evidence refs fail closed.
- `readProjection()`: returns a render/projection summary from public render diff contracts.
- `readEcrpRuntimeReadout()`: returns live Entity/CapabilityState/event readouts derived from the selected backend. Rust-backed readouts identify `mode: 'rust'`, `source: 'rust_bridge'`, authority surface, and declared read sets.
- `readTelemetry()`: returns sequence/tick/composition/command/replay/hash summary.
- `restart()`: unloads/reinitializes/reloads the same ProjectBundle input and resets tick/command counters and lifecycle state.

Package-root helpers over the facade expose derived readouts without asking
consumers to maintain parallel demo authority:

- `readRuntimeSessionPlayableLoopState(session, request?)`: reads lifecycle,
  telemetry, and ECRP state through public `RuntimeSessionFacade` methods and
  returns current-epoch HUD counters, health summaries, target identity, command
  gating, and missing-backend diagnostics. Replay history before the most recent
  restart/request-restart record is excluded from `shotsFired`/`actionTick`, so
  reset UI does not accidentally count historical command records.
- `readRuntimeSessionPlayableEncounterTick(session, request)`: derives the
  generated-tunnel enemy actor from ECRP state, applies pause/terminal/missing
  actor gates, advances one autonomous policy tick through RuntimeSession, and
  returns movement/combat/lifecycle summaries. Browser shells still own the
  schedule/timer and pass the current RuntimeSession camera handle/position.

Lifecycle fixture hashes in the current reference slice:

- initial reset hash: `fnv1a64:d0c05bd05488e8a5`
- enemy defeated lifecycle hash: `fnv1a64:5fbf190733451da1`
- player defeated fixture lifecycle hash: `fnv1a64:32322a108d4f2767`

The current reference helper is `createMockRuntimeSession`, a facade over the existing `RuntimeBridge` mock exposed only from `@asha/runtime-bridge/reference`. It is useful for unit tests, compatibility fixtures, and offline smoke baselines. It is not the product/live authority path for demo or Studio, and selected/native backend launchers must fail closed rather than falling back to this helper. For collision-constrained camera input, the reference facade hosts the upstream static-room collision fixture so consumers can prove wall-stop/open-space behavior without importing demo-local physics. For ECRP content, the reference RuntimeSession owns a loaded project-state projection seeded by `loadEcrpProject`; primary-fire receipts, lifecycle updates, entity events, health state, and render visibility apply to the loaded enemy entity in reference mode. It does not claim native runtime attach, product authority, raw state-store access, or renderer ownership.

Evidence lanes:

- `pnpm --filter @asha/runtime-bridge test:evidence:reference` proves the reference RuntimeSession fixture lane remains explicitly non-product authority.
- `pnpm --filter @asha/runtime-bridge test:evidence:rust` proves the public Rust-backed facade reports backend provenance for collision, combat, lifecycle, encounter, and restart.
- `pnpm --filter @asha/smoke test:evidence:reference` and `pnpm --filter @asha/smoke test:evidence:authority` split smoke evidence into reference and authority lanes.

## Runtime Vocabulary

The public facade uses `RuntimeSession` and `ProjectBundle` vocabulary. Internally, the current bridge still wraps older WorldBundle-shaped DTOs for compatibility (`WorldLoadRequest`), as documented in `docs/vocabulary-compatibility.md`.

## Non-Claims

The reference RuntimeSession reports explicit non-claims. Current examples include:

- `not_native_runtime`
- `not_raw_state_store`
- `not_arbitrary_json_bridge`
- `not_product_authority`
- `not_renderer`

These non-claims mean the reference facade is still not native runtime attach, raw state-store access, product authority, or renderer ownership. They no longer mean the FPS demo owns local combat/health/target authority; that state now comes through RuntimeSession ECRP/lifecycle/action readouts, with Rust-backed sessions carrying Rust bridge provenance where wired.
