---
status: current
audience: agent
tags: [ci, perf, baseline]
supersedes: []
see-also: []
---

# Launchable-voxel performance baseline

A deterministic, **logged** performance scenario over the canonical launch fixture,
run on **one stable host** for trend / regression tracking. It is intentionally *not*
a product performance target and *not* part of the normal CI gate.

> **Same-machine baseline.** Absolute timings are only meaningful relative to other
> runs **on the same host**. Do not compare milliseconds across machines, and do not
> read these numbers as final-product performance — they measure the reference
> (mock-facade) launch/edit/render/save loop in a headless Node process with no GPU.

## What it measures

The harness (`ts/packages/smoke/src/perf.ts`) reuses the smoke building blocks — the
same runtime facade, `ThreeRenderer`, `EditorStore`, and canonical fixture — and runs
the launch→edit→render→save→replay loop, recording:

- **Timings** (`performance.now`, logged/trended, never a gate): `initialize`,
  `world-load`, `render-projection-initial`, `renderer-apply-initial`, `edit-one-cell`,
  `edit-region`, `edit-inverse`, `render-update`, `preview-overlay`, `save`, `reload`,
  `replay`, and an aggregate `edit-render-cycles` loop (mean per cycle = `ms / iterations`).
- **Counters** (the *stable*, comparable fields): peak/leaked render handles, scene
  nodes, overlay cells, material/sprite fallbacks, commands accepted/rejected, total
  render ops applied, replay steps + divergence, outstanding buffers.
- **Structural invariants** (these MAY fail the run hard): `no-handle-leak`,
  `no-preview-remesh`, `bounded-render-ops-per-cycle`, `commands-accepted`,
  `replay-not-diverged`. The exit code reflects **only** these — timings never fail it.

## Running it

```bash
cd ts
ASHA_PERF_HOST=<stable-host-label> pnpm --filter @asha/smoke dev:asha-perf
# native authority path (fails closed honestly if the addon is unavailable):
ASHA_PERF_HOST=<stable-host-label> ASHA_PERF_MODE=authority pnpm --filter @asha/smoke dev:asha-perf
```

Environment knobs (all optional):

| Var | Meaning | Default |
|---|---|---|
| `ASHA_PERF_HOST` | Stable host label — the anchor for same-host comparison | OS hostname |
| `ASHA_PERF_MODE` | `reference` (mock baseline) or `authority` (native path) | `reference` |
| `ASHA_PERF_COMMIT` / `ASHA_PERF_BRANCH` | Override the recorded revision | `git` then `unknown` |

> Set `ASHA_PERF_HOST` to the **same** label every run on a given machine — that is the
> key the trend is grouped by.

## Optional discrete-GPU / WebGL lane

There is a second, **manual** lane for a repeatable machine with a discrete GPU and a
real GL/Electron/WebGL context. It complements the same-machine baseline above; it does
not replace it, does not run in normal CI, and does not create product FPS budgets.

```bash
cd ts
# classified skip artifact when no GPU context is configured (safe on any host):
pnpm --filter @asha/smoke dev:asha-gpu-perf

# real GPU-host run (operator supplies the repeatable host/context metadata):
ASHA_PERF_HOST=<stable-gpu-host-label> \
ASHA_GPU_PERF_ENABLE=1 \
ASHA_GPU_PERF_CONTEXT=electron-webgl \
ASHA_GPU_NAME='<gpu name>' \
ASHA_GPU_DRIVER='<driver version>' \
ASHA_GPU_RUNTIME='<electron/runtime version>' \
ASHA_GPU_BROWSER='<browser/webview version>' \
pnpm --filter @asha/smoke dev:asha-gpu-perf
```

`ASHA_GPU_PERF_CONTEXT` must be one of `electron-webgl`, `browser-webgl`, or
`external-gl`. Without both `ASHA_GPU_PERF_ENABLE=1` and that context, the command writes
a `status: "skipped"` record with `skip.reason: "gpu_context_not_enabled"` and exits
successfully. That is intentional: discrete-GPU availability is never a default gate.

The GPU lane output is written beside the same-host baseline, but under separate names:

- `launch-voxel-gpu-perf.jsonl` — one JSON record appended per GPU-lane invocation.
- `launch-voxel-gpu-perf.latest.json` — the latest GPU-lane record, pretty-printed.

Each GPU record is `{ ok, status, meta, skip, asha, externalCalibrations }`. `meta`
records the ASHA commit/branch/fixture plus `lane: "discrete-gpu-gl-render"`,
`gating: "non-gating"`, render context, GPU/driver/vendor/device, browser/runtime, and
host basics. `asha` contains the same launchable-voxel structural metrics as the baseline
when the lane actually runs. Skipped records have `asha: null` and a classified reason.

Optional external WebGL/browser calibration scores can be attached as contextual data:

```bash
ASHA_GPU_EXTERNAL_CALIBRATION='[
  {"name":"MotionMark","score":123.4,"unit":"score","source":"manual","notes":"operator supplied"}
]'
```

These calibration records are always stamped `gating: "non-gating"`. They are useful for
ballpark host/browser/GPU sanity notes (for example MotionMark/Basemark-style scores or
small local reference scenes), but they are **not** acceptance criteria and must not block
merge/review by default. Omit them freely; omission is not a failure.

