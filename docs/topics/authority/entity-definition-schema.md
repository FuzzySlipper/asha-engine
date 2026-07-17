---
status: current
audience: agent
tags: [ecrp, entity, schema]
supersedes: []
see-also: []
---

# EntityDefinition Schema Status

Status: upstream ECRP bootstrap substrate. Initial proof came from #4027; ProjectBundle batch hardening was added for #4161.

## Owner

- Protocol shape: `engine-rs/crates/protocol/protocol-entity-authoring`
- Validation/bootstrap authority: `engine-rs/crates/services/svc-entity-authoring`
- Runtime capability state: `engine-rs/crates/state/core-entity`
- Generated TypeScript surface: `@asha/contracts`, generated from Rust protocol source

## Current Shape

`EntityDefinition` is durable ProjectBundle/catalog input. It is validated before it can seed runtime authority.

Fields:

- `stable_id`: stable project/catalog identity.
- `display_name`: human-readable name for Studio/demo readouts.
- `source`: ProjectBundle/source-path trace.
- `tags`: typed `TagId` labels applied to the runtime entity.
- `metadata`: string metadata for authoring/readout only.
- `capabilities`: initial capability declarations.

Supported initial capabilities:

- `transform`
- `render`
- `collision`
- `bounds`

An explicit `unknown` capability variant exists so decoded or hand-authored bad stored data can be rejected with an `unknownCapability` diagnostic.

## Validation

`svc-entity-authoring` rejects:

- missing stable id;
- missing display name;
- missing source trace;
- unknown capability declarations;
- duplicate capability declarations;
- non-finite initial values;
- invalid initial values such as zero transform scale axes or inverted bounds.

Invalid definitions fail before runtime mutation.

## Bootstrap Surfaces

`bootstrap_entity_definition` validates the stored definition, creates a runtime entity through the existing entity-authoring authority path, attaches initial capability state, and returns an `entity_definition.bootstrap` readout record with:

- definition stable id and display name;
- runtime entity id;
- ProjectBundle/source trace;
- applied capability kinds;
- resulting entity-store hash.

`bootstrap_project_bundle_entity_definitions` is the durable batch surface for ProjectBundle-shaped bootstrap. It accepts a ProjectBundle id plus deterministic `(runtime entity id, EntityDefinition)` entries and returns a `project_bundle.entity_definitions.bootstrap` readout record with:

- ProjectBundle id;
- per-definition bootstrap records;
- final entity-store hash after all entries apply.

Batch bootstrap preflights every entry before mutating live authority and applies against a staging `EntityStore`. The live store is replaced only if every definition succeeds. It rejects:

- missing ProjectBundle id;
- empty definition batches;
- invalid nested EntityDefinitions;
- EntityDefinition `source.project_bundle` values that do not match the request ProjectBundle;
- duplicate EntityDefinition stable ids in a batch;
- duplicate target runtime entity ids in a batch;
- target runtime entity ids already allocated in SessionState.

Invalid batches and unexpected staged apply rejections leave the live store untouched.

Current compatibility note: runtime `EntitySource` still uses `RuntimeCreated { by: None }` for this first proof. The durable ProjectBundle/source trace is carried in the bootstrap record until a later task decides whether stored-definition provenance belongs in core runtime source identity and snapshot compatibility.

## Non-Claims

This schema does not implement FPS movement, combat, pathfinding, procedural generation, policy execution, or demo-owned content migration.
