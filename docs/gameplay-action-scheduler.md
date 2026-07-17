---
status: current
audience: agent
tags: [gameplay, scheduler, actions, rust]
supersedes: []
see-also: []
---

# Gameplay Action Scheduler

Status: implemented gameplay scheduling authority for Den task #5605.

`rule-scheduler` now contains two intentionally separate systems:

- `ChunkScheduler` remains the transient, version-checked queue for generation,
  meshing, collision rebuild, and upload work.
- `GameplayActionScheduler` is canonical Session authority for shared deferred
  gameplay proposals.

They share deterministic scheduling concerns, but not state, replay meaning, or
execution APIs.

## Explicit Action State

The gameplay scheduler stores either:

- `TickScheduledActionDraft`: propose an operation at or after one authority
  tick; or
- `EventConditionedActionDraft`: propose an operation after one exact gameplay
  event contract and bounded header selector matches, unless a tick timeout
  wins first.

Each accepted action receives an owner-assigned insertion sequence and retains
its stable action id, priority, proposal envelope, source, and causation. Ready
ordering is:

1. execution/timeout tick;
2. declared priority;
3. stable action id; and
4. insertion sequence.

Wall-clock time, arbitrary predicates, callbacks, coroutines, and async runtime
state are absent.

## Read Match, Then Mutate

`due_action_ids`, `matching_action_ids`, and `timed_out_action_ids` are read-only
queries over a frozen queue. Matching an event does not immediately mutate the
queue or apply its proposal.

The scheduler core accepts an explicit `GameplaySchedulerCommand`:

- schedule tick/event action;
- execute a due tick action;
- trigger a matching event action;
- record timeout or cancellation; or
- record the later proposal owner's accepted/rejected routing outcome.

This preserves #5600 wave semantics: all Observe participants can collect
against one frozen wave, then the scheduler owner records the trigger and emits
one `GameplayScheduledDispatch` for normal proposal-owner routing. A scheduled
event cannot retroactively change the fact it observed.

The core command is not a caller-authentication envelope. In the product path,
only a `GameplayRuntimeSchedulerPort` borrowed from one live
`GameplayRuntimeHost` can apply the safe command subset. Schedule requests with
undeclared event or proposal contracts fail before queue mutation. Action ids
are retired after any terminal outcome and cannot execute or be reused.

## Command authority threat model

The trusted downstream Rust composition/transport adapter owns the live host.
Gameplay modules receive only `GameplayModuleContext`; TypeScript receives only
generated transport moments. Neither receives the host or scheduler port.

Possession of `GameplayRuntimeSchedulerPort<'host>` is therefore the command
authorization boundary. The port:

- borrows exactly one live host and has no target Session argument;
- is not cloneable or serializable;
- cannot outlive, replace, snapshot, or restore its borrowed host;
- exposes schedule/trigger/timeout/cancel plus one canonical `route` operation;
  and
- does not expose the internal routing-receipt or delivery-completion commands.

There is intentionally no caller-supplied owner id. The owner in
`GameplayRuntimeSchedulerDefinition` identifies the scheduler in snapshots,
readouts, and routing evidence; it is not an authentication principal. Missing,
foreign, cross-Session, stale-generation, and replayed port evidence are
unrepresentable rather than string-compared. After restore, the adapter must
borrow a fresh port from the restored host. Commands still reject repeated or
retired action ids before state mutation.

## Typed Outcomes

The fact log distinguishes:

- scheduled;
- triggered;
- timed out;
- cancelled;
- rejected for a missing target;
- rejected for stale causation;
- accepted by the destination owner; and
- rejected by the destination owner with diagnostic codes.

Every command receipt includes the before/after scheduler state hashes and the
optional next-boundary dispatch. Fact hashes and diagnostic codes are sorted
before they enter authority evidence.

## Persistence and Replay

Snapshots contain the declared contract set, owner, next insertion sequence,
pending queue, complete outstanding dispatches, retired ids, fact log, and
state hash. Triggering persists the canonical proposal instead of leaving it
only in a transient command receipt. Decode rejects newer schemas, duplicates,
hash drift, and any snapshot whose queue does not equal replaying its facts.

`outstanding_dispatches` is the deterministic pre-route interruption/reload
surface. Dispatches survive save/reload and fact replay and retire only when the
scheduler receives the opaque `GameplayRoutingReceipt` created by the canonical
`GameplayFabricCoordinator` route. The scheduler checks its stored proposal id,
contract, and canonical proposal hash against the receipt. Callers cannot
independently assert the registry, destination owner, accepted flag, fact
hashes, diagnostics, routing hash, or emitted events.

An accepted receipt with owner events enters a second typed recovery state,
`outstanding_event_deliveries`. The routing fact persists the full normalized
event envelopes and routing evidence, not only their hashes. After interruption
the host delivers that recorded batch through `observe_routed_events`; it does
not invoke the owner again. `EventDeliveryCompleted` consumes the batch using
the exact routing hash and event IDs. Snapshot decode and fact replay recompute
routing evidence before recreating this state, so altered events cannot become
pending work.

Playback consumes the recorded facts; verification replay can reconstruct both
pre-route dispatches and post-route event delivery without rerunning authority.
Wrong completion hashes and duplicate completion attempts fail without queue
mutation.

## Public RuntimeSession composition

`GameplayRuntimeHost` owns the product-facing scheduler authority. Project
activation requires a `GameplayRuntimeSchedulerDefinition` whose owner, event
contracts, and proposal contracts validate against the same closed gameplay
registry used by module dispatch. The trusted composition adapter borrows a
typed port with `scheduler_port`, then uses `apply` and `route`. Routing consumes
the actual fabric receipt from the registry-resolved proposal owner, not
caller-authored completion evidence.

The port's `route` operation is also the retry operation. For an outstanding
dispatch it routes authority, records the typed result, delivers any returned
events, and records completion as one host transaction. For an outstanding
event delivery it skips authority and resumes only the recorded Observe batch.
Invalid owner output, scheduler recording failure, or rejected downstream
delivery restores authority, scheduler state, and reaction evidence to their
pre-call values.

The host snapshot embeds the complete scheduler snapshot, including pending
actions, outstanding dispatches, and outstanding owner-event deliveries. Its
public readout provides the scheduler state hash and full counts with an ordered
256-item projection window and an explicit truncation bit. `runtimeHostHash`
binds both that scheduler state hash and the canonical current
EntityStore/prefab authority hash.

The transport-neutral TypeScript host contract exposes the same authority as a
required scheduler load definition, typed `schedulerCommand` and
`schedulerRoute` moments, and a nested scheduler readout. A downstream product
still supplies the small statically linked native provider that borrows the
scoped Rust port while mapping each closed moment; it does not retain the port,
implement a parallel scheduler, or send a raw completion receipt.

The integration fixture models a factory `crafting-completed` event triggering
a progression-counter proposal, then routes that proposal through a closed
fabric registry and records the resulting routing receipt. This is the intended
expressive path: semantic condition, explicit deferred authority, normal
owner-routed mutation.

The public-runtime-host fixture additionally proves both interruption windows:
an outstanding dispatch survives save/restore before routing, and a recorded
owner-event delivery survives save/restore after authority mutation. Retrying
the latter delivers the recorded event exactly once without rerouting authority.
