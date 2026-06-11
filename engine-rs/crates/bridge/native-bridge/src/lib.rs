//! native-bridge — `napi-rs` runtime transport addon (ADR 0006).
//!
//! This is the ONLY crate that depends on `napi`. It wraps the hand-written
//! semantic bodies behind the manifest's bounded verbs and exposes them as
//! `#[napi]` functions for `@asha/native-bridge` to load. App/UI/renderer never
//! import it directly — only `@asha/runtime-bridge` does.
//!
//! Status (#2250): minimal smoke surface — `initialize_engine` + `step_simulation`
//! from the manifest, delegating to `runtime-bridge-api`'s reference body. The
//! generated `#[napi]` export wrappers (one-in/one-out, per `bridge-manifest.toml`)
//! replace these hand-written stubs once the codegen emitter lands.
//!
//! NOTE: this crate is excluded from the offline workspace build; it requires a
//! native toolchain + `@napi-rs/cli`. See Cargo.toml.

use napi_derive::napi;
use runtime_bridge_api::{
    EngineConfig, ReferenceBridge, RuntimeBridge, RuntimeBridgeError, StepInputEnvelope,
};

/// Mirror of the typed boundary error, classified rather than a raw string.
fn to_napi(err: RuntimeBridgeError) -> napi::Error {
    // The `kind` is carried as the status string so the TS facade can re-classify
    // into `RuntimeBridgeError` — no opaque JSON blob.
    napi::Error::new(napi::Status::GenericFailure, format!("{:?}: {}", err.kind, err.message))
}

/// Smoke operation: construct an engine from a deterministic seed, return its
/// opaque handle id. Mirrors manifest `initialize_engine`.
#[napi]
pub fn initialize_engine(seed: i64) -> napi::Result<i64> {
    let mut bridge = ReferenceBridge::new();
    let handle = bridge
        .initialize_engine(EngineConfig { seed: seed as u64 })
        .map_err(to_napi)?;
    Ok(handle.raw() as i64)
}

/// Smoke operation: advance one tick, return the diff count. Mirrors manifest
/// `step_simulation`. (Stateless smoke variant: re-inits per call until the real
/// session-handle plumbing lands.)
#[napi]
pub fn step_simulation(seed: i64, tick: i64) -> napi::Result<u32> {
    let mut bridge = ReferenceBridge::new();
    bridge
        .initialize_engine(EngineConfig { seed: seed as u64 })
        .map_err(to_napi)?;
    let result = bridge
        .step_simulation(StepInputEnvelope { tick: tick as u64 })
        .map_err(to_napi)?;
    Ok(result.diff_count)
}
