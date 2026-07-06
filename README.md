# ASHA — Agent Safety & Harness Architecture

ASHA is engine infrastructure for high fan-out agent development. The repository is shaped so many short-lived coding agents can work in bounded lanes while the compiler, generated contracts, fixtures, dependency checks, and review prompts make cross-lane drift visible.

The core split is:

> **Rust owns authority. TypeScript owns expression and projection. Generated contracts define the border.**

- **Rust** owns canonical state, validation, accepted event application, deterministic services, replay, serialization, bridge surfaces, and render projections.
- **TypeScript policy/catalog packages** author constrained proposals and data, but do not mutate authority.
- **TypeScript shell/render/UI packages** display projected state, collect input, show diagnostics, and submit typed requests through the runtime bridge.
- **Generated contracts** define the Rust/TypeScript border. Generated TypeScript files are committed for worker convenience but must not be hand-edited.

TypeScript never becomes a second authoritative engine.

---

## Repository posture

This repo has moved past the original prototype-phase checklist. Do not infer current work from old phase language. Current planning, implementation queues, and architectural decisions live in Den under project `asha`; use Den tasks/docs/messages as the durable source of truth when available.

Major durable surfaces include:

- Rust authoritative state, commands, events, snapshots, replay, voxel data, voxel edit rules, world-bundle load/save, diagnostics, and render projection infrastructure.
- Generated protocol contracts for TypeScript packages.
- A transport-agnostic runtime bridge with native-addon, reference/mock, and WASM replay-related surfaces.
- A semantic `RuntimeSession` facade for consumer repos: ProjectBundle-shaped ECRP bootstrap, Entity/CapabilityState readouts, collision-constrained camera input, primary-fire runtime action receipts, lifecycle/restart readouts, nav/policy proposal evidence, and deterministic telemetry/replay summaries.
- Three.js retained renderer consuming render diffs.
- Editor tools, DOM/devtools read models, smoke harnesses, fixtures, goldens, and CI governance checks.
- Launchable voxel tooling and FPS/ECRP demo substrate docs describe committed surfaces and known non-claims; active work is tracked in Den, not as README phases.

For the full architecture, start with `docs/design.md` and live Den guidance (`get_agent_guidance(project_id="asha")`).

---

## Repository layout

```text
engine-rs/          Rust workspace (64 crates, including reserved cells)
  crates/
    foundation/     core IDs, math, time, errors, collections, coordinates, assets
    state/          authoritative state, entities, scene, catalog, voxel, commands, events, snapshots
    protocol/       Rust protocol schemas + codegen for TS contracts
    sim/            validation, event application, replay, runner
    services/       deterministic services: rng, spatial, collision, mesh, volume, serialization, policy views
    rules/          domain/rule lanes: lifecycle, process, scheduler, relationship, state machine, voxel edit, world bundle
    render/         render-bridge and render-debug projection lanes
    bridge/         runtime-bridge-api manifest and native-bridge napi addon
    wasm/           wasm-api for replay/golden surfaces
    tools/          fixture-maker, replay-tool, diagnostics, protocol dump, asset import, snapshot diff, state inspector

ts/                 pnpm workspace (21 packages, including reserved cells)
  packages/
    contracts/          generated TypeScript contract types (do not hand-edit generated/)
    script-sdk/         policy view/command helpers, deterministic env, test harness
    script-host/        policy loading, sandboxing, deterministic invocation
    policy-*            constrained policy packages
    catalog-*           typed catalog definitions/examples
    runtime-bridge/     transport-agnostic runtime facade and render-diff decode
    native-bridge/      loader for compiled napi-rs runtime addon
    wasm-replay-bridge/ WASM replay/golden bridge for tests/devtools
    renderer-three/     Three.js handle registry and render-diff application
    editor-tools/       pure editor state, selections, previews, voxel command helpers
    ui-dom/             DOM panels, inspectors, command palette, preview overlays
    cosmetic/           non-authoritative visual effects
    devtools/           diagnostics/readout panels and replay/world-bundle views
    smoke/              end-to-end developer smoke harness
    app/                runtime/app composition and wiring
    electron-main/      process/window/IPC integration

governance/         Ownership rules, ADRs, lane guidance, reviewer prompts
harness/            CI scripts, depgraph verifiers, fixtures, goldens, smoke/perf outputs
docs/               Architecture, contracts, replay, render, voxel, bridge, and determinism docs
```

