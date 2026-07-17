---
status: current
audience: agent
tags: [contracts, governance, codegen]
supersedes: []
see-also: []
---

# Contract governance

## What a contract is

A contract is the generated TypeScript surface derived from a Rust protocol crate.
Contracts define what TypeScript can see (views), say (commands), and receive (diffs, events).

Contracts are **borders**. Changing a contract is changing the Rust/TypeScript boundary.

## Source of truth

Rust protocol crates are the source of truth:
- `protocol-ids`
- `protocol-script`
- `protocol-render`
- `protocol-replay`
- `protocol-telemetry`
- `protocol-scene` — authored scene-graph documents
- `protocol-project-bundle` — project bundle manifests, load plans, save summaries
- `protocol-assets` — asset catalog/lock shapes
- `protocol-diagnostics` — classified diagnostic reports (load/projection/composition)
- `protocol-policy-view` — read-only world view + proposed world commands
- `protocol-game-rules` — schema-only generic effect/modifier catalog and readout DTOs

Generated TypeScript lives in `ts/packages/contracts/src/generated/` and is committed
for worker convenience. It is never hand-edited. `protocol-codegen` is the emitter.

## Change process

1. Edit the Rust protocol crate source.
2. Run `cargo run -p protocol-codegen` to regenerate TypeScript.
3. Commit the Rust source change and the generated TypeScript together in one PR.
4. Update affected golden fixtures in `harness/goldens/protocol/`.
5. List every downstream TS package affected and confirm they still typecheck.
6. Add or update `docs/consumer-compatibility.md` and the package-local
   `compatibility.json` when the change affects downstream consumers.
7. Add a compatibility note if the change breaks existing replay files or saved state.
8. Request `contract-steward` lane review.

## CI enforcement

`harness/ci/check-contracts.sh` regenerates contracts and fails if the working-tree result
differs from committed files. A PR with a manual edit to generated files will fail CI.

## Breaking vs. additive changes

| Change | Classification | Extra steps |
|---|---|---|
| Add new command variant | Additive | Downstream TS typecheck |
| Add new view field | Additive | Downstream TS typecheck |
| Remove or rename a variant/field | Breaking | Compatibility note, migration fixture |
| Change serialization format | Breaking | Replay compatibility note, snapshot migration |

## Protocol families and their consumers

| Protocol | Primary TS consumer | Golden / check expectation |
|---|---|---|
| `protocol-ids` | all packages via `contracts` | `check-contracts.sh` (codegen sync) |
| `protocol-script` | `script-sdk`, `script-host`, policy packages | `check-contracts.sh`; policy sandbox tests |
| `protocol-render` | `runtime-bridge` (decode), `renderer-three` | `check-render-goldens.sh`; render-diff fixtures |
| `protocol-replay` | `devtools` replay viewer, CI replay check | `check-replays.sh` golden reproduction |
| `protocol-telemetry` | `devtools` debug dashboard | `check-contracts.sh` |
| `protocol-scene` | `editor-tools`, `runtime-bridge` (world load) | `check-contracts.sh`; scene fixtures/goldens |
| `protocol-project-bundle` | `runtime-bridge` (load/save), `smoke` | `check-contracts.sh`; project-bundle fixtures |
| `protocol-assets` | `catalog-*`, `renderer-three` (asset refs) | `check-contracts.sh`; asset-catalog fixtures |
| `protocol-diagnostics` | `runtime-bridge`, `devtools`, `smoke` | `check-contracts.sh`; diagnostics fixtures |
| `protocol-policy-view` | `script-sdk`, `script-host` policies | `check-contracts.sh`; policy fixtures |
| `protocol-game-rules` | TS content packages, `runtime-bridge` facades, Studio tooling | `check-contracts.sh`; catalog/receipt fixtures |

Every protocol change must keep `check-contracts.sh` green (generated TS matches Rust)
and re-bless any golden listed above whose shape it intentionally changes.

## Game Rules Contracts

`protocol-game-rules` owns schema-only DTOs for generic effect bundles,
modifier definitions, stack/duration/tick policies, validation diagnostics,
resolution receipts, traces, and replay evidence summaries. Generated
`gameRules.ts` is the public TypeScript border for authoring catalog data and
reading authority receipts.

Authority validation, effect interpretation, modifier application, and replay
commit remain outside the protocol crate. Those belong to future
`svc-game-rules` / rule crates and must not be implemented in generated
contracts, TS content packages, renderers, bridges, or Studio UI code.
