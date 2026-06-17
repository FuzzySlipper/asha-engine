//! native-bridge — `napi-rs` runtime transport addon (ADR 0006).
//!
//! This is the ONLY crate that depends on `napi`. It wraps the hand-written
//! semantic bodies behind the manifest's bounded verbs and exposes them as
//! `#[napi]` functions for `@asha/native-bridge` to load. App/UI/renderer never
//! import it directly — only `@asha/runtime-bridge` does.
//!
//! Status (#2570): stateful native conformance surface for the ASHA demo authority
//! proof. Each exported verb is still bounded and explicit; there is no generic
//! method-name/JSON dispatch.
//!
//! NOTE: this crate is excluded from the offline workspace build; it requires a
//! native toolchain + `@napi-rs/cli`. See Cargo.toml.

use std::collections::BTreeMap;
use std::sync::{Mutex, OnceLock};

use napi_derive::napi;
use runtime_bridge_api::{
    set_voxel_command, CommandBatch, EngineConfig, ReferenceBridge, RuntimeBridge,
    RuntimeBridgeError, RuntimeBridgeErrorKind, StepInputEnvelope, WorldLoadRequest,
};
use serde::Deserialize;

#[derive(Debug, Default)]
struct NativeSessions {
    next_handle: u64,
    bridges: BTreeMap<u64, ReferenceBridge>,
}

static SESSIONS: OnceLock<Mutex<NativeSessions>> = OnceLock::new();

fn sessions() -> &'static Mutex<NativeSessions> {
    SESSIONS.get_or_init(|| {
        Mutex::new(NativeSessions {
            next_handle: 1,
            bridges: BTreeMap::new(),
        })
    })
}

/// Mirror of the typed boundary error, classified rather than a raw string.
fn to_napi(err: RuntimeBridgeError) -> napi::Error {
    // The `kind` is carried in the message so the TS facade can re-classify if it
    // needs to; no opaque JSON blob or panic crosses the boundary.
    napi::Error::new(
        napi::Status::GenericFailure,
        format!("{:?}: {}", err.kind, err.message),
    )
}

fn lock_sessions() -> napi::Result<std::sync::MutexGuard<'static, NativeSessions>> {
    sessions().lock().map_err(|_| {
        to_napi(RuntimeBridgeError::new(
            RuntimeBridgeErrorKind::Internal,
            "native bridge session table lock poisoned",
        ))
    })
}

fn with_bridge<T>(
    handle: i64,
    f: impl FnOnce(&mut ReferenceBridge) -> napi::Result<T>,
) -> napi::Result<T> {
    if handle <= 0 {
        return Err(to_napi(RuntimeBridgeError::new(
            RuntimeBridgeErrorKind::UnknownHandle,
            format!("unknown native bridge handle {handle}"),
        )));
    }
    let mut sessions = lock_sessions()?;
    let bridge = sessions.bridges.get_mut(&(handle as u64)).ok_or_else(|| {
        to_napi(RuntimeBridgeError::new(
            RuntimeBridgeErrorKind::UnknownHandle,
            format!("unknown native bridge handle {handle}"),
        ))
    })?;
    f(bridge)
}

#[napi(object)]
pub struct NativeCompositionStatus {
    pub loaded_world: Option<i64>,
    pub fatal_count: u32,
    pub total_count: u32,
    pub blocks_load: bool,
}

impl From<runtime_bridge_api::CompositionStatus> for NativeCompositionStatus {
    fn from(value: runtime_bridge_api::CompositionStatus) -> Self {
        Self {
            loaded_world: value.loaded_world.map(|v| v as i64),
            fatal_count: value.fatal_count,
            total_count: value.total_count,
            blocks_load: value.blocks_load,
        }
    }
}

#[napi(object)]
pub struct NativeCommandResult {
    pub accepted: u32,
    pub rejected: u32,
    pub rejections: Vec<String>,
}

impl From<runtime_bridge_api::CommandResult> for NativeCommandResult {
    fn from(value: runtime_bridge_api::CommandResult) -> Self {
        Self {
            accepted: value.accepted,
            rejected: value.rejected,
            rejections: value
                .rejections
                .into_iter()
                .map(|r| format!("{r:?}"))
                .collect(),
        }
    }
}

#[napi(object)]
pub struct NativeRenderFrameDiff {
    pub ops: Vec<String>,
}

#[napi(object)]
pub struct NativeWorldSaveSummary {
    pub artifacts_written: u32,
    pub compacted_edits: u32,
    pub retained_edits: u32,
}

