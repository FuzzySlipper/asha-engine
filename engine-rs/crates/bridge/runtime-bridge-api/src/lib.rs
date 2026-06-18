//! Runtime bridge API — the N-API-visible boundary types and the typed surface
//! every transport (native `napi-rs`, mock, WASM replay) implements.
//!
//! # Lane
//!
//! `rust-bridge` (ADR 0006). This crate owns the boundary **types** and the
//! [`RuntimeBridge`] trait. It deliberately does **not** depend on `napi` or
//! `wasm-bindgen`: transport glue lives in `native-bridge` / `wasm-api`, which
//! implement this trait. Semantic operation bodies are hand-written and reviewed;
//! only mechanical glue is generated (see `bridge-manifest.toml`).
//!
//! # Boundary discipline
//!
//! - No `serde_json::Value` / `Box<dyn _>` / dynamic `methodName + json` dispatch.
//! - No raw `StateStore` handle ever crosses this boundary — only the opaque
//!   handle newtypes below.
//! - Large payloads cross as [`RuntimeBufferHandle`]s, not inline bytes.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;

use core_commands::VoxelCommand;
use core_error::ErrorCategory;
use core_space::{
    ChunkCoord, ChunkDims, Face, GridId, VoxelCoord, VoxelGridSpec, WorldPos, WorldVec,
};
use core_voxel::{MaterialCatalog, VoxelMaterialId, VoxelValue};
use protocol_view::{
    CameraCreateRequest, CameraProjectionRequest, CameraProjectionSnapshot, CameraSnapshot,
    FirstPersonCameraInputEnvelope,
};
use rule_voxel_edit::VoxelEditRejection;
use svc_collision::{CollisionProjection, Ray};
use svc_mesh::mesh_chunk_in_world;
use svc_serialization::BundleHash;
use svc_spatial::VoxelWorld;
use svc_volume::VoxelChunk;

pub mod buffer_provider;

pub use buffer_provider::{
    fixtures, BufferKind, BufferLifetime, BufferMetadata, RuntimeBufferProvider,
};

// ── Error taxonomy ────────────────────────────────────────────────────────────

/// Typed, classified error channel shared by every bridge operation. There is no
/// string/JSON error blob escape hatch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeBridgeError {
    pub kind: RuntimeBridgeErrorKind,
    pub message: String,
}

/// Stable classification an orchestrator/renderer can switch on without parsing prose.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RuntimeBridgeErrorKind {
    /// An operation was called before `initialize_engine`.
    NotInitialized,
    /// The input violated an invariant the bridge can check cheaply.
    InvalidInput,
    /// A handle (engine/buffer/replay) is unknown or already released.
    UnknownHandle,
    /// A borrowed buffer view was used after it was released/superseded.
    BufferExpired,
    /// The native transport could not be loaded (addon missing/ABI mismatch).
    NativeUnavailable,
    /// An unexpected internal failure (a bug, not an input problem).
    Internal,
}

impl RuntimeBridgeError {
    pub fn new(kind: RuntimeBridgeErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    /// Map to the shared foundation category so tools can treat bridge failures
    /// uniformly with the rest of the workspace.
    pub fn category(&self) -> ErrorCategory {
        match self.kind {
            RuntimeBridgeErrorKind::InvalidInput => ErrorCategory::Invalid,
            RuntimeBridgeErrorKind::UnknownHandle => ErrorCategory::NotFound,
            RuntimeBridgeErrorKind::BufferExpired => ErrorCategory::Conflict,
            RuntimeBridgeErrorKind::NotInitialized | RuntimeBridgeErrorKind::NativeUnavailable => {
                ErrorCategory::Unsupported
            }
            RuntimeBridgeErrorKind::Internal => ErrorCategory::Internal,
        }
    }
}

impl core::fmt::Display for RuntimeBridgeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "runtime bridge error [{:?}]: {}",
            self.kind, self.message
        )
    }
}

impl std::error::Error for RuntimeBridgeError {}

pub type BridgeResult<T> = Result<T, RuntimeBridgeError>;

// ── Opaque handle types ─────────────────────────────────────────────────────--

macro_rules! handle {
    ($(#[$a:meta])* $name:ident) => {
        $(#[$a])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(u64);
        impl $name {
            pub const fn new(raw: u64) -> Self { Self(raw) }
            pub const fn raw(self) -> u64 { self.0 }
        }
    };
}

handle!(
    /// Opaque engine/session handle. Never a `StateStore`.
    EngineHandle
);
handle!(
    /// Opaque handle to bridge-owned buffer bytes (e.g. mesh geometry).
    RuntimeBufferHandle
);
handle!(
    /// Monotonic cursor into the render-diff stream.
    FrameCursor
);
handle!(
    /// Opaque replay-session handle (quarantined surface).
    ReplaySessionHandle
);

// ── Prototype operation payloads ────────────────────────────────────────────--
//
// PROTOTYPE NOTE: these minimal structs stand in for generated `protocol_runtime`
// types (`EngineConfig`/`StepInputEnvelope`/`StepResult`). The codegen emitter
// (#2250 follow-up) replaces them with protocol-crate types; the *shape* of the
// trait below is the stable part.

/// Engine construction config. A deterministic seed is the only required input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineConfig {
    pub seed: u64,
}

/// Deterministic per-tick input envelope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepInputEnvelope {
    pub tick: u64,
}

/// Result of advancing one tick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepResult {
    pub tick: u64,
    /// Number of render diffs produced this tick (real impl returns a descriptor).
    pub diff_count: u32,
}

/// A borrowed, read-only view over bridge-owned bytes. Valid until the owning
/// buffer is released or the next frame supersedes it (see manifest `get_buffer`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeBufferView<'a> {
    pub handle: RuntimeBufferHandle,
    pub bytes: &'a [u8],
}

// PROTOTYPE NOTE: these stand in for the generated
// `protocol_world_bundle::{WorldBundleManifest, SaveSummary}` /
// `protocol_diagnostics::DiagnosticReportSet` contract types named in the
// manifest. The *shape* of the load/save verbs is the stable part.

/// A bounded request to load a world bundle. Identifies the bundle and its
/// versions; the runtime resolves artifacts itself (never a raw path or JSON).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldLoadRequest {
    pub bundle_schema_version: u32,
    pub protocol_version: u32,
    /// The scene identity the bundle bootstraps (stand-in for the full manifest).
    pub scene_id: u64,
}

