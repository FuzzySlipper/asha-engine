# Declared Gameplay Reads

Gameplay modules need useful knowledge of the Session without receiving a raw
entity store, module-state store, or general query language. The declared-read
surface provides that middle ground: a module names the immutable views it
needs in its manifest, the closed Session registry proves one provider can
supply them, and the runtime freezes owned results before invoking the module.

This is part of the gameplay fabric rather than a new public method for each
gameplay question. It is intentionally a small composable vocabulary.

## Closed Provider Contract

Each `GameplayReadViewRequirement` now declares:

- an open namespaced view contract and exact provider;
- one semantic kind: event identity, entity capability, module-owned named
  view, relationship, prefab part, bounded selection, or owner query;
- the individual fields the consumer needs;
- permitted selector capabilities; and
- a maximum item count.

`GameplayReadViewProviderRegistration` supplies the same kind, all available
fields and selectors, its hard quota, and its deterministic ordering rule. The
registry rejects missing or multiple providers, kind/provider mismatches,
missing fields or selectors, duplicate metadata, and invalid bounds. Its
readout exposes the canonical metadata and a computed provider hash, so needs
validation can distinguish an unavailable view kind, provider, selector, or
field rather than collapsing those failures into “engine support missing.”

Each `GameplayInvocationDescriptor` also carries a closed
`requestId -> view contract` list. Module-level `readViews` describe the
provider capabilities the module may use; they do not grant every invocation
access to every module view. A plan request must match both its invocation's
request id and the view bound to that id. Duplicate bindings, bindings to a
view the module does not declare, and attempts to borrow another invocation's
binding fail before behavior runs.

## Read Plans and Frozen Waves

`GameplayReadPlan` is an invocation-local list of typed requests. A request can:

- bind to an event source, subject, target, or a known Entity id;
- read lifecycle, transform, collision, or controller capability state;
- follow transform-parent, containment, or source-ancestry relationships;
- resolve a stored `PrefabPartReference { prefab, role }` against one runtime
  prefab instance;
- select a bounded, deterministically ordered tag or scope set;
- consume a module-owned named view without naming its backing state schema; or
- issue a bounded nearby-entity, line-of-sight, or path request to the
  registered authority owner.

The assembler validates the whole request against the invocation's closed read
bindings, the module manifest, and the closed registry before returning an
owned `GameplayFrozenReadSet`. It exposes typed values, not store references.
Collection order and evidence hashes are canonical. A typed module view can be
decoded with
`GameplayFrozenRead::decode_named_view<T>()`; ordinary module code does not
construct payload hashes or type-erased bytes.

`GameplayViewSource::freeze_declared_reads` connects this to Observe dispatch.
The resulting set is carried on `GameplayInvocationCall`, and its hash is part
of the delivery hash. A failed assembly becomes `readAssemblyFailed` before
module behavior runs. The default implementation returns no declared set for
legacy view sources, which keeps the transition explicit without granting them
new access.

The frozen set is a wave input. Later owner routing or module facts cannot
change the data already delivered to another handler in that wave. A later
wave assembles a new generation from the new authority state.

## Typed Outcomes and Bounds

Absent capability attachment is a typed capability readout, distinct from an
inactive capability and from an inactive Entity. An absent optional relation is
a typed missing result. Missing or tombstoned Entity ids, dangling relation or
prefab targets, unsupported selectors, undeclared reads, missing fields,
foreign prefab instances, missing owner providers, mismatched owner receipts,
and quota exhaustion are typed diagnostics. Read assembly is immutable, so all
of these failures leave entity and module authority unchanged.

Selections fail on quota exhaustion rather than silently becoming unbounded
scans. Nearby-entity receipts are sorted and deduplicated by Entity id and all
returned Entity references are revalidated. Path step order remains
owner-semantic order rather than being incorrectly sorted.

## Prefab Role Identity

Prefab-part resolution uses only the stored prefab id and role plus the runtime
instance id. It resolves the role through the validated prefab registry and
then uses the instance's explicit part-to-Entity binding. Display labels, part
namespaces, hierarchy paths, and positions are never reinterpreted as role
identity. Variant-removed roles and mismatched instance prefabs fail closed.

The public `GameplayRuntimeHost` retains the validated registry supplied by
`activate_project_with_prefabs`, projects its live `PrefabInstanceAuthority`
into the read index, and derives scope membership from installed authored
trigger definitions. Prefab-role and populated-scope reads therefore use the
same loaded authority visible through the downstream host rather than
placeholder empty indexes.

## Owner-query Boundary

The fabric defines typed request and receipt families; it does not absorb
spatial, collision, or pathfinding semantics. A statically installed
`GameplayOwnerQueryProvider` adapts the appropriate owner service and supplies
its revision in the receipt. This preserves the existing owner as authority
while allowing downstream modules to consume the query through one declared
read mechanism.

This first slice deliberately does not expose a generic ECS query, arbitrary
predicate closure, string expression language, raw Session scan, or mutable
query result. Additional owner query families should be added only when a real
owner surface and downstream pressure justify a typed contract.

## Proof Fixture

`rule-gameplay-fabric/tests/reads.rs` proves that one downstream-shaped module
can receive a target collision readout, follow containment, resolve a prefab
role, consume a bounded owner query, and decode a module-owned named view on a
real coordinator call. Repeated assembly is byte/hash stable. Separate cases
prove same-wave freezing and fail-closed diagnostics without mutation.
The same suite proves that one invocation cannot consume a request binding
owned by a second invocation in its module. The public-host fixture in
`gameplay-runtime-host/src/prefab.rs` resolves a loaded prefab role and a
populated authored scope through a real module invocation.
