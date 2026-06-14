# ASHA — Agent Safety & Harness Architecture

An engine infrastructure designed for high fan-out agent development. The structure optimizes
for hard boundaries, compiler-checkable contracts, deterministic tests, and machine-reviewable
dependency rules so that parallel coding agents can work in isolated lanes without hidden coupling.

---

## Core split

> **Rust owns authority. TypeScript owns expression and projection. Generated contracts define the border.**

- **Rust** — canonical state, validation, event application, deterministic services, replay, simulation
- **TypeScript policy** — receives generated read-only views, returns proposed commands
- **TypeScript shell** — renders and displays what Rust says happened

TypeScript never mutates authoritative state.

---

## Repository layout

```
engine-rs/          Rust workspace (59 crates)
  crates/
    foundation/     core-ids, core-math, core-time, core-error, core-collections, core-space, core-assets
    state/          core-state, core-entity, core-scene, core-catalog, core-voxel, core-events, core-commands, core-snapshot
    protocol/       protocol-{ids,script,render,replay,telemetry,scene,world-bundle,assets,diagnostics,policy-view}, protocol-codegen
    sim/            sim-kernel, sim-validator, sim-applier, sim-replay, sim-runner
    services/       svc-rng, svc-spatial, svc-collision, svc-physics, svc-pathfinding, svc-mesh, svc-volume, svc-serialization, svc-policy-view
    rules/          rule-lifecycle, rule-process, rule-scheduler, rule-relationship, rule-state-machine, rule-voxel-edit, rule-world-bundle
    render/         render-bridge, render-debug
    bridge/         runtime-bridge-api (curated bridge manifest), native-bridge (napi-rs addon)
    wasm/           wasm-api (replay/golden WASM surface)
    tools/          replay-tool, snapshot-diff, protocol-dump, state-inspector, fixture-maker, asset-import, scene-diagnostics, voxel-diagnostics

ts/                 pnpm workspace (18 packages)
  packages/
    contracts/          generated TypeScript contract types (do not hand-edit generated/)
    script-sdk/         view types, command helpers, deterministic env, test harness
    script-host/        policy loader, sandbox, runtime isolation, deterministic invocation
    policy-*/           constrained policy packages
    catalog-*/          typed catalog definitions
    runtime-bridge/     transport-agnostic runtime facade + render-diff decode (ADR 0006)
    native-bridge/      thin loader for the compiled napi-rs runtime addon
    wasm-replay-bridge/ WASM replay/golden path (imported by tests/devtools only)
    renderer-three/     Three.js scene, handle registry, render diff application
    editor-tools/       editor-side scene/command helpers
    ui-dom/             DOM panels, inspectors, command palette
    cosmetic/           non-authoritative visual effects
    devtools/           replay viewer, debug dashboard, state inspector
    smoke/              end-to-end developer smoke harness
    electron-main/      process/window/IPC integration
    app/                runtime loop and wiring

governance/         Lane assignments, ownership rules, ADRs, reviewer prompts
harness/            CI scripts, dep-graph verifiers, goldens, fixtures, bridge manifest tooling
docs/               Architecture and protocol documentation
```

The runtime bridge replaced the former `wasm-bridge` package: app/UI/renderer/devtools
couple only to `@asha/runtime-bridge`, which is backed by `native-bridge` (napi-rs),
the mock, or the `wasm-replay-bridge` (ADR 0006; see `docs/runtime-bridge-boundary.md`).

---

## Dependency direction

**Rust:** `foundation → state → protocol → sim / services / rules → render / wasm / tools`

**TypeScript:**
```
contracts → script-sdk → policy/catalog → script-host
contracts → runtime-bridge → renderer / ui / devtools / editor-tools → app → electron-main
native-bridge / wasm-replay-bridge → runtime-bridge (transport backends only)
```

No lower layer may import a higher layer. The dep-graph verifier enforces this on every CI run.

---

## Getting started

**Rust**
```sh
cd engine-rs
cargo check --workspace
cargo test --workspace
cargo clippy --workspace
```

**TypeScript**
```sh
cd ts
pnpm install
pnpm -r typecheck
pnpm -r test
```

**Full CI check**
```sh
bash harness/ci/check-all.sh
```

**Dep-graph verification**
```sh
bash harness/depgraph/verify-rust-deps.sh
bash harness/depgraph/verify-ts-deps.sh
```

---

## Agent lane assignment

Every crate and package is an agent assignment cell. Ownership, allowed dependencies,
and forbidden imports are machine-readable in `governance/ownership.toml`.

Lane prose rules (what to own, what to avoid, required tests, drift smells) live in `governance/lanes/`.
Architecture and protocol docs live in `docs/`.

---

## Prototype phases

| Phase | Description | Status |
|---|---|---|
| **0** | Governance skeleton — repo structure, workspaces, lane docs, CI, dep-graph checker | **Complete** |
| 1 | Minimal Rust authority core — typed IDs, StateStore, commands, events, tick | **Complete** |
| 2 | Protocol generation — Rust protocol crates, TS contract codegen | **Complete** |
| 3 | Constrained TypeScript policy — script SDK, host, sandbox lint | **Complete** |
| 4 | Replay audit path — recording, playback, divergence reports | **Complete** |
| 5 | Render projection — retained render diffs, WASM bridge, Three.js scene | **Complete** |
| 6 | Parallel agent fan-out trial | Pending |

---

## Key documents

| Document | Purpose |
|---|---|
| `docs/design.md` | Full system design |
| `docs/architecture-overview.md` | Layer model and dependency rules |
| `docs/replay-model.md` | Replay recording, playback, and determinism audit |
| `docs/policy-authoring.md` | How to write and test a policy pack |
| `docs/render-protocol.md` | Retained-mode render diff protocol |
| `docs/determinism.md` | Determinism requirements and enforcement |
| `docs/contract-governance.md` | Protocol change process |
| `governance/ownership.toml` | Machine-readable lane and dependency rules |
| `governance/lanes/` | Per-lane prose rules for agent assignment |