---

## Architecture boundaries

### Authority and projection

The normal flow is:

```text
inputs / tools / policy / UI
  -> proposed commands
  -> Rust validation
  -> accepted domain events
  -> authoritative state mutation
  -> render diffs, telemetry, diagnostics, replay records
  -> TypeScript renderer/UI/devtools projections
```

Keep the categories separate. Do not collapse commands, events, render diffs, telemetry, and replay records into a generic event bus.

### Runtime bridge

The old direct `wasm-bridge` style was replaced by `@asha/runtime-bridge` as the transport-neutral facade. App/UI/renderer/devtools should couple to the runtime bridge, not directly to native/WASM implementation details.

Backends include:

- `@asha/native-bridge` / Rust `native-bridge` napi addon where available;
- reference/mock bridge paths for development and tests;
- `@asha/wasm-replay-bridge` for replay/golden tests and devtools-related surfaces.

Native unavailable or unimplemented operations must fail closed with classified errors, not silently fall back to mock behavior.

### Contracts

Rust protocol crates define the border. TypeScript generated contracts live under `ts/packages/contracts/src/generated/` and are regenerated by project tooling. Do not hand-edit generated files.

A protocol change should include:

- Rust protocol/schema update;
- regenerated TypeScript contracts;
- fixture/golden updates where relevant;
- downstream Rust/TS tests;
- compatibility/diagnostic notes when the change affects runtime behavior.

### Lanes

Every crate/package is an assignment cell. Ownership, implementation status, and allowed dependency edges are machine-readable in `governance/ownership.toml`; prose lane expectations live in `governance/lanes/` and reviewer prompts. Cells marked `implementation_status = "reserved"` are intentional placeholders, not mature implementation surfaces.

Do not “just import” across lanes to make a task pass. If a dependency is needed, update the appropriate governance rule and justify the boundary change.

---

## Common commands

Run from the repository root unless noted.

### Full check suite

```sh
./harness/ci/check-all.sh
```

### Rust

```sh
cd engine-rs
cargo check --workspace
cargo test --workspace
cargo clippy --workspace
```

### TypeScript

```sh
cd ts
pnpm install
pnpm -r build
pnpm -r typecheck
pnpm -r test
```

### Focused governance/golden checks

```sh
./harness/ci/check-depgraph.sh
./harness/ci/check-contracts.sh
./harness/ci/check-replays.sh
./harness/ci/check-render-goldens.sh
./harness/ci/check-bridge.sh
```

### Developer smoke

```sh
cd ts
pnpm dev:asha-smoke
```

Focused evidence lanes split reference fixtures from product/live authority:

```sh
cd ts
pnpm --filter @asha/runtime-bridge test:evidence:reference
pnpm --filter @asha/runtime-bridge test:evidence:rust
pnpm --filter @asha/smoke test:evidence:reference
pnpm --filter @asha/smoke test:evidence:authority
```

### App shell launch

The launchable voxel shell is composed by one transport-agnostic root, `composeAppShell`
in `@asha/app` (`packages/app/src/shell.ts`). Every host — Electron renderer, browser, and
the headless CLI — runs that same composition; only the injected host capabilities, renderer
port, and bridge boot differ. The Electron main process (`@asha/electron-main`) only opens an
accessibility-enabled window pointed at the shared entry; it imports no runtime packages.

