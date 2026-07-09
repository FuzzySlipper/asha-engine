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
use protocol_view::{
    CameraCollisionEvidence, CameraCollisionPolicy, CameraCollisionPolicyMode,
    CameraCollisionShape, CameraCollisionSnapshot, CameraHandle, CollisionAabbEvidence,
    CollisionAxis, CollisionConstrainedCameraInputEnvelope, FirstPersonCameraInput,
};
use runtime_bridge_api::{
    set_voxel_command, CameraCreateRequest, CameraPose, CommandBatch, EnemyDirectNavMovementRequest,
    EngineConfig, FpsBridgeBoundsCapability, FpsBridgeHealth,
    FpsBridgePolicyBinding, FpsBridgeRole, FpsBridgeStoredEntityDefinition,
    FpsBridgeTransformCapability, FpsBridgeWeaponMount, FpsEncounterDirectorSnapshot,
    FpsEncounterLifecycleInput, FpsEncounterStateReadout, FpsEncounterTransitionRequest,
    FpsEncounterTransitionResult, FpsPrimaryFireRequest, FpsPrimaryFireResult,
    FpsRuntimeSessionLoadRequest, FpsRuntimeSessionRestartRequest, FpsRuntimeSessionSnapshot,
    GameExtensionWeaponEffectInvocationRequest, GameRuleCatalog, GameRuleEffectIntentRequest,
    GameRuleModuleManifest, GameRuleResolutionRequest, ProjectBundleLoadRequest, ReferenceBridge,
    RuntimeBridge, RuntimeBridgeError, RuntimeBridgeErrorKind, StepInputEnvelope,
    VoxelAnnotationEditRequest, VoxelAnnotationLayerExportRequest, VoxelAnnotationLayerLoadRequest,
    VoxelAnnotationLayerValidationRequest, VoxelAnnotationQueryRequest, VoxelConversionApplyRequest, VoxelConversionEvidenceRef,
    VoxelConversionMeshAssetRegistrationRequest, VoxelConversionPlanRequest,
    VoxelConversionPreviewRequest, VoxelConversionSourceRegistrationRequest, VoxelModelInfoRequest,
    VoxelModelWindowRequest, VoxelVolumeAssetExportRequest, VoxelVolumeAssetLoadRequest,
    VoxelVolumeAssetSaveRequest, WeaponEffectHookRequest,
};
use serde::{Deserialize, Serialize};

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
    pub loaded_project_bundle: Option<i64>,
    pub fatal_count: u32,
    pub total_count: u32,
    pub blocks_load: bool,
}

impl From<runtime_bridge_api::CompositionStatus> for NativeCompositionStatus {
    fn from(value: runtime_bridge_api::CompositionStatus) -> Self {
        Self {
            loaded_project_bundle: value.loaded_project_bundle.map(|v| v as i64),
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
pub struct NativeProjectBundleSaveSummary {
    pub artifacts_written: u32,
    pub compacted_edits: u32,
    pub retained_edits: u32,
}

impl From<runtime_bridge_api::ProjectBundleSaveSummary> for NativeProjectBundleSaveSummary {
    fn from(value: runtime_bridge_api::ProjectBundleSaveSummary) -> Self {
        Self {
            artifacts_written: value.artifacts_written,
            compacted_edits: value.compacted_edits,
            retained_edits: value.retained_edits,
        }
    }
}

#[napi(object)]
pub struct NativeCameraPose {
    pub position: Vec<f64>,
    pub yaw_degrees: f64,
    pub pitch_degrees: f64,
}

impl NativeCameraPose {
    fn into_bridge(self, field: &str) -> napi::Result<CameraPose> {
        if self.position.len() != 3 || self.position.iter().any(|value| !value.is_finite()) {
            return Err(to_napi(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                format!("{field}.position must contain exactly three finite coordinates"),
            )));
        }
        if !self.yaw_degrees.is_finite() || !self.pitch_degrees.is_finite() {
            return Err(to_napi(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                format!("{field} yaw/pitch must be finite"),
            )));
        }
        Ok(CameraPose {
            position: [
                self.position[0] as f32,
                self.position[1] as f32,
                self.position[2] as f32,
            ],
            yaw_degrees: self.yaw_degrees as f32,
            pitch_degrees: self.pitch_degrees as f32,
        })
    }
}

impl From<runtime_bridge_api::CameraPose> for NativeCameraPose {
    fn from(value: runtime_bridge_api::CameraPose) -> Self {
        Self {
            position: value.position.into_iter().map(f64::from).collect(),
            yaw_degrees: f64::from(value.yaw_degrees),
            pitch_degrees: f64::from(value.pitch_degrees),
        }
    }
}

#[napi(object)]
pub struct NativeCameraBasis {
    pub forward: Vec<f64>,
    pub right: Vec<f64>,
    pub up: Vec<f64>,
}

impl From<runtime_bridge_api::CameraBasis> for NativeCameraBasis {
    fn from(value: runtime_bridge_api::CameraBasis) -> Self {
        Self {
            forward: value.forward.into_iter().map(f64::from).collect(),
            right: value.right.into_iter().map(f64::from).collect(),
            up: value.up.into_iter().map(f64::from).collect(),
        }
    }
}

#[napi(object)]
pub struct NativePerspectiveProjection {
    pub fov_y_degrees: f64,
    pub near: f64,
    pub far: f64,
}

impl NativePerspectiveProjection {
    fn into_bridge(self, field: &str) -> napi::Result<runtime_bridge_api::PerspectiveProjection> {
        if !self.fov_y_degrees.is_finite() || !self.near.is_finite() || !self.far.is_finite() {
            return Err(to_napi(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                format!("{field} must contain finite values"),
            )));
        }
        Ok(runtime_bridge_api::PerspectiveProjection {
            fov_y_degrees: self.fov_y_degrees as f32,
            near: self.near as f32,
            far: self.far as f32,
        })
    }
}

impl From<runtime_bridge_api::PerspectiveProjection> for NativePerspectiveProjection {
    fn from(value: runtime_bridge_api::PerspectiveProjection) -> Self {
        Self {
            fov_y_degrees: f64::from(value.fov_y_degrees),
            near: f64::from(value.near),
            far: f64::from(value.far),
        }
    }
}

#[napi(object)]
pub struct NativeViewportSize {
    pub width: u32,
    pub height: u32,
}

impl From<NativeViewportSize> for runtime_bridge_api::ViewportSize {
    fn from(value: NativeViewportSize) -> Self {
        runtime_bridge_api::ViewportSize {
            width: value.width,
            height: value.height,
        }
    }
}

impl From<runtime_bridge_api::ViewportSize> for NativeViewportSize {
    fn from(value: runtime_bridge_api::ViewportSize) -> Self {
        Self {
            width: value.width,
            height: value.height,
        }
    }
}

#[napi(object)]
pub struct NativeCameraCreateRequest {
    pub initial_pose: NativeCameraPose,
    pub projection: NativePerspectiveProjection,
    pub viewport: NativeViewportSize,
}

impl NativeCameraCreateRequest {
    fn into_bridge(self) -> napi::Result<CameraCreateRequest> {
        Ok(CameraCreateRequest {
            initial_pose: self.initial_pose.into_bridge("initial_pose")?,
            projection: self.projection.into_bridge("projection")?,
            viewport: self.viewport.into(),
        })
    }
}

#[napi(object)]
pub struct NativeCameraSnapshot {
    pub camera: i64,
    pub tick: i64,
    pub pose: NativeCameraPose,
    pub basis: NativeCameraBasis,
    pub projection: NativePerspectiveProjection,
    pub viewport: NativeViewportSize,
}

impl From<runtime_bridge_api::CameraSnapshot> for NativeCameraSnapshot {
    fn from(value: runtime_bridge_api::CameraSnapshot) -> Self {
        Self {
            camera: value.camera.raw() as i64,
            tick: value.tick as i64,
            pose: value.pose.into(),
            basis: value.basis.into(),
            projection: value.projection.into(),
            viewport: value.viewport.into(),
        }
    }
}

