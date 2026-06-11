# native-bridge — packaging notes & blockers (task #2250)

The napi-rs runtime transport addon. Built from this crate into a
`native-bridge.<platform>.node` that `@asha/native-bridge` loads.

## What works now (offline, verified)

- `runtime-bridge-api` (workspace member): boundary types + `RuntimeBridge` trait +
  `ReferenceBridge` reference body. `cargo test -p runtime-bridge-api` → 3 passing.
- `@asha/runtime-bridge` facade + `MockRuntimeBridge` + native factory:
  `node --test` → 6 passing (conformance + mock smoke + native-unavailable classification).
- Dependency guard: only `@asha/runtime-bridge` imports `@asha/native-bridge`
  (`check-depgraph` green).
- The `#[napi]` smoke ops (`initialize_engine`, `step_simulation`) are written and
  delegate to the reference body.

## Blocker: native addon build

`cargo build -p native-bridge` / `napi build` cannot run in this environment:

1. **No `wasm32`/native napi toolchain wired**: `@napi-rs/cli` is not installed
   (`ts/node_modules/.bin` has no `napi`).
2. **Network-gated crates**: `napi`, `napi-derive`, `napi-build` are not vendored;
   fetching them needs registry access. The crate is therefore **excluded** from the
   workspace (`engine-rs/Cargo.toml` `workspace.exclude`) so offline CI stays green.
3. **Per-platform artifacts**: a real runtime needs prebuilt `.node` binaries per
   `{platform, arch, Electron ABI}`; CI cross-build + artifact publishing must be
   designed before this becomes a runtime dependency (ADR 0006 risk #2).

## Next steps to unblock (follow-up task)

1. Add `@napi-rs/cli` to the `ts` toolchain; add `napi`/`napi-derive`/`napi-build`
   (vendored or registry-available).
2. Add `native-bridge` to the workspace (or a dedicated `--features native` build job).
3. `napi build --platform --release`; copy the `.node` next to `@asha/native-bridge/dist`.
4. Add the native addon smoke test (manifest checklist #5): call every `#[napi]` export
   once with a tiny fixture; assert it matches `ReferenceBridge` / `MockRuntimeBridge`.
5. Implement the codegen emitter so the `#[napi]` exports + facade skeleton + conformance
   JSON are generated from `bridge-manifest.toml` (one-in/one-out) rather than hand-written.
