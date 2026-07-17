---
status: current
audience: agent
tags: [voxel, launch, smoke]
supersedes: []
see-also: []
---

# Launchable voxel loop

The end-to-end "first launchable" path: boot the runtime, load a canonical voxel
world, project it to the renderer, pick/select a voxel, preview and commit an edit
through authority, see the render update, and save/reload/replay it durably — all
proven by a single structured smoke run.

This page is the cold-start index: it points at the canonical fixture, the launch and
smoke commands, every regeneration command, and the known limitations, so a worker can
build on the real system without reading chat history. The per-task Den docs
(`launchable-voxel-*`) are **investigatory pointers and design history, not authority**
— where a Den comment and the committed code disagree, the code (and this page) win.

## Architecture soul

> Rust owns authority. TypeScript owns expression and projection. Generated contracts
> define the border.

The renderer and UI never mutate voxel state; they produce proposals/previews and
project authority output. Every edit and pick crosses the runtime-bridge facade to Rust
(`rule-voxel-edit`, `svc-collision`). See `topics/authority/voxel-ui-architecture.md`,
`topics/bridge/runtime-bridge-boundary.md`, and ADR `governance/adr/0008-ui-editor-architecture.md`.

## Where each piece lives

| Stage | Authority (Rust) | Projection/expression (TS) |
|---|---|---|
| Canonical fixture | `engine-rs/crates/tools/fixture-maker` | `harness/fixtures/voxel-world/` |
| Meshing / projection | `svc-mesh`, `render-bridge` voxel projector | `@asha/renderer-three` |
| Picking | `svc-collision` raycast, `rule-voxel-edit::picking` | `@asha/app` `pickAndSelect` / `bridgePicker` |
| Editing | `rule-voxel-edit` validate/apply | `@asha/editor-tools`, `@asha/ui-dom` controls |
| Commands | `runtime-bridge-api` `submit_commands` | `@asha/app` `bridgeCommandSink` |
| Save / reload / replay | `rule-voxel-edit::persist`, `rule-project-bundle` | `@asha/devtools` `buildVoxelDurabilityModel` |
| Composition root | — | `@asha/app` `composeAppShell`, `@asha/electron-main` host |
| Proof | — | `@asha/smoke` 10-stage harness |

## Canonical fixture

One shared abstract voxel world (grid `1`, voxel size `1.0`, cubic `2×2×2` chunks, a
`2×2×1` chunk arrangement, materials `1,2,3`). It is **generated**, not hand-authored.
Full description and consumption notes: `harness/fixtures/voxel-world/README.md`.

```bash
cd engine-rs
cargo run -p fixture-maker -- write    # regenerate harness/fixtures/voxel-world/*
cargo run -p fixture-maker -- check    # verify committed payload (non-zero on drift)
```

The TS smoke/app fixture world is the matching descriptor (`sceneId 1001`, grid `1`,
materials `1,2,3`) in `ts/packages/smoke/src/fixtures.ts` and
`ts/packages/app/src/launch.ts` (`defaultFixtures`).

## Launching the shell

`@asha/app`'s `composeAppShell` is the one transport-agnostic composition root; the
Electron renderer, a browser entry, and the headless CLI all run it (only injected host
capabilities differ). The Electron **main** process (`@asha/electron-main`) just opens an
accessibility-enabled window pointed at the shared entry — it imports no runtime packages.

```bash
cd ts
pnpm --filter @asha/app dev:asha-shell                          # reference (mock) shell
ASHA_SHELL_MODE=authority pnpm --filter @asha/app dev:asha-shell # real native path (unavailable offline)
```

Output is a deterministic `ShellReadout` (runtime mode, fixture/world status, renderer
status, accessible controls, devtools editor inspection), written to
`harness/shell-out/` (gitignored). Runtime mode is reported honestly as
`native` / `reference` / `degraded` / `unavailable` — never a silent native→mock downgrade.

## Smoke proof

The canonical 10-stage launchable proof: `boot → load → render → pick → preview →
command-submit → authority-classify → render-update → save-reload-replay → cleanup`.

```bash
cd ts
pnpm --filter @asha/smoke dev:asha-smoke                           # reference smoke → harness/smoke-out/ (gitignored)
ASHA_SMOKE_MODE=authority pnpm --filter @asha/smoke dev:asha-smoke # real authority path
```

The reference run is deterministic and pinned by the committed golden
`harness/fixtures/smoke/reference-smoke.txt` (drift test in `smoke.test.ts`). It carries
renderer/resource counters (leaked/peak handles, scene/debug nodes, fallbacks,
outstanding buffers) proving the lifecycle is bounded, and a tested **preview-remesh
guardrail** (preview draws debug-layer overlay only, never authority geometry).

## Save / reload / replay durability

A voxel world saves as a base **edit log**, optionally compacted into chunk
**snapshots** plus a retained edit tail. `rule-project-bundle::durability` records the
`postLoad / postEdit / postReload` voxel state fingerprints for the canonical edit sequence
and proves durability (`postEdit == postReload`); tampering fails closed with a
classified error. Full model + the deferred generic-`ReplayRecord` unification:
`topics/authority/replay-model.md`.

