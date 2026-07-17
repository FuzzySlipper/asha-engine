---
status: current
audience: agent
tags: [physics, dynamics, boundary, adr]
supersedes: []
see-also: []
---

# ADR: Kinematic collision, triggers, and future dynamics boundary

Status: **Accepted boundary; dynamics deferred**  
Task: #5607  
Related Wave 1 trigger task: #5662  
Evidence-gated future dynamics campaign: #5663

## Decision

ASHA remains kinematic and query-driven in Wave 1. Collision queries, movement
constraints, trigger overlap lifecycle, and rigid-body dynamics are different
responsibilities and must not be collapsed into a generic physics callback.

- Existing movement and camera paths continue to propose explicit transforms
  that authority validates and collision queries constrain.
- #5662 may add trigger definitions plus canonical overlap enter/exit lifecycle
  on the existing collision substrate. Triggers do not apply forces or collision
  response.
- No rigid-body body type, dynamics capability, solver loop, physics tick hook,
  or Rapier dependency is added by this decision.
- A future dynamics owner must be exclusive with kinematic transform ownership:
  one body is moved by commands/rules or by the dynamics owner, never both.

## Current implementation truth

The repository already contains useful physics-adjacent code, but it does not
contain a rigid-body dynamics system.

### Collision queries

`engine-rs/crates/services/svc-collision` is the only `parry3d-f64` consumer. It
builds a deterministic, versioned collision projection from authoritative voxel
chunks and exposes typed occupancy, ray, AABB overlap, and axis-swept AABB
queries. The Parry shapes are a derived cache, not canonical state, and callers
cannot mutate the internal collision world.

The runtime camera path uses the swept AABB query to accept or block
axis-separated kinematic travel through voxel terrain. It does not integrate
velocity, mass, impulses, or contact response.

### Entity and character movement

`core-entity` owns explicit `MovementCommand` and first-person motion paths.
Eligible entities have authority-owned transform, bounds, and collision
capabilities. Movement resolves an attempted delta in stable X/Y/Z order,
records moved/slid/blocked evidence, and applies the accepted transform. Static
colliders are immovable. FPS and autonomous navigation paths are likewise
command/rule-owned transform changes, not solver-owned bodies.

### The current `svc-physics` crate

`engine-rs/crates/services/svc-physics` is not an empty stub: it contains a
bounded deterministic **kinematic integrator** using `TickDelta`, explicit
seconds-per-tick, velocity, acceleration, gravity, and semi-implicit Euler. It
has no mass, impulses, angular motion, broadphase, contacts, constraints, or
solver state. Its collision-aware mode returns
`collision_query_required` rather than guessing.

At this decision point, repository search finds no production workspace caller
of `integrate_kinematic` outside the crate's own tests. The crate therefore does
not establish Session dynamics ownership and must not be described as an
implemented physics engine.

### Trigger state

`rule-trigger-volume` now owns semantic kinematic trigger definitions and the
canonical active overlap set. It samples the existing EntityStore
collision/bounds/transform capabilities, derives stable enter/continued/exit
transitions, and persists those pairs with the gameplay RuntimeSession.
`rule-gameplay-fabric` adapts accepted facts to `asha.trigger.entered.v1` and
`asha.trigger.exited.v1`; it does not perform overlap detection.

This is endpoint AABB sampling aligned with the existing entity movement
substrate. A teleport whose destination is outside a trigger does not invent a
continuous crossing event. Rotation-aware shapes, swept trigger CCD, and
rigid-body contact response remain outside the implemented claim.

## Vocabulary sketch

These terms are design vocabulary, not a request to compile one speculative
`PhysicsBodyKind` enum. Motion authority and collision response are orthogonal.

| Term | Motion authority | Collision behavior |
|---|---|---|
| **Static** | No runtime movement | Solid/query participant; rejects ordinary transform and movement commands |
| **Kinematic** | Commands, Rules, paths, or schedules propose transforms | Queries constrain accepted movement; no force response |
| **Character** | A kinematic specialization in current ASHA | Character-shaped constraints and grounded semantics may grow without becoming solver-owned |
| **Dynamic** | Future exclusive dynamics owner integrates pose/velocity | Bidirectional contact/force response; not implemented |
| **Trigger** | Orthogonal to motion kind; may be static or kinematic | Non-solid overlap query plus enter/exit lifecycle; never force response |