#[napi(object)]
pub struct NativeFirstPersonCameraInput {
    pub move_forward: f64,
    pub move_right: f64,
    pub move_up: f64,
    pub yaw_delta_degrees: f64,
    pub pitch_delta_degrees: f64,
    pub dt_seconds: f64,
    pub move_speed_units_per_second: f64,
}

impl NativeFirstPersonCameraInput {
    fn into_bridge(self, field: &str) -> napi::Result<FirstPersonCameraInput> {
        let values = [
            self.move_forward,
            self.move_right,
            self.move_up,
            self.yaw_delta_degrees,
            self.pitch_delta_degrees,
            self.dt_seconds,
            self.move_speed_units_per_second,
        ];
        if values.iter().any(|value| !value.is_finite()) {
            return Err(to_napi(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                format!("{field} must contain finite values"),
            )));
        }
        Ok(FirstPersonCameraInput {
            move_forward: self.move_forward as f32,
            move_right: self.move_right as f32,
            move_up: self.move_up as f32,
            yaw_delta_degrees: self.yaw_delta_degrees as f32,
            pitch_delta_degrees: self.pitch_delta_degrees as f32,
            dt_seconds: self.dt_seconds as f32,
            move_speed_units_per_second: self.move_speed_units_per_second as f32,
        })
    }
}

#[napi(object)]
pub struct NativeCameraCollisionShape {
    pub half_extents: Vec<f64>,
}

impl NativeCameraCollisionShape {
    fn into_bridge(self, field: &str) -> napi::Result<CameraCollisionShape> {
        let half_extents = native_f32x3(self.half_extents, &format!("{field}.half_extents"))?;
        Ok(CameraCollisionShape { half_extents })
    }
}

impl From<CameraCollisionShape> for NativeCameraCollisionShape {
    fn from(value: CameraCollisionShape) -> Self {
        Self {
            half_extents: value.half_extents.into_iter().map(f64::from).collect(),
        }
    }
}

#[napi(object)]
pub struct NativeCameraCollisionPolicy {
    pub mode: String,
    pub max_iterations: u32,
}

impl NativeCameraCollisionPolicy {
    fn into_bridge(self, field: &str) -> napi::Result<CameraCollisionPolicy> {
        let mode = match self.mode.as_str() {
            "axis_separable_slide" => CameraCollisionPolicyMode::AxisSeparableSlide,
            other => {
                return Err(to_napi(RuntimeBridgeError::new(
                    RuntimeBridgeErrorKind::InvalidInput,
                    format!("{field}.mode {other:?} is not supported"),
                )));
            }
        };
        let max_iterations = u8::try_from(self.max_iterations).map_err(|_| {
            to_napi(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                format!("{field}.max_iterations must fit in u8"),
            ))
        })?;
        Ok(CameraCollisionPolicy {
            mode,
            max_iterations,
        })
    }
}

impl From<CameraCollisionPolicy> for NativeCameraCollisionPolicy {
    fn from(value: CameraCollisionPolicy) -> Self {
        Self {
            mode: match value.mode {
                CameraCollisionPolicyMode::AxisSeparableSlide => {
                    "axis_separable_slide".to_string()
                }
            },
            max_iterations: u32::from(value.max_iterations),
        }
    }
}

#[napi(object)]
pub struct NativeCollisionConstrainedCameraInputEnvelope {
    pub camera: i64,
    pub grid: i64,
    pub input: NativeFirstPersonCameraInput,
    pub tick: i64,
    pub shape: NativeCameraCollisionShape,
    pub policy: NativeCameraCollisionPolicy,
}

impl NativeCollisionConstrainedCameraInputEnvelope {
    fn into_bridge(self) -> napi::Result<CollisionConstrainedCameraInputEnvelope> {
        Ok(CollisionConstrainedCameraInputEnvelope {
            camera: CameraHandle::new(u64_input(self.camera, "camera")?),
            grid: u64_input(self.grid, "grid")?,
            input: self.input.into_bridge("input")?,
            tick: u64_input(self.tick, "tick")?,
            shape: self.shape.into_bridge("shape")?,
            policy: self.policy.into_bridge("policy")?,
        })
    }
}

#[napi(object)]
pub struct NativeCollisionAabbEvidence {
    pub min: Vec<f64>,
    pub max: Vec<f64>,
}

impl From<CollisionAabbEvidence> for NativeCollisionAabbEvidence {
    fn from(value: CollisionAabbEvidence) -> Self {
        Self {
            min: value.min.into_iter().map(f64::from).collect(),
            max: value.max.into_iter().map(f64::from).collect(),
        }
    }
}

#[napi(object)]
pub struct NativeCameraCollisionEvidence {
    pub grid: i64,
    pub shape: NativeCameraCollisionShape,
    pub policy: NativeCameraCollisionPolicy,
    pub collided: bool,
    pub blocked_axes: Vec<String>,
    pub correction: Vec<f64>,
    pub queried_aabb: NativeCollisionAabbEvidence,
    pub collision_source_hash: String,
    pub collision_projection_hash: String,
}

impl From<CameraCollisionEvidence> for NativeCameraCollisionEvidence {
    fn from(value: CameraCollisionEvidence) -> Self {
        Self {
            grid: value.grid as i64,
            shape: value.shape.into(),
            policy: value.policy.into(),
            collided: value.collided,
            blocked_axes: value
                .blocked_axes
                .into_iter()
                .map(|axis| match axis {
                    CollisionAxis::X => "x".to_string(),
                    CollisionAxis::Y => "y".to_string(),
                    CollisionAxis::Z => "z".to_string(),
                })
                .collect(),
            correction: value.correction.into_iter().map(f64::from).collect(),
            queried_aabb: value.queried_aabb.into(),
            collision_source_hash: value.collision_source_hash,
            collision_projection_hash: value.collision_projection_hash,
        }
    }
}

#[napi(object)]
pub struct NativeCameraCollisionSnapshot {
    pub camera: i64,
    pub tick: i64,
    pub before: NativeCameraSnapshot,
    pub attempted: NativeCameraSnapshot,
    pub after: NativeCameraSnapshot,
    pub collision: NativeCameraCollisionEvidence,
    pub movement_hash: String,
}