## Native voxel update telemetry

Rust-backed sessions expose one bounded, projection-bound structural observation through
`RuntimeSessionFacade.readVoxelUpdateTelemetry({ grid, projectionCursor })`. The request
must name the exact cursor returned by `readProjection()`. A stale or future cursor, a
different grid, or a read before the first projection fails closed. The bridge retains
only the latest observation; it does not accumulate a telemetry log.
Engine initialization, project replacement/close, and workspace-authoring replacement
clear the retained observation and pending counters.

The readout separates the work leading to that projection:

- committed command batches, accepted commands, and estimated touched voxels since the
  previous projection;
- resident chunks, chunks dirtied, projected, and remeshed;
- emitted mesh payloads, total render operations, and dirty chunks still pending after
  projection.

These are structural counters for diagnosing edit-to-projection behavior. They are not
authority hashes, replay evidence, correctness goldens, or storage-layout readback.
There are deliberately no elapsed-time fields: host timing is non-deterministic and
belongs in the existing same-machine trend record above. Reading telemetry is
non-consuming; a later projection deterministically replaces the retained observation.

### Procgen adoption handback

For a native Procgen edit probe, retain the command receipt, call `readProjection()`
once, then query `readVoxelUpdateTelemetry` with the selected grid and the returned
projection cursor. Correlate affected chunks with `readVoxelMeshEvidence`; do not infer
meshing work from renderer-private objects. Record the engine commit and stable host
label beside any external timing, compare only same-host trends, and keep these metrics
diagnostic rather than making them delivery proof or a default CI budget.

## Output

Written under `harness/perf-out/` (gitignored — it is per-host trend data, not a golden):

- `launch-voxel-perf.jsonl` — one JSON record **appended per run** (the trend history).
- `launch-voxel-perf.latest.json` — the latest run, pretty-printed.

The optional GPU/WebGL lane uses separate files in the same directory:
`launch-voxel-gpu-perf.{jsonl,latest.json}`. Keep those records out of the canonical
weak-/same-host trend unless you explicitly choose to compare the GPU host with itself.

Each record is `{ ok, meta, timings, counters, invariants }` (schema in `perf.ts`,
`schema: 1`). `meta` carries `commit / branch / hostLabel / runtimeMode / smokeMode /
fixtureId / fixtureVoxelStateHash` plus host basics (`node / platform / arch / cpus /
cpuModel / totalMemMb`) and a `timestamp`.

## Comparing runs over time

1. **Group by `meta.hostLabel`** (and `runtimeMode`/`smokeMode`). Only compare within a
   group — cross-host millisecond comparison is meaningless.
2. **Anchor on the stable fields.** `meta.fixtureVoxelStateHash`, the counters, and the
   invariant set should be **identical** run-to-run for the same commit; a change there
   is a real structural shift, not noise. Treat those as the regression signal.
3. **Read timings as trends, not thresholds.** Watch a phase's `ms` (or `edit-render-cycles`
   mean) drift across commits on one host. A single run's absolute value is noisy; a
   sustained move across many runs is the signal. There is intentionally **no** committed
   timing golden and **no** CI threshold — wiring one would make CI flaky.

### Field stability cheat-sheet

| Field | Stable enough to assert? | Use |
|---|---|---|
| `counters.*`, `invariants[*].held`, `meta.fixtureVoxelStateHash` | Yes — deterministic | Regression gate (the harness already fails on the invariants) |
| `meta` host/runtime descriptors | Yes (per host) | Grouping key |
| `timings[*].ms`, `edit-render-cycles` mean | No — noisy per run | Trend only, same host, over many runs |
| `meta.timestamp` | No | Ordering only; never compare |

## Why this is not in `check-all.sh`

Timing gates are flaky by nature, so the perf harness is deliberately **separate** from
the offline CI gate. Its correctness contribution — the structural invariants — is
already covered deterministically by `perf.test.ts` (run inside `check-all.sh` via the
smoke package tests), which asserts the record's shape and invariants with an injected
clock and **never** asserts a timing value. The logged timings are for operator/CI-
artifact trend monitoring on a chosen baseline host.

## Limitations

- **Reference baseline by default.** The default run uses the deterministic mock facade,
  so it measures the TS launch/edit/render/save loop, not Rust authority compute. Native
  structural metrics are mirrored by the reference facade for contract testing, but
  only the Rust-backed `readVoxelUpdateTelemetry` path reports actual authority and
  projection work. Neither path reports per-chunk elapsed time.
- **No GPU / no pixel work.** `ThreeRenderer` runs headless (structural scene graph only),
  so renderer timings reflect retained-mode bookkeeping, not real draw cost.
- **GPU lane is operator-supplied context.** The optional GPU/WebGL command records the
  repeatable host/context metadata and carries the ASHA structural metrics beside any
  manual external calibration scores. It does not make real GPU availability a default
  CI/review dependency, and it currently fails/skips clearly rather than inventing a
  browser score or screenshot timing when no context was supplied.
- **No product targets.** This task measures; it sets no FPS/frame budgets and makes no
  optimization changes.

Related: `topics/authority/launchable-voxel.md` (the launch hub), `topics/authority/replay-model.md` (durability),
`harness/fixtures/smoke/README.md` (the shared fixture).
