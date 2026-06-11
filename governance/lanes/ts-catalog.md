# Lane: ts-catalog

## Owns
- `ts/packages/catalog-core` — base catalog types, validation helpers, catalog bundle format
- `ts/packages/catalog-examples` — example catalog definitions used for fixtures and tests

## May import
- `@asha/contracts`

## Must never import
- `@asha/renderer-three`, `@asha/ui-dom`, `@asha/runtime-bridge`, `@asha/native-bridge`, `@asha/wasm-replay-bridge`, `@asha/electron-main`
- Runtime global registries or mutable module-level state

## Required tests
- Catalog definition compiles and typechecks against generated contract types.
- Catalog validation fixture: valid catalog passes, invalid catalog produces typed error.
- Round-trip test: serialize catalog bundle → deserialize → assert equality.

## Required fixtures
- `harness/fixtures/catalogs/` — valid and invalid catalog bundle samples.

## Drift smells reviewers should flag
- Catalog package importing renderer, UI, or bridge packages.
- Runtime mutable global registry (`Map`, `Set`, module-level singleton) inside catalog code.
- Catalog containing authoritative mutation logic instead of pure data declarations.
- Catalog types diverging from corresponding Rust protocol types without a contract-steward note.

## Public API changes that require escalation
- Changes to the catalog bundle serialization format — requires Rust validator review.
- Removal or rename of exported catalog types used by policy packages.
