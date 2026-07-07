# Testing Conformance Goldens Map

## Purpose

Route work around deterministic tests, compatibility proofs, fixtures, goldens,
smoke outputs, and CI gates.

## Owns

- Harness scripts and CI entrypoints.
- Fixtures and goldens that prove Rust authority, generated contracts, bridge
  behavior, render projection, replay, and public-surface compatibility.
- Reviewable evidence files that prevent stubs or half implementations from
  becoming load-bearing structure.

## Does Not Own

- Product UX decisions.
- Current Den task state.
- Ad hoc proof pages that bypass the final architecture path.

## Primary Paths

- [harness/ci](../../harness/ci)
- [harness/fixtures](../../harness/fixtures)
- [harness/goldens](../../harness/goldens)
- [harness/public-surface](../../harness/public-surface)
- [harness/depgraph](../../harness/depgraph)
- [replay-model.md](../replay-model.md)
- [determinism.md](../determinism.md)

## Public Downstream Surfaces

- Public package compatibility artifacts emitted by
  [pack-public-artifacts.mjs](../../ts/scripts/pack-public-artifacts.mjs) under
  `ts/artifacts/public-packages/`.
- Smoke and conformance fixtures consumed through approved package roots.
- Review packets that cite exact commands and generated artifacts.

## Private Or Forbidden Paths

- Do not let synthetic proofs become required product runtime paths.
- Do not hide known limitations only in fixture names or task prose.
- Do not weaken depgraph, vocabulary, public-surface, or bridge checks to make a
  local task pass.

## Proof Gates And Goldens

- [check-all.sh](../../harness/ci/check-all.sh)
- [check-depgraph.sh](../../harness/ci/check-depgraph.sh)
- [check-contracts.sh](../../harness/ci/check-contracts.sh)
- [check-replays.sh](../../harness/ci/check-replays.sh)
- [check-render-goldens.sh](../../harness/ci/check-render-goldens.sh)
- [check-bridge.sh](../../harness/ci/check-bridge.sh)
- [check-vocabulary.sh](../../harness/ci/check-vocabulary.sh)

## Common Agent Mistakes

- Adding a proof-only path that is more complicated than the intended final
  runtime path.
- Updating snapshots without checking whether the semantic behavior should have
  changed.
- Treating reference/mock success as native authority success.

## Follow-up Routing

- Missing evidence for an engine behavior change: add a focused fixture or
  golden in the owning lane.
- Broken public compatibility proof: route to public-surface or contract
  stewardship first.
- Product-demo evidence: keep it downstream unless it reveals a missing engine
  substrate.
