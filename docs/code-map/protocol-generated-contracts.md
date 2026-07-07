# Protocol And Generated Contracts Map

## Purpose

Route changes to the Rust-defined protocol border and generated TypeScript
contracts. Generated contracts define the Rust/TypeScript boundary; hand edits
to generated outputs are forbidden.

## Owns

- Rust protocol crates and contract DTO shape.
- Protocol code generation and generated TypeScript contract exports.
- Compatibility notes when generated surfaces change.

## Does Not Own

- Runtime authority logic.
- TypeScript policy decisions or UI state.
- Private downstream package imports that bypass package roots.

## Primary Paths

- [engine-rs/crates/protocol](../../engine-rs/crates/protocol)
- [engine-rs/crates/protocol/protocol-codegen](../../engine-rs/crates/protocol/protocol-codegen)
- [ts/packages/contracts](../../ts/packages/contracts)
- [contract-governance.md](../contract-governance.md)
- [consumer-compatibility.md](../consumer-compatibility.md)

## Public Downstream Surfaces

- `@asha/contracts` package root.
- Contract compatibility metadata recorded in
  [consumer-compatibility.md](../consumer-compatibility.md).
- Public package status in
  [ts-packages.json](../../harness/public-surface/ts-packages.json).

## Private Or Forbidden Paths

- Do not hand-edit [ts/packages/contracts/src/generated](../../ts/packages/contracts/src/generated).
- Do not import `@asha/contracts/src/*` or `@asha/contracts/dist/generated/*`
  from consumers.
- Protocol crates should not import runtime services, render hosts, UI packages,
  or product-specific demo code.

## Proof Gates And Goldens

- [check-contracts.sh](../../harness/ci/check-contracts.sh)
- [check-depgraph.sh](../../harness/ci/check-depgraph.sh)
- [harness/goldens/protocol](../../harness/goldens/protocol)
- [harness/public-surface/check-public-boundary.py](../../harness/public-surface/check-public-boundary.py)

## Common Agent Mistakes

- Patching generated TypeScript to make a typecheck pass.
- Adding protocol fields without updating compatibility docs and downstream
  readouts.
- Smuggling authority semantics into DTO crates instead of keeping them as
  border vocabulary.

## Follow-up Routing

- Schema or generated type changes: tag `contract-steward`.
- Missing consumer package root: update public-surface metadata and compatibility
  docs, then run public-boundary and contract checks.
- Runtime behavior behind a contract: route to the Rust owner crate after the
  DTO shape is accepted.
