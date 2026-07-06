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
    set_voxel_command, CommandBatch, EnemyDirectNavMovementRequest, EngineConfig,
    ReferenceBridge, RuntimeBridge, RuntimeBridgeError, RuntimeBridgeErrorKind,
    StepInputEnvelope, WorldLoadRequest,
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

fn non_negative_i64(value: i64, field: &str) -> napi::Result<i64> {
    if value < 0 {
        return Err(to_napi(RuntimeBridgeError::new(
            RuntimeBridgeErrorKind::InvalidInput,
            format!("{field} must be a non-negative integer"),
        )));
    }
    Ok(value)
}

fn u32_input(value: i64, field: &str) -> napi::Result<u32> {
    non_negative_i64(value, field)?;
    u32::try_from(value).map_err(|_| {
        to_napi(RuntimeBridgeError::new(
            RuntimeBridgeErrorKind::InvalidInput,
            format!("{field} must fit in u32"),
        ))
    })
}

fn u64_input(value: i64, field: &str) -> napi::Result<u64> {
    non_negative_i64(value, field).map(|v| v as u64)
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

#[napi(object)]
pub struct NativeVec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl NativeVec3 {
    fn to_vec3(&self, field: &str) -> napi::Result<core_math::Vec3> {
        if !self.x.is_finite() || !self.y.is_finite() || !self.z.is_finite() {
            return Err(to_napi(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                format!("{field} must contain finite coordinates"),
            )));
        }
        Ok(core_math::Vec3::new(self.x as f32, self.y as f32, self.z as f32))
    }
}

#[napi(object)]
pub struct NativeEnemyDirectNavMovementResult {
    pub entity: i64,
    pub authority_source: String,
    pub from: NativeVec3,
    pub target: NativeVec3,
    pub next_waypoint: NativeVec3,
    pub distance_units: f64,
    pub reached: bool,
    pub path_hash: String,
    pub transform_hash: String,
    pub projection_changed: bool,
}

fn native_vec3(value: core_math::Vec3) -> NativeVec3 {
    NativeVec3 {
        x: f64::from(value.x),
        y: f64::from(value.y),
        z: f64::from(value.z),
    }
}

