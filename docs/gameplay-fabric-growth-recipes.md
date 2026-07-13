# Gameplay Fabric Growth Recipes

Status: current paved road for adding expressive gameplay through governed
Rust authority.

The gameplay fabric lets a Game Project add meaning without asking ASHA for one
new enum variant, hook, or RuntimeSession method per mechanic. The positive
endpoint is broad: compiled downstream modules can observe semantic facts, own
persistent domain state, transform or guard in-flight decisions, suspend and
resume reactions, emit further namespaced events, and propose mutations to the
owners that already control shared engine state.

The rails make this a fabric rather than an ambient bus:

- topology closes at bootstrap from statically linked providers;
- contracts, codecs, reads, outputs, owners, budgets, and ordering are declared;
- ProjectBundle configuration and bindings resolve stable identities;
- every invocation receives frozen, bounded reads rather than store access;
- module-local facts and shared proposals have different owners;
- suspension continuations belong to the runtime host;
- reaction frames, decision receipts, snapshots, playback, and verification
  make the result inspectable.

## Choose the semantic origin first

Before adding a contract, identify where the meaning becomes known.

- If an existing owner has just accepted a lifecycle, combat, process,
  state-machine, modifier, scheduled, or trigger transition, adapt that typed
  past-tense fact at the owner boundary.
- If several authorities must participate before commit, create a decision
  moment over an operation Workspace and use Guard, Transform, or React.
- If the behavior is purely presentation, consume accepted evidence in a
  projection. Do not route rendering, audio, telemetry, or UI callbacks back
  into gameplay authority.
- If meaning is only a polling inference, first ask whether the responsible
  owner can publish a richer accepted fact. Do not reconstruct semantics from
  display names, hierarchy paths, incidental tags, or frame-to-frame diffs.

One accepted origin can have many disposable presentations. Preserve its
causation and correlation identity while projecting it to render effects,
audio, UI, telemetry, achievements, or tools. Those consumers do not become
owners of the originating fact.

## Add a gameplay event

1. Define an open `GameplayContractRef` in the publisher's namespace with an
   explicit version and a hash derived from its canonical schema descriptor.
2. Define the typed Rust payload and register that descriptor-bound codec in the compiled
   provider.
3. Publish it from an accepted semantic origin. Engine meanings use an
   `asha.*` owner adapter; downstream meanings stay in the game's namespace.
4. Declare each Observe subscription, header selector, output allowance,
   ordering edge, and budget in the immutable module manifest.
5. Test canonical payload bytes, envelope identity/causation, stable ordering,
   registry digest, invocation evidence, and rejected-origin silence.

Do not extend a closed engine-wide `GameEvent` enum and do not create a
fire-and-forget string or JSON bus.

## Add a decision participant

Use the family that matches transaction semantics:

- **Guard** accepts or rejects the pending operation without mutation.
- **Transform** returns a new canonical Workspace generation.
- **React** continues, cancels, or explicitly suspends the decision.

Declare the invocation input/output contracts, reads, ordering, and quotas.
Call `GameplayRuntimeHost::decide` with a typed moment and a statically linked
`GameplayRuntimeDecisionOwner`. The registry-resolved owner validates and
atomically commits only the final Workspace. Missing, malformed, cancelled,
suspended, stale, or unauthorized decisions must never call it.

For suspension, modules may request `React::Suspend` and inspect the authorized
resume token on continuation. They do not validate or store token authority.
The host binds, persists, consumes, and rejects continuation generations before
invocation.

## Add module-owned state

1. Define typed Config, State, Fact, and optional View values plus their
   versioned contracts.
2. Implement `GameplayTypedModuleStateAdapter`: decode configuration, initialize,
   encode/decode state, apply typed facts, migrate versions, and project the
   named view.
3. Register exactly one state owner and adapter in the compiled provider.
4. Emit module-local facts with compare-and-set revision evidence. Never hide
   mutable gameplay state inside the behavior instance.
5. Bind configuration and scope through the ProjectBundle registry.
6. Prove initialization atomicity, foreign/stale rejection, save/restore,
   migration, recorded-fact playback, verification replay, and hash drift.

Session, EntityDefinition, Prefab, and stable prefab-part scopes are available.
Part identity is `PrefabPartReference { prefab, role }`, never a display label or
hierarchy search.

## Add a declared read or owner query

Prefer existing read kinds before adding vocabulary:

- event identity;
- entity lifecycle/transform/collision/controller capability;
- relationship traversal;
- stable prefab part;
- bounded tag/scope selection;
- module-owned named view;
- bounded owner query.

The manifest declares exact fields, selectors, provider, and item bound. The
runtime plan binds the request to event or proposal identities and freezes an
owned `GameplayFrozenReadSet` before behavior runs.

Add an owner-query family only when a real authority owner can provide a typed
request/receipt and a downstream mechanic needs it. Preserve owner-semantic
ordering and revision evidence. Never expose a raw SessionState reference,
generic ECS query, predicate closure, unbounded scan, or mutable result.

