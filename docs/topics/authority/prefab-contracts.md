---
status: current
audience: agent
tags: [prefab, contracts, stored, rust]
supersedes: []
see-also: []
---

# Prefab Contracts and Reuse Ownership

Status: Wave 1 stored contract implemented by Den task #5644. Authoritative
runtime instantiation is described in `topics/authority/prefab-instantiation.md` (#5645).

## Purpose

A prefab is a durable, reusable description of authored parts plus stable local
roles. It gives downstream projects a reference that survives display-label
renames and hierarchy edits without turning authoring metadata into gameplay
identity.

The registry is stored in a ProjectBundle as one durable artifact with role
`prefabRegistry` (conventional path `prefabs/registry.json`). A manifest may
carry zero or one such artifact. Multiple registries or a generated/cache
registry fail manifest validation.

## One home per concept

| Concept | Owns | Does not own |
|---|---|---|
| `ProjectBundle` | Durable artifact inventory, hashes, and the prefab-registry artifact | Runtime instances |
| `PrefabDefinition` | Reusable part composition, stable part namespaces/roles, one-level variant deltas | World placement, live authority, module state |
| `Scene` | Authored world layout and, in #5645, prefab instance placement | Reusable prefab internals |
| `VoxelObject` | Reusable voxel geometry | Gameplay behavior or placement |
| `EntityDefinition` | Stored capability defaults used when authority creates an Entity | Prefab hierarchy or module configuration |
| Gameplay-module configuration | ProjectBundle binding/configuration records introduced by #5661 | Prefab registry internals |

A prefab part references exactly one source:

- a composable `scene/...` asset;
- an `EntityDefinition.stableId`; or
- a reusable `voxel-object/...` asset.

The registry refers to those sources; it does not copy their schemas or claim
their authority.

## Stable identity

The generated ProjectBundle contracts define:

- `PrefabId` — durable identity of one definition in its ProjectBundle;
- `PrefabPartId` — durable identity of one part inside the definition;
- `PrefabInstanceId` — identity reserved for a stored instantiation request;
- `PrefabPartReference { prefab, role }` — the public stored selector used by
  #5660 declared reads and #5661 authored module bindings.

`role` and `namespace` values are slash-scoped lowercase kebab-case keys. They
are not display labels, array positions, expanded runtime Entity ids, hierarchy
paths, or coordinates. Renaming `displayName` therefore does not change a
`PrefabPartReference`.

## Base definitions and variants

A base definition carries parts and role bindings. A Wave 1 variant carries
only a `PrefabVariantDelta`:

- one direct base `PrefabId`;
- zero or more removed roles; and
- typed overrides targeting base roles.

Typed override values are transform, EntityDefinition, same-kind asset
replacement, material, or activation.
The target part must support the value: an asset cannot replace an
EntityDefinition, and a VoxelObject replacement cannot use a Scene asset.

Variants cannot add parts, define new roles, base another variant, form a
cycle, remove a parent while retaining a child, retain an alias role that
resolves to a removed part, or override a removed part through any of its
roles. Removal therefore has part-level safety even though authors select the
part by role. Nested prefabs and deeper variant chains remain outside Wave 1.

## Validation and atomicity

`svc-serialization::load_prefab_registry` decodes and validates the complete
registry before exposing a `ValidatedPrefabRegistry`. The raw decoder is not a
public API, and canonical encoding accepts only a validated registry. Validation
reports every deterministic classified diagnostic and constructs no accepted
registry on failure.

Downstream TypeScript authoring uses
`decodeAndValidateAshaPrefabRegistrySourceDocument` from the public
`@asha/game-workspace` root. It accepts `unknown`, performs a bounded structural
decode, applies the same stored prefab policy with the consumer's known asset
and EntityDefinition identities, and returns `registry: null` on every failure.
Its success registry is an early authoring diagnostic artifact only. Rust still
re-decodes and validates the source during ProjectBundle load and remains the
only runtime authority.

The validator covers:

- registry/definition schema versions and duplicate prefab ids;
- duplicate part ids/namespaces, missing parents, and hierarchy cycles;
- finite transforms with non-zero scale axes;
- unknown or wrong-kind Scene/VoxelObject assets;
- unknown EntityDefinitions;
- invalid/duplicate roles and dangling role targets;
- missing bases, cycles, and variant depth;
- invalid, duplicate, deleted-target, or source-incompatible overrides; and
- deletion/reference safety.

Canonical encoding sorts definitions, parts, roles, removals, and overrides.
Committed valid and invalid ProjectBundle fixtures pin the bytes, validation
diagnostics, and load rejection behavior. Load/encode is a fixed point, and
valid registries round-trip with stable part references intact.

## Generated and downstream contract

Rust protocol types in `protocol-project-bundle` generate the TypeScript
surface in `@asha/contracts` (`generated/projectBundle.ts`). Generated types
include registry/definition/part/variant/override/instance/reference shapes,
schema constants, and classified diagnostics.

Public consumers use explicit prefab fields:

- required prefab ids;
- required `PrefabPartReference` values (prefab id plus stable role);
- a validated prefab-registry artifact; and
- operations for validation/instantiation only when their real public provider
  exists.

The public contract must not express display-name searches, raw Scene-node scans, or private
registry access as supported needs.

Gameplay-module binding is now implemented by the generated
`GameplayModuleBindingRegistry`: whole-prefab bindings use a stable instance
scope and part bindings resolve `PrefabPartReference` through validated
instantiation authority before module state initializes. See
`gameplay-module-bindings.md`.

The public authoring and placement workflow is described in
`prefab-authoring-and-placement.md`. It preserves this stored contract's owner
boundary: TypeScript prepares drafts and readouts, while Rust validates registry
bytes and owns live expansion.

## Non-claims

The stored contract itself does not instantiate live Entity/Scene records or
own authoring UI state. Nested prefabs and propagation of later definition edits
into existing instances remain unsupported. Runtime instantiation,
gameplay-module binding, and consumer authoring tools are separate implemented
owner surfaces; the stored prefab registry does not absorb their authority.