/// A bounded composition status / diagnostics summary (load + save).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompositionStatus {
    /// The currently-loaded world's scene identity, or `None` if empty.
    pub loaded_world: Option<u64>,
    /// Number of `Fatal` composition diagnostics.
    pub fatal_count: u32,
    /// Total composition diagnostics.
    pub total_count: u32,
    /// Whether the diagnostics block a load.
    pub blocks_load: bool,
}

impl CompositionStatus {
    /// An empty, clean status (no world loaded, no diagnostics).
    pub fn empty() -> Self {
        Self {
            loaded_world: None,
            fatal_count: 0,
            total_count: 0,
            blocks_load: false,
        }
    }
}

/// A bounded summary of a save through the real save/compaction path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorldSaveSummary {
    pub artifacts_written: u32,
    pub compacted_edits: u32,
    pub retained_edits: u32,
}

// ── Command submission payloads (launchable-voxel, #2436) ─────────────────────
//
// The launch/edit loop submits **generated** voxel commands (the authority-owned
// `core_commands::VoxelCommand`, mirrored into the TS `voxel.ts` contract) for
// Rust-side validation + apply via `rule-voxel-edit`. No placeholder `{ kind }`
// command tunnel: the batch carries the real typed command union.

/// A batch of proposed voxel commands awaiting Rust-side validation + apply.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CommandBatch {
    pub commands: Vec<VoxelCommand>,
}

/// The classified outcome of a [`RuntimeBridge::submit_commands`] batch: how many
/// commands authority accepted/rejected, plus the classified rejection for each
/// refused command (never a silent drop). Accepted commands have already mutated
/// authority voxel state and marked their chunks dirty.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CommandResult {
    pub accepted: u32,
    pub rejected: u32,
    /// One classified rejection per refused command, in submission order.
    pub rejections: Vec<VoxelEditRejection>,
}

/// Build the public set-voxel command used by transport glue that must stay
/// outside the state/rule crates. This keeps native/wasm adapters from depending
/// directly on authority internals while still carrying the real command union.
pub fn set_voxel_command(grid: u32, x: i64, y: i64, z: i64, material: u16) -> VoxelCommand {
    VoxelCommand::SetVoxel {
        grid: GridId::new(grid),
        coord: VoxelCoord::new(x, y, z),
        value: VoxelValue::solid_raw(material),
    }
}

// ── Pick (voxel raycast) payloads (launchable-voxel picking, #2437) ───────────
//
// The renderer/UI builds a world-space ray from camera + pointer and hands it to
// `pick_voxel`. Rust authority owns the voxel-grid raycast (`svc-collision`); the
// renderer never owns authoritative voxel coordinates. Mirrors the generated
// `voxel.ts` `PickRay` / `VoxelHit` / `PickResult` border shapes.

/// A world-space pick ray. `grid` selects which authority grid to cast against;
/// `origin`/`direction` are world-space `[x, y, z]`; `max_distance` bounds the cast.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PickRay {
    pub grid: u64,
    pub origin: [f64; 3],
    pub direction: [f64; 3],
    pub max_distance: f64,
}

/// An authoritative voxel ray hit (the border mirror of `svc_collision::VoxelHit`,
/// carrying the grid id so the border is self-describing).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VoxelHit {
    pub grid: u64,
    pub voxel: VoxelCoord,
    pub chunk: ChunkCoord,
    pub face: Face,
    pub point: [f64; 3],
    pub distance: f64,
}

/// Why a pick produced no authoritative hit. Mirrors the `noHit` arm of the
/// generated `PickRejection`; `hitMismatch` is reserved for the renderer-hint
/// revalidation path (a later picking slice), so the raw-ray pick only ever
/// reports `NoHit`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PickRejection {
    #[default]
    NoHit,
}

/// The classified outcome of an authority voxel pick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PickResult {
    Hit(VoxelHit),
    Miss(PickRejection),
}

// ── Voxel mesh/remesh evidence (basic graphical voxel proof, #2646) ───────────

/// Compact request for deterministic voxel mesh evidence. If `chunks` is empty,
/// the bridge reports every resident chunk in canonical coordinate order.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct VoxelMeshEvidenceRequest {
    pub grid: u64,
    pub chunks: Vec<ChunkCoord>,
}

/// Compact mesh counters suitable for artifacts without inline geometry arrays.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct VoxelMeshStatsEvidence {
    pub vertices: u32,
    pub indices: u32,
    pub quads: u32,
    pub faces_emitted: u32,
    pub faces_culled: u32,
}

/// Axis-aligned chunk-local mesh bounds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VoxelMeshBoundsEvidence {
    pub min: [f32; 3],
    pub max: [f32; 3],
}

/// Per-chunk compact mesh evidence derived from authoritative voxel state.
#[derive(Debug, Clone, PartialEq)]
pub struct VoxelMeshChunkEvidence {
    pub coord: ChunkCoord,
    pub resident: bool,
    pub visible: bool,
    pub content_hash: Option<String>,
    pub mesh_hash: Option<String>,
    pub stats: Option<VoxelMeshStatsEvidence>,
    pub bounds: Option<VoxelMeshBoundsEvidence>,
    pub material_slots: Vec<u16>,
}

/// Compact mesh snapshot for proof artifacts: no Three.js objects, no inline mesh
/// arrays by default, just stable hashes/stats sufficient to prove remeshing.
#[derive(Debug, Clone, PartialEq)]
pub struct VoxelMeshEvidenceSnapshot {
    pub grid: u64,
    pub fixture_id: String,
    pub world_hash: String,
    pub meshing_strategy: String,
    pub chunks: Vec<VoxelMeshChunkEvidence>,
    pub diagnostics: Vec<String>,
}

// ── The bridge surface ────────────────────────────────────────────────────────

