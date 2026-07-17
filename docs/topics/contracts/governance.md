---
status: current
audience: agent
tags: [contracts, codegen, governance, protocol]
supersedes: []
see-also: [design.md, contract-governance.md]
---

# Contract Governance

A contract is the generated TypeScript surface derived from a Rust protocol crate. Rust protocol crates are the source of truth.

## Rules

- Generated files are committed for worker convenience but are not hand-edited.
- Codegen runs in CI.
- Generated diffs require protocol-steward review.
- Fixtures show before/after contract shape when protocols change.

## Change Process

A protocol change includes: Rust protocol/schema update, regenerated TypeScript contracts, fixture/golden updates, downstream Rust/TS tests, and compatibility/diagnostic notes when the change affects runtime behavior.

See `docs/contract-governance.md` for the full process.
