# Public Gameplay-module SDK and Static Composition

`asha-gameplay-module-sdk` is the public Rust path for downstream gameplay
authority. A game crate depends on `public-rust/gameplay-module-sdk`; it does not
import `engine-rs/crates/*` or copy engine contracts.

The surface is intentionally static for Wave 1. A module compiles a manifest,
typed codecs and state/view adapters, configuration schema metadata, and a real
behavior instance into a `GameplayStaticModuleProvider`. Session bootstrap
combines providers once through the existing `GameplayFabricRegistryBuilder`.
There is no second registry, dynamic callback table, or runtime plugin loader.

## Public module altitude

Normal behavior implements `GameplayModuleBehavior` and receives a
`GameplayModuleContext`. The context exposes typed event decoding, declared
frozen reads, named-view decoding, event-bound identities, tick, and decision
Workspace decoding. It does not expose entity/module stores, query-provider
implementations, proposal owners, or mutable Session state.

During a pre-commit React continuation,
`GameplayModuleContext::decision_resume_token` distinguishes the
coordinator-authorized resume from the initial decision. The token is evidence,
not module-owned authority: the public runtime host validates and consumes it
before invoking the module.

`GameplayModuleActions::emit<T>` and `propose<T>` accept the same typed codec
definition registered by the provider. They fill canonical payloads, hashes,
candidate ids, and envelope boilerplate for:

- namespaced gameplay events;
- shared proposals that still route to their registered Rust owner;
- module-local facts that the module-state coordinator validates/applies;
- Guard, Transform, and React decision outputs; and
- diagnostic trace codes.

The coordinator replaces candidate chronology and emitter values with its
authoritative root/wave sequence. Module facts are validated against the
closed registry and recorded in `GameplayObserveReceipt`; applying them remains
an explicit `GameplayModuleStateStore` authority step.

## Provider composition

`GameplayStaticModuleProvider` carries the real behavior instance plus the
existing manifest, linked-provider identity, typed codec registrations,
proposal/state owners, read providers, state adapters, and configuration schema
metadata. `GameplayStaticCompositionBuilder` consumes those exact registration
types and builds one `GameplayFabricRegistry`.

Providers also supply `GameplayModuleBuildProvenance`. Its computed source
identity covers the package name/version, supplied source bytes, sorted Cargo
feature set, and lockfile bytes. SDK identity derives from the linked public SDK
package/version and gameplay contract version. Contract identity derives from
the canonical manifest with identity fields removed. The historical
`artifact_hash` field now has the explicit meaning *linked provenance identity*:
SDK + contract + source provenance + concrete behavior type. It does not claim
reproducible machine-code hashing. `linked_from_manifest` computes provider
evidence independently, so stale manifest identities fail composition instead
of being copied into a tautological match.

Composition fails before activation for duplicate behavior instances,
linked-provider/version/contract/artifact disagreement, missing codecs or
owners, read kind/provider/selector/field mismatch, invalid configuration
schema ownership, or state/view adapter mismatch. Configuration metadata is
durable schema information; it is not live mutable state. ProjectBundle binding,
stable target resolution, atomic state initialization, and save/reload behavior
are specified in `docs/gameplay-module-bindings.md`.

Two independent fixture modules prove disjoint namespaces, configurations,
states, views, codecs, contracts, and behavior. Their real code executes through
the common coordinator. Changing one behavior changes its output and overall
receipt hashes. Module-local facts initialize and update their independent
typed state adapters.

## Downstream dependency and proof

```toml
asha-gameplay-module-sdk = { path = "../asha-engine/public-rust/gameplay-module-sdk" }
```

The committed downstream-shaped crate at
`harness/fixtures/gameplay-module-sdk/downstream-module` has no private engine
dependency. Its real multiplier behavior runs through
`GameplayStaticComposition::observe_session_event`; changing the multiplier
changes verified hashes. This narrow public Session helper fails shared
proposals closed because downstream code never supplies owner authority. The
owning engine RuntimeSession uses the same registry and invocation host with
its private owner router.

Run the public fixture and scaffold gate with:

```bash
./harness/ci/check-gameplay-module-sdk.sh
```

Run the ProjectBundle-authored downstream conformance path with:

```bash
./harness/ci/check-gameplay-module-conformance.sh
```

See [Gameplay-module conformance](gameplay-module-conformance.md) for the
machine report, human trace, negative fixtures, playback, and verification
replay contract.

Create a new module skeleton with:

```bash
./harness/tools/new-gameplay-module.sh \
  /path/to/game/crates/my-module \
  my-module \
  my.game.module
```

The command creates and tests a public-facade-only crate with a real static
provider/manifest. It also emits a TypeScript configuration projection. That
generated TypeScript is projection/configuration only: it cannot register
behavior, route proposals, or mutate authority.

## Legacy weapon compatibility

`LegacyWeaponEffectTransformBehavior<M>` adapts the prior public
`GameRuleModule` weapon hook into the common Transform family. The test uses a
real downstream module whose close- and long-range proposals differ, routes the
transformed Workspace through the registered owner, and proves distinct final
Workspace hashes. This is named compatibility code, not the permanent provider
model; delete it after legacy weapon consumers have moved to native gameplay
module contracts.
