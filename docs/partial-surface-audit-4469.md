# Partial Surface Audit For Task 4469

Status: current evaluation for Den task #4469.

This note records which active, partially suspected ASHA cells need expansion
work and which are intentionally bounded enough for the current architecture.
It is not a proof harness. It is a routing note for future implementation
agents so partial surfaces do not become forgotten load-bearing scaffolds.

## Evaluation Rules

- Prefer current code and tests over old planning prose.
- Create implementation tasks only for concrete gaps.
- Do not create synthetic proof work when a focused implementation task is the
  sane path.
- Keep Rust authority and TypeScript projection/config boundaries intact.

## Concrete Follow-Up Tasks

| Task | Surface | Reason |
|---|---|---|
| #4745 | `render-debug` | The crate is real, but it only emits entity point labels while docs describe point, line, and label debug overlay primitives. |
| #4746 | `wasm-api` / `@asha/wasm-replay-bridge` | The code is replay-only and has an opt-in gate, but some docs/lane text still imply broader runtime WASM exports or stale empty-crate state. |
| #4747 | `svc-pathfinding` | Projection pathfinding is real, but direct navigation still has a straight-line helper that does not consume `NavProjection`; lane text also mentions caching. |
| #4748 | `rule-lifecycle` | The crate now owns FPS RuntimeSession authority, while the rust-rule lane still describes generic lifecycle rules. The generic-vs-FPS boundary needs to be explicit in code or durable docs. |
| #4749 | `@asha/runtime-bridge` / `@asha/runtime-session` | RuntimeSession semantic facade code is still concentrated in the transport package. Split semantic session ownership into `@asha/runtime-session`. |
| #4750 | `@asha/game-workspace` | The package is real and tested, but manifest/authoring modules are large TS descriptive surfaces that should be split with a shrink-only shape ratchet. |

## Complete Enough For Now

### `sim-runner` And `sim-replay`

Current state:

- `sim-runner` has explicit tick execution, `Recorder`, checkpoint handling,
  playback, and 18 local tests.
- `sim-replay` owns deterministic text encode/decode, divergence classes, diff
  routing, and 17 local tests.
- `docs/replay-model.md` now describes their authority split and `replay-tool`
  drives committed golden replays through `harness/ci/check-replays.sh`.

Decision:

These are no longer partial in the #4458 sense. The remaining replay-family
debt is the known voxel durability unification path already recorded in
`docs/replay-model.md` as Den task #2440, not a new `sim-runner`/`sim-replay`
expansion task.

### `svc-rng`

Current state:

- The crate exposes explicit `RngSeed` and `ScopedRng` streams.
- It forbids ambient randomness and has local tests for deterministic replay,
  scope divergence, and bounded output rejection.
- `docs/determinism.md` names `svc-rng` as the only authoritative randomness
  source.

Decision:

Complete enough for the current lane promise. Future work should be driven by
specific consumers that need a new deterministic distribution or sampling API,
not by a vague "expand RNG" task.

### `rule-scheduler`

Current state:

- The crate exposes deterministic chunk work queueing, budgeted draining,
  version/staleness classification, diagnostics snapshots, and eight local
  tests.
- The API is intentionally abstract over execution and does not call meshing,
  collision, render, or generator crates.

Decision:

Complete enough for current scheduler lane promises. Further work should be
consumer-led, such as adding a concrete scheduler integration in a voxel or
runtime pipeline, rather than expanding the scheduler in isolation.

### `catalog-examples` And `cosmetic`

Current state:

- Both were placeholder packages during #4458.
- Both now have real source files and tests.

Decision:

They are no longer part of the partial-surface audit. If they need more work,
that should come from consumer-driven catalog or projection requirements.

## Notes For Reviewers

- The tasks above intentionally avoid broad "prove the surface" language.
- Any future child task under #4469 should name the implementation gap, the lane
  boundary it must preserve, and the focused local checks that prove it.
- If an implementation leaves a stub or known limitation behind, update Den's
  `known-limitations` document rather than relying on this audit note.
