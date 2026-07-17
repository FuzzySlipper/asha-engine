---
status: current
audience: agent
tags: [adr, bridge, boundary]
supersedes: []
see-also: []
---

# ADR 0006 — Runtime bridge boundary (napi-rs runtime, WASM replay)

**Status:** Accepted

The Electron runtime executes the authoritative Rust core natively via `napi-rs`.
WASM remains the canonical replay/golden verification target, not the runtime transport.

Neither transport is a public interface. App / UI / renderer / devtools depend only on a narrow,
transport-agnostic facade (`@asha/runtime-bridge`). Only the facade imports the native addon
(`@asha/native-bridge`); the WASM path (`@asha/wasm-replay-bridge`) is imported by
tests/devtools only. Generated contracts remain the semantic/governance border; native and WASM
are transport glue underneath it.

No `callRust(methodName, json)` dispatch, no raw JSON escape hatch, no `StateStore` handles
exposed to TS. Bindings follow the middle path: curated manifest + generated boring glue +
hand-written semantic operation bodies.

Full shape, dependency graph, and migration plan: `docs/runtime-bridge-boundary.md`.
Source decision note: Den doc `runtime-boundary-napi-wasm-replay-strategy`.
