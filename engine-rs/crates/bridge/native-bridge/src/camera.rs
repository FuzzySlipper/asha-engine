use napi_derive::napi;
use protocol_view::{
    CameraCollisionEvidence, CameraCollisionPolicy, CameraCollisionPolicyMode,
    CameraCollisionShape, CameraCollisionSnapshot, CameraHandle, CameraProjectionRequest,
    CollisionAabbEvidence, CollisionAxis, CollisionConstrainedCameraInputEnvelope,
    FirstPersonCameraInput, FirstPersonCameraInputEnvelope, FirstPersonMovementMode,
};
use runtime_bridge_api::{
    CameraControllerReadRequest, CameraCreateRequest, CameraModeCommand,
    CameraNavigationInputEnvelope, CameraPose, RuntimeBridge, RuntimeBridgeError,
    RuntimeBridgeErrorKind,
};
use serde::Serialize;

use crate::{to_napi, u64_input, wire::parse_wire_json, with_bridge};

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

impl From<CameraPose> for NativeCameraPose {
    fn from(value: CameraPose) -> Self {
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
        Self {
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

#[napi(object)]
pub struct NativeFirstPersonCameraInputEnvelope {
    pub camera: i64,
    pub input: NativeFirstPersonCameraInput,
    pub tick: i64,
}

impl NativeFirstPersonCameraInputEnvelope {
    fn into_bridge(self) -> napi::Result<FirstPersonCameraInputEnvelope> {
        Ok(FirstPersonCameraInputEnvelope {
            camera: CameraHandle::new(u64_input(self.camera, "camera")?),
            input: self.input.into_bridge("input")?,
            tick: u64_input(self.tick, "tick")?,
        })
    }
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
                CameraCollisionPolicyMode::AxisSeparableSlide => "axis_separable_slide".to_string(),
            },
            max_iterations: u32::from(value.max_iterations),
        }
    }
}

#[napi(object)]
pub struct NativeCollisionConstrainedCameraInputEnvelope {
    pub camera: i64,
    pub grid: i64,
    pub movement_mode: String,
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
            movement_mode: match self.movement_mode.as_str() {
                "grounded" => FirstPersonMovementMode::Grounded,
                "freeFlight" => FirstPersonMovementMode::FreeFlight,
                _ => {
                    return Err(napi::Error::from_reason(
                        "movementMode must be grounded or freeFlight",
                    ))
                }
            },
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
    pub movement_mode: String,
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
            movement_mode: match value.movement_mode {
                FirstPersonMovementMode::Grounded => "grounded".to_string(),
                FirstPersonMovementMode::FreeFlight => "freeFlight".to_string(),
            },
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

fn native_f32x3(values: Vec<f64>, field: &str) -> napi::Result<[f32; 3]> {
    if values.len() != 3 || values.iter().any(|value| !value.is_finite()) {
        return Err(to_napi(RuntimeBridgeError::new(
            RuntimeBridgeErrorKind::InvalidInput,
            format!("{field} must contain exactly three finite values"),
        )));
    }
    Ok([values[0] as f32, values[1] as f32, values[2] as f32])
}

fn serialize_json_result<T: Serialize>(value: &T, operation: &str) -> napi::Result<String> {
    serde_json::to_string(value).map_err(|error| {
        to_napi(RuntimeBridgeError::new(
            RuntimeBridgeErrorKind::Internal,
            format!("{operation} result could not be serialized: {error}"),
        ))
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
pub fn apply_camera_mode_command(handle: i64, command_json: String) -> napi::Result<String> {
    let command = parse_wire_json::<CameraModeCommand>("apply_camera_mode_command", &command_json)?;
    with_bridge(handle, |bridge| {
        let receipt = bridge.apply_camera_mode_command(command).map_err(to_napi)?;
        serialize_json_result(&receipt, "camera mode command")
    })
}

#[napi]
pub fn apply_camera_navigation_input(handle: i64, envelope_json: String) -> napi::Result<String> {
    let envelope = parse_wire_json::<CameraNavigationInputEnvelope>(
        "apply_camera_navigation_input",
        &envelope_json,
    )?;
    with_bridge(handle, |bridge| {
        let receipt = bridge
            .apply_camera_navigation_input(envelope)
            .map_err(to_napi)?;
        serialize_json_result(&receipt, "camera navigation input")
    })
}

#[napi]
pub fn read_camera_controller_state(handle: i64, request_json: String) -> napi::Result<String> {
    let request = parse_wire_json::<CameraControllerReadRequest>(
        "read_camera_controller_state",
        &request_json,
    )?;
    with_bridge(handle, |bridge| {
        let state = bridge
            .read_camera_controller_state(request)
            .map_err(to_napi)?;
        serialize_json_result(&state, "camera controller read")
    })
}

#[napi]
pub fn apply_first_person_camera_input(
    handle: i64,
    envelope: NativeFirstPersonCameraInputEnvelope,
) -> napi::Result<NativeCameraSnapshot> {
    let envelope = envelope.into_bridge()?;
    with_bridge(handle, |bridge| {
        bridge
            .apply_first_person_camera_input(envelope)
            .map(NativeCameraSnapshot::from)
            .map_err(to_napi)
    })
}

#[derive(serde::Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CameraProjectionRequestJson {
    camera: u64,
    viewport: Option<ViewportJson>,
}

#[derive(serde::Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ViewportJson {
    width: u32,
    height: u32,
}

#[napi]
pub fn read_camera_projection(handle: i64, request_json: String) -> napi::Result<String> {
    let request =
        parse_wire_json::<CameraProjectionRequestJson>("read_camera_projection", &request_json)?;
    with_bridge(handle, |bridge| {
        let snapshot = bridge
            .read_camera_projection(CameraProjectionRequest {
                camera: CameraHandle::new(request.camera),
                viewport: request
                    .viewport
                    .map(|viewport| runtime_bridge_api::ViewportSize {
                        width: viewport.width,
                        height: viewport.height,
                    }),
            })
            .map_err(to_napi)?;
        serde_json::to_string(&serde_json::json!({
            "camera": snapshot.camera.raw(),
            "tick": snapshot.tick,
            "pose": {
                "position": snapshot.pose.position,
                "yawDegrees": snapshot.pose.yaw_degrees,
                "pitchDegrees": snapshot.pose.pitch_degrees,
            },
            "basis": {
                "forward": snapshot.basis.forward,
                "right": snapshot.basis.right,
                "up": snapshot.basis.up,
            },
            "projection": {
                "fovYDegrees": snapshot.projection.fov_y_degrees,
                "near": snapshot.projection.near,
                "far": snapshot.projection.far,
            },
            "viewport": {
                "width": snapshot.viewport.width,
                "height": snapshot.viewport.height,
            },
            "viewMatrix": snapshot.view_matrix,
            "projectionMatrix": snapshot.projection_matrix,
            "viewProjectionMatrix": snapshot.view_projection_matrix,
            "projectionHash": snapshot.projection_hash,
        }))
        .map_err(|error| {
            to_napi(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::Internal,
                format!("camera projection result could not be serialized: {error}"),
            ))
        })
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
