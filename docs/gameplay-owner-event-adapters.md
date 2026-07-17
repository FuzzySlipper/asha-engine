---
status: current
audience: agent
tags: [gameplay, events, adapters, rust]
supersedes: []
see-also: []
---

# Engine Owner Facts on the Gameplay Fabric

Status: implemented first standard adapter set for Den task #5632.

Existing authority owners keep their current accepted fact and replay types.
`rule-gameplay-fabric` adapts those accepted outcomes into typed `asha.*`
gameplay events at the semantic boundary; it does not move combat, lifecycle,
state-machine, process, or modifier rules into an event subscriber.

## Standard Event Provider

`register_standard_owner_events` installs one statically linked engine provider,
its immutable manifest, and typed Rust codecs into a
`GameplayFabricRegistryBuilder`. The provider publishes these v1 contracts:

| Owner meaning | Gameplay contracts |
|---|---|
| Entity lifecycle | `asha.entity.created.v1`, `destroyed.v1`, `lifecycle-changed.v1` |
| Capability activation | `asha.entity.capability-activation-changed.v1` |
| Kinematic triggers | `asha.trigger.entered.v1`, `exited.v1` |
| Combat | `asha.combat.fire-hit.v1`, `fire-missed.v1`, `damage-applied.v1`, `entity-defeated.v1` |
| State machines | `asha.state-machine.attached.v1`, `transitioned.v1` |
| Processes | `asha.process.started.v1`, `mode-set.v1`, `stopped.v1` |
| Generic values/modifiers | `asha.game-rules.value-delta-resolved.v1`, `modifier-applied.v1` |
| Named runtime moments | `asha.session.tick.v1`, `asha.scheduler.moment-due.v1` |

Contracts carry versioned content hashes and normal typed codecs. The immutable
registry readout exposes their contracts, provider, topology, and digest without
adding a closed engine-wide enum for downstream game meanings.

## Semantic-Origin Adapters

The public adapter functions accept existing typed accepted results:

- `EntityLifecycleEvent` and `CapabilityActivationEvent`;
- `CombatReadout` and its ordered `CombatEvent` facts;
- `StateMachineEvent`;
- process-owned `DomainEvent` variants;
- accepted `GameRuleResolutionReceipt` values plus their typed request; and
- explicit Session tick or scheduler moment inputs.
- accepted kinematic trigger overlap enter/exit facts.

They preserve the useful facts already known by the owner: shooter and target,
damage before/after, defeat, machine transition and revision, process/mode,
value channel and delta, modifier duration/cadence, source/target, request and
replay hashes, and scheduled proposal kind. Entity references are carried in
the standard envelope headers as well as typed payloads where that makes the
semantic record self-contained.

Headers, tags, entity collections, event sequences, canonical payloads, and
hashes are deterministic. The adapter never applies authority. Rejected generic
game-rule resolution returns no events; other adapters accept only an owner's
past-tense accepted event/readout type, so a rejection has no value to adapt.

The FPS RuntimeSession combat path now records these gameplay events directly
on `FpsPrimaryFireReceipt`. If event adaptation were ever to fail, the combat
store is restored before the operation returns, avoiding a reported rejection
with hidden health mutation.

## Reaction Resolution

The existing `svc-game-rules` reaction resolver remains the algorithm for
declared reads, allowed effects/modifiers, priority, and stable-id ordering.
`resolve_declared_reactions` is invoked from the common `React` host path; the
coordinator retains transaction staging, Workspace validation, owner revision
checks, cancellation/suspension, routing, and evidence. There is no second
reaction dispatcher.

## Legacy Weapon Compatibility

`compatibility::run_legacy_weapon_effect_transform` is a bounded compatibility adapter for the
old `GameRuleModule::evaluate_weapon_effect` hook. It builds a closed one-module
registry, invokes the real legacy behavior as the standard `Transform` family,
updates a typed damage Workspace, and routes that Workspace to the existing
combat owner before the FPS combat path applies it.

The bridge preserves the legacy public request/receipt shape and now adds a
`gameplayFabric.transformAccepted` trace entry containing registry, final
Workspace, and decision-receipt hashes. Module rejection stays typed; invalid
proposal kind, target, channel, or hash fails at the common owner route and no
primary-fire mutation runs.

This compatibility adapter is intentionally named, namespaced, and bounded.
The preferred static provider and one-cell composition path exists; #5734 owns
the remaining Demo migration and deletion. New behavior must use that normal
provider path. The wrapper is not root-re-exported and is not precedent for
adding another bespoke hook.

## Non-Claims

- This is the first useful standard event set, not a migration of every
  historical `DomainEvent` consumer.
- Events do not become a mutation or replay-authority alternative to owner facts.
- Product-specific factory, RPG, Rulebench, or demo meanings remain in their
  downstream namespaces.
- Real downstream static provider composition is implemented; remaining work is
  migration of the named legacy wrapper, not invention of the public path.
