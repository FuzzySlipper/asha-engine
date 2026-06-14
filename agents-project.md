# ASHA Local Bootstrap

Project-specific live guidance lives in Den at `[doc: asha/design]`.

Use project ID `asha` for Den tasks, messages, documents, librarian queries, and guidance lookups.

## Architecture Soul

> Rust owns authority. TypeScript owns expression and projection. Generated contracts define the border.

- **Rust** is authoritative: canonical state, validation, event application, deterministic services, replay, serialization, heavy simulation.
- **TypeScript** proposes commands via constrained policy/catalog packages, and displays projected state via shell/render/UI.
- TypeScript **never mutates** authoritative state. Rust validates all commands.
- Every crate/package is an **agent assignment cell** with machine-checkable dependency rules.
- Protocols are **generated** from Rust; hand-editing generated files is forbidden.

See `docs/design.md` for the full system design.

## Repository Structure

```
/agent-engine          # repo name in design, maps to /home/dev/asha
  /governance           # lane docs, ADRs, reviewer prompts, ownership config
  /harness              # CI, lints, depgraph checkers, goldens, fixtures
  /engine-rs            # Rust cargo workspace
    /crates
      /foundation       # core-ids, core-math, core-time, core-error, core-collections
      /state            # core-state, core-events, core-commands, core-snapshot
      /protocol         # protocol-{script,render,replay,telemetry,codegen}
      /sim              # sim-{kernel,validator,applier,replay,runner}
      /services         # svc-{rng,spatial,collision,physics,pathfinding,serialization,volume,mesh}
      /rules            # rule-{lifecycle,process,scheduler,relationship,state-machine}
      /render           # render-{bridge,debug}
      /wasm             # wasm-api
      /tools            # replay-tool, snapshot-diff, protocol-dump, state-inspector, fixture-maker
  /ts                   # pnpm workspace
    /packages
      /contracts        # generated TypeScript from Rust protocol crates
      /script-sdk       # policy authoring SDK
      /script-host      # policy execution sandbox
      /policy-core      # default/noop policies
      /catalog-core     # typed catalog definitions
      /runtime-bridge   # transport-agnostic runtime facade + render-diff decode (ADR 0006)
      /native-bridge    # loader for the compiled napi-rs runtime addon
      /wasm-replay-bridge # WASM replay/golden path (tests/devtools only)
      /renderer-three # scene projection from render diffs
      /ui-dom           # panels, inspectors, command palette
      /app              # composition and wiring
      /electron-main    # thin wrapper
  /assets, /data, /docs
```

## Local Commands (Phase 0 — skeleton only)

```bash
# Rust
cargo build -p sim-kernel
cargo test -p sim-kernel
cargo clippy -p sim-kernel

# TypeScript
pnpm install
pnpm typecheck --filter @agent-engine/contracts
pnpm test --filter @agent-engine/policy-core

# Harness
./harness/ci/check-all.sh          # full check suite
./harness/ci/check-rust.sh         # rust-only
./harness/ci/check-ts.sh           # ts-only
./harness/ci/check-depgraph.sh     # dependency graph verification

# Governance
./harness/ci/check-contracts.sh    # generated contract diff
./harness/ci/check-replays.sh      # replay golden tests
./harness/ci/check-render-goldens.sh # render diff golden tests
```

## Prototype Phases

| Phase | What | Exit |
|-------|------|------|
| 0 | Governance skeleton | Empty crates compile, depgraph passes |
| 1 | Rust authority core | Typed IDs, StateStore, command/event applier |
| 2 | Protocol generation | Rust protocol → TS contracts, CI guard |
| 3 | Constrained TS policy | Policy proposes command, Rust validates |
| 4 | Replay audit path | Golden replay passes, divergence reported |
| 5 | Render projection | Rust emits render diffs, TS consumes |
| 6 | Parallel fan-out trial | Independent lanes merge cleanly |

## Agent Lane Quick Reference

| Lane | Language | Crate/Package dir | May not |
|------|----------|-------------------|---------|
| rust-foundation | Rust | engine-rs/crates/foundation/* | Know about state/protocols/render |
| rust-state | Rust | engine-rs/crates/state/* | Know about render/UI/TS |
| rust-service | Rust | engine-rs/crates/services/* | Introduce policy concepts |
| rust-rule | Rust | engine-rs/crates/rules/* | Domain-specific nouns |
| rust-render | Rust | engine-rs/crates/render/* | Render (emit diffs only); import wasm-api |
| rust-wasm-bridge | Rust | engine-rs/crates/wasm/* | Product/policy/render decisions |
| contract-steward | Rust/TS | engine-rs/crates/protocol/*, ts/packages/contracts/ | Hand-edit generated files |
| ts-policy | TS | ts/packages/policy-* | Import renderer/UI/bridge |
| ts-catalog | TS | ts/packages/catalog-* | Mutate state |
| ts-shell | TS | ts/packages/{renderer,ui,app}* | Policy decisions |
| ts-tools | TS | ts/packages/devtools* | Leak tool scope to runtime |

## Design Principles

- **Boring architecture**: Libraries you call > frameworks that call you
- **Infrastructure first**: Prove the machinery before adding domain content
- **Replay is audit bureaucracy**: Every state change must be replayable
- **The desired failure mode**: The agent cannot compile the wrong thing

## TypeScript House Style

TypeScript in this repo is written for agent governance, not clever human terseness.

Prefer longer, clearer code over compact clever code. Use named intermediate
values for meaningful decisions. Split work into small functions with explicit
verbs. Avoid generic abstractions until duplication has stabilized. Keep mutation
local and visible. Do not create ambient state, manager classes, global registries,
or hidden runtime coupling.

A good TypeScript diff should be easy for a reviewer agent to inspect mechanically:
imports reveal lane boundaries, functions reveal intent, tests reveal behavior,
and public API changes are explicit.

When in doubt, write the boring version.

## Rust House Style

Rust in this repo should be boring authority code. Prefer explicit state, explicit errors, explicit events, and narrow crate APIs. Do not introduce clever abstractions, runtime escape hatches, or framework-shaped machinery unless a lane owner explicitly approves them.