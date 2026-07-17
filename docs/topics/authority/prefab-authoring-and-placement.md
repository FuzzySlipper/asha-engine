---
status: current
audience: consumer
tags: [prefab, authoring, placement, consumer]
supersedes: []
see-also: []
---

# Prefab Authoring and Public Placement

Status: public consumer workflow implemented by Den task #5646.

## The consumer path

Downstream projects can create, replace, delete, inspect, serialize, and place
prefabs without importing engine-private crates or generated-file subpaths.
The path deliberately separates expression from authority:

1. `@asha/game-workspace` owns consumer-side prefab drafts and workspace
   readouts. Its root export provides explicit create, replace, delete, and
   instantiate commands; prefab browser and selection readouts; stable
   part-role inspection; gameplay binding/configuration readouts; and canonical
   registry source serialization.
2. A consumer stores that source as its ProjectBundle `prefabRegistry`
   artifact and passes the source plus an explicit catalog and placement list
   through `GameplayRuntimePrefabBootstrap`.
3. `asha-gameplay-runtime-host` calls the public decode-and-validate loader,
   receives a `ValidatedPrefabRegistry`, and routes every authored or accepted
   player placement through `PrefabInstanceAuthority`.
4. `@asha/runtime-session` carries the same typed bootstrap and exposes bounded
   prefab and module-state readouts. A consumer-owned native provider statically
   links the real Rust module composition.
5. Render/UI code projects the authoritative part readout. It does not infer
   placed identity from authoring arrays, labels, or coordinates.

TypeScript validation is early author feedback only. It catches common draft
mistakes but does not replace Rust registry validation, catalog validation,
atomic Entity creation, gameplay binding resolution, or snapshot restore.

## Stable gameplay address

Gameplay addresses a part as `{ prefab, role }`. Instantiation resolves that
stable role to an instance-specific Entity. The public runtime readout includes
the accepted command, placement origin, effective part source and translation,
role map, provenance hash, and override count. This lets a downstream UI show
two placements of one multi-part definition while gameplay routes each
interaction to the correct instance and module-state scope.

Display names, hierarchy paths, array indexes, and world coordinates are never
gameplay identity. An authored placement and a player placement use the same
authority path; `origin` is durable evidence, not a second execution model.

## Authoring and runtime lifecycle

- `createPrefab` adds a new stable definition.
- `replacePrefab` edits the stored draft while preserving its prefab id and
  stable part roles.
- `deletePrefab` fails when an instance or variant still references the
  definition.
- `instantiatePrefab` records an explicit id, seed, transform, origin, and
  typed per-instance overrides.
- canonical serialization gives the host inspectable ProjectBundle source;
  Rust rejects invalid bytes before publishing a runtime host.
- gameplay-host snapshots retain accepted commands, role resolution,
  overrides, Entity provenance, scoped module state, and hashes. Restore runs
  the same validation and composition boundary and must reproduce the readout.

Demo's visible acceptance creates one two-part interaction-console definition and
places it twice. The authored instance and player instance retain the same
stable sensor role, use distinct body EntityDefinition overrides, resolve to
distinct Entities, run a prefab-part-scoped typed gameplay event, survive
save/restore, and appear as two visible world-space placements.

## Structural limits

This surface is intentionally not a mutable runtime plugin registry. It does
not support nested prefabs, arbitrary JavaScript authority callbacks, raw store
access, or propagation of later definition edits into already accepted live
instances. Those are explicit non-claims rather than behavior a downstream
project should approximate with private imports.

The extension point is the gameplay fabric: typed events, declared reads,
stable prefab-part bindings, scoped module configuration/state, and bounded
readouts. New gameplay should extend those public contracts instead of adding a
parallel downstream Entity or prefab authority.
