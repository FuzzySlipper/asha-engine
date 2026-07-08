# Lane: rust-wasm-bridge

## Owns
- `engine-rs/crates/wasm/wasm-api` — the narrow replay/golden WASM authority boundary

## May depend on
All Rust workspace crates below it (foundation, state, protocol, sim, services, rules, render).
`wasm-bindgen` and related WASM tooling crates.

## Must never touch
- Product-domain logic, renderer behavior, or TypeScript policy decisions.
- Runtime transport exports such as init, tick, command submission, render-diff retrieval,
  telemetry retrieval, or raw memory views.
- Tool crates (`replay-tool`, etc.) — those are native-only binaries.

## Required tests
- Native Rust tests for the replay divergence exports.
- WASM build must succeed when the opt-in WASM gate is run:
  `cargo build --target wasm32-unknown-unknown -p wasm-api --release`.
- `harness/ci/check-wasm-replay.sh` must build `wasm-api`, run `wasm-bindgen`, and run
  `@asha/wasm-replay-bridge` tests with the real WASM authority module present.

## Required fixtures
- Replay artifacts covering identical records, hash/checkpoint divergence, and malformed
  artifacts. These may be small inline fixtures or committed replay goldens.

## Drift smells reviewers should flag
- Product-domain logic accumulating in the `wasm-api` public export module.
- Renderer decisions or policy execution appearing in WASM exports.
- Memory view helpers leaking raw pointers into structured protocol messages.
- WASM API surface growing beyond replay decode/diff/classification.
- Skipped `@asha/wasm-replay-bridge` authority tests being described as WASM coverage.

## Public API changes that require escalation
All public export changes require escalation because `@asha/wasm-replay-bridge` is typed
against this surface. Any change requires a `contract-steward` review and downstream TS
typecheck. The current public exports are `classify_divergence` and
`divergence_class_labels`; adding runtime transport exports is out of lane.
