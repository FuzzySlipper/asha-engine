# @asha/wasm-replay-bridge — replay/golden WASM path (task #2251, ADR 0006)

WASM is the canonical **replay authority** (docs/determinism.md), not the runtime
transport. This package runs a `ReplayRecord` under WASM semantics for golden checks
and classifies native-vs-WASM divergence. Imported by **tests/devtools only**.

## wasm-bridge piece classification (runtime vs replay)

The legacy `@asha/wasm-bridge` mixed transport-neutral decode with a runtime
assumption. Disposition under ADR 0006:

| Piece in `@asha/wasm-bridge` | Concern | Destination |
|---|---|---|
| `decodeRenderDiff` / `decodeRenderFrameDiff` | transport-neutral payload → contract types | `@asha/runtime-bridge` (`readRenderDiffs` output) |
| `RenderDiffStream` | facade-level frame buffering | `@asha/runtime-bridge` |
| `FrameMemory` ("WASM-owned bytes") | **runtime assumption** | reshape → facade `RuntimeBufferView` over native bridge-owned buffers; drop the "WASM-owned" framing |
| package name / runtime-transport role | **runtime assumption** | this package: replay/golden/devtools WASM only |

Nothing here decodes render diffs or drives a scene — those are runtime concerns now
behind the facade. This package keeps **only** replay/golden/devtools duties:
`replayHashes`, the `ReferenceReplayRunner` baseline, and `classifyDivergence`.

> Migration note: the decode/stream move out of legacy `wasm-bridge` and the renderer
> repoint to `@asha/runtime-bridge` are completed. This package is replay/golden/devtools
> WASM only; runtime render-diff decode belongs behind `@asha/runtime-bridge`.

## What works now (offline, verified)

- **`loadWasmReplayAuthority()`**: loads the compiled `wasm-api` module (the real
  `sim-replay` `decode`+`diff`+`DivergenceClass` logic, compiled to wasm32) and runs it
  from Node. `classifyRecords(expected, actual)` classifies two replay artifacts (the
  `harness/goldens/replays/*.replay` text format) under WASM semantics — `match` /
  `hash-checkpoint-mismatch` / `command-mismatch` / … / `malformed-artifact`.
- `classifyDivergence` / `compareReplay` / `ReferenceReplayRunner`: pure, toolchain-free
  per-step hash comparison utilities (`match` / `hash_divergence` / `length_divergence`).
- Tests → 9 passing: 5 pure + 4 running the **real WASM module** (verified against the
  committed golden artifact; tampered post hash → `hash-checkpoint-mismatch` at the step).

## Build & verify

```bash
harness/ci/check-wasm-replay.sh   # cargo build --target wasm32 + wasm-bindgen (nodejs) + tests
```

Builds `wasm-api` to `wasm32-unknown-unknown`, runs `wasm-bindgen --target nodejs` into
`dist/wasm/` (gitignored), and runs the package tests (the WASM-authority tests then run
instead of skipping). The wasm32 target and `wasm-bindgen` CLI (0.2.123) are required;
when the module is absent the WASM tests skip so offline `check-all` stays green.

## Remaining follow-up (not a blocker)

Wire `classifyRecords` into `check-replays.sh` so golden replays also assert
**native-vs-WASM agreement** (replay-tool native result vs the WASM authority), not just
native reproduction. The machinery is in place; this is the CI integration step.
