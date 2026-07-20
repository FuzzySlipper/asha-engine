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
- [runtime-bridge-boundary.md](../topics/bridge/runtime-bridge-boundary.md)

## Public Downstream Surfaces

- `@asha/runtime-bridge` package root.
- `RuntimeBridge` remains the one compatibility root. `runtimeBridgePorts(root)`
  exposes fixed, compile-time capability views over that same object for
  consumers and test doubles that need a narrower contract.
- `@asha/runtime-bridge/reference` only where explicitly approved.
- `@asha/runtime-session` for transport-neutral RuntimeSession contracts and semantics.

## Fixed Capability Cells

The TypeScript root composes nine interfaces and the Rust `EngineBridge` stores
state under the same nine owners. These are review boundaries, not services or
plugins: there is no capability locator, independent transport, or second
runtime root.

| Cell | Authority responsibility | Lifetime / hash rule |
| --- | --- | --- |
| input | input session and resolved actions | session / input evidence |
| timeSimulation | cadence, fixed ticks, simulation queue | session / time state |
| sceneEntities | scene document and entity state | session / document hash |
| voxelAssetsBuffers | voxel worlds, assets, annotations, buffers | mixed explicit and session / state and resource hashes |
| camera | camera and controller state | session / projection hash |
| gameplay | FPS session and rule modules | session / session and replay hashes |
| projection | render and presentation projection | frame / frame hash |
| runtimeProjectLifecycle | canonical project admission, active identity, and close | project / generation and revision |
| replayEvidence | replay hashes and conversion evidence | session / evidence hashes |

The runtime-project lifecycle cell owns canonical admission and close. Other
cells remain one EngineBridge authority root and are reset or rebound from the
Rust-owned project activation transaction; callers cannot assemble or retain a
parallel bootstrap topology. Manual buffer handles still require
`releaseBuffer`; other voxel state is session-owned.

`RUNTIME_BRIDGE_PORT_CONTRACTS` is the public, inspectable lifecycle contract.
`ENGINE_BRIDGE_CAPABILITY_PORTS` mirrors it in Rust tests. Rust authority modules
delegate through boring owned fields; adding a port never grants another port
broad root access. The gameplay cell is the only bridge capability intended to
feed the one native gameplay composition cell described by the post-wave-one
campaign.

## Private Or Forbidden Paths

- Consumers must not import `@asha/native-bridge` or `@asha/wasm-replay-bridge`.
- Consumers must not call native addon symbols directly.
- Do not add generic method-name RPC or arbitrary JSON dispatch to make a
  feature reachable.

## Acceptance Gates And Goldens

- [check-bridge.sh](../../harness/ci/check-bridge.sh)
- [check-wasm-replay.sh](../../harness/ci/check-wasm-replay.sh)
- [check-native.sh](../../harness/ci/check-native.sh)
- [harness/bridge/validate-manifest.py](../../harness/bridge/validate-manifest.py)
- [harness/bridge/check-bridge-guardrails.sh](../../harness/bridge/check-bridge-guardrails.sh)

## Common Agent Mistakes

- Letting mock/reference behavior silently stand in for native authority.
- Adding a bridge verb before the Rust authority and generated DTOs exist.
- Turning a capability cell into a service locator, registry, or second bridge.
- Taking a full `RuntimeBridge` in a focused consumer or test when one of the
  exported capability interfaces is sufficient.
- Returning unclassified strings instead of typed bridge errors.

## Follow-up Routing

- New runtime operation: start with Rust authority and protocol DTOs, then update
  [bridge-manifest.toml](../../engine-rs/crates/bridge/runtime-bridge-api/bridge-manifest.toml).
- Native implementation gap: keep the facade fail-closed and create a native
  bridge follow-up.
- Consumer needs an easier API: add a semantic facade in `@asha/runtime-session`
  or another approved package root.
