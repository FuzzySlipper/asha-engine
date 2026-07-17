---
status: current
audience: agent
tags: [bridge, runtime, native, wasm, boundary]
supersedes: []
see-also: [runtime-session-facade.md, runtime-bridge-boundary.md, consumer-compatibility.md]
---

# Runtime Bridge Boundary

The runtime bridge is the transport-neutral facade between TypeScript consumers and the Rust authoritative core. Native is the product path; WASM is the replay/golden verification target.

## Boundary Shape

- Semantic contracts: `@asha/contracts` (generated) / `protocol-*` crates
- Runtime bridge facade: `@asha/runtime-bridge` / `runtime-bridge-api`
- Native transport: `@asha/native-bridge` / `native-bridge` (napi-rs)
- Replay verification: `@asha/wasm-replay-bridge` / `wasm-api`

## Dependency Rules

- Only `@asha/runtime-bridge` may import `@asha/native-bridge`.
- `@asha/wasm-replay-bridge` may be imported only by devtools and test code.
- Policy/catalog packages import contracts only.
- No transport-specific type appears in any package's public API except native/wasm internals.

## Disallowed

- `callRust(methodName, json)`-style dispatch
- Raw JSON escape hatches
- Exposing `StateStore` handles to TS
- UI/renderer importing the native addon
- Duplicate hand-written schemas in the bridge

See `docs/runtime-bridge-boundary.md` for the full ADR and migration history.
