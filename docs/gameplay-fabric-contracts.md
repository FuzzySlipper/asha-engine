# Gameplay Fabric Contracts and Immutable Registry

Status: implemented contract and bootstrap-registry foundation for Den task
#5630. Runtime invocation and owner routing are described in
[`gameplay-fabric-runtime.md`](gameplay-fabric-runtime.md); persistent module
state, Session snapshots, and replay are described in
[`gameplay-module-state-replay.md`](gameplay-module-state-replay.md); bounded
consumer reads are described in
[`gameplay-declared-reads.md`](gameplay-declared-reads.md).

The gameplay fabric broadens ASHA's original hook-shaped game extension model
without turning the engine into an untyped event bus or a dynamic plugin host.
It gives compiled Rust gameplay modules enough declared vocabulary to cooperate
across subsystem boundaries while preserving the structural rails that keep
authority, replay, and downstream ownership inspectable.

The design touchstone is a **fabric**: modules connect through typed,
predeclared contracts assembled once for a Session. A module does not discover
ambient callbacks, mutate a global registry, receive raw Session state, or
publish arbitrary JSON.

## Open Meanings, Stable Roles

`GameplayContractRef` identifies a semantic contract with:

- an owned namespace such as `game.combat`;
- a kebab-case name such as `damage-applied`;
- a positive schema version; and
- a computed schema hash derived from a canonical descriptor of the payload
  fields, value shapes, optionality, and codec semantics.

Hash syntax is fail-closed: current identities are full `fnv1a64` or SHA-256
digests, not algorithm-prefixed labels such as `sha256:damage-v1`.

Event, proposal, read-view, fact, state, invocation-input, and
invocation-output meanings use this open reference shape. Adding a downstream
game concept therefore does not require adding a product-specific engine enum.

The engine does keep a small stable vocabulary for *how* a module participates:

- **Observe** reads a committed event and may produce namespaced events,
  module-local facts, shared proposals, or trace output within its declarations.
- **Guard** accepts or rejects a pending decision through a typed result.
- **Transform** replaces a typed pending Workspace under an exact input hash.
- **React** continues, cancels, or explicitly suspends a pending decision and
  may transform its Workspace.

These are invocation families, not mutation privileges. Gameplay events remain
immutable facts, and proposals still require their declared Rust authority
owner to validate and apply them.

## Manifest Shape

`GameplayModuleManifest` is the successor to the original
`GameRuleModuleManifest`. It declares the whole static boundary of one compiled
module:

- module, provider, version, SDK, contract, artifact, and source identities;
- event schemas the module publishes;
- subscriptions and header selectors;
- invocation descriptors and their bounded input/output contracts;
- required read views and fields;
- proposal kinds and their authority owners;
- module-owned state and fact schemas;
- deterministic ordering constraints; and
- per-root execution and payload budgets.

The manifest describes compatibility and topology. It contains no function
pointers, TypeScript callbacks, transport handles, or mutable registry access.
The linked Rust provider must independently agree with the manifest's module,
version, contract hash, and artifact hash.

## Immutable Session Registry

`svc-gameplay-fabric` supplies `GameplayFabricRegistryBuilder` for bootstrap
composition and `GameplayFabricRegistry` for the immutable result. The builder
is intentionally the only mutable part. Construction sorts all declarations,
validates the complete graph, and either returns one closed registry or a
canonical list of typed diagnostics.

Validation fails before a Session can receive the registry when it finds:

- invalid or overlapping namespace ownership;
- duplicate modules, providers, event kinds, subscriptions, invocations, or
  codecs;
- schema/hash disagreement between manifests and codecs;
- missing or mismatched linked providers;
- publication or state ownership in a foreign namespace;
- unknown subscription events or missing/non-Observe subscription invocations;
- missing, multiple, or mismatched proposal/state owners;
- missing, multiple, mismatched, or field-incomplete read-view providers;
- zero execution budgets; or
- unknown ordering targets or an ordering cycle.

Successful construction produces a deterministic registry digest, canonical
topology dump, and `GameplayRegistryReadout`. The digest includes SDK,
contract, source, linked-provenance, proposal-schema, invocation input/output,
and exact invocation-local read topology. Reversing registration order does
not change these artifacts.

Read-view registrations are also closed topology. They name a semantic view
kind, available fields, supported selectors, an item quota, and an ordering
rule. The registry computes their evidence hashes and rejects partial manifest
matches; modules cannot turn a declared field into access to the backing store.

## Typed Edges and Private Erasure

`GameplayProposalEnvelope` carries a pending proposal's contract, deterministic
sequence, emitter, causation, optional originating event, targets, canonical
payload, and payload hash. It deliberately does not carry a caller-selected
authority owner; routing resolves the exact owner from the immutable registry.

Modules register `TypedGameplayEventCodec<T>` values. Their public edges encode
and decode a concrete Rust payload type. Type erasure exists only inside the
registry/queue boundary so heterogeneous events can be indexed and replayed;
callers must request the exact registered `T` or receive a typed codec error.

The codec carries the same canonical schema descriptor used to derive its
contract and codec identity. Root `event<T>` and proposal constructors resolve
the registered codec; module `emit<T>` and `propose<T>` take the typed codec
directly. Raw JSON/bytes are persistence, wire, and replay internals rather than
the normal authoring API. Every root, module output, owner output, proposal, and
restored reaction-frame envelope is decoded and canonically re-encoded before
invocation or mutation. Unknown kinds, wrong typed codecs, valid but
noncanonical bytes, and wrong payload hashes fail closed.

This keeps the extension surface open to new gameplay meanings without making
the transport shape the gameplay API. There is no `serde_json::Value` bus, no
string-to-callback registry, and no raw Session-state context.

## TypeScript Projection

The Rust protocol types generate the `Gameplay*` contracts and re-export them
from the `@asha/contracts` package root. TypeScript can project manifests, envelopes,
diagnostics, topology, and validation results for tools and consumer setup. The
generated surface exposes no codec registration, handler registration, module
invocation, proposal application, or authority mutation operation.

## Compatibility and Next Slices

The legacy `GameRuleModuleManifest`, `GameRuleModule` trait, and three bespoke
hook kinds remain compatibility-only while their existing callers migrate.
The successor contracts are already re-exported by the public Rust facade so
consumer crates can compile against the stable vocabulary without importing
private engine crates.

The behavioral layers are now implemented:

- the public static RuntimeSession host executes Observe and pre-commit
  decisions with declared reads, continuations, owner routing, and snapshots;
- ProjectBundle-authored bindings atomically bootstrap module configuration and
  typed state; and
- real `asha-demo` and Rulebench consumers exercise real-time and adjudicative
  gameplay pressure through public roots.

New slices must consume this registry. They must not add runtime registration,
an open mutation API, dynamic JavaScript authority callbacks, or a second
parallel gameplay bus.
