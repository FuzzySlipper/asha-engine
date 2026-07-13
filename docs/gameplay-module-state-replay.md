# Gameplay Module State, Session Snapshots, and Reaction Replay

Status: implemented authority substrate for Den task #5633. The owning
`RuntimeSession` composes this state with its other authority stores; #5661 owns
ProjectBundle-authored bindings and end-to-end bootstrap configuration.

This surface lets a statically linked downstream Rust module own persistent
gameplay state without acquiring raw `SessionState` access or making every
game-specific counter, machine, quest, encounter, or cooldown an engine field.
The result remains part of Session persistence, hashing, replay, and inspection.

## Typed Module Boundary

Modules implement `GameplayTypedModuleStateAdapter` with concrete Rust types for:

- authored initialization configuration;
- authoritative state;
- accepted module facts; and
- an optional named view.

The trait only receives `&self`; mutation is expressed by returning a new typed
state value from `initialize`, `apply_fact`, or `migrate`. A
`GameplayModuleStateRegistration` erases those types only inside the
heterogeneous coordinator and persistence boundary. The erased adapter and raw
state bytes are not publicly constructible or readable.

Every registration must match the immutable gameplay-fabric registry's module,
state schema, fact schema, and exact owner. An undeclared schema, foreign module,
missing owner, owner mismatch, or duplicate adapter fails before use.

## State Scopes and Facts

`GameplayModuleStateScope` has two explicit shapes:

- `Session` for module-wide domain state such as a quest ledger or score; and
- `Entity { entity }` for a module-owned entity facet such as machine progress.

The store indexes state by its registered schema and scope. Session and entity
records cannot alias each other. Initialization is staged and applied only after
every config payload has decoded and initialized successfully, so a bad entity
facet cannot leave earlier Session state partially installed.

`GameplayModuleFact` names its module, fact/state schemas, scope, expected
revision, canonical payload, and payload hash. Application checks the closed
registry, module namespace, payload hash, unique fact id, target record, and
compare-and-set revision before invoking the typed adapter. A rejected or stale
fact leaves both state and replay evidence unchanged.

Live Observe delivery applies each wave's complete module-fact batch
atomically after all same-wave invocations and owner routes have completed.
This makes accepted prior-wave state visible to the next wave without exposing
partially evaluated state inside the current wave. The owning RuntimeSession
also retains a pre-root checkpoint; if a later wave rejects, it restores state
records, accepted-fact ids, and fact evidence together.

Migration uses the same staged typed adapter boundary and revision guard. A
failed migration leaves the prior bytes, version evidence, revision, and hash
untouched.

## Named Views and Readouts

A module may project its typed state through a versioned
`GameplayContractRef`. `named_view` succeeds only when the immutable registry
contains the matching declared provider and its provider identity agrees with
the state owner. The result carries scope, revision, provider, canonical
payload, and view hash.

Other modules consume that view through the declared/frozen read vocabulary;
they do not receive the state store. Tooling receives only bounded
`GameplayModuleStateReadout` metadata: module, state contract, scope, revision,
state hash, and initialization provenance.

## RuntimeSession Save, Load, and Hashing

Module snapshots are schema-versioned and bind:

- the immutable registry digest;
- ordered state records and their owner/schema/scope/revision;
- initialization provenance;
- applied fact ids and full accepted fact evidence; and
- the canonical module-state hash.

Restore reconstructs a checked index and rejects duplicate records, wrong
owners, changed registry/schema evidence, bad state or fact hashes, foreign fact
targets, and disagreement between applied ids and accepted facts.

`encode_session_snapshot` composes that module snapshot with the owning
RuntimeSession's typed authority snapshot and authority-state hash. The envelope
records the authority artifact hash, module-state hash, registry digest, and a
final Session hash. `decode_session_snapshot` returns a
`GameplaySessionRestore` only after all layers validate. Because the final hash
includes the registry, base authority hash, and module-state hash, changing any
module state necessarily changes overall Session identity.

The gameplay-fabric crate treats the base authority snapshot as opaque bytes;
the Session lane that owns that typed snapshot remains responsible for decoding
and applying it. This is composition, not a generic state bag.

## Reaction Frames and Two Replay Modes

`GameplayReactionFrame` captures the inspectable causal boundary for one fabric
reaction:

- registry digest, ordered module set, and module artifact/contract hashes;
- source owner facts and hashes;
- delivered event envelopes and hashes;
- frozen view generations;
- wave-barrier evidence binding each frozen generation to before/after entity,
  module, prefab, and trigger authority hashes and accepted route/fact hashes;
- module/subscription/input/output invocation evidence;
- proposal and resolved-owner routing receipts;
- full accepted module facts;
- before/after module-state hashes;
- typed diagnostic evidence;
- final Session hash; and
- a canonical frame hash.

The two replay modes stay deliberately separate:

1. `playback_frame` initializes the store and applies the frame's recorded
   accepted module facts. It does not dispatch events or invoke module behavior.
2. `run_verification_replay` asks a statically linked verification runner to
   rerun the fabric and returns categorized divergences for registry/code,
   source facts, events, views, invocation outputs, proposals/routing, module
   facts, state, diagnostics, final Session state, and frame integrity.

Artifact, schema, and ordering drift changes the closed-registry evidence.
Event, proposal, fact, and post-state drift is classified independently so a
gameplay developer can see where a reaction first diverged rather than receiving
only one final hash mismatch.

Barrier hashes and their state continuity participate in frame hashing and
verification replay input. Snapshot/restore therefore preserves both accepted
multi-wave transitions and rejected-root evidence; malformed, reordered, or
disconnected barriers classify as view divergence before final-state comparison.

Runtime-host restore additionally admits every serialized root and delivered
event through the current closed registry codec after verifying the frame and
registry hashes. Rehashing a tampered snapshot is insufficient: malformed,
noncanonical, wrong-kind, or wrong-payload-hash replay bytes are rejected before
the frame becomes live state.

## Structural Boundaries

- There is no runtime state-owner registration or mutable callback registry.
- There is no public `Any`, JSON-value state map, or raw mutable store query.
- One module cannot apply facts to another module's namespace or state schema.
- Gameplay events remain semantic routing evidence, not a competing mutation
  path; accepted owner/module facts reconstruct authority.
- ProjectBundle binding and prefab/part override selection remain in #5661.
- Named read-set assembly and bounded owner-backed queries remain in #5660.
