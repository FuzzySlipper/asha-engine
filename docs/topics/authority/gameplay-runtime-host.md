---
status: current
audience: agent
tags: [gameplay, runtime, host, rust]
supersedes: []
see-also: []
---

# Public Gameplay Runtime Host

Status: implemented gameplay subsystem; the preferred provider boundary is the
one-cell builder documented in `runtime-session-static-composition.md`.

`asha-gameplay-runtime-host` is the public Rust cell that turns a statically
linked gameplay-module composition into an owning RuntimeSession subsystem. It
exists because module authoring and conformance alone do not make a module
available to a live downstream game.

The host keeps the ECRP split explicit:

- downstream Rust contributes compiled module behavior through
  `asha-gameplay-module-sdk`;
- generated ProjectBundle contracts carry module bindings and semantic trigger
  definitions;
- the host owns validated prefab placement, module state, trigger overlap state,
  declared-read assembly, scheduled gameplay actions, reaction frames, and
  routing into engine authority owners;
- TypeScript advances and projects the host through a closed transport. It does
  not register callbacks, decode private stores, or apply accepted facts.

## Static composition boundary

A consumer builds one native provider cell that depends only on approved public
roots:

```toml
[dependencies]
asha-gameplay-module-sdk = { path = "../asha-engine/public-rust/gameplay-module-sdk" }
asha-gameplay-runtime-host = { path = "../asha-engine/public-rust/gameplay-runtime-host" }
```

The provider constructs its `GameplayStaticComposition` in Rust and passes it
to `GameplayRuntimeHost::activate_project`. There is no runtime crate discovery,
dynamic module loading, JavaScript callback table, or mutable global registry.
Changing linked module code changes the registry/artifact evidence and reaction
frame hashes.

`GameplayRuntimeProjectInput` carries the normal ProjectBundle load plan and
artifacts, the closed static composition, generated module bindings, typed
entity targets, gameplay spatial bootstrap data, declared read plans, and
generated `GameplayTriggerDefinition` values. It also carries one
`GameplayRuntimeSchedulerDefinition`: the scheduler owner plus the event and
proposal contracts it may retain. Activation rejects scheduler contracts that
are absent from the closed fabric registry. The prefab-aware activation path
also accepts `GameplayRuntimePrefabBootstrap`: canonical registry source, its
explicit validation catalog, and ordered authored or accepted player placement
commands. The host publishes no live state until registry validation, prefab
expansion, module binding, and normal project activation all succeed.

The current explicit `GameplayRuntimeSpatialEntity` input is transitional. It
keeps geometry typed and Rust-owned, but the same transform/bounds/collision
data should ultimately arrive through the existing generated entity-definition
load contract. It is not a second world format.

## Authority loop

The host has capability-height operations rather than one method per game
meaning:

- `observe` delivers an accepted semantic owner event through the closed fabric;
- `decide` runs the closed Guard -> Transform -> React pre-commit pipeline and
  routes one final canonical Workspace to a statically linked Rust owner;
- `reconcile_triggers` samples the authoritative EntityStore and delivers
  exactly-once enter/exit events;
- `move_actor_and_reconcile` applies collision-constrained movement first and
  samples triggers against the accepted pose;
- `scheduler_port` lends the trusted Rust composition/transport adapter a
  non-cloneable borrow of this host's scheduler command authority;
- the port's `apply` operation schedules, triggers, times out, or cancels a
  typed tick/event-conditioned action while retaining complete recoverable
  proposal state;
- the port's `route` operation routes an outstanding dispatch through the same
  closed registry and concrete owner router used by module proposals, then
  records the typed routing fact exactly once;
- `readout` exposes bounded hashes/counts plus the same bounded reaction-frame
  projections returned by advance; it never exposes mutable stores;
- `compose_snapshot` and `restore_project` persist and restore module state,
  trigger active pairs, binding provenance, authority state, reaction frames,
  decision receipts, pending continuation generations, and the complete
  scheduler queue/fact/outstanding-dispatch state.

`runtimeHostHash` includes a canonical snapshot hash of current EntityStore and
prefab-instance authority plus the scheduler state hash. An accepted movement
therefore changes the public host hash even when it does not cross a trigger or
change any module state. The nested scheduler readout exposes its owner, full
state hash, total counts, and up to 256 ordered pending actions/outstanding
dispatches with an explicit truncation bit.

The scheduler port is the honest command authorization boundary. It borrows one
live host, accepts no owner/session/generation token, and cannot be cloned,
serialized, redirected to another host, or carried through snapshot/restore.
The configured scheduler owner is routing and evidence identity only. Gameplay
modules and TypeScript do not receive the port; the statically linked native
adapter borrows it only while translating one closed transport moment. A
restored host therefore requires a new borrow, while duplicate action commands
still fail through scheduler replay/id retirement checks before mutation.

Each invocation freezes entity, module-state, trigger, and registry evidence.
The committed downstream fixture proves both a module-named state read and a
bounded current-trigger-overlap query. Accepted module-local facts update only
their registered state adapter.

