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

> Migration note: the decode/stream **move** out of `wasm-bridge` and the renderer
> repoint to `@asha/runtime-bridge` is staged but deferred so existing Phase 5
> render-goldens stay green; it is the remaining step of the #2248 migration plan.

## What works now (offline, verified)

- `classifyDivergence` / `compareReplay`: pure native-vs-WASM hash comparison →
  `match` | `hash_divergence` (with first diverging `StepIndex`) | `length_divergence`.
- `ReferenceReplayRunner`: deterministic, toolchain-free baseline so a replay fixture
  runs through *a* replay path in CI.
- `node --test` → 6 passing (fixture replay + classifier cases + classified blocker).

## Blocker: WASM module build

`loadWasmReplayModule()` requires the compiled `wasm-api` replay module, which cannot
be built here:

1. **No `wasm32` target installed**: `rustup target list --installed` has no
   `wasm32-unknown-unknown`; this is Arch system rust without `rustup` target
   management. `cargo build --target wasm32-unknown-unknown -p wasm-api` fails.
2. **`wasm-api/src/lib.rs` is currently empty**: the replay export surface
   (`replayHashes` / init / tick / diff retrieval per design §8.8) is not implemented yet.

Fallback evidence: the reference runner + classifier prove the replay comparison
machinery end-to-end; only the *authoritative WASM hashes* are pending the target.

## Unblock path (follow-up)

1. Install the `wasm32-unknown-unknown` target (or wire a wasm build container).
2. Implement `wasm-api` replay exports that re-execute a `ReplayRecord` and return
   per-step post hashes.
3. Build to wasm, load via `loadWasmReplayModule`, and feed both native + WASM hashes
   into `classifyDivergence` in `check-replays.sh` so golden replays assert WASM-authority
   agreement, not just native goldens.