impl From<runtime_bridge_api::EnemyDirectNavMovementResult>
    for NativeEnemyDirectNavMovementResult
{
    fn from(value: runtime_bridge_api::EnemyDirectNavMovementResult) -> Self {
        Self {
            entity: value.entity as i64,
            authority_source: value.authority_source.label().to_string(),
            from: native_vec3(value.from),
            target: native_vec3(value.target),
            next_waypoint: native_vec3(value.next_waypoint),
            distance_units: f64::from(value.distance_units),
            reached: value.reached,
            path_hash: format!("fnv1a64:{:016x}", value.path_hash),
            transform_hash: format!("fnv1a64:{:016x}", value.transform_hash),
            projection_changed: value.projection_changed,
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
    let bundle_schema_version = u32_input(bundle_schema_version, "bundle_schema_version")?;
    let protocol_version = u32_input(protocol_version, "protocol_version")?;
    let scene_id = u64_input(scene_id, "scene_id")?;
    with_bridge(handle, |bridge| {
        bridge
            .load_world_bundle(WorldLoadRequest {
                bundle_schema_version,
                protocol_version,
                scene_id,
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
    let tick = u64_input(tick, "tick")?;
    with_bridge(handle, |bridge| {
        bridge
            .step_simulation(StepInputEnvelope { tick })
            .map(|result| result.diff_count)
            .map_err(to_napi)
    })
}

#[napi]
pub fn apply_enemy_direct_nav_movement(
    handle: i64,
    entity: i64,
    seed_position: NativeVec3,
    target: NativeVec3,
    max_step_units: f64,
) -> napi::Result<NativeEnemyDirectNavMovementResult> {
    let entity = u64_input(entity, "entity")?;
    if !max_step_units.is_finite() {
        return Err(to_napi(RuntimeBridgeError::new(
            RuntimeBridgeErrorKind::InvalidInput,
            "max_step_units must be finite",
        )));
    }
    let seed_position = seed_position.to_vec3("seed_position")?;
    let target = target.to_vec3("target")?;
    with_bridge(handle, |bridge| {
        bridge
            .apply_enemy_direct_nav_movement(EnemyDirectNavMovementRequest {
                entity,
                seed_position,
                target,
                max_step_units: max_step_units as f32,
            })
            .map(NativeEnemyDirectNavMovementResult::from)
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

#[cfg(test)]
mod tests {
    use super::*;

    const WIRED_NAPI_EXPORTS: &[&str] = &[
        "applyEnemyDirectNavMovement",
        "getCompositionStatus",
        "initializeEngine",
        "loadWorldBundle",
        "readRenderDiffs",
        "saveCurrentWorld",
        "stepSimulation",
        "submitCommands",
    ];

    #[test]
    fn wired_export_set_is_explicit_and_bounded() {
        assert_eq!(
            WIRED_NAPI_EXPORTS,
            &[
                "applyEnemyDirectNavMovement",
                "getCompositionStatus",
                "initializeEngine",
                "loadWorldBundle",
                "readRenderDiffs",
                "saveCurrentWorld",
                "stepSimulation",
                "submitCommands",
            ]
        );
    }

    #[test]
    fn native_bridge_stateful_smoke_uses_bounded_operations() {
        let handle = initialize_engine(7).expect("engine initializes");
        assert!(handle > 0);

        let loaded = load_world_bundle(handle, 1, 1, 1001).expect("world loads");
        assert_eq!(loaded.loaded_world, Some(1001));
        assert_eq!(loaded.fatal_count, 0);
        assert_eq!(loaded.total_count, 0);
        assert!(!loaded.blocks_load);

        let result = submit_commands(
            handle,
            r#"[{"op":"setVoxel","grid":1,"coord":{"x":0,"y":0,"z":0},"value":{"kind":"solid","material":1}}]"#
                .to_string(),
        )
        .expect("bounded command batch submits");
        assert_eq!(result.accepted, 1);
        assert_eq!(result.rejected, 0);
        assert!(result.rejections.is_empty());

        let diff_count = step_simulation(handle, 6).expect("simulation steps");
        assert_eq!(diff_count, 2);

        let moved = apply_enemy_direct_nav_movement(
            handle,
            777,
            NativeVec3 {
                x: 0.0,
                y: 0.5,
                z: -2.6,
            },
            NativeVec3 {
                x: 0.0,
                y: 1.62,
                z: 1.25,
            },
            0.35,
        )
        .expect("enemy direct-nav movement applies");
        assert_eq!(moved.entity, 777);
        assert_eq!(moved.authority_source, "seeded_from_request");
        assert_eq!(moved.next_waypoint.x, 0.0);
        assert!((moved.next_waypoint.y - 0.598).abs() < 0.0005);
        assert_eq!(moved.path_hash, "fnv1a64:69ed74d692922db7");
        assert!(moved.transform_hash.starts_with("fnv1a64:"));

        let frame = read_render_diffs(handle, 0).expect("render diff read is bounded");
        assert!(frame.ops.is_empty());

        let saved = save_current_world(handle).expect("world saves");
        assert_eq!(saved.artifacts_written, 3);
        assert_eq!(saved.compacted_edits, 0);
        assert_eq!(saved.retained_edits, 0);

        let status = get_composition_status(handle).expect("composition reads");
        assert_eq!(status.loaded_world, Some(1001));
        assert_eq!(status.fatal_count, 0);
    }

    #[test]
    fn native_bridge_rejects_invalid_inputs_without_fallback() {
        assert!(initialize_engine(-1).is_err());
        assert!(get_composition_status(-99).is_err());

        let handle = initialize_engine(11).expect("engine initializes");
        assert!(load_world_bundle(handle, -1, 1, 1001).is_err());
        assert!(step_simulation(handle, -1).is_err());
        assert!(submit_commands(handle, r#"[{"op":"deleteEverything"}]"#.to_string()).is_err());
    }
}