Keeping Trigger orthogonal prevents a future API from confusing “detects an
overlap” with “is moved by a solver.”

## Mutual exclusion and ownership requirements

Any future dynamics campaign must settle these rules before code lands:

1. Each movable body has one declared motion owner for a tick.
2. A dynamic body rejects ordinary movement/transform commands. Teleport,
   spawn, sleep/wake, and kinematic handoff require named owner commands and
   accepted facts.
3. Switching Kinematic/Character to Dynamic (or back) is an atomic authority
   transition with explicit velocity, contact-cache, and replay semantics.
4. `svc-collision` remains a query/derived-projection service. It does not
   silently become the owner of body state because a solver consumes its shapes.
5. Trigger active-pair state remains owned and replayable independently of
   transient solver contact caches.
6. Gameplay-fabric modules may observe accepted collision/dynamics facts and
   submit owner-routed proposals; the fabric is not the solver loop.
7. Save/load, hashing, stale handles, deletion, activation, and projection must
   name the authoritative dynamic state and its lifecycle.

## Tick and substep posture

There is no separate physics scheduler today. Existing kinematic paths use
explicit fixed tick inputs or caller-supplied bounded `dt_seconds` and produce
authority evidence at named command/tick boundaries.

A future solver must use a fixed rational relationship to the Session tick. Two
acceptable candidates should be measured against the driving game:

- one dynamics step per Session tick: simplest ordering, replay, and debugging;
- a fixed integer number of substeps per Session tick: better contact stability
  and fast-body behavior at higher CPU cost.

An independent wall-clock physics loop is rejected. The eventual substep count
must be configuration validated before Session start, must not vary with frame
rate, and must produce stable ordering for commands, contacts, facts, snapshots,
and hashes. The solver/precision choice must include native and canonical replay
parity evidence. No default such as “60 Hz under 20 Hz simulation” is accepted
without the consumer benchmark that justifies it.

## Evidence gate for a dynamics campaign

Future campaign #5663 remains dormant until all of these are available:

1. A named ASHA Game Project and user-facing scenario require a genuinely
   dynamic outcome such as bidirectional prop collisions, impulses/throwing,
   stacked bodies, joints, or ragdolls.
2. The scenario documents why kinematic paths, authored trajectories, triggers,
   and event-conditioned scheduling are insufficient.
3. Representative body/contact counts, world scale, fast-body cases, and CPU
   budget are captured in a runnable fixture or benchmark.
4. Acceptance defines the required replay/determinism tolerance and native
   versus canonical replay evidence.
5. The Planner accepts a mutation-owner matrix, kinematic/dynamic transition
   contract, save/load model, and failure diagnostics.
6. The solver/library choice is evaluated against that evidence, including
   whether Rapier is warranted and which collision projections can be reused.

Physics props, ragdolls, joints, or belt items are examples, not automatic
authorization. A request must satisfy the gate above rather than merely mention
one of those nouns.

## Relationship to #5662

#5662 implements the Wave 1 trigger slice: typed trigger roles, endpoint overlap
sampling, canonical active pairs, deterministic enter/exit owner facts, bounded
read views, RuntimeSession persistence, and gameplay-event adaptation. It does
not add velocity integration, mass, friction, restitution, impulses, solver
contacts, joints, CCD, ragdolls, or a physics tick.

Completing #5662 is evidence that trigger gameplay works; it is not evidence
that the dynamics gate has been met.

## Deferred and rejected

Deferred behind the evidence gate: rigid-body integration, Rapier, dynamic
materials, impulses, friction/restitution, joints, CCD, ragdolls, and a
solver-owned character controller.

Rejected now:

- placeholder `PhysicsBody` / `PhysicsCapability` types with no owner or solver;
- `on_physics_tick` callbacks or gameplay-fabric handlers as solver execution;
- silently treating the current kinematic integrator as production dynamics;
- letting both transform commands and a solver write the same pose;
- treating renderer or collision-cache state as authority.