/// The bounded set of verbs every transport implements. There is no generic
/// `call(method, json)` — adding a verb here is a reviewed boundary change.
pub trait RuntimeBridge {
    fn initialize_engine(&mut self, config: EngineConfig) -> BridgeResult<EngineHandle>;
    fn step_simulation(&mut self, input: StepInputEnvelope) -> BridgeResult<StepResult>;
    /// Submit a batch of proposed voxel commands for Rust-side validation + apply
    /// (mirrors manifest `submit_commands`). Accepted commands mutate authority and
    /// mark dirty chunks; rejected commands are classified and leave state unchanged.
    fn submit_commands(&mut self, batch: CommandBatch) -> BridgeResult<CommandResult>;
    /// Raycast a world-space [`PickRay`] against authority voxel state and return the
    /// nearest classified [`PickResult`] (mirrors manifest `pick_voxel`). Rust owns
    /// the voxel-grid raycast; the renderer only builds the ray. Reads authority —
    /// never mutates it.
    fn pick_voxel(&self, ray: PickRay) -> BridgeResult<PickResult>;
    /// Read compact deterministic voxel mesh evidence for resident/requested chunks.
    /// This summarizes authority-derived `svc-mesh` output with hashes/stats, not
    /// renderer-owned objects or inline Three.js geometry.
    fn read_voxel_mesh_evidence(
        &self,
        request: VoxelMeshEvidenceRequest,
    ) -> BridgeResult<VoxelMeshEvidenceSnapshot>;
    fn create_camera(&mut self, request: CameraCreateRequest) -> BridgeResult<CameraSnapshot>;
    fn apply_first_person_camera_input(
        &mut self,
        input: FirstPersonCameraInputEnvelope,
    ) -> BridgeResult<CameraSnapshot>;
    fn read_camera_projection(
        &self,
        request: CameraProjectionRequest,
    ) -> BridgeResult<CameraProjectionSnapshot>;
    fn get_buffer(&self, handle: RuntimeBufferHandle) -> BridgeResult<RuntimeBufferView<'_>>;
    fn release_buffer(&mut self, handle: RuntimeBufferHandle) -> BridgeResult<()>;

    // ── World load/save composition (#2363) ──
    /// Load a world bundle into authority. Fails closed (and leaves any prior
    /// world untouched) on an unsupported version.
    fn load_world_bundle(&mut self, request: WorldLoadRequest) -> BridgeResult<CompositionStatus>;
    /// Save the current world. Fails closed with `NotInitialized` if none loaded.
    fn save_current_world(&mut self) -> BridgeResult<WorldSaveSummary>;
    /// Read composition status/diagnostics without mutating authority.
    fn get_composition_status(&self) -> BridgeResult<CompositionStatus>;
    /// Unload the staged/live world, returning to an empty runtime.
    fn unload_world(&mut self) -> BridgeResult<()>;
}

// ── Tiny in-crate implementation for smoke tests ──────────────────────────────
//
// Proves the boundary types round-trip without any transport. The real native
// body lives in `native-bridge`; this is the deterministic reference the mock and
// native paths must match.

/// A minimal deterministic bridge used for boundary smoke tests. Large payloads
/// are owned by the [`RuntimeBufferProvider`]; the seed buffer is allocated as the
/// first handle (`0`) at init so the boundary `get_buffer`/`release_buffer` verbs
/// exercise the real provider rather than a bespoke `Vec`.
#[derive(Debug, Default)]
pub struct ReferenceBridge {
    engine: Option<EngineHandle>,
    buffers: buffer_provider::RuntimeBufferProvider,
    /// The currently-loaded world's scene identity (the staged/live world).
    loaded_world: Option<u64>,
    /// Live voxel authority for the launch/edit loop (launchable-voxel, #2436).
    /// Present once `initialize_engine` has set up the runtime.
    voxel: Option<VoxelWorld>,
    /// The material catalog voxel edits validate against.
    materials: MaterialCatalog,
    /// Bridge-owned runtime view cameras (view/projection evidence, not gameplay authority).
    cameras: BTreeMap<u64, CameraSnapshot>,
    next_camera: u64,
}

/// The bundle schema / protocol versions this reference bridge understands.
const REFERENCE_SUPPORTED_VERSION: u32 = 1;

impl ReferenceBridge {
    pub fn new() -> Self {
        Self::default()
    }

    /// The default launch grid: id 1, voxel size 1.0, cubic 2×2×2 chunks (matches
    /// the canonical voxel fixture). Chunk dims come from the spec, not a global.
    fn launch_grid() -> VoxelGridSpec {
        VoxelGridSpec::new(
            GridId::new(1),
            1.0,
            ChunkDims::cubic(2).expect("nonzero dims"),
        )
        .expect("positive voxel size")
    }

    fn material_for_chunk(coord: ChunkCoord) -> u16 {
        const MATERIAL_IDS: [u16; 3] = [1, 2, 3];
        let idx = (coord.x * 2 + coord.y).rem_euclid(MATERIAL_IDS.len() as i64) as usize;
        MATERIAL_IDS[idx]
    }

    fn launch_world() -> VoxelWorld {
        let spec = Self::launch_grid();
        let mut world = VoxelWorld::new(spec);
        let dims = spec.chunk_dims();
        for coord in [
            ChunkCoord::new(0, 0, 0),
            ChunkCoord::new(1, 0, 0),
            ChunkCoord::new(0, 1, 0),
            ChunkCoord::new(1, 1, 0),
        ] {
            let mut chunk = VoxelChunk::from_spec(&spec);
            chunk
                .fill_region(
                    core_space::LocalVoxelCoord::new(0, 0, 0),
                    core_space::LocalVoxelCoord::new(dims.x(), dims.y(), 1),
                    VoxelValue::solid_raw(Self::material_for_chunk(coord)),
                )
                .expect("canonical launch chunk fill within bounds");
            world.insert(coord, chunk);
        }
        let _ = world.drain_dirty();
        world
    }

    fn world_hash(world: &VoxelWorld) -> String {
        let mut buf = String::new();
        for (coord, chunk) in world.resident_chunks() {
            buf.push_str(&format!(
                "{},{},{}={:016x};",
                coord.x,
                coord.y,
                coord.z,
                chunk.content_hash().0
            ));
        }
        BundleHash::of_str(&buf).to_hex()
    }

    fn mesh_payload_hash(mesh: &svc_mesh::MeshPayload) -> String {
        format!("fnv1a64:{}", Self::fnv1a64(&mesh.to_fixture_string()))
    }

