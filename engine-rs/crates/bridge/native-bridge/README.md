# native-bridge — packaging notes (task #2250)

The napi-rs runtime transport addon. Built from this crate (standalone workspace,
excluded from the engine-rs workspace) into a cdylib loaded by `@asha/native-bridge`
as `native-bridge.node`.

## Build & verify

```bash
harness/ci/check-native.sh        # cargo build --release + install .node + smoke + facade tests
```

This builds the cdylib, installs it to `ts/packages/native-bridge/dist/native-bridge.node`
(gitignored — platform-specific), calls every `#[napi]` export with a tiny fixture, and
runs the `@asha/runtime-bridge` facade tests (the native-parity test then runs instead of
skipping). Verified: `initializeEngine(7)=7`, `stepSimulation(7,6)=2` — exact parity with
`ReferenceBridge` / `MockRuntimeBridge`.

The crate is kept **excluded** from the engine-rs workspace so the default offline
build/CI doesn't require the napi crates / native toolchain; `check-native.sh` is opt-in.

## Remaining follow-ups (not blockers)

1. **Per-platform/Electron-ABI artifacts**: ship prebuilt `.node` per `{platform, arch,
   Electron ABI}` via `@napi-rs/cli` (`napi build --platform`) + a CI cross-build/publish
   job before this is a hard runtime dependency (ADR 0006 risk #2). `check-native.sh`
   currently produces only the host artifact.
2. **Codegen emitter**: generate the `#[napi]` export signatures + facade skeleton +
   conformance JSON from `bridge-manifest.toml` (one-in/one-out) rather than hand-writing
   them. See `harness/codegen/bridge-emit.*`.
