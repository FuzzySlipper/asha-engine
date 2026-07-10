//! Public camera/view DTOs for ASHA runtime view/projection evidence.
//!
//! # Lane
//!
//! `contract-steward` — owns the border shape for deterministic camera input,
//! pose snapshots, and projection evidence. This crate is pure protocol data: it
//! has no renderer behavior, no gameplay/player-controller semantics, and no
//! access to authority state.
//!
//! # Border ownership
//!
//! A [`CameraHandle`] names bridge-owned runtime view state scoped to an
//! initialized runtime session. It is not a pointer, a renderer object, or a
//! `StateStore` handle. Consumers propose bounded first-person camera input and
//! read deterministic pose/projection snapshots through manifest-backed runtime
//! bridge operations.
//!
//! # Matrix convention
//!
//! Projection snapshots use column-major 4×4 matrices, matching WebGL/Three.js
//! upload order. The generated TypeScript contract documents the same
//! convention so consumers can compare hashes or matrices without guessing.
//!
//! # Forbidden convenience logic
//!
//! Do not add movement integration, projection math, renderer adapters,
//! collision, sprint/crouch/head-bob, or product/game vocabulary here. Those
//! behaviors belong in runtime bridge implementation tasks, not the protocol
//! border.

#![forbid(unsafe_code)]

use core_space::{Face, VoxelCoord};

/// Opaque bridge-owned camera handle for runtime view/projection state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CameraHandle(pub u64);

impl CameraHandle {
    #[inline]
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    #[inline]
    pub const fn raw(self) -> u64 {
        self.0
    }
}

/// Camera pose in world units/degrees.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraPose {
    pub position: [f32; 3],
    pub yaw_degrees: f32,
    pub pitch_degrees: f32,
}

/// Orthogonal basis vectors derived from a camera pose.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraBasis {
    pub forward: [f32; 3],
    pub right: [f32; 3],
    pub up: [f32; 3],
}

/// Perspective projection parameters.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PerspectiveProjection {
    pub fov_y_degrees: f32,
    pub near: f32,
    pub far: f32,
}

/// Pixel viewport dimensions for projection evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViewportSize {
    pub width: u32,
    pub height: u32,
}

/// Request to create a bridge-owned runtime view camera.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraCreateRequest {
    pub initial_pose: CameraPose,
    pub projection: PerspectiveProjection,
    pub viewport: ViewportSize,
}

/// Bounded first-person input for deterministic camera movement evidence.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FirstPersonCameraInput {
    pub move_forward: f32,
    pub move_right: f32,
    pub move_up: f32,
    pub yaw_delta_degrees: f32,
    pub pitch_delta_degrees: f32,
    pub dt_seconds: f32,
    pub move_speed_units_per_second: f32,
}

/// One camera input proposal for a specific deterministic tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FirstPersonCameraInputEnvelope {
    pub camera: CameraHandle,
    pub input: FirstPersonCameraInput,
    pub tick: u64,
}

/// Request to read current projection evidence for a camera.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraProjectionRequest {
    pub camera: CameraHandle,
    pub viewport: Option<ViewportSize>,
}

/// Camera pose/basis snapshot after create or input application.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraSnapshot {
    pub camera: CameraHandle,
    pub tick: u64,
    pub pose: CameraPose,
    pub basis: CameraBasis,
    pub projection: PerspectiveProjection,
    pub viewport: ViewportSize,
}

/// Camera pose plus deterministic projection matrices.
#[derive(Debug, Clone, PartialEq)]
pub struct CameraProjectionSnapshot {
    pub camera: CameraHandle,
    pub tick: u64,
    pub pose: CameraPose,
    pub basis: CameraBasis,
    pub projection: PerspectiveProjection,
    pub viewport: ViewportSize,
    /// Column-major 4×4 view matrix.
    pub view_matrix: [f32; 16],
    /// Column-major 4×4 projection matrix.
    pub projection_matrix: [f32; 16],
    /// Column-major 4×4 view-projection matrix.
    pub view_projection_matrix: [f32; 16],
    pub projection_hash: String,
}

