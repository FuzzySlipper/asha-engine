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

Generated TypeScript lives in `ts/packages/contracts/src/generated/` and is committed
for worker convenience. It is never hand-edited.

## Change process

1. Edit the Rust protocol crate source.
2. Run `cargo run -p protocol-codegen` to regenerate TypeScript.
3. Commit the Rust source change and the generated TypeScript together in one PR.
4. Update affected golden fixtures in `harness/goldens/protocol/`.
5. List every downstream TS package affected and confirm they still typecheck.
6. Add a compatibility note if the change breaks existing replay files or saved state.
7. Request `contract-steward` lane review.

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

| Protocol | Primary TS consumer |
|---|---|
| `protocol-script` | `script-sdk`, `script-host`, policy packages |
| `protocol-render` | `runtime-bridge` (decode), `renderer-three` |
| `protocol-replay` | `devtools` replay viewer, CI replay check |
| `protocol-telemetry` | `devtools` debug dashboard |
| `protocol-ids` | all packages via `contracts` |
