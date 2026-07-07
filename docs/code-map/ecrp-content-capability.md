# ECRP Content And Capability Map

## Purpose

Route work around Entity Capability Rules Policies, stored ProjectBundle content,
runtime CapabilityState, policy bindings, catalogs, and game-rule substrate.

## Owns

- EntityDefinition and ProjectBundle validation/bootstrap surfaces.
- CapabilityState readouts and rule-owner boundaries.
- Generic game-rules value, modifier, reaction, trace, and replay substrate.
- Typed catalog/config surfaces that TypeScript may author but Rust validates.

## Does Not Own

- ECS framework semantics, generic component bags, hidden schedulers, or dynamic
  plugin registries.
- Demo-local engine machinery.
- TypeScript authority mutation paths.

## Primary Paths

- [engine-rs/crates/state/core-entity](../../engine-rs/crates/state/core-entity)
- [engine-rs/crates/state/core-game-rules](../../engine-rs/crates/state/core-game-rules)
- [engine-rs/crates/services/svc-entity-authoring](../../engine-rs/crates/services/svc-entity-authoring)
- [engine-rs/crates/services/svc-game-rules](../../engine-rs/crates/services/svc-game-rules)
- [engine-rs/crates/rules/rule-game-modifier](../../engine-rs/crates/rules/rule-game-modifier)
- [ts/packages/catalog-core](../../ts/packages/catalog-core)
- [entity-definition-schema.md](../entity-definition-schema.md)
- [game-rules-substrate-architecture.md](../game-rules-substrate-architecture.md)

## Public Downstream Surfaces

- `@asha/contracts` generated ECRP and game-rule DTOs.
- `@asha/runtime-session` ECRP load/readout helpers.
- `@asha/catalog-core` typed catalog validation for consumer-owned data.
- `@asha/game-workspace` manifest/workspace validation.

## Private Or Forbidden Paths

- Do not use `Component` or `Archetype` naming for new public ECRP surfaces.
- Do not let TypeScript policies mutate CapabilityState.
- Do not create game-specific turn/action-economy assumptions in upstream
  generic game-rules substrate.

## Proof Gates And Goldens

- [check-vocabulary.sh](../../harness/ci/check-vocabulary.sh)
- [check-depgraph.sh](../../harness/ci/check-depgraph.sh)
- [harness/fixtures/game-rules](../../harness/fixtures/game-rules)
- [harness/fixtures/entities](../../harness/fixtures/entities)
- [harness/fixtures/gameplay-presets](../../harness/fixtures/gameplay-presets)

## Common Agent Mistakes

- Treating stored EntityDefinitions as live authority after bootstrap.
- Adding generic JSON capability stores instead of typed owner tables.
- Moving game-specific policy logic upstream when only the validation substrate
  belongs in the engine.

## Follow-up Routing

- ECRP vocabulary or owner-matrix changes: route through Den guidance and
  [ecrp-capability-rule-ownership.md](../ecrp-capability-rule-ownership.md).
- Generic reusable rules: tag `game-rules`, `rust-state`, `rust-service`, or
  `rust-rule`.
- Downstream-specific catalogs or actions: route to the game repo after upstream
  validation surfaces exist.