    fn mesh_evidence_for(
        world: &VoxelWorld,
        coord: ChunkCoord,
    ) -> (VoxelMeshChunkEvidence, Vec<String>) {
        let Some(chunk) = world.get(coord) else {
            return (
                VoxelMeshChunkEvidence {
                    coord,
                    resident: false,
                    visible: false,
                    content_hash: None,
                    mesh_hash: None,
                    stats: None,
                    bounds: None,
                    material_slots: Vec::new(),
                },
                Vec::new(),
            );
        };

        match mesh_chunk_in_world(world, coord) {
            Some(Ok(mesh)) if !mesh.indices.is_empty() => {
                let stats = mesh.stats;
                (
                    VoxelMeshChunkEvidence {
                        coord,
                        resident: true,
                        visible: true,
                        content_hash: Some(format!("{:016x}", chunk.content_hash().0)),
                        mesh_hash: Some(Self::mesh_payload_hash(&mesh)),
                        stats: Some(VoxelMeshStatsEvidence {
                            vertices: stats.vertices,
                            indices: stats.indices,
                            quads: stats.quads,
                            faces_emitted: stats.faces_emitted,
                            faces_culled: stats.faces_culled,
                        }),
                        bounds: Some(VoxelMeshBoundsEvidence {
                            min: mesh.bounds.min,
                            max: mesh.bounds.max,
                        }),
                        material_slots: mesh.groups.iter().map(|g| g.material_slot).collect(),
                    },
                    Vec::new(),
                )
            }
            Some(Ok(_)) => (
                VoxelMeshChunkEvidence {
                    coord,
                    resident: true,
                    visible: false,
                    content_hash: Some(format!("{:016x}", chunk.content_hash().0)),
                    mesh_hash: None,
                    stats: None,
                    bounds: None,
                    material_slots: Vec::new(),
                },
                Vec::new(),
            ),
            Some(Err(err)) => (
                VoxelMeshChunkEvidence {
                    coord,
                    resident: true,
                    visible: false,
                    content_hash: Some(format!("{:016x}", chunk.content_hash().0)),
                    mesh_hash: None,
                    stats: None,
                    bounds: None,
                    material_slots: Vec::new(),
                },
                vec![format!(
                    "mesh error for chunk {},{},{}: {err}",
                    coord.x, coord.y, coord.z
                )],
            ),
            None => unreachable!("world.get(coord) already proved resident"),
        }
    }

    fn require_initialized(&self, op: &str) -> BridgeResult<()> {
        if self.engine.is_none() {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::NotInitialized,
                format!("{op} called before initialize_engine"),
            ));
        }
        Ok(())
    }

    fn basis_from_pose(pose: protocol_view::CameraPose) -> protocol_view::CameraBasis {
        let yaw = pose.yaw_degrees.to_radians();
        let pitch = pose.pitch_degrees.to_radians();
        let cp = pitch.cos();
        let sp = pitch.sin();
        let sy = yaw.sin();
        let cy = yaw.cos();
        protocol_view::CameraBasis {
            forward: [sy * cp, sp, -cy * cp],
            right: [cy, 0.0, sy],
            up: [-sy * sp, cp, cy * sp],
        }
    }

    fn validate_viewport(viewport: protocol_view::ViewportSize) -> BridgeResult<()> {
        if viewport.width == 0 || viewport.height == 0 {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                "viewport dimensions must be positive",
            ));
        }
        Ok(())
    }

    fn validate_create_request(request: &CameraCreateRequest) -> BridgeResult<()> {
        Self::validate_viewport(request.viewport)?;
        if !(request.projection.fov_y_degrees.is_finite()
            && request.projection.near.is_finite()
            && request.projection.far.is_finite())
            || request.projection.fov_y_degrees <= 0.0
            || request.projection.fov_y_degrees >= 180.0
            || request.projection.near <= 0.0
            || request.projection.far <= request.projection.near
        {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                "invalid perspective projection parameters",
            ));
        }
        if !request.initial_pose.position.iter().all(|v| v.is_finite())
            || !request.initial_pose.yaw_degrees.is_finite()
            || !request.initial_pose.pitch_degrees.is_finite()
        {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                "camera pose values must be finite",
            ));
        }
        Ok(())
    }

    fn matrix_key(values: &[f32]) -> String {
        values
            .iter()
            .map(|v| format!("{v:.3}"))
            .collect::<Vec<_>>()
            .join(",")
    }

    fn fnv1a64(text: &str) -> String {
        let mut hash = 0xcbf29ce484222325u64;
        for byte in text.as_bytes() {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x100000001b3);
        }
        format!("{hash:016x}")
    }

    fn multiply_matrix4(a: [f32; 16], b: [f32; 16]) -> [f32; 16] {
        let mut out = [0.0; 16];
        for col in 0..4 {
            for row in 0..4 {
                let mut sum = 0.0;
                for k in 0..4 {
                    sum += a[k * 4 + row] * b[col * 4 + k];
                }
                out[col * 4 + row] = sum;
            }
        }
        out
    }

    fn projection_snapshot(
        snapshot: CameraSnapshot,
        viewport: protocol_view::ViewportSize,
    ) -> CameraProjectionSnapshot {
        let right = snapshot.basis.right;
        let up = snapshot.basis.up;
        let forward = snapshot.basis.forward;
        let position = snapshot.pose.position;
        let dot_right = right[0] * position[0] + right[1] * position[1] + right[2] * position[2];
        let dot_up = up[0] * position[0] + up[1] * position[1] + up[2] * position[2];
        let dot_forward =
            forward[0] * position[0] + forward[1] * position[1] + forward[2] * position[2];
        let view_matrix = [
            right[0],
            up[0],
            -forward[0],
            0.0,
            right[1],
            up[1],
            -forward[1],
            0.0,
            right[2],
            up[2],
            -forward[2],
            0.0,
            -dot_right,
            -dot_up,
            dot_forward,
            1.0,
        ];
        let aspect = viewport.width as f32 / viewport.height as f32;
        let f = 1.0 / (snapshot.projection.fov_y_degrees.to_radians() / 2.0).tan();
        let near = snapshot.projection.near;
        let far = snapshot.projection.far;
        let projection_matrix = [
            f / aspect,
            0.0,
            0.0,
            0.0,
            0.0,
            f,
            0.0,
            0.0,
            0.0,
            0.0,
            (far + near) / (near - far),
            -1.0,
            0.0,
            0.0,
            (2.0 * far * near) / (near - far),
            0.0,
        ];
        let view_projection_matrix = Self::multiply_matrix4(projection_matrix, view_matrix);
        let mut hash_values = Vec::with_capacity(48);
        hash_values.extend_from_slice(&view_matrix);
        hash_values.extend_from_slice(&projection_matrix);
        hash_values.extend_from_slice(&view_projection_matrix);
        let projection_hash = format!("fnv1a64:{}", Self::fnv1a64(&Self::matrix_key(&hash_values)));
        CameraProjectionSnapshot {
            camera: snapshot.camera,
            tick: snapshot.tick,
            pose: snapshot.pose,
            basis: snapshot.basis,
            projection: snapshot.projection,
            viewport,
            view_matrix,
            projection_matrix,
            view_projection_matrix,
            projection_hash,
        }
    }
}

