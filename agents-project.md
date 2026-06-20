# ASHA Local Bootstrap

Project-specific live guidance lives in Den at `[doc: asha/design]`.

Use project ID `asha` for Den tasks, messages, documents, librarian queries, and guidance lookups.

## Source-of-truth posture

This local file is bootstrap context for agents entering the repository. It is not the current planning queue.

- **Den** owns current task state, implementation queues, durable planning docs, and known limitations.
- **Repo docs** describe architecture and committed implementation surfaces.
- **The code/tests** are the implementation truth when they conflict with old planning prose.
- The old prototype phase list is historical only; do not infer active work from it.

## Architecture Soul

> Rust owns authority. TypeScript owns expression and projection. Generated contracts define the border.

- **Rust** is authoritative: canonical state, validation, event application, deterministic services, replay, serialization, heavy simulation, and render projection generation.
- **TypeScript** proposes commands via constrained policy/catalog packages, displays projected state via shell/render/UI, and provides devtools/operator readouts.
- TypeScript **never mutates** authoritative state. Rust validates all commands.
- Every crate/package is an **agent assignment cell** with machine-checkable dependency rules.
- Protocols are **generated** from Rust; hand-editing generated files is forbidden.

See `docs/design.md` for the full system design and `README.md` for current repo orientation.

## Repository Structure

```
/agent-engine          # repo name in design, maps to /home/dev/asha
  /governance           # lane docs, ADRs, reviewer prompts, ownership config
  /harness              # CI, lints, depgraph checkers, goldens, fixtures, smoke/perf output
  /engine-rs            # Rust cargo workspace
    /crates
      /foundation       # core IDs, math, time, errors, collections, coordinates, assets
      /state            # core-state, core-entity, core-scene, core-catalog, core-voxel, commands/events/snapshots
      /protocol         # protocol schemas + protocol-codegen
      /sim              # sim-kernel, validator, applier, replay, runner
      /services         # rng, spatial, collision, physics, pathfinding, serialization, volume, mesh, policy-view
      /rules            # lifecycle, process, scheduler, relationship, state-machine, voxel-edit, world-bundle
      /render           # render-bridge, render-debug
      /bridge           # runtime-bridge-api manifest; native-bridge napi addon is built explicitly
      /wasm             # wasm-api replay/golden surface
      /tools            # replay, diagnostics, protocol dump, state inspector, fixture maker, asset import
  /ts                   # pnpm workspace
    /packages
      /contracts        # generated TypeScript from Rust protocol crates
      /script-sdk       # policy authoring SDK
      /script-host      # policy execution sandbox
      /policy-*         # constrained policies
      /catalog-*        # typed catalog definitions/examples
      /runtime-bridge   # transport-neutral runtime facade + render-diff decode
      /native-bridge    # loader for compiled napi-rs runtime addon
      /wasm-replay-bridge # WASM replay/golden bridge for tests/devtools
      /renderer-three   # Three.js projection from render diffs
      /editor-tools     # pure editor state, previews, command builders
      /ui-dom           # panels, inspectors, command palette
      /devtools         # diagnostics/readout panels
      /smoke            # launchable/smoke/perf harnesses
      /app              # composition and wiring
      /electron-main    # thin host wrapper
  /docs
```

## Local Commands

```bash
# Full gate
./harness/ci/check-all.sh

# Focused gates
./harness/ci/check-rust.sh      # includes cargo fmt --check, cargo check, cargo clippy --workspace -- -D warnings, cargo test
./harness/ci/check-ts.sh
./harness/ci/check-depgraph.sh
./harness/ci/check-contracts.sh
./harness/ci/check-replays.sh
./harness/ci/check-render-goldens.sh
./harness/ci/check-bridge.sh

# Rust lane quick checks when a full Rust gate is too broad
(cd engine-rs && cargo clippy --workspace -- -D warnings)
(cd engine-rs && cargo clippy -p <crate-name> --all-targets -- -D warnings)

# Launchable voxel smoke / shell / perf
cd ts
pnpm --filter @asha/smoke dev:asha-smoke
pnpm --filter @asha/app dev:asha-shell
ASHA_PERF_HOST=<stable-host-label> pnpm --filter @asha/smoke dev:asha-perf
```

See `docs/launchable-voxel.md` and `docs/perf-baseline.md` for command details, output paths, and known limitations.

## Agent Lane Quick Reference

| Lane | Language | Crate/Package dir | May not |
|------|----------|-------------------|---------|
| rust-foundation | Rust | engine-rs/crates/foundation/* | Know about state/protocols/render |
| rust-state | Rust | engine-rs/crates/state/* | Know about render/UI/TS |
| rust-service | Rust | engine-rs/crates/services/* | Introduce policy/product concepts |
| rust-rule | Rust | engine-rs/crates/rules/* | Depend on renderer/UI truth |
| rust-render | Rust | engine-rs/crates/render/* | Render directly or own authority |
| rust-wasm-bridge | Rust | engine-rs/crates/wasm/* | Product/policy/render decisions |
| contract-steward | Rust/TS | engine-rs/crates/protocol/*, ts/packages/contracts/ | Hand-edit generated files |
| ts-policy | TS | ts/packages/policy-* | Import renderer/UI/bridge/browser globals |
| ts-catalog | TS | ts/packages/catalog-* | Mutate authority |
| ts-shell | TS | renderer/ui/app/electron/runtime-bridge packages | Validate/apply authority |
| ts-tools | TS | ts/packages/devtools*, smoke tooling | Leak tool omniscience into runtime |

## Design Principles

- **Boring architecture**: Libraries you call > frameworks that call you.
- **Infrastructure first**: Prove machinery before adding product content.
- **Replay is audit bureaucracy**: Every state change must be replayable or intentionally documented as outside replay scope.
- **Desired failure mode**: The agent cannot compile the wrong thing.

## TypeScript House Style

TypeScript in this repo is written for agent governance, not clever human terseness.

Prefer longer, clearer code over compact clever code. Use named intermediate values for meaningful decisions. Split work into small functions with explicit verbs. Avoid generic abstractions until duplication has stabilized. Keep mutation local and visible. Do not create ambient state, manager classes, global registries, or hidden runtime coupling.

A good TypeScript diff should be easy for a reviewer agent to inspect mechanically: imports reveal lane boundaries, functions reveal intent, tests reveal behavior, and public API changes are explicit.

When in doubt, write the boring version.

## Rust House Style

Rust in this repo should be boring authority code. Prefer explicit state, explicit errors, explicit events, and narrow crate APIs. Do not introduce clever abstractions, runtime escape hatches, or framework-shaped machinery unless a lane owner explicitly approves them.
