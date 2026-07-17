---
status: current
audience: agent
tags: [trigger, volumes, kinematic, gameplay]
supersedes: []
see-also: []
---

# Kinematic trigger volumes

Status: implemented for Den task #5662.

Kinematic triggers provide doors, zones, pickups, encounter volumes, and similar
gameplay with deterministic enter/exit facts. They reuse existing entity
collision geometry and stop short of rigid-body dynamics.

## Ownership

`core-entity` remains authoritative for entity lifecycle, transform, bounds,
collision participation, and collision activation. `rule-trigger-volume` owns
only:

- the typed semantic trigger definition: trigger entity, scope, and tags;
- the canonical ordered set of active `{trigger, subject}` pairs;
- accepted enter/exit facts, revision, diagnostics, snapshots, and hashes.

`rule-project-bundle` composes this state into
`GameplayBoundProjectBundleSession`. `reconcile_triggers` samples the current
EntityStore, adapts accepted facts into standard gameplay events, and delivers
them to the immutable Session module topology. The gameplay fabric never detects
collisions and modules never receive the private EntityStore.

## Tick semantics

At each named reconciliation moment:

1. Active trigger definitions are resolved against live collision, bounds, and
   transform capabilities.
2. Active non-trigger collision participants are tested using world-space AABBs
   in ascending entity-id order.
3. The next pair set is compared with the durable prior set.
4. Exits are emitted first in pair order, followed by enters in pair order.
5. Continued pairs emit no default event.

An enter happens exactly once when a pair first appears. An exit happens exactly
once when it disappears. Deactivation, entity disable/destruction, missing
providers, or movement out of the volume cannot leave a stale pair.

Spawn-inside and teleport-into produce enter at the next reconciliation.
Teleport-out produces exit. Teleport-through with an outside endpoint produces
no event: the current contract is endpoint AABB sampling, not continuous
collision detection.

## Save, reload, and reads

Trigger snapshots contain schema version, definitions, active pairs, revision,
and a content hash. RuntimeSession gameplay snapshots bind this state beside
module state. Reloading an unchanged overlap therefore does not duplicate enter.

`GameplayOwnerQuery::CurrentTriggerOverlaps` exposes a bounded, ordered,
revision/hash-bearing read through the existing closed owner-query boundary.
Missing providers, stale identities, undeclared fields/selectors, and quota
exhaustion remain typed failures.

## Gameplay events and downstream use

Accepted facts become:

- `asha.trigger.entered.v1`
- `asha.trigger.exited.v1`

The envelope carries the trigger as source, the overlapping entity as subject,
definition scope/tags, causation, and a typed payload with trigger, subject,
action, tick, cause, and pair hash. Downstream modules opt into the standard
owner-event provider with
`GameplayStaticCompositionBuilder::include_standard_owner_events`.

The compiled downstream fixture subscribes to the standard enter event and emits
an inspectable `trigger-reaction-proposed` follow-up for `door.open`, without
importing collision or Session stores.

## Explicit limitations

- Axis-aligned entity bounds only; current entity collision semantics ignore
  rotation and scale for AABB placement.
- Endpoint sampling only; no swept trigger CCD or default stay callback.
- No collision response, velocity, forces, mass, friction, restitution, joints,
  rigid bodies, or solver tick.
- Trigger definitions enter the public static RuntimeSession host through the
  generated `GameplayTriggerDefinition` contract and typed Rust composition.
  The current authored host input is explicit and hash-bound; Studio authoring
  ergonomics beyond that generated DTO remain separate product work.

The broader dynamics boundary and evidence gate remain in
[Kinematic collision, triggers, and future dynamics](physics-dynamics-boundary.md).
The configured downstream interaction recipe is in
[Gameplay fabric growth recipes](gameplay-fabric-growth-recipes.md).
