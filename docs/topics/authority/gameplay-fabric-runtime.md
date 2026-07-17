---
status: current
audience: agent
tags: [gameplay, fabric, runtime, rust]
supersedes: []
see-also: []
---

# Gameplay Fabric Runtime Coordination

Status: current canonical runtime coordinator after #5748/#5750/#5751. This
surface executes the four stable invocation families over the immutable
registry, routes every proposal through one owner-routing primitive, admits
payloads through canonical typed codecs, and advances wave-frozen generations
under root-atomic rollback. Host, owner, and read-view implementations are
statically composed by the `RuntimeSession`; there is no runtime callback
registry.

The endpoint is expressive gameplay composition: downstream Rust modules can
observe semantic events, participate before an operation commits, and request
shared changes without acquiring ambient authority. Deterministic ordering and
evidence make those interactions explainable after they become complicated.

## Assignment Cell

`rule-gameplay-fabric` is a `rust-rule` crate. It may call the open gameplay
protocol, immutable registry, and existing generic reaction resolver. It cannot
import raw core state, renderer truth, bridge operations, WASM, or TypeScript.

Its public ports are intentionally narrow:

- `GameplayInvocationHost` is the one statically linked module entrypoint;
- `GameplayViewSource` freezes an immutable authority/read-view generation;
- `GameplayProposalRouter` routes post-commit follow-up proposals;
- `GameplayDecisionOwner` exposes an owner revision and one atomic pre-commit
  route.

The owning RuntimeSession uses the narrower mutable `GameplayWaveAuthority`
transaction port for live Observe delivery. That port exposes immutable view
freezing to invocations and reserves proposal routing plus module-fact
application for the barrier after the complete wave has returned. It is an
ownership seam for the Session aggregate, not a module API or another handler
registry.

Every proposal caller converges on the same closed-registry route inside
`GameplayFabricCoordinator`. The route resolves the owner, calls the statically
linked Rust port once, validates and normalizes its result, and returns an
opaque `GameplayRoutingReceipt`. That receipt binds the registry digest,
proposal identity/hash, resolved owner, acceptance, facts, diagnostics, routing
hash, and the complete ordered owner-event batch. Observe, decisions, and the
gameplay scheduler do not reconstruct routing evidence or resolve owners on
their own.

Observe invocation calls may also carry a `GameplayFrozenReadSet` assembled
from the module's declared plan. Its hash participates in delivery evidence;
read assembly failure stops before module behavior. The concrete vocabulary,
quotas, prefab-role resolution, and owner-query boundary are documented in
[`gameplay-declared-reads.md`](gameplay-declared-reads.md).

These are composition ports, not discoverable handlers. Modules, invocations,
subscriptions, contracts, owners, and ordering all come from the closed
`GameplayFabricRegistry`.

## Post-commit Observe Waves

`GameplayFabricCoordinator::observe` processes one committed root event in
breadth-first waves:

1. Validate that the event contract exists in the registry.
2. Freeze one view generation for the current wave.
3. Match exact contracts and bounded header selectors.
4. Invoke subscribers in validated module order and manifest-local order.
5. Buffer every module-invocation output without routing a proposal.
6. Reject undeclared events, proposals, decision results, or excess output.
7. Canonicalize event/proposal ids, chronology, emitter, and causation.
8. Resolve proposal owners from the registry and route them.
9. Validate owner-event declaration and canonical payload hash, replace its
   chronology/emitter/causation with the resolved route, and put the receipt's
   ordered events plus module events into the next wave.
10. Atomically apply the wave's validated module facts, record the authority
    barrier, and only then freeze the next generation.

Every invocation in a wave receives the same `FrozenGameplayViews`. Authority
may change only after all invocations in that wave have returned. The next wave
then freezes a new generation over current entity-owner, module-state,
prefab-instance, trigger, registry, and root evidence, so declared reads can
observe accepted prior-wave changes without same-wave order becoming a hidden
semantic input.

The public RuntimeSession host checkpoints root-scoped mutable authority before
the first wave. A later host failure, invalid output, stale module fact, or
budget exhaustion rejects the complete root and restores the pre-root entity
and module-state generations. Intermediate barriers remain in the rejected
reaction frame as attempted causal evidence, while the frame's before/after
state hashes prove that none of those writes became durable.