impl From<CameraCollisionSnapshot> for NativeCameraCollisionSnapshot {
    fn from(value: CameraCollisionSnapshot) -> Self {
        Self {
            camera: value.camera.raw() as i64,
            tick: value.tick as i64,
            before: value.before.into(),
            attempted: value.attempted.into(),
            after: value.after.into(),
            collision: value.collision.into(),
            movement_hash: value.movement_hash,
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
        Ok(core_math::Vec3::new(
            self.x as f32,
            self.y as f32,
            self.z as f32,
        ))
    }
}

fn native_f32x3(values: Vec<f64>, field: &str) -> napi::Result<[f32; 3]> {
    if values.len() != 3 || values.iter().any(|value| !value.is_finite()) {
        return Err(to_napi(RuntimeBridgeError::new(
            RuntimeBridgeErrorKind::InvalidInput,
            format!("{field} must contain exactly three finite values"),
        )));
    }
    Ok([values[0] as f32, values[1] as f32, values[2] as f32])
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

impl From<runtime_bridge_api::EnemyDirectNavMovementResult> for NativeEnemyDirectNavMovementResult {
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

#[napi(object)]
pub struct NativeFpsTransformCapability {
    pub translation: NativeVec3,
    pub rotation: Vec<f64>,
    pub scale: NativeVec3,
}

#[napi(object)]
pub struct NativeFpsBoundsCapability {
    pub min: NativeVec3,
    pub max: NativeVec3,
}

#[napi(object)]
pub struct NativeFpsHealth {
    pub current: u32,
    pub max: u32,
}

#[napi(object)]
pub struct NativeFpsWeaponMount {
    pub weapon_id: String,
    pub damage: u32,
    pub range_units: u32,
    pub ammo: u32,
    pub cooldown_ticks_after_fire: u32,
}

#[napi(object)]
pub struct NativeFpsPolicyBinding {
    pub binding_id: String,
    pub policy_id: String,
    pub view_kind: String,
    pub view_version: String,
    pub allowed_intents: Vec<String>,
    pub runtime_moment: String,
}

#[napi(object)]
pub struct NativeFpsStoredEntityDefinition {
    pub entity: i64,
    pub stable_id: String,
    pub display_name: String,
    pub source_path: String,
    pub tags: Vec<String>,
    pub role: String,
    pub transform: Option<NativeFpsTransformCapability>,
    pub bounds: Option<NativeFpsBoundsCapability>,
    pub render_visible: Option<bool>,
    pub static_collider: Option<bool>,
    pub health: Option<NativeFpsHealth>,
    pub weapon: Option<NativeFpsWeaponMount>,
    pub policy_binding: Option<NativeFpsPolicyBinding>,
}

#[napi(object)]
pub struct NativeFpsLifecycleStatus {
    pub state: String,
    pub entity: Option<i64>,
    pub tick: Option<i64>,
}

#[napi(object)]
pub struct NativeFpsEntityHealthReadout {
    pub entity: i64,
    pub current: u32,
    pub max: u32,
}

#[napi(object)]
pub struct NativeFpsPolicyBindingReadout {
    pub entity: i64,
    pub binding_id: String,
    pub policy_id: String,
    pub view_kind: String,
    pub view_version: String,
    pub allowed_intents: Vec<String>,
    pub runtime_moment: String,
}

#[napi(object)]
pub struct NativeFpsReplayEvidence {
    pub replay_unit: String,
    pub entity_hash: String,
    pub health_hash: String,
    pub record_hash: String,
}

#[napi(object)]
pub struct NativeFpsReadSetEvidence {
    pub view_kind: String,
    pub owner: String,
    pub read_set: Vec<String>,
}

#[napi(object)]
pub struct NativeFpsRuntimeSessionSnapshot {
    pub backend: String,
    pub authority_surface: String,
    pub project_bundle: String,
    pub session_epoch: i64,
    pub lifecycle_status: NativeFpsLifecycleStatus,
    pub player_entity: i64,
    pub enemy_entity: i64,
    pub health: Vec<NativeFpsEntityHealthReadout>,
    pub policy_bindings: Vec<NativeFpsPolicyBindingReadout>,
    pub replay_records: Vec<NativeFpsReplayEvidence>,
    pub read_sets: Vec<NativeFpsReadSetEvidence>,
    pub entity_hash: String,
    pub health_hash: String,
    pub replay_hash: String,
}

#[napi(object)]
pub struct NativeFpsPrimaryFireResult {
    pub backend: String,
    pub authority_surface: String,
    pub mutation_owner: String,
    pub workspace_trace: Vec<String>,
    pub shooter: i64,
    pub target: Option<i64>,
    pub target_health_before: Option<NativeFpsHealth>,
    pub target_health_after: Option<NativeFpsHealth>,
    pub lifecycle_status: NativeFpsLifecycleStatus,
    pub target_render_visible: Option<bool>,
    pub entity_hash: String,
    pub health_hash: String,
    pub replay_hash: String,
}

#[napi(object)]
pub struct NativeGameExtensionWeaponEffectInvocationResult {
    pub hook_receipt_json: String,
    pub replay_evidence_json: String,
    pub primary_fire: Option<NativeFpsPrimaryFireResult>,
}

#[napi(object)]
pub struct NativeFpsEncounterLifecycleInput {
    pub outcome_kind: String,
    pub terminal: bool,
    pub enemy_dead: bool,
    pub player_dead: bool,
    pub lifecycle_hash: String,
}

#[napi(object)]
pub struct NativeFpsEncounterTransitionRequest {
    pub preset_id: String,
    pub action: String,
    pub lifecycle: NativeFpsEncounterLifecycleInput,
}

#[napi(object)]
pub struct NativeFpsEncounterStateReadout {
    pub preset_id: String,
    pub status: String,
    pub spawned_enemy_ids: Vec<String>,
    pub defeated_enemy_ids: Vec<String>,
    pub revision: i64,
    pub last_transition: String,
}

#[napi(object)]
pub struct NativeFpsEncounterDirectorSnapshot {
    pub backend: String,
    pub authority_surface: String,
    pub mutation_owner: String,
    pub workspace_trace: Vec<String>,
    pub state: NativeFpsEncounterStateReadout,
    pub lifecycle: NativeFpsEncounterLifecycleInput,
    pub read_sets: Vec<NativeFpsReadSetEvidence>,
    pub encounter_hash: String,
    pub replay_hash: String,
}

#[napi(object)]
pub struct NativeFpsEncounterTransitionResult {
    pub backend: String,
    pub authority_surface: String,
    pub mutation_owner: String,
    pub workspace_trace: Vec<String>,
    pub accepted: bool,
    pub rejection_reason: Option<String>,
    pub event_kind: Option<String>,
    pub state: NativeFpsEncounterStateReadout,
    pub lifecycle: NativeFpsEncounterLifecycleInput,
    pub encounter_hash: String,
    pub replay_hash: String,
}

fn native_hash(value: u64) -> String {
    format!("fnv1a64:{value:016x}")
}

fn native_fps_role(value: &str) -> napi::Result<FpsBridgeRole> {
    match value {
        "player" => Ok(FpsBridgeRole::Player),
        "enemy" => Ok(FpsBridgeRole::Enemy),
        "neutral" => Ok(FpsBridgeRole::Neutral),
        other => Err(to_napi(RuntimeBridgeError::new(
            RuntimeBridgeErrorKind::InvalidInput,
            format!("unknown FPS role '{other}'"),
        ))),
    }
}

fn optional_native_fps_role(
    value: Option<String>,
    field: &str,
) -> napi::Result<Option<FpsBridgeRole>> {
    match value {
        Some(role) => native_fps_role(role.as_str()).map(Some).map_err(|_| {
            to_napi(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                format!("{field} must be player, enemy, or neutral"),
            ))
        }),
        None => Ok(None),
    }
}

fn native_fps_lifecycle_status(
    value: runtime_bridge_api::FpsBridgeLifecycleStatus,
) -> NativeFpsLifecycleStatus {
    match value {
        runtime_bridge_api::FpsBridgeLifecycleStatus::Active => NativeFpsLifecycleStatus {
            state: "active".into(),
            entity: None,
            tick: None,
        },
        runtime_bridge_api::FpsBridgeLifecycleStatus::EnemyDefeated { entity, tick } => {
            NativeFpsLifecycleStatus {
                state: "enemy_defeated".into(),
                entity: Some(entity as i64),
                tick: Some(tick as i64),
            }
        }
    }
}

fn bridge_fps_transform(
    value: NativeFpsTransformCapability,
    field: &str,
) -> napi::Result<FpsBridgeTransformCapability> {
    if value.rotation.len() != 4 || value.rotation.iter().any(|v| !v.is_finite()) {
        return Err(to_napi(RuntimeBridgeError::new(
            RuntimeBridgeErrorKind::InvalidInput,
            format!("{field}.rotation must be a finite quaternion"),
        )));
    }
    let translation = value.translation.to_vec3(&format!("{field}.translation"))?;
    let scale = value.scale.to_vec3(&format!("{field}.scale"))?;
    Ok(FpsBridgeTransformCapability {
        translation: [translation.x, translation.y, translation.z],
        rotation: [
            value.rotation[0] as f32,
            value.rotation[1] as f32,
            value.rotation[2] as f32,
            value.rotation[3] as f32,
        ],
        scale: [scale.x, scale.y, scale.z],
    })
}

fn bridge_fps_bounds(
    value: NativeFpsBoundsCapability,
    field: &str,
) -> napi::Result<FpsBridgeBoundsCapability> {
    let min = value.min.to_vec3(&format!("{field}.min"))?;
    let max = value.max.to_vec3(&format!("{field}.max"))?;
    Ok(FpsBridgeBoundsCapability {
        min: [min.x, min.y, min.z],
        max: [max.x, max.y, max.z],
    })
}

fn bridge_fps_definitions(
    definitions: Vec<NativeFpsStoredEntityDefinition>,
) -> napi::Result<Vec<FpsBridgeStoredEntityDefinition>> {
    definitions
        .into_iter()
        .enumerate()
        .map(|(index, value)| {
            let field = format!("definitions[{index}]");
            Ok(FpsBridgeStoredEntityDefinition {
                entity: u64_input(value.entity, &format!("{field}.entity"))?,
                stable_id: value.stable_id,
                display_name: value.display_name,
                source_path: value.source_path,
                tags: value.tags,
                role: native_fps_role(&value.role)?,
                transform: value
                    .transform
                    .map(|transform| bridge_fps_transform(transform, &format!("{field}.transform")))
                    .transpose()?,
                bounds: value
                    .bounds
                    .map(|bounds| bridge_fps_bounds(bounds, &format!("{field}.bounds")))
                    .transpose()?,
                render_visible: value.render_visible,
                static_collider: value.static_collider,
                health: value.health.map(|health| FpsBridgeHealth {
                    current: health.current,
                    max: health.max,
                }),
                weapon: value.weapon.map(|weapon| FpsBridgeWeaponMount {
                    weapon_id: weapon.weapon_id,
                    damage: weapon.damage,
                    range_units: weapon.range_units,
                    ammo: weapon.ammo,
                    cooldown_ticks_after_fire: weapon.cooldown_ticks_after_fire,
                }),
                policy_binding: value.policy_binding.map(|binding| FpsBridgePolicyBinding {
                    binding_id: binding.binding_id,
                    policy_id: binding.policy_id,
                    view_kind: binding.view_kind,
                    view_version: binding.view_version,
                    allowed_intents: binding.allowed_intents,
                    runtime_moment: binding.runtime_moment,
                }),
            })
        })
        .collect()
}

impl From<FpsRuntimeSessionSnapshot> for NativeFpsRuntimeSessionSnapshot {
    fn from(value: FpsRuntimeSessionSnapshot) -> Self {
        Self {
            backend: value.backend,
            authority_surface: value.authority_surface,
            project_bundle: value.project_bundle,
            session_epoch: value.session_epoch as i64,
            lifecycle_status: native_fps_lifecycle_status(value.lifecycle_status),
            player_entity: value.player_entity as i64,
            enemy_entity: value.enemy_entity as i64,
            health: value
                .health
                .into_iter()
                .map(|health| NativeFpsEntityHealthReadout {
                    entity: health.entity as i64,
                    current: health.current,
                    max: health.max,
                })
                .collect(),
            policy_bindings: value
                .policy_bindings
                .into_iter()
                .map(|binding| NativeFpsPolicyBindingReadout {
                    entity: binding.entity as i64,
                    binding_id: binding.binding_id,
                    policy_id: binding.policy_id,
                    view_kind: binding.view_kind,
                    view_version: binding.view_version,
                    allowed_intents: binding.allowed_intents,
                    runtime_moment: binding.runtime_moment,
                })
                .collect(),
            replay_records: value
                .replay_records
                .into_iter()
                .map(|record| NativeFpsReplayEvidence {
                    replay_unit: record.replay_unit,
                    entity_hash: native_hash(record.entity_hash),
                    health_hash: native_hash(record.health_hash),
                    record_hash: native_hash(record.record_hash),
                })
                .collect(),
            read_sets: value
                .read_sets
                .into_iter()
                .map(|read_set| NativeFpsReadSetEvidence {
                    view_kind: read_set.view_kind,
                    owner: read_set.owner,
                    read_set: read_set.read_set,
                })
                .collect(),
            entity_hash: native_hash(value.entity_hash),
            health_hash: native_hash(value.health_hash),
            replay_hash: native_hash(value.replay_hash),
        }
    }
}

impl From<FpsPrimaryFireResult> for NativeFpsPrimaryFireResult {
    fn from(value: FpsPrimaryFireResult) -> Self {
        Self {
            backend: value.backend,
            authority_surface: value.authority_surface,
            mutation_owner: value.mutation_owner,
            workspace_trace: value.workspace_trace,
            shooter: value.shooter as i64,
            target: value.target.map(|target| target as i64),
            target_health_before: value.target_health_before.map(|health| NativeFpsHealth {
                current: health.current,
                max: health.max,
            }),
            target_health_after: value.target_health_after.map(|health| NativeFpsHealth {
                current: health.current,
                max: health.max,
            }),
            lifecycle_status: native_fps_lifecycle_status(value.lifecycle_status),
            target_render_visible: value.target_render_visible,
            entity_hash: native_hash(value.entity_hash),
            health_hash: native_hash(value.health_hash),
            replay_hash: native_hash(value.replay_hash),
        }
    }
}

impl From<NativeFpsEncounterLifecycleInput> for FpsEncounterLifecycleInput {
    fn from(value: NativeFpsEncounterLifecycleInput) -> Self {
        Self {
            outcome_kind: value.outcome_kind,
            terminal: value.terminal,
            enemy_dead: value.enemy_dead,
            player_dead: value.player_dead,
            lifecycle_hash: value.lifecycle_hash,
        }
    }
}

impl From<FpsEncounterLifecycleInput> for NativeFpsEncounterLifecycleInput {
    fn from(value: FpsEncounterLifecycleInput) -> Self {
        Self {
            outcome_kind: value.outcome_kind,
            terminal: value.terminal,
            enemy_dead: value.enemy_dead,
            player_dead: value.player_dead,
            lifecycle_hash: value.lifecycle_hash,
        }
    }
}

impl From<FpsEncounterStateReadout> for NativeFpsEncounterStateReadout {
    fn from(value: FpsEncounterStateReadout) -> Self {
        Self {
            preset_id: value.preset_id,
            status: value.status,
            spawned_enemy_ids: value.spawned_enemy_ids,
            defeated_enemy_ids: value.defeated_enemy_ids,
            revision: value.revision as i64,
            last_transition: value.last_transition,
        }
    }
}

fn native_fps_read_sets(
    read_sets: Vec<runtime_bridge_api::FpsReadSetEvidence>,
) -> Vec<NativeFpsReadSetEvidence> {
    read_sets
        .into_iter()
        .map(|read_set| NativeFpsReadSetEvidence {
            view_kind: read_set.view_kind,
            owner: read_set.owner,
            read_set: read_set.read_set,
        })
        .collect()
}

impl From<FpsEncounterDirectorSnapshot> for NativeFpsEncounterDirectorSnapshot {
    fn from(value: FpsEncounterDirectorSnapshot) -> Self {
        Self {
            backend: value.backend,
            authority_surface: value.authority_surface,
            mutation_owner: value.mutation_owner,
            workspace_trace: value.workspace_trace,
            state: value.state.into(),
            lifecycle: value.lifecycle.into(),
            read_sets: native_fps_read_sets(value.read_sets),
            encounter_hash: native_hash(value.encounter_hash),
            replay_hash: native_hash(value.replay_hash),
        }
    }
}

impl From<FpsEncounterTransitionResult> for NativeFpsEncounterTransitionResult {
    fn from(value: FpsEncounterTransitionResult) -> Self {
        Self {
            backend: value.backend,
            authority_surface: value.authority_surface,
            mutation_owner: value.mutation_owner,
            workspace_trace: value.workspace_trace,
            accepted: value.accepted,
            rejection_reason: value.rejection_reason,
            event_kind: value.event_kind,
            state: value.state.into(),
            lifecycle: value.lifecycle.into(),
            encounter_hash: native_hash(value.encounter_hash),
            replay_hash: native_hash(value.replay_hash),
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

fn parse_voxel_conversion_plan_request(
    request_json: &str,
) -> napi::Result<VoxelConversionPlanRequest> {
    serde_json::from_str(request_json).map_err(|err| {
        to_napi(RuntimeBridgeError::new(
            RuntimeBridgeErrorKind::InvalidInput,
            format!("invalid voxel conversion plan request JSON: {err}"),
        ))
    })
}

fn parse_voxel_conversion_source_registration_request(
    request_json: &str,
) -> napi::Result<VoxelConversionSourceRegistrationRequest> {
    serde_json::from_str(request_json).map_err(|err| {
        to_napi(RuntimeBridgeError::new(
            RuntimeBridgeErrorKind::InvalidInput,
            format!("invalid voxel conversion source registration request JSON: {err}"),
        ))
    })
}

fn parse_voxel_conversion_mesh_asset_registration_request(
    request_json: &str,
) -> napi::Result<VoxelConversionMeshAssetRegistrationRequest> {
    serde_json::from_str(request_json).map_err(|err| {
        to_napi(RuntimeBridgeError::new(
            RuntimeBridgeErrorKind::InvalidInput,
            format!("invalid voxel conversion mesh asset registration request JSON: {err}"),
        ))
    })
}

fn parse_voxel_conversion_preview_request(
    request_json: &str,
) -> napi::Result<VoxelConversionPreviewRequest> {
    serde_json::from_str(request_json).map_err(|err| {
        to_napi(RuntimeBridgeError::new(
            RuntimeBridgeErrorKind::InvalidInput,
            format!("invalid voxel conversion preview request JSON: {err}"),
        ))
    })
}

fn parse_voxel_conversion_apply_request(
    request_json: &str,
) -> napi::Result<VoxelConversionApplyRequest> {
    serde_json::from_str(request_json).map_err(|err| {
        to_napi(RuntimeBridgeError::new(
            RuntimeBridgeErrorKind::InvalidInput,
            format!("invalid voxel conversion apply request JSON: {err}"),
        ))
    })
}

fn parse_voxel_conversion_evidence(
    evidence_json: &str,
) -> napi::Result<Vec<VoxelConversionEvidenceRef>> {
    serde_json::from_str(evidence_json).map_err(|err| {
        to_napi(RuntimeBridgeError::new(
            RuntimeBridgeErrorKind::InvalidInput,
            format!("invalid voxel conversion evidence JSON: {err}"),
        ))
    })
}

fn parse_voxel_model_info_request(request_json: &str) -> napi::Result<VoxelModelInfoRequest> {
    serde_json::from_str(request_json).map_err(|err| {
        to_napi(RuntimeBridgeError::new(
            RuntimeBridgeErrorKind::InvalidInput,
            format!("invalid voxel model info request JSON: {err}"),
        ))
    })
}

fn parse_voxel_model_window_request(request_json: &str) -> napi::Result<VoxelModelWindowRequest> {
    serde_json::from_str(request_json).map_err(|err| {
        to_napi(RuntimeBridgeError::new(
            RuntimeBridgeErrorKind::InvalidInput,
            format!("invalid voxel model window request JSON: {err}"),
        ))
    })
}

fn parse_voxel_volume_asset_export_request(
    request_json: &str,
) -> napi::Result<VoxelVolumeAssetExportRequest> {
    serde_json::from_str(request_json).map_err(|err| {
        to_napi(RuntimeBridgeError::new(
            RuntimeBridgeErrorKind::InvalidInput,
            format!("invalid voxel volume asset export request JSON: {err}"),
        ))
    })
}

fn parse_voxel_volume_asset_load_request(
    request_json: &str,
) -> napi::Result<VoxelVolumeAssetLoadRequest> {
    serde_json::from_str(request_json).map_err(|err| {
        to_napi(RuntimeBridgeError::new(
            RuntimeBridgeErrorKind::InvalidInput,
            format!("invalid voxel volume asset load request JSON: {err}"),
        ))
    })
}

fn parse_voxel_annotation_validation_request(
    request_json: &str,
) -> napi::Result<VoxelAnnotationLayerValidationRequest> {
    serde_json::from_str(request_json).map_err(|err| {
        to_napi(RuntimeBridgeError::new(
            RuntimeBridgeErrorKind::InvalidInput,
            format!("invalid voxel annotation validation request JSON: {err}"),
        ))
    })
}

fn parse_voxel_annotation_load_request(
    request_json: &str,
) -> napi::Result<VoxelAnnotationLayerLoadRequest> {
    serde_json::from_str(request_json).map_err(|err| {
        to_napi(RuntimeBridgeError::new(
            RuntimeBridgeErrorKind::InvalidInput,
            format!("invalid voxel annotation load request JSON: {err}"),
        ))
    })
}

fn parse_voxel_annotation_query_request(
    request_json: &str,
) -> napi::Result<VoxelAnnotationQueryRequest> {
    serde_json::from_str(request_json).map_err(|err| {
        to_napi(RuntimeBridgeError::new(
            RuntimeBridgeErrorKind::InvalidInput,
            format!("invalid voxel annotation query request JSON: {err}"),
        ))
    })
}

fn parse_voxel_annotation_edit_request(
    request_json: &str,
) -> napi::Result<VoxelAnnotationEditRequest> {
    serde_json::from_str(request_json).map_err(|err| {
        to_napi(RuntimeBridgeError::new(
            RuntimeBridgeErrorKind::InvalidInput,
            format!("invalid voxel annotation edit request JSON: {err}"),
        ))
    })
}

fn parse_voxel_annotation_export_request(
    request_json: &str,
) -> napi::Result<VoxelAnnotationLayerExportRequest> {
    serde_json::from_str(request_json).map_err(|err| {
        to_napi(RuntimeBridgeError::new(
            RuntimeBridgeErrorKind::InvalidInput,
            format!("invalid voxel annotation export request JSON: {err}"),
        ))
    })
}

fn parse_game_rule_module_manifests(
    manifests_json: &str,
) -> napi::Result<Vec<GameRuleModuleManifest>> {
    serde_json::from_str(manifests_json).map_err(|err| {
        to_napi(RuntimeBridgeError::new(
            RuntimeBridgeErrorKind::InvalidInput,
            format!("invalid game rule module manifest JSON: {err}"),
        ))
    })
}

fn parse_weapon_effect_hook_request(request_json: &str) -> napi::Result<WeaponEffectHookRequest> {
    serde_json::from_str(request_json).map_err(|err| {
        to_napi(RuntimeBridgeError::new(
            RuntimeBridgeErrorKind::InvalidInput,
            format!("invalid weapon-effect hook request JSON: {err}"),
        ))
    })
}

fn parse_game_rule_catalog(catalog_json: &str) -> napi::Result<GameRuleCatalog> {
    serde_json::from_str(catalog_json).map_err(|err| {
        to_napi(RuntimeBridgeError::new(
            RuntimeBridgeErrorKind::InvalidInput,
            format!("invalid game-rule catalog JSON: {err}"),
        ))
    })
}

fn parse_game_rule_resolution_request(
    request_json: &str,
) -> napi::Result<GameRuleResolutionRequest> {
    serde_json::from_str(request_json).map_err(|err| {
        to_napi(RuntimeBridgeError::new(
            RuntimeBridgeErrorKind::InvalidInput,
            format!("invalid game-rule resolution request JSON: {err}"),
        ))
    })
}

fn voxel_conversion_json<T: serde::Serialize>(value: &T) -> napi::Result<String> {
    serde_json::to_string(value).map_err(|err| {
        to_napi(RuntimeBridgeError::new(
            RuntimeBridgeErrorKind::Internal,
            format!("failed to serialize voxel conversion DTO: {err}"),
        ))
    })
}

fn game_extension_json<T: Serialize>(value: &T) -> napi::Result<String> {
    serde_json::to_string(value).map_err(|err| {
        to_napi(RuntimeBridgeError::new(
            RuntimeBridgeErrorKind::Internal,
            format!("failed to serialize game extension DTO: {err}"),
        ))
    })
}

fn game_rule_json<T: Serialize>(value: &T) -> napi::Result<String> {
    serde_json::to_string(value).map_err(|err| {
        to_napi(RuntimeBridgeError::new(
            RuntimeBridgeErrorKind::Internal,
            format!("failed to serialize game-rule DTO: {err}"),
        ))
    })
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
pub fn load_project_bundle(
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
            .load_project_bundle(ProjectBundleLoadRequest {
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
pub fn create_camera(
    handle: i64,
    request: NativeCameraCreateRequest,
) -> napi::Result<NativeCameraSnapshot> {
    let request = request.into_bridge()?;
    with_bridge(handle, |bridge| {
        bridge
            .create_camera(request)
            .map(NativeCameraSnapshot::from)
            .map_err(to_napi)
    })
}

#[napi]
pub fn apply_collision_constrained_camera_input(
    handle: i64,
    envelope: NativeCollisionConstrainedCameraInputEnvelope,
) -> napi::Result<NativeCameraCollisionSnapshot> {
    let envelope = envelope.into_bridge()?;
    with_bridge(handle, |bridge| {
        bridge
            .apply_collision_constrained_camera_input(envelope)
            .map(NativeCameraCollisionSnapshot::from)
            .map_err(to_napi)
    })
}

#[napi]
pub fn load_fps_runtime_session(
    handle: i64,
    project_bundle: String,
    definitions: Vec<NativeFpsStoredEntityDefinition>,
    game_rule_modules_json: String,
) -> napi::Result<NativeFpsRuntimeSessionSnapshot> {
    let definitions = bridge_fps_definitions(definitions)?;
    let game_rule_modules = parse_game_rule_module_manifests(&game_rule_modules_json)?;
    with_bridge(handle, |bridge| {
        bridge
            .load_fps_runtime_session(FpsRuntimeSessionLoadRequest {
                project_bundle,
                definitions,
                game_rule_modules,
            })
            .map(NativeFpsRuntimeSessionSnapshot::from)
            .map_err(to_napi)
    })
}

#[napi]
pub fn read_fps_runtime_session(handle: i64) -> napi::Result<NativeFpsRuntimeSessionSnapshot> {
    with_bridge(handle, |bridge| {
        bridge
            .read_fps_runtime_session()
            .map(NativeFpsRuntimeSessionSnapshot::from)
            .map_err(to_napi)
    })
}

#[napi]
pub fn apply_fps_primary_fire(
    handle: i64,
    tick: i64,
    origin: NativeVec3,
    direction: NativeVec3,
    shooter_role: Option<String>,
    target_role: Option<String>,
) -> napi::Result<NativeFpsPrimaryFireResult> {
    let tick = u64_input(tick, "tick")?;
    let origin = origin.to_vec3("origin")?;
    let direction = direction.to_vec3("direction")?;
    let shooter_role = optional_native_fps_role(shooter_role, "shooterRole")?;
    let target_role = optional_native_fps_role(target_role, "targetRole")?;
    with_bridge(handle, |bridge| {
        bridge
            .apply_fps_primary_fire(FpsPrimaryFireRequest {
                tick,
                origin: [
                    f64::from(origin.x),
                    f64::from(origin.y),
                    f64::from(origin.z),
                ],
                direction: [
                    f64::from(direction.x),
                    f64::from(direction.y),
                    f64::from(direction.z),
                ],
                shooter_role,
                target_role,
            })
            .map(NativeFpsPrimaryFireResult::from)
            .map_err(to_napi)
    })
}

#[napi]
pub fn invoke_game_extension_weapon_effect(
    handle: i64,
    hook_json: String,
    tick: i64,
    origin: NativeVec3,
    direction: NativeVec3,
    shooter_role: Option<String>,
    target_role: Option<String>,
) -> napi::Result<NativeGameExtensionWeaponEffectInvocationResult> {
    let hook = parse_weapon_effect_hook_request(&hook_json)?;
    let tick = u64_input(tick, "tick")?;
    let origin = origin.to_vec3("origin")?;
    let direction = direction.to_vec3("direction")?;
    let shooter_role = optional_native_fps_role(shooter_role, "shooterRole")?;
    let target_role = optional_native_fps_role(target_role, "targetRole")?;
    with_bridge(handle, |bridge| {
        let result = bridge
            .invoke_game_extension_weapon_effect(GameExtensionWeaponEffectInvocationRequest {
                hook,
                primary_fire: FpsPrimaryFireRequest {
                    tick,
                    origin: [
                        f64::from(origin.x),
                        f64::from(origin.y),
                        f64::from(origin.z),
                    ],
                    direction: [
                        f64::from(direction.x),
                        f64::from(direction.y),
                        f64::from(direction.z),
                    ],
                    shooter_role,
                    target_role,
                },
            })
            .map_err(to_napi)?;
        Ok(NativeGameExtensionWeaponEffectInvocationResult {
            hook_receipt_json: game_extension_json(&result.hook_receipt)?,
            replay_evidence_json: game_extension_json(&result.replay_evidence)?,
            primary_fire: result.primary_fire.map(NativeFpsPrimaryFireResult::from),
        })
    })
}

#[napi]
pub fn validate_game_rule_catalog(handle: i64, catalog_json: String) -> napi::Result<String> {
    let catalog = parse_game_rule_catalog(&catalog_json)?;
    with_bridge(handle, |bridge| {
        let receipt = bridge
            .validate_game_rule_catalog(catalog)
            .map_err(to_napi)?;
        game_rule_json(&receipt)
    })
}

#[napi]
pub fn submit_game_rule_effect_intent(
    handle: i64,
    catalog_json: String,
    request_json: String,
) -> napi::Result<String> {
    let catalog = parse_game_rule_catalog(&catalog_json)?;
    let request = parse_game_rule_resolution_request(&request_json)?;
    with_bridge(handle, |bridge| {
        let receipt = bridge
            .submit_game_rule_effect_intent(GameRuleEffectIntentRequest { catalog, request })
            .map_err(to_napi)?;
        game_rule_json(&receipt)
    })
}

#[napi]
pub fn read_game_rule_runtime_readout(handle: i64) -> napi::Result<String> {
    with_bridge(handle, |bridge| {
        let readout = bridge.read_game_rule_runtime_readout().map_err(to_napi)?;
        game_rule_json(&readout)
    })
}

#[napi]
pub fn restart_fps_runtime_session(
    handle: i64,
    expected_epoch: i64,
) -> napi::Result<NativeFpsRuntimeSessionSnapshot> {
    let expected_epoch = u64_input(expected_epoch, "expected_epoch")?;
    with_bridge(handle, |bridge| {
        bridge
            .restart_fps_runtime_session(FpsRuntimeSessionRestartRequest { expected_epoch })
            .map(NativeFpsRuntimeSessionSnapshot::from)
            .map_err(to_napi)
    })
}

#[napi]
pub fn read_fps_encounter_director(
    handle: i64,
    lifecycle: NativeFpsEncounterLifecycleInput,
) -> napi::Result<NativeFpsEncounterDirectorSnapshot> {
    with_bridge(handle, |bridge| {
        bridge
            .read_fps_encounter_director(lifecycle.into())
            .map(NativeFpsEncounterDirectorSnapshot::from)
            .map_err(to_napi)
    })
}

#[napi]
pub fn apply_fps_encounter_transition(
    handle: i64,
    request: NativeFpsEncounterTransitionRequest,
) -> napi::Result<NativeFpsEncounterTransitionResult> {
    with_bridge(handle, |bridge| {
        bridge
            .apply_fps_encounter_transition(FpsEncounterTransitionRequest {
                preset_id: request.preset_id,
                action: request.action,
                lifecycle: request.lifecycle.into(),
            })
            .map(NativeFpsEncounterTransitionResult::from)
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
pub fn save_project_bundle(handle: i64) -> napi::Result<NativeProjectBundleSaveSummary> {
    with_bridge(handle, |bridge| {
        bridge
            .save_project_bundle()
            .map(NativeProjectBundleSaveSummary::from)
            .map_err(to_napi)
    })
}

#[napi]
pub fn get_project_bundle_composition_status(handle: i64) -> napi::Result<NativeCompositionStatus> {
    with_bridge(handle, |bridge| {
        bridge
            .get_project_bundle_composition_status()
            .map(NativeCompositionStatus::from)
            .map_err(to_napi)
    })
}

#[napi]
pub fn plan_voxel_conversion(handle: i64, request_json: String) -> napi::Result<String> {
    let request = parse_voxel_conversion_plan_request(&request_json)?;
    with_bridge(handle, |bridge| {
        let plan = bridge.plan_voxel_conversion(request).map_err(to_napi)?;
        voxel_conversion_json(&plan)
    })
}

#[napi]
pub fn register_voxel_conversion_source(handle: i64, request_json: String) -> napi::Result<String> {
    let request = parse_voxel_conversion_source_registration_request(&request_json)?;
    with_bridge(handle, |bridge| {
        let registration = bridge
            .register_voxel_conversion_source(request)
            .map_err(to_napi)?;
        voxel_conversion_json(&registration)
    })
}

#[napi]
pub fn register_voxel_conversion_mesh_asset(
    handle: i64,
    request_json: String,
) -> napi::Result<String> {
    let request = parse_voxel_conversion_mesh_asset_registration_request(&request_json)?;
    with_bridge(handle, |bridge| {
        let registration = bridge
            .register_voxel_conversion_mesh_asset(request)
            .map_err(to_napi)?;
        voxel_conversion_json(&registration)
    })
}

#[napi]
pub fn preview_voxel_conversion(handle: i64, request_json: String) -> napi::Result<String> {
    let request = parse_voxel_conversion_preview_request(&request_json)?;
    with_bridge(handle, |bridge| {
        let preview = bridge.preview_voxel_conversion(request).map_err(to_napi)?;
        voxel_conversion_json(&preview)
    })
}

#[napi]
pub fn apply_voxel_conversion(handle: i64, request_json: String) -> napi::Result<String> {
    let request = parse_voxel_conversion_apply_request(&request_json)?;
    with_bridge(handle, |bridge| {
        let receipt = bridge.apply_voxel_conversion(request).map_err(to_napi)?;
        voxel_conversion_json(&receipt)
    })
}

#[napi]
pub fn export_voxel_conversion_evidence(
    handle: i64,
    evidence_json: String,
) -> napi::Result<String> {
    let evidence = parse_voxel_conversion_evidence(&evidence_json)?;
    with_bridge(handle, |bridge| {
        let exported = bridge
            .export_voxel_conversion_evidence(evidence)
            .map_err(to_napi)?;
        voxel_conversion_json(&exported)
    })
}

#[napi]
pub fn read_voxel_model_info(handle: i64, request_json: String) -> napi::Result<String> {
    let request = parse_voxel_model_info_request(&request_json)?;
    with_bridge(handle, |bridge| {
        let readout = bridge.read_voxel_model_info(request).map_err(to_napi)?;
        voxel_conversion_json(&readout)
    })
}

#[napi]
pub fn read_voxel_model_window(handle: i64, request_json: String) -> napi::Result<String> {
    let request = parse_voxel_model_window_request(&request_json)?;
    with_bridge(handle, |bridge| {
        let readout = bridge.read_voxel_model_window(request).map_err(to_napi)?;
        voxel_conversion_json(&readout)
    })
}

fn parse_voxel_volume_asset_save_request(
    request_json: &str,
) -> napi::Result<VoxelVolumeAssetSaveRequest> {
    serde_json::from_str(request_json).map_err(|err| {
        to_napi(RuntimeBridgeError::new(
            RuntimeBridgeErrorKind::InvalidInput,
            format!("invalid voxel volume asset save request JSON: {err}"),
        ))
    })
}

#[napi]
pub fn export_voxel_volume_asset(handle: i64, request_json: String) -> napi::Result<String> {
    let request = parse_voxel_volume_asset_export_request(&request_json)?;
    with_bridge(handle, |bridge| {
        let receipt = bridge.export_voxel_volume_asset(request).map_err(to_napi)?;
        voxel_conversion_json(&receipt)
    })
}

#[napi]
pub fn save_voxel_volume_asset(handle: i64, request_json: String) -> napi::Result<String> {
    let request = parse_voxel_volume_asset_save_request(&request_json)?;
    with_bridge(handle, |bridge| {
        let receipt = bridge.save_voxel_volume_asset(request).map_err(to_napi)?;
        voxel_conversion_json(&receipt)
    })
}

#[napi]
pub fn load_voxel_volume_asset(handle: i64, request_json: String) -> napi::Result<String> {
    let request = parse_voxel_volume_asset_load_request(&request_json)?;
    with_bridge(handle, |bridge| {
        let receipt = bridge.load_voxel_volume_asset(request).map_err(to_napi)?;
        voxel_conversion_json(&receipt)
    })
}

#[napi]
pub fn validate_voxel_annotation_layer(handle: i64, request_json: String) -> napi::Result<String> {
    let request = parse_voxel_annotation_validation_request(&request_json)?;
    with_bridge(handle, |bridge| {
        let report = bridge
            .validate_voxel_annotation_layer(request)
            .map_err(to_napi)?;
        voxel_conversion_json(&report)
    })
}

#[napi]
pub fn load_voxel_annotation_layer(handle: i64, request_json: String) -> napi::Result<String> {
    let request = parse_voxel_annotation_load_request(&request_json)?;
    with_bridge(handle, |bridge| {
        let receipt = bridge
            .load_voxel_annotation_layer(request)
            .map_err(to_napi)?;
        voxel_conversion_json(&receipt)
    })
}

#[napi]
pub fn read_voxel_annotation_query(handle: i64, request_json: String) -> napi::Result<String> {
    let request = parse_voxel_annotation_query_request(&request_json)?;
    with_bridge(handle, |bridge| {
        let readout = bridge
            .read_voxel_annotation_query(request)
            .map_err(to_napi)?;
        voxel_conversion_json(&readout)
    })
}

#[napi]
pub fn apply_voxel_annotation_edit(handle: i64, request_json: String) -> napi::Result<String> {
    let request = parse_voxel_annotation_edit_request(&request_json)?;
    with_bridge(handle, |bridge| {
        let receipt = bridge
            .apply_voxel_annotation_edit(request)
            .map_err(to_napi)?;
        voxel_conversion_json(&receipt)
    })
}

#[napi]
pub fn export_voxel_annotation_layer(handle: i64, request_json: String) -> napi::Result<String> {
    let request = parse_voxel_annotation_export_request(&request_json)?;
    with_bridge(handle, |bridge| {
        let receipt = bridge
            .export_voxel_annotation_layer(request)
            .map_err(to_napi)?;
        voxel_conversion_json(&receipt)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const WIRED_NAPI_EXPORTS: &[&str] = &[
        "applyCollisionConstrainedCameraInput",
        "applyEnemyDirectNavMovement",
        "applyFpsEncounterTransition",
        "applyFpsPrimaryFire",
        "applyVoxelConversion",
        "applyVoxelAnnotationEdit",
        "exportVoxelConversionEvidence",
        "exportVoxelAnnotationLayer",
        "exportVoxelVolumeAsset",
        "getProjectBundleCompositionStatus",
        "initializeEngine",
        "invokeGameExtensionWeaponEffect",
        "loadVoxelAnnotationLayer",
        "loadVoxelVolumeAsset",
        "loadProjectBundle",
        "loadFpsRuntimeSession",
        "planVoxelConversion",
        "readVoxelAnnotationQuery",
        "readFpsEncounterDirector",
        "readRenderDiffs",
        "readFpsRuntimeSession",
        "readVoxelModelInfo",
        "readVoxelModelWindow",
        "registerVoxelConversionSource",
        "registerVoxelConversionMeshAsset",
        "restartFpsRuntimeSession",
        "saveProjectBundle",
        "saveVoxelVolumeAsset",
        "stepSimulation",
        "submitCommands",
        "validateVoxelAnnotationLayer",
    ];

    #[test]
    fn wired_export_set_is_explicit_and_bounded() {
        assert_eq!(
            WIRED_NAPI_EXPORTS,
            &[
                "applyCollisionConstrainedCameraInput",
                "applyEnemyDirectNavMovement",
                "applyFpsEncounterTransition",
                "applyFpsPrimaryFire",
                "applyVoxelConversion",
                "applyVoxelAnnotationEdit",
                "exportVoxelConversionEvidence",
                "exportVoxelAnnotationLayer",
                "exportVoxelVolumeAsset",
                "getProjectBundleCompositionStatus",
                "initializeEngine",
                "invokeGameExtensionWeaponEffect",
                "loadVoxelAnnotationLayer",
                "loadVoxelVolumeAsset",
                "loadProjectBundle",
                "loadFpsRuntimeSession",
                "planVoxelConversion",
                "readVoxelAnnotationQuery",
                "readFpsEncounterDirector",
                "readRenderDiffs",
                "readFpsRuntimeSession",
                "readVoxelModelInfo",
                "readVoxelModelWindow",
                "registerVoxelConversionSource",
                "registerVoxelConversionMeshAsset",
                "restartFpsRuntimeSession",
                "saveProjectBundle",
                "saveVoxelVolumeAsset",
                "stepSimulation",
                "submitCommands",
                "validateVoxelAnnotationLayer",
            ]
        );
    }

    fn native_fps_definitions(enemy_health: u32) -> Vec<NativeFpsStoredEntityDefinition> {
        vec![
            NativeFpsStoredEntityDefinition {
                entity: 101,
                stable_id: "actor/custom-player".into(),
                display_name: "Custom Player".into(),
                source_path: "catalogs/actors/player.entity.json".into(),
                tags: vec!["player".into()],
                role: "player".into(),
                transform: Some(NativeFpsTransformCapability {
                    translation: NativeVec3 {
                        x: 0.0,
                        y: 1.5,
                        z: 0.0,
                    },
                    rotation: vec![0.0, 0.0, 0.0, 1.0],
                    scale: NativeVec3 {
                        x: 1.0,
                        y: 1.0,
                        z: 1.0,
                    },
                }),
                bounds: Some(NativeFpsBoundsCapability {
                    min: NativeVec3 {
                        x: 2.2,
                        y: 1.0,
                        z: 1.0,
                    },
                    max: NativeVec3 {
                        x: 2.8,
                        y: 2.0,
                        z: 2.0,
                    },
                }),
                render_visible: Some(true),
                static_collider: Some(false),
                health: Some(NativeFpsHealth {
                    current: 88,
                    max: 88,
                }),
                weapon: Some(NativeFpsWeaponMount {
                    weapon_id: "weapon.custom.primary".into(),
                    damage: 75,
                    range_units: 16,
                    ammo: 3,
                    cooldown_ticks_after_fire: 4,
                }),
                policy_binding: None,
            },
            NativeFpsStoredEntityDefinition {
                entity: 777,
                stable_id: "actor/custom-enemy".into(),
                display_name: "Custom Enemy".into(),
                source_path: "catalogs/actors/enemy.entity.json".into(),
                tags: vec!["enemy".into()],
                role: "enemy".into(),
                transform: Some(NativeFpsTransformCapability {
                    translation: NativeVec3 {
                        x: 0.0,
                        y: 1.5,
                        z: 5.2,
                    },
                    rotation: vec![0.0, 0.0, 0.0, 1.0],
                    scale: NativeVec3 {
                        x: 1.0,
                        y: 1.0,
                        z: 1.0,
                    },
                }),
                bounds: Some(NativeFpsBoundsCapability {
                    min: NativeVec3 {
                        x: 2.2,
                        y: 1.0,
                        z: 5.0,
                    },
                    max: NativeVec3 {
                        x: 2.8,
                        y: 2.0,
                        z: 5.8,
                    },
                }),
                render_visible: Some(true),
                static_collider: Some(false),
                health: Some(NativeFpsHealth {
                    current: enemy_health,
                    max: enemy_health,
                }),
                weapon: None,
                policy_binding: Some(NativeFpsPolicyBinding {
                    binding_id: "binding.enemy.custom.v0".into(),
                    policy_id: "policy.enemy.custom.v0".into(),
                    view_kind: "runtime_session.nav_policy_view.v0".into(),
                    view_version: "v0".into(),
                    allowed_intents: vec!["runtime.intent.primary_fire.v0".into()],
                    runtime_moment: "runtime.tick.enemy_policy.v0".into(),
                }),
            },
        ]
    }

    #[test]
    fn native_bridge_stateful_smoke_uses_bounded_operations() {
        let handle = initialize_engine(7).expect("engine initializes");
        assert!(handle > 0);

        let loaded = load_project_bundle(handle, 1, 1, 1001).expect("ProjectBundle loads");
        assert_eq!(loaded.loaded_project_bundle, Some(1001));
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

        let fps_loaded = load_fps_runtime_session(
            handle,
            "custom-demo".into(),
            native_fps_definitions(75),
            "[]".into(),
        )
        .expect("fps runtime session loads");
        assert_eq!(fps_loaded.backend, "reference_bridge_rust");
        assert_eq!(fps_loaded.player_entity, 101);
        assert_eq!(fps_loaded.enemy_entity, 777);
        assert_eq!(fps_loaded.policy_bindings.len(), 1);
        assert!(fps_loaded.replay_hash.starts_with("fnv1a64:"));

        let fps_fire = apply_fps_primary_fire(
            handle,
            9,
            NativeVec3 {
                x: 2.5,
                y: 1.5,
                z: 1.5,
            },
            NativeVec3 {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            },
            None,
            None,
        )
        .expect("fps primary fire applies");
        assert_eq!(fps_fire.target, Some(777));
        assert_eq!(fps_fire.lifecycle_status.state, "enemy_defeated");
        assert_eq!(
            fps_fire
                .target_health_after
                .as_ref()
                .map(|health| health.current),
            Some(0)
        );

        let fps_read = read_fps_runtime_session(handle).expect("fps session reads");
        assert_eq!(fps_read.replay_records.len(), 2);
        assert_eq!(fps_read.replay_hash, fps_fire.replay_hash);

        let fps_restarted = restart_fps_runtime_session(handle, fps_read.session_epoch)
            .expect("fps session restarts");
        assert_eq!(fps_restarted.session_epoch, fps_read.session_epoch + 1);
        assert_eq!(fps_restarted.lifecycle_status.state, "active");

        let frame = read_render_diffs(handle, 0).expect("render diff read is bounded");
        assert!(frame.ops.is_empty());

        let saved = save_project_bundle(handle).expect("ProjectBundle saves");
        assert_eq!(saved.artifacts_written, 3);
        assert_eq!(saved.compacted_edits, 0);
        assert_eq!(saved.retained_edits, 0);

        let status = get_project_bundle_composition_status(handle).expect("composition reads");
        assert_eq!(status.loaded_project_bundle, Some(1001));
        assert_eq!(status.fatal_count, 0);
    }

    #[test]
    fn native_bridge_rejects_invalid_inputs_without_fallback() {
        assert!(initialize_engine(-1).is_err());
        assert!(get_project_bundle_composition_status(-99).is_err());

        let handle = initialize_engine(11).expect("engine initializes");
        assert!(load_project_bundle(handle, -1, 1, 1001).is_err());
        assert!(step_simulation(handle, -1).is_err());
        assert!(submit_commands(handle, r#"[{"op":"deleteEverything"}]"#.to_string()).is_err());
    }
}
