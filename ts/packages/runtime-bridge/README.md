# `@asha/runtime-bridge`

`@asha/runtime-bridge` is the Tier 1 public TypeScript runtime facade for ASHA engine consumers such as `asha-testing`, `asha-demo`, and `asha-studio`.

Consumers should import only the package root:

```ts
import {
  createMockRuntimeBridge,
  createNativeRuntimeBridge,
  frameCursor,
  type RuntimeBridge,
} from '@asha/runtime-bridge';
```

## Boundary contract

Allowed through this facade:

- initialize an engine/runtime session;
- initialize a semantic `RuntimeSession` from a validated ProjectBundle-shaped request;
- load a world bundle-shaped DTO;
- submit generated contract command batches;
- step deterministic authority ticks;
- read render/projection diffs;
- read semantic telemetry/replay/hash summaries;
- read generated tunnel fixture evidence through typed RuntimeSession readouts;
- read combat and nav/pathfinding fixture evidence through typed RuntimeSession readouts;
- restart/reset a semantic session without exposing authority state;
- get/release opaque runtime buffer handles;
- save or inspect current world/composition state;
- use classified `RuntimeBridgeError` failures.

Forbidden for downstream consumers:

- no direct `@asha/native-bridge` imports;
- no `@asha/wasm-replay-bridge` runtime imports;
- no `../asha/ts/packages/*/src/*` imports;
- no generated contract file edits or local contract forks;
- no raw `call(methodName, json)` bridge tunnels;
- no mutable `StateStore`, unchecked event application, renderer internals, or editor internals.

The raw native addon wrapper remains internal transport plumbing. This package is the only public package that may import it, and unwired native operations must fail closed with `operation_unimplemented` rather than inheriting mock behavior.

## Internal layout

The package root remains the only public import path. Internally, `src/index.ts` is a barrel over concern-focused modules: `bridge.ts` owns handle/error/DTO/interface types, `mock.ts` owns the reference bridge used by tests and deterministic consumers, `native.ts` is the only raw `@asha/native-bridge` importer, and `launcher.ts` owns the `GameRuntimeLauncher` session facade.

`RuntimeSession` is the narrow semantic facade for game repos and Studio. It exposes `initialize`, `submitCommands`, `tick`, `readProjection`, `readTelemetry`, and `restart` over a public `RuntimeBridge` implementation. The first reference implementation wraps the mock bridge and keeps explicit non-claims for native runtime, raw StateStore access, arbitrary JSON bridge calls, gameplay loop, and renderer ownership.

`GameRuntimeLauncher` stays in this package for now because it is a thin public orchestration facade over `RuntimeBridge` and must preserve the same fail-closed backend/profile rules as the transport facade. If launcher policy grows beyond bridge-backed launch/session read models, split it into a future domain package that depends on `@asha/runtime-bridge` instead of moving raw transport access upward.

## Metadata and checks

The package declares its Tier 1 role in `package.json` under `asha.publicSurface`. The CI bridge check runs `harness/public-surface/check-public-boundary.py` to keep the engine-owned TS public surface manifest, compatibility anchors, raw transport status, and the Rust `runtime-bridge-api` metadata aligned with the Den public-surface design.

## Browser FPS Input

`BrowserFpsInputCollector` is the package-root browser input surface for early FPS
demo wiring. It accepts structural event objects compatible with DOM keyboard,
mouse, and pointer events, then drains one typed command per tick:

```ts
{
  kind: 'runtime.apply_first_person_camera_input',
  envelope: FirstPersonCameraInputEnvelope
}
```

The envelope is accepted by `RuntimeSessionFacade.applyFirstPersonCameraInput`.
Pointer-lock request/release are returned separately as typed shell intents because
the browser owns pointer-lock side effects. Primary fire press/release is returned
as `runtime.propose_runtime_action_intent` with a `RuntimeActionIntentEnvelope`;
`RuntimeSessionFacade.submitRuntimeActionIntent` accepts primary-fire press
proposals and returns typed combat/fire/health readout evidence in the reference
slice.
