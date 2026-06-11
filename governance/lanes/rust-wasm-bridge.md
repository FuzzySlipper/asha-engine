# Lane: rust-wasm-bridge

## Owns
- `engine-rs/crates/wasm/wasm-api` — the narrow WASM host boundary

## May depend on
All Rust workspace crates below it (foundation, state, protocol, sim, services, rules, render).
`wasm-bindgen` and related WASM tooling crates.

## Must never touch
- Product-domain logic, renderer behavior, or TypeScript policy decisions.
- Tool crates (`replay-tool`, etc.) — those are native-only binaries.

## Required tests
- Initialization export smoke test (headless, no browser).
- Command submission round-trip test.
- Render diff retrieval test.
- WASM build must succeed: `cargo build --target wasm32-unknown-unknown -p wasm-api`.

## Required fixtures
- Minimal init + one tick fixture to confirm the export surface doesn't break.

## Drift smells reviewers should flag
- Product-domain logic accumulating in `exports.rs`.
- Renderer decisions or policy execution appearing in WASM exports.
- Memory view helpers leaking raw pointers into structured protocol messages.
- WASM API surface growing beyond: init, tick, command submit, replay hooks, diff retrieval, telemetry retrieval.

## Public API changes that require escalation
All public export changes require escalation — the TypeScript `wasm-replay-bridge` package
is generated/typed against this surface. Any change requires a `contract-steward` review
and downstream TS typecheck.