The host supplies normal `GameplayEventEnvelope` and
`GameplayProposalEnvelope` values. Module-controlled chronology, emitter, and
causation fields are overwritten at the coordinator boundary. A module never
selects a proposal owner.

An accepted route cannot silently discard its events. Observe enqueues the
receipt batch at its allocated next-wave sequence. Standalone callers must
either enqueue that same batch or reject the route before committing their
surrounding transaction. `observe_routed_events` is the recovery entrypoint for
an already-routed batch: it preserves the recorded wave, ordering, and
causation instead of pretending the events are new roots.

Lifecycle facts and named tick/scheduled moments use normal event contracts and
event phases. They do not add `on_created`, `on_tick`, or other feature-specific
trait methods.

## Pre-commit Decision Moments

`GameplayFabricCoordinator::decide` uses the same invocation entrypoint with a
`GameplayDecisionMoment` input. Its transaction order is fixed:

1. **Guard** invocations accept, reject, or abstain.
2. **Transform** invocations replace a typed operation `Workspace` using an
   exact input hash.
3. **React** invocations continue, cancel, or suspend and may also transform the
   Workspace.
4. The coordinator rechecks the authority owner's revision.
5. Exactly one final proposal is routed to that owner for atomic application.

The pre-commit owner returns the narrower `GameplayDecisionRoutingOutput`,
which has no event field. A decision therefore cannot publish a post-commit
event that the transaction has no safe delivery point for; event-producing
authority work belongs to the post-commit proposal route.

Guard rejection, reaction cancellation, suspension, stale owner revision, a
stale Workspace hash, or any invalid output returns without calling the owner.
A suspension receipt carries a coordinator-issued `GameplayDecisionContinuation`
bound to the decision id, proposal, transformed Workspace generation, registry
digest, owner, and expected owner revision. The explicit Session-owned
`GameplayDecisionContinuations` store gates resumption: missing, mismatched, and
already-consumed tokens fail before module invocation. A correct token is
consumed before work resumes; a later suspension deterministically rotates its
generation. Resuming against a changed owner revision consumes the continuation
and then fails as `Stale` before module invocation or owner routing.

`resolve_declared_reactions` adapts the existing `svc-game-rules` reaction
resolver into a `React` implementation. It preserves declared-read/effect
validation and priority-then-stable-id ordering instead of creating a second
reaction algorithm.

## Budgets and Evidence

The runtime applies Session, module, invocation, and subscription limits for:

- waves;
- events;
- proposals;
- invocations and deliveries; and
- canonical payload bytes.

Exhaustion records a typed diagnostic and stops at a visible boundary. It never
silently truncates or recursively dispatches.

Observe receipts contain registry, view-generation, delivery, output, event,
proposal, routing, fact, diagnostic, and final receipt hashes. Each
`GameplayWaveBarrierEvidence` binds the frozen generation, before/after hashes
for entity authority, module state, prefab instances, and triggers, plus the
accepted routing and module-fact hashes. Decision receipts record the
initial/final Workspace hashes, stage invocations, owner routing, status,
suspension token, diagnostics, and receipt hash. Identical inputs and registry
digests produce identical evidence.

`verify_gameplay_routing_evidence` independently recomputes the route hash from
serialized typed evidence and its ordered event batch. Recovery/replay rejects
altered payload hashes, event ordering, owner emitters, or routing hashes before
reconstructing delivery state.

## Structural Restrictions

- Invocation inputs contain frozen declared-view evidence, not raw
  `SessionState`; undeclared raw reads are unavailable by construction.
- Pre-commit invocations cannot emit post-commit events or follow-up proposals
  before the operation commits.
- Observe invocations cannot return Guard/Transform/React decisions.
- Events are semantic signals, never a mutation path.
- Runtime registration, reentrant dispatch, parallel evaluation, dynamic
  loading, network routing, and TypeScript authority remain out of scope.

The downstream SDK/module-host slice will supply typed read views and ergonomic
typed payload recovery over these ports. Persistent typed module state, composed
Session snapshots, and reaction playback/verification are described in
[`gameplay-module-state-replay.md`](gameplay-module-state-replay.md). None of
these layers introduces another dispatcher or mutation surface.

The first engine owner-fact adapters, standard `asha.*` event set, and bounded
legacy weapon `Transform` compatibility path are described in
[`gameplay-owner-event-adapters.md`](gameplay-owner-event-adapters.md).
