# Static RuntimeSession Composition

Status: preferred native provider shape after Wave 1 stabilization.

`asha-runtime-session-composition` is the public Rust entrypoint for a game
that links its own gameplay modules. A downstream native addon builds its
closed `GameplayStaticComposition`, supplies its ProjectBundle bindings,
prefabs, declared reads, triggers, and scheduler definition, and consumes a
`StaticRuntimeSessionBuilder` to obtain one `EngineBridge` root.

```toml
[dependencies]
asha-gameplay-module-sdk = { path = "../asha-engine/public-rust/gameplay-module-sdk" }
asha-runtime-session-composition = { path = "../asha-engine/public-rust/runtime-session-composition" }
```

```rust,no_run
# use asha_runtime_session_composition::{GameplayRuntimeProjectInput, StaticRuntimeSessionBuilder};
# fn project_input() -> GameplayRuntimeProjectInput { todo!() }
let bridge = StaticRuntimeSessionBuilder::activate_project(project_input())?
    .build()?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

The returned bridge is the native provider cell. It owns one EntityStore. FPS
combat/lifecycle rules borrow that store from the bridge rather than embedding
another store, and the gameplay host temporarily receives ownership of the
same store for reactions, decisions, scheduler routing, trigger reconciliation,
snapshotting, and readout. Ownership is transferred, never cloned into a live
shadow world.

## In-process authority flow

- Accepted FPS combat events enter the gameplay fabric before presentation
  projection is committed. A rejected reaction restores FPS and entity state.
- Accepted collision-constrained first-person poses update the registry-resolved
  player entity and reconcile triggers against that exact transform.
- Accepted autonomous movement mutates the same EntityStore and then reconciles
  trigger authority; no actor id, extent, transform, or owner event is ferried
  through TypeScript.
- Pre-commit decisions and scheduler commands/routes use explicit methods on
  the composed Rust cell. Their owner ports remain statically linked and
  revision guarded.

TypeScript receives only the ordinary `RuntimeBridge` returned by the provider's
`createRuntimeBridge` factory. `RuntimeSessionFacadeOptions`, the native provider
contract, and browser-host have no `gameplayHost` option. Browser-host exposes
only `/asha/browser-host/runtime-bridge/<method>`; there is no second gameplay
endpoint or lifecycle.

## Lifecycle and evidence

`read_composed_runtime_session` binds the sole entity-authority hash to the
gameplay registry, module state, scheduler, pending continuations, reaction
frames, and current FPS epoch/replay hash. `checkpoint_composed_runtime_session`
adds the canonical gameplay snapshot. A fresh downstream composition builder
can restore that checkpoint without publishing mutable stores.

One browser bridge-client id maps to one composed bridge. Browser close clears
the bridge pool. Project unload drops FPS state, module host, scheduler,
continuations, gameplay replay evidence, and project entity authority together;
engine-session resources retain the explicit lifetime documented by
`RuntimeBridge` until their owning session is closed.

## Deliberate limits

- Composition is static Rust linking. There is no dynamic library discovery,
  generic RPC, callback registry, or TypeScript semantic-event route.
- The older standalone `asha-gameplay-runtime-host` remains an implementation
  and migration boundary for named Wave 1 consumers. It is not the preferred
  provider topology; its quarantine and deletion conditions are owned by the
  compatibility campaign task.
- Native wire declarations, operation limits, runtime validation, and structured
  error envelopes are generated or bound from the bridge/protocol source of
  truth. This builder establishes the authority object graph behind that
  validated boundary; it does not provide an alternate unvalidated route.