```sh
cd ts
pnpm --filter @asha/app dev:asha-shell                        # reference (mock) shell → harness/shell-out/
ASHA_SHELL_MODE=authority pnpm --filter @asha/app dev:asha-shell   # real native path (unavailable offline)
```

The headless launch is the CI-safe composition target: it boots the runtime, loads the
selected fixture, projects authority through the facade, and writes a deterministic
`ShellReadout` (runtime mode, fixture/world status, renderer status, accessible controls, and
the devtools editor inspection). Runtime mode is reported honestly as `native` / `reference` /
`degraded` / `unavailable` — there is no silent native→mock downgrade.

Check the relevant package scripts before adding new commands; this workspace intentionally prefers explicit package/lane surfaces over hidden global magic.

---

## Key documents

| Document | Purpose |
|---|---|
| `docs/launchable-voxel.md` | **Launchable voxel loop hub**: fixture, launch/smoke commands, regeneration, known limitations |
| `docs/perf-baseline.md` | Same-host perf baseline harness (`dev:asha-perf`) plus optional non-gating GPU/WebGL lane (`dev:asha-gpu-perf`): trend tracking, field stability |
| `docs/design.md` | Canonical repository architecture, layer model, dependency direction, and design principles |
| `docs/architecture-overview.md` | Short orientation pointer to the canonical architecture and governance docs |
| `governance/architecture.md` | Governance-specific TS metadata axes and boundary notes |
| `docs/runtime-bridge-boundary.md` | Runtime bridge facade and transport boundary |
| `docs/contract-governance.md` | Protocol/codegen change process |
| `docs/replay-model.md` | Replay recording, playback, and determinism audit |
| `docs/render-protocol.md` | Retained-mode render diff protocol |
| `docs/determinism.md` | Determinism requirements and enforcement |
| `docs/tunnel-generator-substrate.md` | Deterministic enclosed tunnel generator schema, import path, and projection evidence |
| `docs/combat-authority-substrate.md` | Combat/health/fire-intent authority surface and replay evidence |
| `docs/ecrp-runtime-session-readout.md` | RuntimeSession ProjectBundle-shaped ECRP load/readout surface and CapabilityState behavior |
| `docs/ecrp-fps-object-model.md` | Public FPS object-model map from generated-tunnel roles to ECRP capabilities and runtime surfaces |
| `docs/ecrp-capability-rule-ownership.md` | ECRP rule-owner matrix and current FPS RuntimeSession authority slice |
| `docs/runtime-session-facade.md` | Current `RuntimeSessionFacade` methods, non-claims, and reference/native boundary |
| `docs/nav-pathfinding-substrate.md` | Read-only voxel navigation projection and deterministic path query evidence |
| `docs/policy-authoring.md` | Policy package authoring and testing |
| `docs/voxel-coordinates.md` | Voxel/grid/chunk coordinate conventions |
| `docs/voxel-mesh-seam.md` | Voxel meshing/seam design notes |
| `docs/voxel-ui-architecture.md` | Voxel editor/UI architecture notes |
| `governance/ownership.toml` | Machine-readable lane ownership and dependency rules |
| `governance/lanes/` | Per-lane prose rules for agent assignment |

---

## Notes for outside agents

- Resolve live Den guidance before substantial work: `get_agent_guidance(project_id="asha")`.
- Treat Den tasks/docs/messages as the source of truth for current planning state.
- Read `AGENTS.md` for local bootstrap, but remember it is generated from Den guidance plus `agents-project.md` and may lag between regenerations.
- Preserve the authority boundary: TypeScript proposes/projects; Rust validates/applies.
- Do not introduce product-domain concepts into infrastructure tasks unless the active Den task explicitly asks for them.
- Do not hand-edit generated contracts.
- Prefer adding or updating fixtures/goldens when changing state, protocol, replay, render projection, bridge, or voxel behavior.
- Keep mock/reference/native behavior visibly classified; never hide native gaps behind silent fallback.
