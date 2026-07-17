---
status: current
audience: agent
tags: [ecrp, capability, activation]
supersedes: []
see-also: []
---

# Typed Capability Activation

Status: implemented authority/state foundation for Den task #5631.

Capability activation is not entity lifecycle and is not capability attachment.
It is a separate typed state available only to capability families whose owner
needs to suspend participation without deleting configuration.

## Activatable Inventory

The closed initial inventory is:

| Capability | Owner | Inactive behavior |
|---|---|---|
| `collision` | `CollisionRule` | The collider stays attached but does not participate in movement eligibility or collision sweeps. |
| `controller` | `ControllerRule` | The association stays attached but active-controller queries return no controller. |

Transform, bounds, containment, and asset binding are structural and do not gain
activation state. Render projection retains its existing `visible` semantic;
adding a second activation bit would make projection behavior ambiguous.

## Three Distinct Axes

For a known entity, `CapabilityActivationReadout` reports:

- `absent`: no capability record is attached;
- `inactive`: the capability record exists but its owner suspended it; or
- `active`: the capability record exists and its activation state is active.

The readout separately reports entity lifecycle (`active`, `disabled`, or
`tombstoned`) and `effectiveActive`. Disabling an entity suppresses effective use
without rewriting capability activation. Re-enabling restores effective use for
capabilities that were active and leaves explicitly inactive capabilities
inactive. Destroying an entity clears its live capabilities and activation state.

Newly attached collision and controller capabilities default to active. A
version-1 session snapshot that predates activation fields migrates attached
instances to active; version 2 persists the explicit state.

## Typed Authority Path

`core-entity` owns the atomic command, event, errors, query, hashing, snapshot,
and active-only accessors. `svc-entity-authoring` owns the Rule-owner gate and
maps results to generated `protocol-entity-authoring` projections.

- `CollisionRule` may change only collision activation.
- `ControllerRule` may change only controller activation.
- a wrong owner receives a typed `forbiddenOwner` diagnostic before mutation;
- invalid entity/capability/transitions receive typed rejection diagnostics;
- accepted transitions return the owner fact plus the current readout.

TypeScript receives generated request/outcome/readout shapes so tools can propose
and explain the transition. It receives no raw store access and cannot decide
acceptance.

## Determinism and Replay

Activation participates in `EntityStore::hash`, canonical session snapshots,
and the entity fixture dump. Replaying the same attachment and activation
commands produces the same hash. Save/load preserves inactive state exactly,
and rejected or forbidden transitions leave the hash unchanged.

No activation transition dispatches callbacks or gameplay events from storage.
Standard gameplay-event adaptation belongs to #5632, and invocation timing
belongs to #5600.
