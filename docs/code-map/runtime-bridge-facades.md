# Runtime Bridge And Native/WASM Facades Map

## Purpose

Route work around the transport-neutral bridge facade, native addon boundary,
WASM replay bridge, and fail-closed runtime operation exposure.

## Owns

- Curated runtime bridge operation manifest.
- `RuntimeBridge` facade shape and conformance.
- Rust `authority::EngineBridge` coordination behind native addon marshaling.
- Explicit non-product reference/mock classification on TypeScript fixture surfaces.
- WASM replay bridge surfaces used for tests and tools.

## Does Not Own

- Direct consumer imports of raw native or WASM transports.
- Gameplay authority that belongs in Rust services/rules.
- Renderer or UI ownership of runtime state.

## Primary Paths

- [engine-rs/crates/bridge/runtime-bridge-api](../../engine-rs/crates/bridge/runtime-bridge-api)
- [EngineBridge authority modules](../../engine-rs/crates/bridge/runtime-bridge-api/src/authority)
- [engine-rs/crates/bridge/native-bridge](../../engine-rs/crates/bridge/native-bridge)
- [engine-rs/crates/wasm/wasm-api](../../engine-rs/crates/wasm/wasm-api)
- [ts/packages/runtime-bridge](../../ts/packages/runtime-bridge)
- [ts/packages/native-bridge](../../ts/packages/native-bridge)
- [ts/packages/wasm-replay-bridge](../../ts/packages/wasm-replay-bridge)
- [runtime-bridge-boundary.md](../runtime-bridge-boundary.md)

## Public Downstream Surfaces

- `@asha/runtime-bridge` package root.
- `@asha/runtime-bridge/reference` only where explicitly approved.
- `@asha/runtime-session` for transport-neutral RuntimeSession contracts and semantics.

## Private Or Forbidden Paths

- Consumers must not import `@asha/native-bridge` or `@asha/wasm-replay-bridge`.
- Consumers must not call native addon symbols directly.
- Do not add generic method-name RPC or arbitrary JSON dispatch to make a
  feature reachable.

## Proof Gates And Goldens

- [check-bridge.sh](../../harness/ci/check-bridge.sh)
- [check-wasm-replay.sh](../../harness/ci/check-wasm-replay.sh)
- [check-native.sh](../../harness/ci/check-native.sh)
- [harness/bridge/validate-manifest.py](../../harness/bridge/validate-manifest.py)
- [harness/bridge/check-bridge-guardrails.sh](../../harness/bridge/check-bridge-guardrails.sh)

## Common Agent Mistakes

- Letting mock/reference behavior silently stand in for native authority.
- Adding a bridge verb before the Rust authority and generated DTOs exist.
- Returning unclassified strings instead of typed bridge errors.

## Follow-up Routing

- New runtime operation: start with Rust authority and protocol DTOs, then update
  [bridge-manifest.toml](../../engine-rs/crates/bridge/runtime-bridge-api/bridge-manifest.toml).
- Native implementation gap: keep the facade fail-closed and create a native
  bridge follow-up.
- Consumer needs an easier API: add a semantic facade in `@asha/runtime-session`
  or another approved package root.
