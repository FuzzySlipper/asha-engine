---
status: current
audience: agent
tags: [prefab, instantiation, runtime, rust]
supersedes: []
see-also: []
---

# Deterministic Prefab Instantiation

Status: authoritative Rust instantiation substrate implemented by Den task
#5645, with public downstream bootstrap and placement proof in #5646.

## Command path

`rule-project-bundle::PrefabInstanceAuthority` accepts one
`InstantiatePrefabCommand` for both authored placement and accepted player
placement. `PrefabPlacementOrigin` is evidence, not a second execution path.
The command consumes the stored `PrefabInstanceRecord` and a
`ValidatedPrefabRegistry`; it does not reinterpret raw registry JSON or copy the
prefab schema.

Expansion is staged on a clone and swapped only after every definition,
variant, override, id, Entity creation, transform, and lifecycle operation has
been accepted. A rejected command therefore leaves the live instance map and
EntityStore hash unchanged.

`ProjectBundleStage::instantiate_prefab` stages the entire live load result and
commits the prefab instance map together with the Session's existing
`runtime_entities` store. `PrefabInstanceAuthority` does not own a parallel
EntityStore. Scene bootstrap entities remain in spatial Session authority;
prefab-created runtime entities share the normal non-scene Session store.

## Identity and role resolution

Each retained part receives deterministic instance-scoped `EntityId` and
`SceneNodeId` values derived from:

- a distinct entity/node domain tag;
- prefab and instance ids;
- the explicit seed;
- the stable prefab part id; and
- the complete placement-transform bit pattern.

The derived ids are constrained to the JSON-safe 53-bit integer range. This is
required because these ids cross generated TypeScript/JSON number borders;
unrestricted `u64` hashes can round during save/reload.

Every retained role produces a sorted `PrefabPartResolution`. Alias roles that
refer to one part resolve to the same node and Entity. Display names, hierarchy
paths, array positions, and coordinates never participate in role identity.

## Variants and overrides

One-level variants start from their validated base, remove whole parts through
the stored role set, and apply the variant layer before per-instance overrides.
Overrides remain stored separately on `PrefabInstanceRecord`; the resolved
part readout reports effective values without rewriting the definition.

Supported typed override fields are:

- transform;
- EntityDefinition source;
- same-kind Scene or VoxelObject asset source;
- material asset for Scene/VoxelObject parts; and
- activation, projected to Entity lifecycle (`Active` or `Disabled`).

Conflicting aliases that attempt to override the same effective part field in
one layer reject deterministically. Asset kinds, EntityDefinition ids, finite
transforms, roles, and material applicability are checked before commit.

## Provenance, facts, and persistence

Created entities carry `EntitySource::PrefabInstance` with prefab, instance,
part, and canonical stable-role provenance. The receipt returns explicit sorted
instance/part facts for lifecycle/gameplay adapters; expansion is not a hidden
callback.

`PrefabInstanceSnapshot` stores the accepted command sequence, resolved stable
role map, effective variant/instance overrides, provenance, and final state
hash. Registry-backed replay still runs the same authoritative command path.
The owning ProjectBundle Session save embeds the snapshot beside the normal
EntityStore records in `session/state.snapshot.json`; reload validates the two
halves against each other before restoring either as live authority. Missing,
malformed, or divergent prefab metadata fails closed. This preserves alias-role
resolution and keeps overrides distinguishable from template values after a
real Session save/reload without creating a parallel EntityStore.

## Deliberate boundaries

- Runtime code loading and mutable registries are not supported.
- A prefab role resolves identity; it does not grant gameplay authority.
- Material override metadata does not render directly.
- Gameplay-module binding/configuration is owned by the generated ProjectBundle
  registry and the public static gameplay host.
- Public-surface manifests and provider regressions cover the real
  `@asha/game-workspace`, `@asha/runtime-session`, and
  `asha-gameplay-runtime-host` surfaces; consumers do not need private registry access.
- Nested prefabs and propagation of definition edits into already accepted
  instances remain unsupported.