/// Explicit V1 editor/testbench camera collision shape.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraCollisionShape {
    pub half_extents: [f32; 3],
}

/// The intentionally simple collision policy for V1 camera movement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CameraCollisionPolicyMode {
    AxisSeparableSlide,
}

/// Bounded collision policy evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CameraCollisionPolicy {
    pub mode: CameraCollisionPolicyMode,
    pub max_iterations: u8,
}

/// Locomotion basis selected explicitly by a collision-constrained camera input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FirstPersonMovementMode {
    Grounded,
    FreeFlight,
}

/// Bounded generated-level preset accepted by runtime collision materialization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum GeneratedTunnelPreset {
    #[serde(rename = "tiny-enclosed")]
    TinyEnclosed,
}

/// Request to install one deterministic generated tunnel as collision authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedTunnelRuntimeApplyRequest {
    pub preset: GeneratedTunnelPreset,
    pub seed: u64,
}

/// Authority receipt for the installed generated tunnel collision projection.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedTunnelRuntimeFrame {
    pub world_offset: [f64; 3],
    pub playable_min: [f64; 3],
    pub playable_max: [f64; 3],
}

/// Authority receipt for the installed generated tunnel collision projection.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedTunnelRuntimeApplyReceipt {
    pub preset: GeneratedTunnelPreset,
    pub seed: u64,
    pub grid: u64,
    pub config_hash: String,
    pub output_hash: String,
    pub collision_source_hash: String,
    pub collision_projection_hash: String,
    pub runtime_frame: GeneratedTunnelRuntimeFrame,
}

/// One constrained camera input proposal for a specific tick/grid.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CollisionConstrainedCameraInputEnvelope {
    pub camera: CameraHandle,
    pub grid: u64,
    pub movement_mode: FirstPersonMovementMode,
    pub input: FirstPersonCameraInput,
    pub tick: u64,
    pub shape: CameraCollisionShape,
    pub policy: CameraCollisionPolicy,
}

/// Axis-aligned world AABB queried against voxel collision.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CollisionAabbEvidence {
    pub min: [f32; 3],
    pub max: [f32; 3],
}

/// Axis blocked by the V1 axis-separable collision policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CollisionAxis {
    X,
    Y,
    Z,
}

/// Collision details for an attempted camera move.
#[derive(Debug, Clone, PartialEq)]
pub struct CameraCollisionEvidence {
    pub grid: u64,
    pub movement_mode: FirstPersonMovementMode,
    pub shape: CameraCollisionShape,
    pub policy: CameraCollisionPolicy,
    pub collided: bool,
    pub blocked_axes: Vec<CollisionAxis>,
    pub correction: [f32; 3],
    pub queried_aabb: CollisionAabbEvidence,
    pub collision_source_hash: String,
    pub collision_projection_hash: String,
}

/// Before/attempted/after camera evidence for constrained movement.
#[derive(Debug, Clone, PartialEq)]
pub struct CameraCollisionSnapshot {
    pub camera: CameraHandle,
    pub tick: u64,
    pub before: CameraSnapshot,
    pub attempted: CameraSnapshot,
    pub after: CameraSnapshot,
    pub collision: CameraCollisionEvidence,
    pub movement_hash: String,
}

/// Screen-point coordinate convention.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenPointSpace {
    Normalized01,
    Pixel,
}

/// Screen/crosshair point used to derive a camera ray.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScreenPoint {
    pub x: f32,
    pub y: f32,
    pub space: ScreenPointSpace,
}

/// Request to derive a pick ray from bridge-owned camera/projection evidence.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScreenPointToPickRayRequest {
    pub camera: CameraHandle,
    pub grid: u64,
    pub viewport: Option<ViewportSize>,
    pub screen_point: ScreenPoint,
    pub max_distance: f64,
}

