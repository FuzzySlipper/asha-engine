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
- load a world bundle-shaped DTO;
- submit generated contract command batches;
- step deterministic authority ticks;
- read render/projection diffs;
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

## Metadata and checks

The package declares its Tier 1 role in `package.json` under `asha.publicSurface`. The CI bridge check runs `harness/public-surface/check-public-boundary.py` to keep the engine-owned TS public surface manifest, compatibility anchors, raw transport status, and the Rust `runtime-bridge-api` metadata aligned with the Den public-surface design.