Observe is a root transaction with deterministic wave barriers. All
invocations in one wave receive one immutable generation. After they return,
the host routes their proposals, applies their module-fact batch atomically,
records before/after entity, module, prefab-instance, and trigger hashes, and
then freezes the next wave. A failure in any later wave restores the entity and
module-state checkpoints from before the root; the rejected reaction frame
retains the attempted barriers while its state before/after hashes prove the
rollback.

Pre-commit consumers implement `GameplayRuntimeDecisionOwner`. This is a
statically linked Rust port with two responsibilities: return the current
revision for the registry-resolved owner and atomically route the final
operation. It is not a callback registry, owner-discovery mechanism, or
TypeScript authority seam. Decision invocations receive the same declared-read
plans as Observe invocations; proposal source/target identities are presented
as decision-moment bindings without pretending that the proposal is already a
committed event.

Suspension is host authority. A module may return `React::Suspend` during the
initial decision and inspect `GameplayModuleContext::decision_resume_token` on
an authorized continuation. The host binds the coordinator-issued token to the
decision, closed registry, owner, operation hash, expected owner revision, and
exact Workspace generation. Missing, wrong, replayed, or stale tokens fail
before module invocation. Pending tokens and deterministic generation counters
survive the host snapshot; the bounded decision receipt ledger makes the same
views, invocation outputs, routing, diagnostics, and hashes available to replay
and first-mismatch tooling. Restore independently recomputes every decision
receipt hash and validates nested frozen-read values, effective configuration,
routing evidence, continuation identity, registry identity, and Workspace
hashes before any token can be authorized.

Shared proposals do not use a downstream callback. The initial concrete owner
route is `asha.entity.set-capability-activation.v1`; the host decodes it, checks
the registry-resolved owner and target, and applies it through
`svc-entity-authoring` Rule ownership. Unsupported owners fail closed.

## Native/browser composition

`asha-runtime-session-composition` installs this host inside `EngineBridge`.
The downstream addon remains the static link point for game module crates, but
TypeScript sees one `createRuntimeBridge` factory and one RuntimeBridge root.
Accepted owner events and movement facts enter this host directly from their
Rust authority source. The former `GameplayRuntimeHostTransport`, five-method
facade lifecycle, browser provider property, and second HTTP endpoint are not
part of the preferred surface.

For event, decision, state, read/query, binding, proposal, trigger, and module
addition recipes, see
[Gameplay fabric growth recipes](gameplay-fabric-growth-recipes.md).

`@asha/browser-host` proxies only the integrated RuntimeBridge operation set.
Downstream browser code cannot add a parallel gameplay JSON dispatcher or
per-game host routes.

## Atomic load and replay

Product code constructs a staging `GameplayRuntimeHost`, checks activation and
its readout, and swaps it into the provider only after success. Invalid
provider/config/read/output/trigger data therefore cannot partially replace the
live host.

The downstream fixture executes the full loop twice and compares reaction
frames, wave barriers, and host hashes. It also schedules a typed proposal, observes its
recoverable outstanding dispatch, routes it through the closed owner, and
restores the same scheduler readout. It saves and restores the same host, then rebuilds with
a changed module multiplier and proves that invocation/frame/host hashes drift.
The saved host snapshot is hash-bound to its nested authority/module/trigger
state and rejects mismatched authored trigger definitions. Prefab-aware
snapshots also retain accepted placement commands, resolved roles, effective
overrides, Entity provenance, and prefab-scoped module state. Restore validates
the same registry/bootstrap evidence before republishing the host.

## Direct integration gate

Run the owning main-repo gate with:

```bash
./harness/ci/check-gameplay-runtime-host.sh
```

The gate runs the host crate once and one targeted public one-cell provider
lifecycle proof. It is part of `check-all.sh`; that orchestration excludes the
host from its earlier workspace test pass so the dedicated semantic suite is
not executed twice. A standalone `check-rust.sh` still tests the complete Rust
workspace.

The direct suite covers wave barriers and root rollback, module-state
application, canonical owner routing/events and owner rejection, scheduler
interruption/recovery, trigger reconciliation, prefab expansion and declared
reads, exactly-once continuation replay, nested snapshot/hash verification,
and two-Session isolation. The public provider proof covers load, FPS restart,
unload, project switch by rebuilding the static cell, close/drop, and provider
resource release using a real linked module, registry, codecs, bindings, and
state adapter.

Each run writes bounded JSON-lines evidence to
`harness/smoke-out/gameplay-runtime-host/integration-evidence.jsonl` (gitignored).
Entries identify the Session label, wave or scheduled action, registry digest,
runtime-host hash where applicable, and at most eight evidence hashes. A failed
command also writes only its last 200 output lines to `failure.log` in the same
directory.

## Deliberate limits

- Wave 1 is static linking only.
- The host retains a bounded 256-frame replay/readout window; a product may
  stream older frames to its replay archive. Decision receipts use the same
  bound independently. Scheduler pending/outstanding projections are also
  bounded to 256 entries, while snapshots retain the complete authority state.
- The current concrete shared owner route is capability activation. Further
  generic proposal applications should be added owner by owner.
- The public transport is implemented; each downstream product still builds
  its small native binding cell so its actual Rust module crate is linked.
- Renderer/camera projection remains outside gameplay authority. Movement enters
  the EntityStore only through an accepted authority operation.
- Nested prefabs and automatic propagation of changed definitions into already
  accepted instances are not supported.
