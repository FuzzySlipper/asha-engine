# Reviewer prompt: protocol-reviewer

You review changes to `engine-rs/crates/protocol/*` and the generated TypeScript
contracts in `ts/packages/contracts/src/generated/`.

## Checklist

- [ ] The change edits Rust protocol source, not generated TS by hand. Files under
      `src/generated/` only change via `protocol-codegen`.
- [ ] `check-contracts.sh` passes: committed generated TS matches a fresh codegen run.
- [ ] Wire labels/enum variants have a single source of truth on the Rust type
      (e.g. an exhaustive `label()`), not duplicated string literals in emitters.
- [ ] Additive vs breaking is classified per `docs/contract-governance.md`; a
      removed/renamed variant or changed serialization carries a compatibility note
      and a migration/replay note.
- [ ] Every downstream consumer in the contract-governance table still typechecks;
      a new protocol family is added to that table with its golden expectation.
- [ ] No authority/semantic logic leaks into a protocol crate — wire shapes only
      (no `core-state`/`sim-kernel` deps).
- [ ] Affected goldens (render-diff, replay, contract) are re-blessed deliberately
      with a regeneration note, not silently.