impl RuntimeBridge for ReferenceBridge {
    fn initialize_engine(&mut self, config: EngineConfig) -> BridgeResult<EngineHandle> {
        let handle = EngineHandle::new(config.seed);
        self.engine = Some(handle);
        // Deterministic: seed buffer is the first provider handle (0), so the
        // boundary buffer verbs below operate on the real lifetime model.
        self.buffers.reset();
        let seed_handle = self.buffers.create(
            buffer_provider::BufferKind::Opaque,
            buffer_provider::BufferLifetime::Manual,
            None,
            config.seed.to_le_bytes().to_vec(),
        );
        debug_assert_eq!(seed_handle.raw(), 0);

        // Stand up the voxel authority for the launch/edit loop: a resident origin
        // chunk so edits land, plus the launch material catalog. Start clean so a
        // later submit's dirty marking is observable.
        let world = Self::launch_world();
        self.voxel = Some(world);
        self.materials = MaterialCatalog::new([1, 2, 3].into_iter().map(VoxelMaterialId::new));
        self.cameras.clear();
        self.next_camera = 1;

        Ok(handle)
    }

    fn submit_commands(&mut self, batch: CommandBatch) -> BridgeResult<CommandResult> {
        let materials = &self.materials;
        let world = self.voxel.as_mut().ok_or_else(|| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::NotInitialized,
                "submit_commands called before initialize_engine",
            )
        })?;

        let mut accepted = 0u32;
        let mut rejections = Vec::new();
        for cmd in &batch.commands {
            // Validate (no mutation), then apply on accept. A rejected command is
            // classified and never touches authority state.
            match rule_voxel_edit::validate(cmd, world, materials) {
                Ok(events) => {
                    for event in &events {
                        rule_voxel_edit::apply(world, event).map_err(|rej| {
                            RuntimeBridgeError::new(
                                RuntimeBridgeErrorKind::Internal,
                                format!("validated voxel command failed to apply: {rej}"),
                            )
                        })?;
                    }
                    accepted += 1;
                }
                Err(rejection) => rejections.push(rejection),
            }
        }

        Ok(CommandResult {
            accepted,
            rejected: rejections.len() as u32,
            rejections,
        })
    }

    fn pick_voxel(&self, ray: PickRay) -> BridgeResult<PickResult> {
        let world = self.voxel.as_ref().ok_or_else(|| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::NotInitialized,
                "pick_voxel called before initialize_engine",
            )
        })?;
        // Fail closed on a ray that names a grid the runtime is not hosting, rather
        // than silently casting against the wrong (only) grid.
        if ray.grid != world.grid().id().raw() as u64 {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                "pick_voxel ray targets an unknown grid",
            ));
        }

        // Authority owns the raycast: build the collision projection from authority
        // voxel state and cast. (The reference bridge rebuilds per pick; a native
        // bridge can cache the projection — this stays the correctness reference.)
        let projection = CollisionProjection::build(world);
        let origin = WorldPos::new(ray.origin[0], ray.origin[1], ray.origin[2]);
        let dir = WorldVec::new(ray.direction[0], ray.direction[1], ray.direction[2]);
        match projection.raycast(Ray::new(origin, dir), ray.max_distance) {
            Some(hit) => Ok(PickResult::Hit(VoxelHit {
                grid: ray.grid,
                voxel: hit.voxel,
                chunk: hit.chunk,
                face: hit.face,
                point: [hit.point.x, hit.point.y, hit.point.z],
                distance: hit.distance,
            })),
            None => Ok(PickResult::Miss(PickRejection::NoHit)),
        }
    }

    fn read_voxel_mesh_evidence(
        &self,
        request: VoxelMeshEvidenceRequest,
    ) -> BridgeResult<VoxelMeshEvidenceSnapshot> {
        let world = self.voxel.as_ref().ok_or_else(|| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::NotInitialized,
                "read_voxel_mesh_evidence called before initialize_engine",
            )
        })?;
        if request.grid != world.grid().id().raw() as u64 {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                "read_voxel_mesh_evidence request targets an unknown grid",
            ));
        }

        let mut coords = if request.chunks.is_empty() {
            world
                .resident_chunks()
                .map(|(coord, _)| coord)
                .collect::<Vec<_>>()
        } else {
            request.chunks
        };
        coords.sort();
        coords.dedup();

        let mut chunks = Vec::with_capacity(coords.len());
        let mut diagnostics = Vec::new();
        for coord in coords {
            let (evidence, mut diag) = Self::mesh_evidence_for(world, coord);
            chunks.push(evidence);
            diagnostics.append(&mut diag);
        }

        Ok(VoxelMeshEvidenceSnapshot {
            grid: request.grid,
            fixture_id: "basic-voxel-landscape-interaction".to_string(),
            world_hash: Self::world_hash(world),
            meshing_strategy: "visible-face".to_string(),
            chunks,
            diagnostics,
        })
    }

    fn step_simulation(&mut self, input: StepInputEnvelope) -> BridgeResult<StepResult> {
        if self.engine.is_none() {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::NotInitialized,
                "step_simulation called before initialize_engine",
            ));
        }
        Ok(StepResult {
            tick: input.tick,
            diff_count: (input.tick % 4) as u32,
        })
    }

    fn create_camera(&mut self, request: CameraCreateRequest) -> BridgeResult<CameraSnapshot> {
        self.require_initialized("create_camera")?;
        Self::validate_create_request(&request)?;
        let camera = protocol_view::CameraHandle::new(self.next_camera);
        self.next_camera += 1;
        let snapshot = CameraSnapshot {
            camera,
            tick: 0,
            pose: request.initial_pose,
            basis: Self::basis_from_pose(request.initial_pose),
            projection: request.projection,
            viewport: request.viewport,
        };
        self.cameras.insert(camera.raw(), snapshot);
        Ok(snapshot)
    }

    fn apply_first_person_camera_input(
        &mut self,
        envelope: FirstPersonCameraInputEnvelope,
    ) -> BridgeResult<CameraSnapshot> {
        self.require_initialized("apply_first_person_camera_input")?;
        let prior = *self.cameras.get(&envelope.camera.raw()).ok_or_else(|| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::UnknownHandle,
                "unknown camera handle",
            )
        })?;
        let input = envelope.input;
        let finite = input.move_forward.is_finite()
            && input.move_right.is_finite()
            && input.move_up.is_finite()
            && input.yaw_delta_degrees.is_finite()
            && input.pitch_delta_degrees.is_finite()
            && input.dt_seconds.is_finite()
            && input.move_speed_units_per_second.is_finite();
        if !finite || input.dt_seconds < 0.0 || input.move_speed_units_per_second < 0.0 {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                "dt_seconds and move_speed_units_per_second must be non-negative",
            ));
        }
        let distance = input.dt_seconds * input.move_speed_units_per_second;
        let basis = prior.basis;
        let pose = protocol_view::CameraPose {
            position: [
                prior.pose.position[0]
                    + (basis.forward[0] * input.move_forward
                        + basis.right[0] * input.move_right
                        + basis.up[0] * input.move_up)
                        * distance,
                prior.pose.position[1]
                    + (basis.forward[1] * input.move_forward
                        + basis.right[1] * input.move_right
                        + basis.up[1] * input.move_up)
                        * distance,
                prior.pose.position[2]
                    + (basis.forward[2] * input.move_forward
                        + basis.right[2] * input.move_right
                        + basis.up[2] * input.move_up)
                        * distance,
            ],
            yaw_degrees: prior.pose.yaw_degrees + input.yaw_delta_degrees,
            pitch_degrees: (prior.pose.pitch_degrees + input.pitch_delta_degrees)
                .clamp(-89.0, 89.0),
        };
        let snapshot = CameraSnapshot {
            tick: envelope.tick,
            pose,
            basis: Self::basis_from_pose(pose),
            ..prior
        };
        self.cameras.insert(envelope.camera.raw(), snapshot);
        Ok(snapshot)
    }

    fn read_camera_projection(
        &self,
        request: CameraProjectionRequest,
    ) -> BridgeResult<CameraProjectionSnapshot> {
        self.require_initialized("read_camera_projection")?;
        let snapshot = *self.cameras.get(&request.camera.raw()).ok_or_else(|| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::UnknownHandle,
                "unknown camera handle",
            )
        })?;
        let viewport = request.viewport.unwrap_or(snapshot.viewport);
        Self::validate_viewport(viewport)?;
        Ok(Self::projection_snapshot(snapshot, viewport))
    }

    fn get_buffer(&self, handle: RuntimeBufferHandle) -> BridgeResult<RuntimeBufferView<'_>> {
        self.buffers.view(handle)
    }

    fn release_buffer(&mut self, handle: RuntimeBufferHandle) -> BridgeResult<()> {
        self.buffers.dispose(handle)
    }

    fn load_world_bundle(&mut self, request: WorldLoadRequest) -> BridgeResult<CompositionStatus> {
        // Fail closed on a newer bundle; the prior loaded world is left untouched
        // (we only mutate `loaded_world` on success — the staged commit/swap).
        if request.bundle_schema_version > REFERENCE_SUPPORTED_VERSION
            || request.protocol_version > REFERENCE_SUPPORTED_VERSION
        {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                format!(
                    "unsupported bundle schema {} / protocol {}",
                    request.bundle_schema_version, request.protocol_version
                ),
            ));
        }
        self.loaded_world = Some(request.scene_id);
        Ok(CompositionStatus {
            loaded_world: Some(request.scene_id),
            ..CompositionStatus::empty()
        })
    }

    fn save_current_world(&mut self) -> BridgeResult<WorldSaveSummary> {
        if self.loaded_world.is_none() {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::NotInitialized,
                "save_current_world called with no world loaded",
            ));
        }
        // Deterministic stand-in for the real save/compaction summary.
        Ok(WorldSaveSummary {
            artifacts_written: 3,
            compacted_edits: 0,
            retained_edits: 0,
        })
    }

    fn get_composition_status(&self) -> BridgeResult<CompositionStatus> {
        Ok(CompositionStatus {
            loaded_world: self.loaded_world,
            ..CompositionStatus::empty()
        })
    }

    fn unload_world(&mut self) -> BridgeResult<()> {
        self.loaded_world = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn step_before_init_is_typed_error() {
        let mut bridge = ReferenceBridge::new();
        let err = bridge
            .step_simulation(StepInputEnvelope { tick: 1 })
            .unwrap_err();
        assert_eq!(err.kind, RuntimeBridgeErrorKind::NotInitialized);
        assert_eq!(err.category(), ErrorCategory::Unsupported);
    }

    #[test]
    fn save_before_load_fails_closed() {
        let mut bridge = ReferenceBridge::new();
        let err = bridge.save_current_world().unwrap_err();
        assert_eq!(err.kind, RuntimeBridgeErrorKind::NotInitialized);
        // And status reflects no loaded world.
        assert_eq!(bridge.get_composition_status().unwrap().loaded_world, None);
    }

    #[test]
    fn camera_view_surface_round_trips_and_fails_closed() {
        use protocol_view::{
            CameraHandle, CameraPose, FirstPersonCameraInput, PerspectiveProjection, ViewportSize,
        };

        let mut bridge = ReferenceBridge::new();
        let request = CameraCreateRequest {
            initial_pose: CameraPose {
                position: [0.0, 1.6, 0.0],
                yaw_degrees: 0.0,
                pitch_degrees: 0.0,
            },
            projection: PerspectiveProjection {
                fov_y_degrees: 60.0,
                near: 0.1,
                far: 1000.0,
            },
            viewport: ViewportSize {
                width: 1280,
                height: 720,
            },
        };
        assert_eq!(
            bridge.create_camera(request).unwrap_err().kind,
            RuntimeBridgeErrorKind::NotInitialized
        );

        bridge.initialize_engine(EngineConfig { seed: 1 }).unwrap();
        let created = bridge.create_camera(request).unwrap();
        assert_eq!(created.camera.raw(), 1);
        assert_eq!(created.pose, request.initial_pose);

        let moved = bridge
            .apply_first_person_camera_input(FirstPersonCameraInputEnvelope {
                camera: created.camera,
                tick: 1,
                input: FirstPersonCameraInput {
                    move_forward: 1.0,
                    move_right: 0.0,
                    move_up: 0.0,
                    yaw_delta_degrees: 15.0,
                    pitch_delta_degrees: -5.0,
                    dt_seconds: 1.0 / 60.0,
                    move_speed_units_per_second: 3.0,
                },
            })
            .unwrap();
        assert_eq!(moved.tick, 1);
        assert_ne!(moved.pose, created.pose);

        let projected = bridge
            .read_camera_projection(CameraProjectionRequest {
                camera: moved.camera,
                viewport: None,
            })
            .unwrap();
        assert_eq!(projected.view_matrix.len(), 16);
        assert_eq!(projected.projection_hash, "fnv1a64:071327a4920ab097");

        assert_eq!(
            bridge
                .read_camera_projection(CameraProjectionRequest {
                    camera: moved.camera,
                    viewport: Some(ViewportSize {
                        width: 1280,
                        height: 0,
                    }),
                })
                .unwrap_err()
                .kind,
            RuntimeBridgeErrorKind::InvalidInput
        );

        assert_eq!(
            bridge
                .read_camera_projection(CameraProjectionRequest {
                    camera: CameraHandle::new(999),
                    viewport: None,
                })
                .unwrap_err()
                .kind,
            RuntimeBridgeErrorKind::UnknownHandle
        );
    }

    #[test]
    fn load_save_status_unload_round_trip() {
        let mut bridge = ReferenceBridge::new();
        let status = bridge
            .load_world_bundle(WorldLoadRequest {
                bundle_schema_version: 1,
                protocol_version: 1,
                scene_id: 100,
            })
            .unwrap();
        assert_eq!(status.loaded_world, Some(100));
        assert!(!status.blocks_load);

        let save = bridge.save_current_world().unwrap();
        assert_eq!(save.artifacts_written, 3);

        assert_eq!(
            bridge.get_composition_status().unwrap().loaded_world,
            Some(100)
        );

        bridge.unload_world().unwrap();
        assert_eq!(bridge.get_composition_status().unwrap().loaded_world, None);
        // Save after unload fails closed again.
        assert_eq!(
            bridge.save_current_world().unwrap_err().kind,
            RuntimeBridgeErrorKind::NotInitialized
        );
    }

    #[test]
    fn load_unsupported_version_fails_closed_without_mutating() {
        let mut bridge = ReferenceBridge::new();
        // Load a valid world first.
        bridge
            .load_world_bundle(WorldLoadRequest {
                bundle_schema_version: 1,
                protocol_version: 1,
                scene_id: 7,
            })
            .unwrap();
        // A too-new bundle is rejected and must NOT replace the loaded world.
        let err = bridge
            .load_world_bundle(WorldLoadRequest {
                bundle_schema_version: 99,
                protocol_version: 1,
                scene_id: 8,
            })
            .unwrap_err();
        assert_eq!(err.kind, RuntimeBridgeErrorKind::InvalidInput);
        assert_eq!(
            bridge.get_composition_status().unwrap().loaded_world,
            Some(7),
            "a failed load must not swap out the prior world"
        );
    }

    #[test]
    fn init_then_step_is_deterministic() {
        let mut bridge = ReferenceBridge::new();
        let h = bridge.initialize_engine(EngineConfig { seed: 7 }).unwrap();
        assert_eq!(h.raw(), 7);
        let r = bridge
            .step_simulation(StepInputEnvelope { tick: 6 })
            .unwrap();
        assert_eq!(
            r,
            StepResult {
                tick: 6,
                diff_count: 2
            }
        );
    }

    // ── Voxel command submission → Rust authority (launchable-voxel, #2436) ──

    use core_space::{LocalVoxelCoord, VoxelCoord};
    use core_voxel::VoxelValue;

    fn init_bridge() -> ReferenceBridge {
        let mut bridge = ReferenceBridge::new();
        bridge.initialize_engine(EngineConfig { seed: 1 }).unwrap();
        bridge
    }

    fn set_voxel(coord: VoxelCoord, material: u16) -> VoxelCommand {
        VoxelCommand::SetVoxel {
            grid: GridId::new(1),
            coord,
            value: VoxelValue::solid_raw(material),
        }
    }

    #[test]
    fn submit_before_init_fails_closed() {
        let mut bridge = ReferenceBridge::new();
        let err = bridge.submit_commands(CommandBatch::default()).unwrap_err();
        assert_eq!(err.kind, RuntimeBridgeErrorKind::NotInitialized);
    }

    #[test]
    fn accepted_voxel_command_mutates_authority_and_marks_dirty() {
        let mut bridge = init_bridge();
        // The batch carries a generated VoxelCommand — not a `{ kind }` placeholder.
        let result = bridge
            .submit_commands(CommandBatch {
                commands: vec![set_voxel(VoxelCoord::new(0, 0, 0), 1)],
            })
            .unwrap();
        assert_eq!(result.accepted, 1);
        assert_eq!(result.rejected, 0);
        assert!(result.rejections.is_empty());

        let world = bridge.voxel.as_ref().unwrap();
        let chunk = world.get(ChunkCoord::new(0, 0, 0)).unwrap();
        assert_eq!(
            chunk.get(LocalVoxelCoord::new(0, 0, 0)),
            Some(VoxelValue::solid_raw(1)),
            "authority voxel state changed"
        );
        assert!(
            world.is_dirty(ChunkCoord::new(0, 0, 0)),
            "the edited chunk is marked dirty"
        );
    }

    #[test]
    fn rejected_unknown_material_is_classified_and_does_not_mutate() {
        let mut bridge = init_bridge();
        let before = bridge
            .voxel
            .as_ref()
            .unwrap()
            .get(ChunkCoord::new(0, 0, 0))
            .unwrap()
            .content_hash();

        let result = bridge
            .submit_commands(CommandBatch {
                commands: vec![set_voxel(VoxelCoord::new(0, 0, 0), 99)],
            })
            .unwrap();
        assert_eq!(result.accepted, 0);
        assert_eq!(result.rejected, 1);
        assert!(matches!(
            result.rejections[0],
            VoxelEditRejection::UnknownMaterial(_)
        ));

        let after = bridge
            .voxel
            .as_ref()
            .unwrap()
            .get(ChunkCoord::new(0, 0, 0))
            .unwrap()
            .content_hash();
        assert_eq!(
            before, after,
            "a rejected command must not mutate authority"
        );
    }

    #[test]
    fn rejected_non_resident_chunk_is_classified() {
        let mut bridge = init_bridge();
        let result = bridge
            .submit_commands(CommandBatch {
                commands: vec![set_voxel(VoxelCoord::new(100, 0, 0), 1)],
            })
            .unwrap();
        assert_eq!(result.rejected, 1);
        assert!(matches!(
            result.rejections[0],
            VoxelEditRejection::ChunkNotResident { .. }
        ));
    }

    #[test]
    fn mesh_evidence_reports_fixture_chunks_and_changes_after_edit() {
        let mut bridge = init_bridge();
        let before = bridge
            .read_voxel_mesh_evidence(VoxelMeshEvidenceRequest {
                grid: 1,
                chunks: vec![ChunkCoord::new(0, 0, 0)],
            })
            .unwrap();
        assert_eq!(before.fixture_id, "basic-voxel-landscape-interaction");
        assert_eq!(before.world_hash, "27f89a36b51a8cb7");
        assert_eq!(before.meshing_strategy, "visible-face");
        assert_eq!(before.chunks.len(), 1);
        let before_chunk = &before.chunks[0];
        assert!(before_chunk.resident);
        assert!(before_chunk.visible);
        let before_hash = before_chunk.mesh_hash.clone().expect("mesh hash");
        assert_eq!(before_chunk.material_slots, vec![1]);
        assert_eq!(before_chunk.stats.unwrap().quads, 12);

        bridge
            .submit_commands(CommandBatch {
                commands: vec![set_voxel(VoxelCoord::new(1, 1, 1), 2)],
            })
            .unwrap();
        let after = bridge
            .read_voxel_mesh_evidence(VoxelMeshEvidenceRequest {
                grid: 1,
                chunks: vec![ChunkCoord::new(0, 0, 0)],
            })
            .unwrap();
        let after_chunk = &after.chunks[0];
        assert_ne!(after.world_hash, before.world_hash);
        assert_ne!(after_chunk.mesh_hash.as_ref().unwrap(), &before_hash);
        assert_eq!(after_chunk.material_slots, vec![1, 2]);
        assert!(after_chunk.stats.unwrap().quads > before_chunk.stats.unwrap().quads);
    }

    #[test]
    fn mesh_evidence_fails_closed_before_init_and_unknown_grid() {
        let bridge = ReferenceBridge::new();
        assert_eq!(
            bridge
                .read_voxel_mesh_evidence(VoxelMeshEvidenceRequest {
                    grid: 1,
                    chunks: Vec::new(),
                })
                .unwrap_err()
                .kind,
            RuntimeBridgeErrorKind::NotInitialized
        );

        let bridge = init_bridge();
        assert_eq!(
            bridge
                .read_voxel_mesh_evidence(VoxelMeshEvidenceRequest {
                    grid: 999,
                    chunks: Vec::new(),
                })
                .unwrap_err()
                .kind,
            RuntimeBridgeErrorKind::InvalidInput
        );
    }

    #[test]
    fn mixed_batch_accepts_valid_and_classifies_invalid_in_order() {
        let mut bridge = init_bridge();
        let result = bridge
            .submit_commands(CommandBatch {
                commands: vec![
                    set_voxel(VoxelCoord::new(1, 0, 0), 2), // resident, known material → accept
                    set_voxel(VoxelCoord::new(0, 0, 0), 77), // unknown material → reject
                ],
            })
            .unwrap();
        assert_eq!(result.accepted, 1);
        assert_eq!(result.rejected, 1);
        assert!(matches!(
            result.rejections[0],
            VoxelEditRejection::UnknownMaterial(_)
        ));
    }

    // ── Voxel picking → Rust authority raycast (launchable-voxel, #2437) ──

    /// A ray from x=-5 toward +X along y=0.5,z=0.5 — through voxel (0,0,0)'s span.
    fn pick_ray_plus_x() -> PickRay {
        PickRay {
            grid: 1,
            origin: [-5.0, 0.5, 0.5],
            direction: [1.0, 0.0, 0.0],
            max_distance: 100.0,
        }
    }

    #[test]
    fn pick_before_init_fails_closed() {
        let bridge = ReferenceBridge::new();
        let err = bridge.pick_voxel(pick_ray_plus_x()).unwrap_err();
        assert_eq!(err.kind, RuntimeBridgeErrorKind::NotInitialized);
    }

    #[test]
    fn pick_hits_solid_voxel_with_authoritative_face() {
        let mut bridge = init_bridge();
        bridge
            .submit_commands(CommandBatch {
                commands: vec![set_voxel(VoxelCoord::new(0, 0, 0), 1)],
            })
            .unwrap();
        match bridge.pick_voxel(pick_ray_plus_x()).unwrap() {
            PickResult::Hit(hit) => {
                assert_eq!(hit.grid, 1);
                assert_eq!(hit.voxel, VoxelCoord::new(0, 0, 0));
                assert_eq!(hit.chunk, ChunkCoord::new(0, 0, 0));
                // The +X-travelling ray strikes the voxel's -X face.
                assert_eq!(hit.face, Face::NegX);
                assert!((hit.distance - 5.0).abs() < 1e-6);
            }
            PickResult::Miss(r) => panic!("expected a hit, got {r:?}"),
        }
    }

    #[test]
    fn pick_empty_space_misses() {
        // The canonical launch terrain occupies z=0 only; a ray above the slab misses.
        let bridge = init_bridge();
        let mut ray = pick_ray_plus_x();
        ray.origin = [-5.0, 0.5, 1.5];
        assert_eq!(
            bridge.pick_voxel(ray).unwrap(),
            PickResult::Miss(PickRejection::NoHit)
        );
    }

    #[test]
    fn pick_unknown_grid_fails_closed() {
        let bridge = init_bridge();
        let mut ray = pick_ray_plus_x();
        ray.grid = 999;
        let err = bridge.pick_voxel(ray).unwrap_err();
        assert_eq!(err.kind, RuntimeBridgeErrorKind::InvalidInput);
    }

    #[test]
    fn buffer_view_round_trips_and_unknown_handle_errors() {
        let mut bridge = ReferenceBridge::new();
        bridge
            .initialize_engine(EngineConfig { seed: 0x01020304 })
            .unwrap();
        let view = bridge.get_buffer(RuntimeBufferHandle::new(0)).unwrap();
        assert_eq!(view.bytes, &0x01020304u64.to_le_bytes());
        let err = bridge.get_buffer(RuntimeBufferHandle::new(99)).unwrap_err();
        assert_eq!(err.kind, RuntimeBridgeErrorKind::UnknownHandle);
    }
}