/// Camera-derived world-space ray plus source projection hash.
#[derive(Debug, Clone, PartialEq)]
pub struct PickRaySnapshot {
    pub camera: CameraHandle,
    pub tick: u64,
    pub grid: u64,
    pub screen_point: ScreenPoint,
    pub origin: [f64; 3],
    pub direction: [f64; 3],
    pub max_distance: f64,
    pub camera_projection_hash: String,
    pub ray_hash: String,
}

/// Classified selection outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoxelSelectionOutcome {
    Hit,
    Miss,
}

/// Combined camera-to-ray plus authority raycast selection evidence.
#[derive(Debug, Clone, PartialEq)]
pub struct VoxelSelectionSnapshot {
    pub pick_ray: PickRaySnapshot,
    pub outcome: VoxelSelectionOutcome,
    pub selected_voxel: Option<VoxelCoord>,
    pub selected_face: Option<Face>,
    pub edit_anchor: Option<VoxelCoord>,
    pub selection_hash: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camera_handle_is_opaque_u64_newtype() {
        let handle = CameraHandle::new(42);
        assert_eq!(handle.raw(), 42);
    }

    #[test]
    fn camera_snapshot_carries_only_protocol_data() {
        let camera = CameraHandle::new(7);
        let pose = CameraPose {
            position: [0.0, 1.6, 0.0],
            yaw_degrees: 0.0,
            pitch_degrees: 0.0,
        };
        let basis = CameraBasis {
            forward: [0.0, 0.0, -1.0],
            right: [1.0, 0.0, 0.0],
            up: [0.0, 1.0, 0.0],
        };
        let projection = PerspectiveProjection {
            fov_y_degrees: 60.0,
            near: 0.1,
            far: 1000.0,
        };
        let viewport = ViewportSize {
            width: 1280,
            height: 720,
        };

        let snapshot = CameraSnapshot {
            camera,
            tick: 1,
            pose,
            basis,
            projection,
            viewport,
        };

        assert_eq!(snapshot.camera, camera);
        assert_eq!(snapshot.pose.position, [0.0, 1.6, 0.0]);
        assert_eq!(snapshot.viewport.width, 1280);
    }

    #[test]
    fn generated_tunnel_runtime_apply_dtos_match_the_generated_border() {
        let request = GeneratedTunnelRuntimeApplyRequest {
            preset: GeneratedTunnelPreset::TinyEnclosed,
            seed: 17,
        };
        assert_eq!(
            serde_json::to_value(request).unwrap(),
            serde_json::json!({ "preset": "tiny-enclosed", "seed": 17 })
        );

        let receipt = GeneratedTunnelRuntimeApplyReceipt {
            preset: GeneratedTunnelPreset::TinyEnclosed,
            seed: 17,
            grid: 0,
            config_hash: "e1d156c6b55137a7".to_string(),
            output_hash: "1471496d88d70647".to_string(),
            collision_source_hash: "205242bd77238525".to_string(),
            collision_projection_hash: "fnv1a64:627389be013a3154".to_string(),
            runtime_frame: GeneratedTunnelRuntimeFrame {
                world_offset: [-3.5, -1.0, -5.5],
                playable_min: [-2.5, 0.0, -4.5],
                playable_max: [2.5, 4.0, 4.5],
            },
        };
        let encoded = serde_json::to_value(&receipt).unwrap();
        assert_eq!(
            encoded,
            serde_json::json!({
                "preset": "tiny-enclosed",
                "seed": 17,
                "grid": 0,
                "configHash": "e1d156c6b55137a7",
                "outputHash": "1471496d88d70647",
                "collisionSourceHash": "205242bd77238525",
                "collisionProjectionHash": "fnv1a64:627389be013a3154",
                "runtimeFrame": {
                    "worldOffset": [-3.5, -1.0, -5.5],
                    "playableMin": [-2.5, 0.0, -4.5],
                    "playableMax": [2.5, 4.0, 4.5]
                }
            })
        );
        assert_eq!(
            serde_json::from_value::<GeneratedTunnelRuntimeApplyReceipt>(encoded).unwrap(),
            receipt
        );
    }
}
