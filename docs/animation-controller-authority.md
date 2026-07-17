---
status: current
audience: agent
tags: [animation, authority, rust]
supersedes: []
see-also: []
---

# Animation controller authority

Status: implemented foundation for task #5648  
Authority crate: `rule-animation-controller`  
Generic transition authority: `rule-state-machine`

## Purpose

The animation controller turns named gameplay-facing parameters into a small,
deterministic controller state: the current graph state, any transition in
progress, selected clips, blend weight, and playback speed. It is the authority
input to later presentation projection. It does not sample a skeletal pose.

This is deliberately closer to Unity's Animator Controller than to Unity's
AnimationMixer internals:

- authored graphs describe parameters, states, motions, and transitions;
- authority decides which state and blend are active on fixed simulation ticks;
- the renderer later samples clips, crossfades, interpolates render frames, and
  owns all joint and bone transforms.

The endpoint is expressive gameplay feedback under inspectable authority, not
animation for its own sake and not a replay of renderer matrices.

## State-machine reuse decision

The controller **reuses** `rule-state-machine`; it does not duplicate its
transition application rules.

The existing `StateMachineStore` combines a generic finite-state specification
with one particular in-memory instance store. Animation needs its own parameter
state, transition progress, snapshots, and replay log, so placing animation
instances directly in that store would create two persistence owners.

Task #5648 therefore extracts and uses
`apply_transition_to_instance(spec, instance, request)`. The function preserves
the generic state membership, allowed-edge, expected-current-state, and
expected-revision checks. `rule-animation-controller` supplies a narrow adapter:

- stable graph IDs become `ProcessId` values;
- stable state IDs become `ModeId` values;
- hash collisions are rejected during catalog validation;
- condition selection, parameters, trigger consumption, blend resolution, and
  transition duration remain animation-specific authority;
- the controller owns persistence and reconstructs the generic
  `MachineInstance` from a validated snapshot.

Both crates remain in the `rust-rule` lane. No service-to-rule dependency is
introduced, and the generic state machine learns no animation or renderer
vocabulary.

## Catalog contracts

`AnimationCatalog` schema v1 contains named clip assets and versioned graphs.
Each graph declares:

- one clip asset and initial state;
- float, bool, and trigger parameters with typed defaults;
- states with either one clip or one linear two-clip blend;
- ordered transitions with typed conditions and fixed-tick durations.

Float parameters use signed thousandths (`value_milli`) inside authority.
Public authoring adapters may display ordinary decimal values, but the accepted
catalog and controller state do not depend on platform floating-point behavior.
Blend weights use `0..=1000`.

Transition priority is explicit: lower values win, and priorities must be
unique for transitions sharing a source state. This gives authors useful
overlapping conditions without making selection depend on array order.

Catalog loading returns only `ValidatedAnimationCatalog`. Validation rejects:

- unsupported schemas and malformed or duplicate stable IDs;
- missing assets, clips, states, or parameters;
- parameter/default/condition/blend type mismatches;
- invalid linear blend ranges;
- equal-priority transitions from one source state;
- states unreachable from the initial state;
- stable graph/state ID hash collisions.

An invalid graph is never installed into live controller authority.

## Fixed-tick evaluation

Controllers accept contiguous fixed ticks. A tick either advances an active
transition or selects the first matching transition by validated priority.
Triggers are consumed only when a selected transition references them.

`AnimationControllerState` contains:

- entity, graph identity, and graph version;
- current state and generic FSM revision;
- typed parameter values;
- resolved primary/secondary clip, fixed blend weight, and playback speed;
- optional transition identity, endpoints, duration/progress, and resolved
  target motion;
- a deterministic state hash.

The state hash is bound to the canonical graph hash and controller semantics,
not to the owning entity ID. Two entities evaluated from the same graph with
the same parameter and transition state therefore produce the same semantic
controller-state hash; persistence still keys each instance by entity.

Parameter writes are retained immediately but projection-facing changes are
published on evaluation ticks. An idle tick whose resolved controller state did
not change produces no `AnimationControllerChange`. Transition progress does
change controller state and is therefore visible while a crossfade is active.

## Persistence and replay

Every accepted attach, parameter write, trigger, and tick receives a contiguous
input sequence and contributes to the replay hash. Replaying those records into
a fresh authority with the same validated catalog reproduces controller state
and snapshot hashes.

Snapshots carry the schema version, validated catalog hash, controller state,
fixed-tick cursor, last emitted state hash, and input records. Decode requires
the same validated catalog and recomputes each controller state hash before the
artifact is accepted.

Neither snapshots nor replay records contain joints, bones, matrices, sampled
poses, renderer handles, wall-clock time, or animation callbacks. Those remain
presentation concerns. Resolved controller changes project through the G1
animation domain and drive renderer-local clip sampling. Gameplay-origin timing
facts and their verification-replay semantics are described in
`docs/animation-timing-semantics.md`.

## Deliberate Wave 1 limits

- one linear two-clip blend per state, without nested blend trees;
- no animation layers, masks, IK, procedural pose generation, or material curves;
- no renderer keyframe callback can mutate authority;
- clip asset content delivery remains the existing renderer-host responsibility;
  this catalog validates stable asset and clip identities, not GLB bytes;
- gameplay-critical timing and correlation are accepted controller facts; they
  are never inferred from renderer playback completion.
