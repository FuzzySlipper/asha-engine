---
status: current
audience: consumer
tags: [bridge, composition, runtime-session, native, provider]
supersedes: []
see-also: [runtime-session-facade.md, gameplay-module-sdk.md, gameplay-runtime-host.md]
---

# Static RuntimeSession Composition

`asha-runtime-session-composition` is the public Rust entrypoint for a game that
links its own gameplay modules. A downstream native addon builds its closed
`GameplayStaticComposition` and consumes a `DeferredRuntimeSessionBuilder` to
obtain one unloaded `EngineBridge` root. Canonical ProjectBundle admission later
supplies bindings, prefabs, declared reads, triggers, scheduler definitions,
scenes, and resources. `StaticRuntimeSessionBuilder` and caller-assembled
runtime topology are compatibility-only until Demo migration.

## Key Properties

- Static Rust linking. No dynamic discovery, generic RPC, callback registry, or TypeScript semantic-event route.
- One EntityStore owned by the bridge. FPS combat/lifecycle rules borrow it; the gameplay host temporarily receives it for reactions, decisions, scheduler routing, trigger reconciliation, snapshotting, and readout.
- TypeScript sees only the ordinary `RuntimeBridge` returned by the provider's `createRuntimeBridge` factory.

## Composed Operations

- `readComposedRuntimeSession()` — registry, binding, module-state, scheduler, reaction, decision, entity-authority, and FPS replay hashes.
- `readGameplayModuleView(request)` — selects one registry-owned named view by typed contract and scope.
- `applyGameplayPrefabPartInteraction(request)` — resolves prefab role and emits standard owner event through the closed gameplay registry.

See `topics/bridge/runtime-session-static-composition.md` for the full document.