Runtime-diverged **entity** authority — runtime-created entities, diverged
transforms, capability tables, relations, and source traces — persists separately as
a durable `sessionStateSnapshot` artifact (`core_entity` snapshot codec, composed by
`rule-project-bundle::spatial_session_state`, restored by the executor's `RestoreSessionState` load
stage). It is emitted only when runtime state diverged from the bootstrapped scene
baseline, never collapses voxel persistence into the scene document, and reloads
fail-closed. The mixed-world round-trip (scene-sourced + runtime-created, spatial +
non-spatial, contained/attached, voxel edit + entity change in one save) is proven by
`scene-diagnostics::session_state_round_trip` and the committed fixtures under
`harness/fixtures/session-state/` (Den #2484).

```bash
cd engine-rs
cargo run -p rule-project-bundle --example dump_durability > ../harness/fixtures/project-bundle/voxel-durability.txt
cargo test -p rule-project-bundle -p rule-voxel-edit        # checks the durability + persist goldens
cargo test -p core-entity -p scene-diagnostics            # checks the session-state snapshot codec + equivalence goldens
```

## Performance baseline

A deterministic, **logged** perf scenario over the same canonical fixture, for
same-host trend/regression tracking (not a CI gate, not a product target):

```bash
cd ts
ASHA_PERF_HOST=<stable-label> pnpm --filter @asha/smoke dev:asha-perf  # → harness/perf-out/ (gitignored)
```

It reuses the smoke building blocks and records phase timings + structural counters,
failing hard **only** on the structural invariants (leaks, preview remesh, bounded
per-cycle render ops, replay divergence, command acceptance) — timings are trended,
never thresholded. A separate manual GPU/WebGL lane writes
`launch-voxel-gpu-perf.{jsonl,latest.json}` only when explicitly opted in; external
WebGL/browser calibration is context-only and non-gating. Full field-stability guide +
how to compare runs: `topics/ci/perf-baseline.md`.

## Regeneration command index

| Artifact | Command |
|---|---|
| Canonical voxel fixture | `cd engine-rs && cargo run -p fixture-maker -- write` |
| Voxel persist sample golden | covered by `cargo test -p rule-voxel-edit` (inline `include_str!` goldens) |
| ProjectBundle compacted save | `cargo run -p rule-project-bundle --example dump_compacted_save > harness/fixtures/project-bundle/compacted-save.txt` |
| Voxel durability checkpoints | `cargo run -p rule-project-bundle --example dump_durability > harness/fixtures/project-bundle/voxel-durability.txt` |
| Session-state snapshot + equivalence | `BLESS=1 cargo test -p scene-diagnostics --test session_state_goldens` → `harness/fixtures/session-state/` |
| Regen-conflict diagnostic | `cargo run -p rule-project-bundle --example dump_regen_conflict > harness/fixtures/project-bundle/regen-conflict.txt` |
| Structural render goldens | `cd ts && pnpm --filter @asha/renderer-three test` (bless the `<name>.snapshot`); see `harness/goldens/render-diffs/README.md` |
| Golden replays | record with `sim-runner::Recorder`; see `topics/authority/replay-model.md` |
| Reference smoke golden | re-render `formatResult(runSmoke(reference))` → `harness/fixtures/smoke/reference-smoke.txt`; see its README |

## Known limitations (deferred, on purpose)

These are deliberate first-launchable scope cuts, not oversights. Pick one up only with
intent and a fresh decision — do not assume they are unimplemented by accident.

- **Generic-replay unification** — voxel save/reload uses a dedicated fingerprint path,
  not the tick-stepped `ReplayRecord`. Deferred; kept comparable via the shared
  `BundleHash`. (`topics/authority/replay-model.md`, Den #2440.)
- **Native-unwired operations** — the napi addon is excluded from the offline build, so
  `submit_commands`, `pick_voxel`, and most facade verbs fail closed with
  `operation_unimplemented` on the native path. The authority smoke/shell report this as
  `degraded`/`unavailable`, never a mock success. Wiring is tracked separately.
- **Browser dev target** — only the headless CLI and the Electron host run the shared
  composition root today; a browser/dev-server entry is optional future work (it must
  reuse `composeAppShell`, not fork it).
- **Advanced brush shapes** — the editor offers `single` and `box` only. Sphere/line/
  custom brushes are out of scope.
- **Pixel/screenshot goldens** — the render gate is the structural snapshot; true
  pixel goldens (real WebGL/offscreen) are deferred (`harness/goldens/screenshots/README.md`).
- **Performance budgets** — there is a logged same-host perf *baseline* (`dev:asha-perf`,
  `topics/ci/perf-baseline.md`) for trend tracking, plus a lowest-priority manual GPU/WebGL
  lane (`dev:asha-gpu-perf`) for discrete-host context. There is still no enforced
  timing/throughput budget and no product FPS target. Wiring a CI timing gate is
  deliberately avoided (it would be flaky); only the structural invariants fail hard.

## Related docs

| Repo doc | Purpose |
|---|---|
| `topics/authority/voxel-ui-architecture.md` | Editor/UI authority boundary (ADR 0008) |
| `topics/authority/voxel-mesh-seam.md` | Meshing / projection seam (`VoxelChunkProjector`) |
| `topics/authority/voxel-coordinates.md` | Grid/chunk/voxel coordinate conventions |
| `topics/bridge/runtime-bridge-boundary.md` | Facade surface + error taxonomy |
| `topics/authority/replay-model.md` | Replay + voxel durability evidence |
| `topics/ci/perf-baseline.md` | Same-host perf baseline harness (`dev:asha-perf`) plus optional non-gating GPU/WebGL lane (`dev:asha-gpu-perf`): trend tracking, field stability |
| `harness/fixtures/voxel-world/README.md` | Canonical fixture details |
| `harness/fixtures/project-bundle/README.md` | Save/compaction/durability goldens |
| `harness/fixtures/smoke/README.md` | Smoke golden + regeneration |

Den docs (`launchable-voxel-01`…`-10`) carry the original design intent and the
accepted/rejected discussion; treat them as history and pointers, and reconcile against
this page and the code.
