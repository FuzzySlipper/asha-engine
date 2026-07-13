# `@asha/runtime-bridge`

`@asha/runtime-bridge` is the Tier 1 public TypeScript runtime transport and concrete adapter package for ASHA engine consumers such as `asha-testing`, `asha-demo`, and `asha-studio`. Transport-neutral `RuntimeSessionFacade` contracts live in `@asha/runtime-session`.

Production-facing consumers should import the package root:

```ts
import {
  createNativeRuntimeBridge,
  frameCursor,
  type RuntimeBridge,
} from '@asha/runtime-bridge';
```

Reference/mock helpers are opt-in through the explicit reference entrypoint:

```ts
import {
  createMockRuntimeBridge,
  createMockRuntimeSession,
} from '@asha/runtime-bridge/reference';
```

## Boundary contract

Allowed through this facade:

- initialize an engine/runtime session;
- initialize a semantic `RuntimeSession` from a validated ProjectBundle-shaped request;
- load ProjectBundle-shaped ECRP content (`ProjectBundle`, `EntityDefinition[]`, `SceneDocument`) into the reference RuntimeSession state;
- load a world bundle-shaped DTO;
- submit generated contract command batches;
- step deterministic authority ticks;
- read render/projection diffs;
- read semantic telemetry/replay/hash summaries;
- read generated tunnel, combat, nav/pathfinding, lifecycle, and ECRP CapabilityState evidence through typed RuntimeSession readouts;
- submit typed primary-fire runtime action intents whose reference receipts and ECRP health/lifecycle/render readouts agree with the loaded ProjectBundle state;
- restart/reset a semantic session without exposing authority state;
- get/release opaque runtime buffer handles;
- save or inspect current world/composition state;
- use classified `RuntimeBridgeError` failures.

Forbidden for downstream consumers:

- no direct `@asha/native-bridge` imports;
- no `@asha/wasm-replay-bridge` runtime imports;
- no `../asha-engine/ts/packages/*/src/*` imports;
- no generated contract file edits or local contract forks;
- no package-root mock/reference backend imports;
- no raw `call(methodName, json)` bridge tunnels;
- no mutable `StateStore`, unchecked event application, renderer internals, or editor internals.

The raw native addon wrapper remains internal transport plumbing. This package is the only public package that may import it, and unwired native operations must fail closed with `operation_unimplemented` rather than inheriting mock behavior.

Every native facade call is checked against the operation descriptor generated from the Rust
bridge manifest. Inputs are rejected before addon invocation and outputs before consumer delivery,
including unknown fields/variants, missing fields, wrong scalar types, noncanonical handles, and
operation-specific byte limits. Generated protocol DTOs and their recursive validators come from
one Rust-derived schema IR; explicit custom validators cover the remaining transition DTOs.

Native Rust failures use a versioned structured envelope. `RuntimeBridgeError` preserves its
existing `kind` and message behavior while also exposing nullable `operation`/`path`, `retryable`,
bounded `details`, and `provenance`. Consumers should switch on `kind` and may use the extra fields
for diagnostics; they must not parse error prose.

## Internal layout

The package root is the production-safe transport surface. `@asha/runtime-bridge/reference` is the only approved public subpath, and it is for deterministic reference/mock helpers used by demos, tests, and compatibility smokes that intentionally opt into the fixture backend. Internally, `src/index.ts` is an exports-only barrel: `bridge.ts` owns transport errors and the bounded bridge interface, `mock.ts` owns the reference bridge, `native.ts` is the only raw `@asha/native-bridge` importer, `runtime-session-adapter.ts` constructs concrete transport-backed sessions against neutral `@asha/runtime-session` contracts, and `launcher.ts` owns the `GameRuntimeLauncher` composition facade.

`RuntimeSession` is the narrow semantic facade for game repos and Studio. It exposes initialized session state, ProjectBundle-shaped ECRP load/readout, generated command submission, deterministic ticks, camera/collision inputs, typed runtime action intents, lifecycle/restart receipts, generated-tunnel/combat/nav readouts, render projection summaries, telemetry, and restart. `createRuntimeSessionFacade` accepts an explicit `RuntimeBridge`; `createMockRuntimeSession` lives under `@asha/runtime-bridge/reference` for consumers that intentionally want the reference backend. The current reference implementation wraps the mock bridge where native runtime attach is not yet available, but its ECRP/action/lifecycle readouts are no longer demo-local counters: primary-fire receipts and CapabilityState readouts are derived from the loaded runtime project state. It keeps explicit non-claims for native runtime, raw StateStore access, arbitrary JSON bridge calls, and renderer ownership.

`GameRuntimeLauncher` stays in this package for now because it is a thin public orchestration facade over `RuntimeBridge` and must preserve the same fail-closed backend/profile rules as the transport facade. If launcher policy grows beyond bridge-backed launch/session read models, split it into a future domain package that depends on `@asha/runtime-bridge` instead of moving raw transport access upward.

## Metadata and checks

The package declares its Tier 1 role in `package.json` under `asha.publicSurface`. The CI bridge check runs `harness/public-surface/check-public-boundary.py` to keep the engine-owned TS public surface manifest, compatibility anchors, raw transport status, and the Rust `runtime-bridge-api` metadata aligned with the Den public-surface design.

## Browser named input

`BrowserInputHost` is the package-root browser normalization surface. It owns
keyboard/mouse DOM listener attachment and submits generated `RawInputSample`
values through an initialized `RuntimeSessionFacade`. Context priority,
consumption, and named-action resolution remain in the Rust Session rule.

`BrowserFpsResolvedActionConsumer` adapts only resolved `gameplay.*` actions to
camera movement/look state. The renderer host accepts a public RuntimeSession
input port and does not keep a parallel WASD table. Editor tools similarly
consume only `editor.*` actions. See `docs/named-input-actions.md` for the
catalog, modal-context, diagnostics, and non-claim details.