impl From<runtime_bridge_api::WorldSaveSummary> for NativeWorldSaveSummary {
    fn from(value: runtime_bridge_api::WorldSaveSummary) -> Self {
        Self {
            artifacts_written: value.artifacts_written,
            compacted_edits: value.compacted_edits,
            retained_edits: value.retained_edits,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", tag = "op")]
enum NativeCommandInput {
    SetVoxel {
        grid: u32,
        coord: NativeCoord,
        value: NativeVoxelValue,
    },
}

#[derive(Debug, Deserialize)]
struct NativeCoord {
    x: i64,
    y: i64,
    z: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
enum NativeVoxelValue {
    Solid { material: u16 },
}

fn parse_commands(commands_json: &str) -> napi::Result<CommandBatch> {
    let inputs: Vec<NativeCommandInput> = serde_json::from_str(commands_json).map_err(|err| {
        to_napi(RuntimeBridgeError::new(
            RuntimeBridgeErrorKind::InvalidInput,
            format!("invalid command batch JSON: {err}"),
        ))
    })?;
    let commands = inputs
        .into_iter()
        .map(|input| match input {
            NativeCommandInput::SetVoxel { grid, coord, value } => match value {
                NativeVoxelValue::Solid { material } => {
                    set_voxel_command(grid, coord.x, coord.y, coord.z, material)
                }
            },
        })
        .collect();
    Ok(CommandBatch { commands })
}

/// Construct a stateful native reference bridge from a deterministic seed and
/// return the opaque handle used by subsequent native operations.
#[napi]
pub fn initialize_engine(seed: i64) -> napi::Result<i64> {
    if seed < 0 {
        return Err(to_napi(RuntimeBridgeError::new(
            RuntimeBridgeErrorKind::InvalidInput,
            "seed must be non-negative",
        )));
    }
    let mut bridge = ReferenceBridge::new();
    bridge
        .initialize_engine(EngineConfig { seed: seed as u64 })
        .map_err(to_napi)?;

    let mut sessions = lock_sessions()?;
    let handle = sessions.next_handle;
    sessions.next_handle = sessions.next_handle.checked_add(1).ok_or_else(|| {
        to_napi(RuntimeBridgeError::new(
            RuntimeBridgeErrorKind::Internal,
            "native bridge handle counter overflowed",
        ))
    })?;
    sessions.bridges.insert(handle, bridge);
    Ok(handle as i64)
}

#[napi]
pub fn load_world_bundle(
    handle: i64,
    bundle_schema_version: i64,
    protocol_version: i64,
    scene_id: i64,
) -> napi::Result<NativeCompositionStatus> {
    with_bridge(handle, |bridge| {
        bridge
            .load_world_bundle(WorldLoadRequest {
                bundle_schema_version: bundle_schema_version as u32,
                protocol_version: protocol_version as u32,
                scene_id: scene_id as u64,
            })
            .map(NativeCompositionStatus::from)
            .map_err(to_napi)
    })
}

#[napi]
pub fn submit_commands(handle: i64, commands_json: String) -> napi::Result<NativeCommandResult> {
    let batch = parse_commands(&commands_json)?;
    with_bridge(handle, |bridge| {
        bridge
            .submit_commands(batch)
            .map(NativeCommandResult::from)
            .map_err(to_napi)
    })
}

#[napi]
pub fn step_simulation(handle: i64, tick: i64) -> napi::Result<u32> {
    with_bridge(handle, |bridge| {
        bridge
            .step_simulation(StepInputEnvelope { tick: tick as u64 })
            .map(|result| result.diff_count)
            .map_err(to_napi)
    })
}

#[napi]
pub fn read_render_diffs(handle: i64, _cursor: i64) -> napi::Result<NativeRenderFrameDiff> {
    with_bridge(handle, |_bridge| {
        Ok(NativeRenderFrameDiff { ops: Vec::new() })
    })
}

#[napi]
pub fn save_current_world(handle: i64) -> napi::Result<NativeWorldSaveSummary> {
    with_bridge(handle, |bridge| {
        bridge
            .save_current_world()
            .map(NativeWorldSaveSummary::from)
            .map_err(to_napi)
    })
}

#[napi]
pub fn get_composition_status(handle: i64) -> napi::Result<NativeCompositionStatus> {
    with_bridge(handle, |bridge| {
        bridge
            .get_composition_status()
            .map(NativeCompositionStatus::from)
            .map_err(to_napi)
    })
}