## Add authored configuration or a binding

1. Add configuration schema/codec metadata to the compiled provider.
2. Use `GameplayModuleBindingRegistryBuilder` from the public SDK to bind the
   exact module, state schema, configuration, reads, outputs, and stable target.
3. Load the registry through the normal ProjectBundle plan.
4. Let Rust validate provider/artifact/config identity, target resolution,
   scope uniqueness, and all requested contracts before initializing any state.
5. Keep later prefab-instance layers explicit and hash-bound.

Generated TypeScript may author and project these DTOs. It does not register
behavior or become configuration-validation authority.

## Add a shared mutation proposal

Use a shared proposal when the desired change belongs to an existing engine or
domain owner rather than the module's own state.

1. Define a typed proposal contract and exactly one proposal-owner registration.
2. Have the module emit the proposal through `GameplayModuleActions`.
3. Extend the public host owner router at capability height, not with a method
   named for one game's event.
4. Validate owner, target, operation hash, expected revision, and payload before
   calling the existing Rule/service.
5. Record accepted/rejected routing evidence in the reaction frame or decision
   receipt.

Do not give the module a mutable callback, store reference, or owner-discovery
API. Unsupported owners fail closed.

## Add a scheduled gameplay action

Use the `GameplayRuntimeHost` scheduler port when a typed proposal must become
eligible at an authority tick or after a matching gameplay event, rather than
inventing a timer callback or polling loop.

1. Include the event and proposal contracts in the closed gameplay registry.
2. Declare the scheduler owner and its permitted contracts in
   `GameplayRuntimeSchedulerDefinition` at project activation.
3. In the trusted Rust adapter, borrow `host.scheduler_port()` only while
   mapping one typed `schedulerCommand` moment to `port.apply`. A triggered
   action becomes a recoverable outstanding dispatch; it does not call the
   destination owner implicitly.
4. Map `schedulerRoute` to `port.route`. The host routes the retained proposal
   through the registry-resolved owner and records the real gameplay-fabric
   receipt exactly once.
5. Inspect the bounded scheduler readout and preserve the complete host
   snapshot for interruption recovery.
6. Test save/restore before routing, accepted and rejected owner outcomes, and
   replay of a retired action id.

Do not retain the scoped port, treat the configured scheduler owner string as a
credential, keep the proposal only in a TypeScript timer, synthesize a routing
receipt, or add a downstream scheduler alongside the host authority.

## Add a trigger-driven interaction

1. Author a generated `GameplayTriggerDefinition` with stable trigger identity,
   typed geometry, eligibility, scope, and tags.
2. Load it through `GameplayRuntimeProjectInput`; do not construct a private
   collision world downstream.
3. Move or update subjects through accepted EntityStore authority operations.
4. Reconcile overlap state through `GameplayRuntimeHost`. Enter and exit owner
   facts are exactly once; continued overlap is state, not a default callback.
5. Subscribe the gameplay module to `asha.trigger.entered.v1` or
   `asha.trigger.exited.v1`, and declare any current-overlap owner query it needs.
6. Test spawn-inside, teleport, deactivate/reactivate, destruction, save/reload,
   stable ordering, and visible downstream behavior.

This is kinematic trigger authority. It makes no rigid-body, impulse, joint, or
solver claim.

## Start a downstream module

Create and test a public-root-only skeleton:

```bash
./harness/tools/new-gameplay-module.sh \
  /path/to/game/crates/my-module \
  my-module \
  my.game.module
```

Run the committed public gates:

```bash
./harness/ci/check-gameplay-module-sdk.sh
./harness/ci/check-gameplay-module-conformance.sh
./harness/ci/check-gameplay-runtime-host.sh
```

The external fixture is
`harness/fixtures/gameplay-module-sdk/downstream-module`. The real consumer
proofs are:

- `asha-demo` #5636: configured trigger-driven real-time module behavior through
  the public static host;
- `asha-rulebench` #5638: pre-commit Transform/React suspension, persistent
  module state, owner routing, replay evidence, and workbench inspection.

Conformance proves a module is structurally honest. A human-facing consumer
proof demonstrates that the paved road supports actual gameplay.

## Promotion rule

Keep game-specific capability families, event contracts, state, owners, and
decision semantics downstream while they are still domain-shaped. Promote a
typed primitive when multiple consumers need the same semantics and the public
owner/read/binding/replay contract survives both.

Promotion is compatible movement of authority, not a rewrite into an engine
singleton. A promoted module may keep the same open contracts and evidence
shape while its owner moves to a more generic public cell.

## Non-claims

- No dynamic loading, JavaScript authority, runtime handler discovery, or
  mutable global registry.
- No permission for modules to inspect arbitrary stores or invent shared
  mutation owners.
- No claim that every gameplay mechanic belongs in the fabric; local explicit
  Rust rules remain appropriate when no cross-owner composition is needed.
- No claim that deterministic evidence is the product endpoint. The endpoint is
  expressive gameplay whose authority and failures remain inspectable.
