---
status: current
audience: consumer
tags: [gameplay, sdk, module, rust]
supersedes: []
see-also: []
---

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

## Boring authoring helpers

The public SDK keeps one authored `GameplayModuleInvocationTopology` per
invocation. `GameplayDerivedModuleTopology::derive` produces the manifest
subscriptions, invocation descriptors, read-view requirements, provider
registrations, and `GameplayRuntimeDeclaredReadPlan` values from that one
value. Apply it to the manifest before computing provenance, pass it to
`GameplayStaticModuleProvider::derived_topology`, and pass its
`declared_reads()` to the runtime host. There is no separate read-plan hash
string: the registry digest binds the declared topology and each invocation
records the computed frozen read-set/value hashes.

A minimal Observe module is deliberately plain:

```rust
let input = gameplay_contract("game.weather", "tick", 1, WEATHER_TICK_SCHEMA);
let invocation = GameplayModuleInvocationTopology::observe(
    "game.weather.tick.observe",
    "game.weather.tick.observe",
    input.clone(),
    input.clone(),
    GameplayHeaderSelector {
        source: None,
        target: None,
        scope: None,
        required_tags: Vec::new(),
    },
    8,     // deliveries per root
    1,     // outputs
    1_024, // payload bytes
);
let topology = GameplayDerivedModuleTopology::derive(
    "game.weather.module",
    vec![invocation],
)?;

topology.apply_to_manifest(&mut manifest)?;
let provider = GameplayStaticModuleProvider::linked_from_manifest(
    manifest,
    &provenance,
    WeatherBehavior,
)
.event_codec(gameplay_serde_json_codec_registration::<WeatherTick>(
    input,
    WEATHER_TICK_SCHEMA,
))
.derived_topology(&topology);
```

The generated module template uses this complete, executable shape rather than
emitting an inert behavior with no subscription or codec.

A stateful/reactive module adds its read selector and state semantics without
repeating them across manifest and host structures:

```rust
let observe = GameplayModuleInvocationTopology::observe(
    "game.door.trigger.observe",
    "game.door.trigger.observe",
    trigger_entered,
    door_reacted,
    selector,
    4,
    2,
    2_048,
)
.read(gameplay_session_state_read(
    "door-state",
    door_state_view,
    "provider.game-door",
    vec!["open".to_owned()],
    "single-module-state",
));
let topology = GameplayDerivedModuleTopology::derive(
    "game.door.module",
    vec![observe],
)?;

impl GameplaySerdeModuleStateAdapter for DoorStateAdapter {
    type Config = DoorConfig;
    type State = DoorState;
    type Fact = DoorFact;
    type View = DoorView;

    fn module_id(&self) -> &str { "game.door.module" }
    fn state_schema(&self) -> GameplayContractRef { door_state_contract() }
    fn fact_schema(&self) -> GameplayContractRef { door_fact_contract() }
    fn owner(&self) -> GameplayOwnerRef { door_owner() }
    fn initialize(&self, config: &DoorConfig) -> Result<DoorState, String> {
        Ok(DoorState { open: config.starts_open })
    }
    fn apply_fact(&self, _: &DoorState, fact: &DoorFact) -> Result<DoorState, String> {
        Ok(DoorState { open: fact.open })
    }
    fn migrate(&self, _: u32, state: &DoorState) -> Result<DoorState, String> {
        Ok(state.clone())
    }
    fn view_schema(&self) -> Option<GameplayContractRef> { Some(door_view_contract()) }
    fn project_view(&self, state: &DoorState) -> Result<DoorView, String> {
        Ok(DoorView { open: state.open })
    }
}

let configuration = GameplaySerdeConfiguration::<DoorConfig>::new(
    "game.door.module",
    door_configuration_contract(),
    vec![GameplayConfigurationFieldMetadata {
        name: "startsOpen".to_owned(),
        value_type: "bool".to_owned(),
        required: true,
    }],
);
let provider = provider
    .derived_topology(&topology)
    .state_adapter(gameplay_serde_state_adapter(DoorStateAdapter))
    .serde_configuration(configuration);
```

The serde state wrapper owns its contract/owner values and supplies the
canonical JSON decode/encode edges. Module authors implement only initialize,
fact application, migration, and optional view projection. This removes the
`OnceLock`/panic static-reference pattern without hiding state ownership.

### Ergonomic evidence

For the committed downstream fixture, the complete authored Rust source moved
from 1,522 to 1,425 lines while adding a source-of-truth assertion for the
derived topology. More importantly, each Pulse and decision read is now
declared once instead of being repeated across manifest invocation,
manifest read-view, provider registration, and runtime read-plan structures;
Observe subscriptions are derived from the same invocation value. The scaffold
grew from 88 to 132 lines because it now composes a real subscription and codec
instead of an unreachable behavior. These counts are review evidence, not an
API quality target: budgets, provider ids, selector capabilities, ordering,
owners, schemas, and provenance remain literal.

## Provider composition

`GameplayStaticModuleProvider` carries the real behavior instance plus the
existing manifest, linked-provider identity, typed codec registrations,
proposal/state owners, read providers, state adapters, and configuration schema
metadata. `GameplayStaticCompositionBuilder` consumes those exact registration
types and builds one `GameplayFabricRegistry`.

Providers also supply `GameplayModuleBuildProvenance`. Its computed source
identity covers the package name/version, supplied source bytes, sorted Cargo
feature set, lockfile bytes, and any explicitly supplied toolchain/build
environment labels. SDK identity derives from the public SDK package and its
declared gameplay contract version, so an unrelated engine package-version
bump is not itself a semantic incompatibility. Contract identity derives from
the canonical manifest with identity fields removed. The historical
`artifact_hash` field now has the explicit meaning *linked provenance identity*:
SDK + contract + source provenance + concrete behavior type. It does not claim
reproducible machine-code hashing. `linked_from_manifest` computes provider
evidence independently, so stale manifest identities fail composition instead
of being copied into a tautological match.

That strict provider/manifest check protects the linked binary itself. Authored
ProjectBundle loading separately compares semantic compatibility by default:
source, lockfile, feature, build-environment, or concrete-artifact drift becomes
a typed advisory when public module contracts and topology still match. An
explicit exact load retains the full provenance comparison and rejects drift.

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

The SDK no longer exports `LegacyWeaponEffectTransformBehavior<M>`. It had no
downstream consumer and duplicated the actual bridge adapter. The one retained
legacy weapon path is namespaced under `rule_gameplay_fabric::compatibility`,
is quarantined to `asha-demo`, and is deleted by #5734 after the Demo rule is a
native Transform. New modules cannot reach compatibility through the preferred
SDK root.
