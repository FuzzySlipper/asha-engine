# Determinism

## Why it matters

Agent-written code is hard to audit at runtime. Deterministic replay means any change
can be regression-tested against prior behavior without a human playing through scenarios.
A determinism violation is a governance failure, not just a bug.

## Canonical replay target

WASM semantics are the replay authority. If native and WASM produce different hashes
for the same event sequence, the WASM result is correct and the native path must be fixed
or the divergence must be explicitly classified and tested.

This separation is concrete in `@asha/wasm-replay-bridge` (ADR 0006): the WASM replay
path stays in that package (replay/golden/devtools only), while the runtime transport
lives behind `@asha/runtime-bridge`. `classifyDivergence(native, wasm)` is the typed
native-vs-WASM check — `match` / `hash_divergence` (with the first diverging step) /
`length_divergence` — with WASM treated as authoritative.

The actual compiled WASM authority is the `wasm-api` crate's replay-only export surface:
`classify_divergence(expected, actual)` and `divergence_class_labels()`. Build and run it with
`harness/ci/check-wasm-replay.sh`. If the module is absent, `@asha/wasm-replay-bridge` marks the
WASM-authority tests as skipped; that skip preserves ordinary offline CI but is not a substitute
for the opt-in WASM replay gate.

## Sources of non-determinism to eliminate

| Source | Rule |
|---|---|
| Wall-clock time | Forbidden in sim/policy/service paths. Use `core-time` tick counters only. |
| Ambient RNG | Forbidden. All randomness comes from `svc-rng` with explicit seeds. |
| Hash map iteration order | Use `IndexMap` or sorted structures in authoritative paths. |
| Float arithmetic differences | Use fixed-point or explicitly reproducible float ops. Avoid `f32`/`f64` accumulation in canonical paths. |
| Thread scheduling | Authoritative event application is single-threaded and sequential. |
| Network / filesystem | Forbidden in the simulation path. |
| DOM / browser APIs | Forbidden in policy. |

## How determinism is enforced

1. `svc-rng` is the only source of randomness; it takes an explicit seed.
2. `core-time` provides tick-based time; wall-clock is not exposed to authoritative code.
3. Replay golden tests run the same event sequence and assert the same state hash.
4. CI `check-replays.sh` runs golden replays on every PR.
5. The opt-in `check-wasm-replay.sh` gate builds the replay-only WASM module and runs the
   classified WASM authority tests when the wasm32/wasm-bindgen toolchain is available.
6. Policy sandbox lint forbids `Date` and `Math.random`.

## State hashing

`core-snapshot` produces a deterministic hash of `StateStore` at configurable intervals.
Hashes are recorded in replay files and checked during playback.
A hash mismatch during replay identifies the exact tick where divergence began.

## Adding a determinism-sensitive service

1. Accept a seed or RNG stream from `svc-rng` as an explicit parameter.
2. Write a deterministic fixture test: fixed seed + fixed inputs → fixed output.
3. Run the test on both native and WASM targets.
4. Add the fixture to `harness/fixtures/` and reference it in the service's tests.
