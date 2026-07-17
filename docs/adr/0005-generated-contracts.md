---
status: current
audience: agent
tags: [adr, contracts, codegen]
supersedes: []
see-also: []
---

# ADR 0005 — Generated contracts

**Status:** Accepted

Rust protocol crates are the source of truth.
Generated TypeScript lives in ts/packages/contracts/src/generated/.
Generated files are committed, not hand-edited.
A CI check fails if generated output diverges from source.
