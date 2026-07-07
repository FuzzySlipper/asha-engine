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
use core_entity::{
    EntityLifecycleCommand, EntitySource, EntityStore, EntityTransform, TransformCommand,
    TransformError,
};
use core_error::ErrorCategory;
use core_ids::EntityId;
use core_math::Vec3;
use core_space::{
    ChunkCoord, ChunkDims, Direction6, Face, GridId, VoxelCoord, VoxelGridSpec, WorldPos, WorldVec,
};
use core_voxel::{MaterialCatalog, VoxelMaterialId, VoxelValue};
use protocol_diagnostics::DiagnosticSeverity;
use protocol_entity_authoring::{
    AuthoringTransform, EntityDefinition, EntityDefinitionCapability, EntityDefinitionSourceTrace,
};
#[cfg(test)]
use protocol_view::CameraCollisionPolicy;
use protocol_view::{
    CameraCollisionEvidence, CameraCollisionPolicyMode, CameraCollisionShape,
    CameraCollisionSnapshot, CameraCreateRequest, CameraPose, CameraProjectionRequest,
    CameraProjectionSnapshot, CameraSnapshot, CollisionAabbEvidence, CollisionAxis,
    CollisionConstrainedCameraInputEnvelope, FirstPersonCameraInput,
    FirstPersonCameraInputEnvelope, PickRaySnapshot, ScreenPoint, ScreenPointSpace,
    ScreenPointToPickRayRequest, ViewportSize, VoxelSelectionOutcome, VoxelSelectionSnapshot,
};
pub use protocol_voxel_conversion::{
    VoxelConversionApplyRequest, VoxelConversionDiagnostic, VoxelConversionDiagnosticCode,
    VoxelConversionEvidenceRef, VoxelConversionPlan, VoxelConversionPlanRequest,
    VoxelConversionPreview, VoxelConversionPreviewRequest, VoxelConversionReceipt,
};
use rule_lifecycle::{
    load_fps_project_bundle, FpsEncounterLastTransition,
    FpsEncounterLifecycleInput as RuleFpsEncounterLifecycleInput, FpsEncounterState,
    FpsEncounterStatus, FpsEncounterTransitionAction, FpsEncounterTransitionReceipt,
    FpsLifecycleStatus, FpsPolicyBinding, FpsPrimaryFireReceipt, FpsProjectBundleLoadInput,
    FpsRenderProjectionState, FpsRuntimeError, FpsRuntimeRole, FpsRuntimeSessionState,
    FpsStoredEntityDefinition, FpsWeaponMount,
};
use rule_voxel_edit::VoxelEditRejection;
use svc_collision::{CollisionProjection, Ray};
use svc_combat::HealthState;
use svc_mesh::mesh_chunk_in_world;
use svc_pathfinding::{
    propose_direct_nav_movement, DirectNavMovementError, DirectNavMovementRequest,
};
use svc_serialization::BundleHash;
use svc_spatial::VoxelWorld;
use svc_volume::VoxelChunk;
use svc_voxel_conversion::{MeshTriangle, PlannedConversion, StaticMeshSource};

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

/// Bounded request to apply an enemy direct-nav movement transaction.
///
/// `seed_position` is used only when this bridge session has not yet seen the
/// entity. Once seeded, Rust authority reads the current transform from its
/// [`EntityStore`] and ignores any stale caller-side position.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EnemyDirectNavMovementRequest {
    pub entity: u64,
    pub seed_position: Vec3,
    pub target: Vec3,
    pub max_step_units: f32,
}

/// Where the movement transaction read the starting transform from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnemyDirectNavAuthoritySource {
    SeededFromRequest,
    RustEntityStore,
}

impl EnemyDirectNavAuthoritySource {
    pub fn label(self) -> &'static str {
        match self {
            EnemyDirectNavAuthoritySource::SeededFromRequest => "seeded_from_request",
            EnemyDirectNavAuthoritySource::RustEntityStore => "rust_entity_store",
        }
    }
}

/// Result of a Rust-owned enemy direct-nav movement application.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EnemyDirectNavMovementResult {
    pub entity: u64,
    pub authority_source: EnemyDirectNavAuthoritySource,
    pub from: Vec3,
    pub target: Vec3,
    pub next_waypoint: Vec3,
    pub distance_units: f32,
    pub reached: bool,
    pub path_hash: u64,
    pub transform_hash: u64,
    pub projection_changed: bool,
}

/// Why an enemy direct-nav movement transaction was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnemyDirectNavMovementError {
    InvalidEntity,
    Navigation(DirectNavMovementError),
    Transform(TransformError),
}

impl EnemyDirectNavMovementError {
    pub fn label(self) -> &'static str {
        match self {
            EnemyDirectNavMovementError::InvalidEntity => "invalidEntity",
            EnemyDirectNavMovementError::Navigation(error) => error.label(),
            EnemyDirectNavMovementError::Transform(error) => error.label(),
        }
    }
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

// ── FPS/ECRP RuntimeSession authority payloads (#4347) ───────────────────────
//
// These DTOs are the narrow public bridge shape for the FPS RuntimeSession
// substrate. Stored definitions enter as typed data, Rust validates/bootstrap
// them through rule-lifecycle/svc-entity-authoring, and readouts project typed
// authority state. There is no generic ProjectBundle JSON tunnel here.

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FpsBridgeRole {
    Player,
    Enemy,
    Neutral,
}

impl FpsBridgeRole {
    pub fn label(self) -> &'static str {
        match self {
            FpsBridgeRole::Player => "player",
            FpsBridgeRole::Enemy => "enemy",
            FpsBridgeRole::Neutral => "neutral",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FpsBridgeTransformCapability {
    pub translation: [f32; 3],
    pub rotation: [f32; 4],
    pub scale: [f32; 3],
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FpsBridgeBoundsCapability {
    pub min: [f32; 3],
    pub max: [f32; 3],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FpsBridgeHealth {
    pub current: u32,
    pub max: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FpsBridgeWeaponMount {
    pub weapon_id: String,
    pub damage: u32,
    pub range_units: u32,
    pub ammo: u32,
    pub cooldown_ticks_after_fire: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FpsBridgePolicyBinding {
    pub binding_id: String,
    pub policy_id: String,
    pub view_kind: String,
    pub view_version: String,
    pub allowed_intents: Vec<String>,
    pub runtime_moment: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FpsBridgeStoredEntityDefinition {
    pub entity: u64,
    pub stable_id: String,
    pub display_name: String,
    pub source_path: String,
    pub tags: Vec<String>,
    pub role: FpsBridgeRole,
    pub transform: Option<FpsBridgeTransformCapability>,
    pub bounds: Option<FpsBridgeBoundsCapability>,
    pub render_visible: Option<bool>,
    pub static_collider: Option<bool>,
    pub health: Option<FpsBridgeHealth>,
    pub weapon: Option<FpsBridgeWeaponMount>,
    pub policy_binding: Option<FpsBridgePolicyBinding>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FpsRuntimeSessionLoadRequest {
    pub project_bundle: String,
    pub definitions: Vec<FpsBridgeStoredEntityDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FpsRuntimeSessionRestartRequest {
    pub expected_epoch: u64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FpsPrimaryFireRequest {
    pub tick: u64,
    pub origin: [f64; 3],
    pub direction: [f64; 3],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FpsReadSetEvidence {
    pub view_kind: String,
    pub owner: String,
    pub read_set: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FpsReplayEvidence {
    pub replay_unit: String,
    pub entity_hash: u64,
    pub health_hash: u64,
    pub record_hash: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FpsEntityHealthReadout {
    pub entity: u64,
    pub current: u32,
    pub max: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FpsPolicyBindingReadout {
    pub entity: u64,
    pub binding_id: String,
    pub policy_id: String,
    pub view_kind: String,
    pub view_version: String,
    pub allowed_intents: Vec<String>,
    pub runtime_moment: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FpsBridgeLifecycleStatus {
    Active,
    EnemyDefeated { entity: u64, tick: u64 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FpsRuntimeSessionSnapshot {
    pub backend: String,
    pub authority_surface: String,
    pub project_bundle: String,
    pub session_epoch: u64,
    pub lifecycle_status: FpsBridgeLifecycleStatus,
    pub player_entity: u64,
    pub enemy_entity: u64,
    pub health: Vec<FpsEntityHealthReadout>,
    pub policy_bindings: Vec<FpsPolicyBindingReadout>,
    pub replay_records: Vec<FpsReplayEvidence>,
    pub read_sets: Vec<FpsReadSetEvidence>,
    pub entity_hash: u64,
    pub health_hash: u64,
    pub replay_hash: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FpsPrimaryFireResult {
    pub backend: String,
    pub authority_surface: String,
    pub mutation_owner: String,
    pub workspace_trace: Vec<String>,
    pub shooter: u64,
    pub target: Option<u64>,
    pub target_health_before: Option<FpsBridgeHealth>,
    pub target_health_after: Option<FpsBridgeHealth>,
    pub lifecycle_status: FpsBridgeLifecycleStatus,
    pub target_render_visible: Option<bool>,
    pub entity_hash: u64,
    pub health_hash: u64,
    pub replay_hash: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FpsEncounterLifecycleInput {
    pub outcome_kind: String,
    pub terminal: bool,
    pub enemy_dead: bool,
    pub player_dead: bool,
    pub lifecycle_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FpsEncounterTransitionRequest {
    pub preset_id: String,
    pub action: String,
    pub lifecycle: FpsEncounterLifecycleInput,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FpsEncounterStateReadout {
    pub preset_id: String,
    pub status: String,
    pub spawned_enemy_ids: Vec<String>,
    pub defeated_enemy_ids: Vec<String>,
    pub revision: u64,
    pub last_transition: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FpsEncounterDirectorSnapshot {
    pub backend: String,
    pub authority_surface: String,
    pub mutation_owner: String,
    pub workspace_trace: Vec<String>,
    pub state: FpsEncounterStateReadout,
    pub lifecycle: FpsEncounterLifecycleInput,
    pub read_sets: Vec<FpsReadSetEvidence>,
    pub encounter_hash: u64,
    pub replay_hash: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FpsEncounterTransitionResult {
    pub backend: String,
    pub authority_surface: String,
    pub mutation_owner: String,
    pub workspace_trace: Vec<String>,
    pub accepted: bool,
    pub rejection_reason: Option<String>,
    pub event_kind: Option<String>,
    pub state: FpsEncounterStateReadout,
    pub lifecycle: FpsEncounterLifecycleInput,
    pub encounter_hash: u64,
    pub replay_hash: u64,
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
    /// Apply first-person view input while constraining translation against the
    /// authority-derived voxel collision projection.
    fn apply_collision_constrained_camera_input(
        &mut self,
        input: CollisionConstrainedCameraInputEnvelope,
    ) -> BridgeResult<CameraCollisionSnapshot>;
    /// Derive a camera/projection-sourced ray and authority selection evidence.
    fn select_voxel(
        &self,
        request: ScreenPointToPickRayRequest,
    ) -> BridgeResult<VoxelSelectionSnapshot>;
    /// Read compact deterministic voxel mesh evidence for resident/requested chunks.
    /// This summarizes authority-derived `svc-mesh` output with hashes/stats, not
    /// renderer-owned objects or inline Three.js geometry.
    fn read_voxel_mesh_evidence(
        &self,
        request: VoxelMeshEvidenceRequest,
    ) -> BridgeResult<VoxelMeshEvidenceSnapshot>;
    /// Plan a bounded static-mesh to voxel conversion through Rust authority.
    /// The request/response are generated protocol DTOs; no Studio-specific
    /// transport or renderer buffer shape crosses this boundary.
    fn plan_voxel_conversion(
        &mut self,
        request: VoxelConversionPlanRequest,
    ) -> BridgeResult<VoxelConversionPlan>;
    /// Preview the most recently planned conversion, guarded by the plan hash.
    fn preview_voxel_conversion(
        &mut self,
        request: VoxelConversionPreviewRequest,
    ) -> BridgeResult<VoxelConversionPreview>;
    /// Apply the current conversion output into voxel authority via the existing
    /// generated voxel command path, guarded by plan/preview hashes.
    fn apply_voxel_conversion(
        &mut self,
        request: VoxelConversionApplyRequest,
    ) -> BridgeResult<VoxelConversionReceipt>;
    /// Export selected evidence refs from the current conversion authority state.
    fn export_voxel_conversion_evidence(
        &self,
        evidence: Vec<VoxelConversionEvidenceRef>,
    ) -> BridgeResult<Vec<VoxelConversionEvidenceRef>>;
    /// Load an FPS/ECRP ProjectBundle-shaped session through Rust authority.
    /// Stored definitions are validated/bootstraped by rule-lifecycle and
    /// svc-entity-authoring; failure leaves any prior FPS session untouched.
    fn load_fps_runtime_session(
        &mut self,
        request: FpsRuntimeSessionLoadRequest,
    ) -> BridgeResult<FpsRuntimeSessionSnapshot>;
    /// Read typed FPS/ECRP RuntimeSession projection from Rust authority.
    fn read_fps_runtime_session(&self) -> BridgeResult<FpsRuntimeSessionSnapshot>;
    /// Submit a primary-fire intent. Rust owns combat, lifecycle, replay/hash,
    /// and render-visibility effects; callers receive projection evidence only.
    fn apply_fps_primary_fire(
        &mut self,
        request: FpsPrimaryFireRequest,
    ) -> BridgeResult<FpsPrimaryFireResult>;
    /// Restart the FPS/ECRP session by replaying the validated stored bundle into
    /// a fresh authority session, guarded by the caller's current epoch.
    fn restart_fps_runtime_session(
        &mut self,
        request: FpsRuntimeSessionRestartRequest,
    ) -> BridgeResult<FpsRuntimeSessionSnapshot>;
    /// Read the Rust-owned encounter/spawn director projection for the loaded
    /// FPS/ECRP RuntimeSession. Configuration is descriptive; transition state
    /// and hashes come from rule-lifecycle authority.
    fn read_fps_encounter_director(
        &self,
        lifecycle: FpsEncounterLifecycleInput,
    ) -> BridgeResult<FpsEncounterDirectorSnapshot>;
    /// Apply a Rust-owned encounter transition for the loaded FPS/ECRP session.
    fn apply_fps_encounter_transition(
        &mut self,
        request: FpsEncounterTransitionRequest,
    ) -> BridgeResult<FpsEncounterTransitionResult>;
    fn create_camera(&mut self, request: CameraCreateRequest) -> BridgeResult<CameraSnapshot>;
    fn apply_first_person_camera_input(
        &mut self,
        input: FirstPersonCameraInputEnvelope,
    ) -> BridgeResult<CameraSnapshot>;
    /// Apply a Rust-owned enemy direct-nav movement transaction. The operation
    /// combines the `svc-pathfinding` direct-nav proposal with `core-entity`
    /// transform authority so callers receive projection evidence instead of
    /// mutating runtime transforms themselves.
    fn apply_enemy_direct_nav_movement(
        &mut self,
        request: EnemyDirectNavMovementRequest,
    ) -> BridgeResult<EnemyDirectNavMovementResult>;
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
    /// Minimal authority-owned runtime entity state for bridge-level actor
    /// movement verbs. TypeScript may propose targets, but transform mutation is
    /// applied here through `core-entity`.
    entities: EntityStore,
    /// FPS/ECRP RuntimeSession authority state. Stored definitions seed this
    /// through rule-lifecycle; TS callers only receive typed readouts/receipts.
    fps_session: Option<FpsRuntimeSessionState>,
    fps_seed: Option<FpsRuntimeSessionLoadRequest>,
    fps_epoch: u64,
    /// Last planned voxel conversion. This is bridge-owned authority state used
    /// by preview/apply hash guards; callers cannot provide their own output.
    voxel_conversion_plan: Option<PlannedConversion>,
    voxel_conversion_evidence: Vec<VoxelConversionEvidenceRef>,
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

    fn fixture_quad_source() -> StaticMeshSource {
        StaticMeshSource {
            asset_id: "mesh/quad".to_string(),
            asset_kind: "mesh".to_string(),
            asset_version: 1,
            source_hash: "sha256:quad".to_string(),
            mesh_primitive: None,
            positions: vec![
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [1.0, 1.0, 0.0],
                [0.0, 1.0, 0.0],
            ],
            triangles: vec![
                MeshTriangle {
                    indices: [0, 1, 2],
                    source_material_slot: 0,
                },
                MeshTriangle {
                    indices: [0, 2, 3],
                    source_material_slot: 1,
                },
            ],
        }
    }

    fn source_for_voxel_conversion(_request: &VoxelConversionPlanRequest) -> StaticMeshSource {
        // Current bridge-owned fixture source. The conversion service validates
        // the caller's source ref against it and emits typed diagnostics for
        // unsupported source identity/hash mismatch instead of TS deciding.
        Self::fixture_quad_source()
    }

    fn voxel_conversion_diagnostic(
        code: VoxelConversionDiagnosticCode,
        reference: impl Into<String>,
        message: impl Into<String>,
    ) -> VoxelConversionDiagnostic {
        VoxelConversionDiagnostic {
            code,
            severity: DiagnosticSeverity::Error,
            reference: reference.into(),
            message: message.into(),
        }
    }

    fn rejected_voxel_conversion_receipt(
        plan_id: String,
        diagnostics: Vec<VoxelConversionDiagnostic>,
    ) -> VoxelConversionReceipt {
        VoxelConversionReceipt {
            plan_id,
            applied: false,
            output_hash: None,
            output_voxel_count: 0,
            output_bounds: None,
            diagnostics,
            evidence: Vec::new(),
        }
    }

    fn conversion_commands(planned: &PlannedConversion) -> BridgeResult<Option<CommandBatch>> {
        let Some(output) = &planned.output else {
            return Ok(None);
        };
        let grid = u32::try_from(planned.plan.target.grid).map_err(|_| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                "voxel conversion target grid must fit in u32",
            )
        })?;
        let commands = output
            .voxels
            .iter()
            .map(|voxel| {
                let material = voxel.value.material().ok_or_else(|| {
                    RuntimeBridgeError::new(
                        RuntimeBridgeErrorKind::Internal,
                        "voxel conversion output contained a non-solid voxel",
                    )
                })?;
                Ok(set_voxel_command(
                    grid,
                    voxel.coord.x,
                    voxel.coord.y,
                    voxel.coord.z,
                    material.raw(),
                ))
            })
            .collect::<BridgeResult<Vec<_>>>()?;
        Ok(Some(CommandBatch { commands }))
    }

    fn remember_voxel_conversion_evidence(
        &mut self,
        refs: impl IntoIterator<Item = VoxelConversionEvidenceRef>,
    ) {
        for evidence in refs {
            if !self.voxel_conversion_evidence.contains(&evidence) {
                self.voxel_conversion_evidence.push(evidence);
            }
        }
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

    fn fps_runtime_error(error: FpsRuntimeError) -> RuntimeBridgeError {
        RuntimeBridgeError::new(
            RuntimeBridgeErrorKind::InvalidInput,
            format!("FPS RuntimeSession authority rejected request: {error:?}"),
        )
    }

    fn fps_session(&self, op: &str) -> BridgeResult<&FpsRuntimeSessionState> {
        self.fps_session.as_ref().ok_or_else(|| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::NotInitialized,
                format!("{op} called before load_fps_runtime_session"),
            )
        })
    }

    fn fps_session_mut(&mut self, op: &str) -> BridgeResult<&mut FpsRuntimeSessionState> {
        self.fps_session.as_mut().ok_or_else(|| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::NotInitialized,
                format!("{op} called before load_fps_runtime_session"),
            )
        })
    }

    fn convert_fps_load_request(
        request: &FpsRuntimeSessionLoadRequest,
    ) -> BridgeResult<FpsProjectBundleLoadInput> {
        let mut definitions = Vec::with_capacity(request.definitions.len());
        for entry in &request.definitions {
            let entity = EntityId::new(entry.entity);
            let mut capabilities = Vec::new();
            if let Some(transform) = &entry.transform {
                capabilities.push(EntityDefinitionCapability::Transform {
                    transform: AuthoringTransform {
                        translation: transform.translation,
                        rotation: transform.rotation,
                        scale: transform.scale,
                    },
                });
            }
            if let Some(bounds) = entry.bounds {
                capabilities.push(EntityDefinitionCapability::Bounds {
                    min: bounds.min,
                    max: bounds.max,
                });
            }
            if let Some(visible) = entry.render_visible {
                capabilities.push(EntityDefinitionCapability::Render { visible });
            }
            if let Some(static_collider) = entry.static_collider {
                capabilities.push(EntityDefinitionCapability::Collision { static_collider });
            }

            definitions.push(FpsStoredEntityDefinition {
                entity,
                definition: EntityDefinition {
                    stable_id: entry.stable_id.clone(),
                    display_name: entry.display_name.clone(),
                    source: EntityDefinitionSourceTrace {
                        project_bundle: request.project_bundle.clone(),
                        relative_path: entry.source_path.clone(),
                    },
                    tags: Vec::new(),
                    metadata: Vec::new(),
                    capabilities,
                },
                role: match entry.role {
                    FpsBridgeRole::Player => FpsRuntimeRole::Player,
                    FpsBridgeRole::Enemy => FpsRuntimeRole::Enemy,
                    FpsBridgeRole::Neutral => FpsRuntimeRole::Neutral,
                },
                health: entry
                    .health
                    .map(|health| HealthState::new(health.current, health.max)),
                weapon: entry.weapon.as_ref().map(|weapon| FpsWeaponMount {
                    weapon_id: weapon.weapon_id.clone(),
                    damage: weapon.damage,
                    range_units: weapon.range_units,
                    ammo: weapon.ammo,
                    cooldown_ticks_after_fire: weapon.cooldown_ticks_after_fire,
                }),
                render_projection: entry
                    .render_visible
                    .map(|visible| FpsRenderProjectionState {
                        projection: match entry.role {
                            FpsBridgeRole::Player => "first_person_camera",
                            FpsBridgeRole::Enemy => "target_actor",
                            FpsBridgeRole::Neutral => "neutral_actor",
                        }
                        .to_string(),
                        visible,
                    }),
                policy_binding: entry
                    .policy_binding
                    .as_ref()
                    .map(|binding| FpsPolicyBinding {
                        binding_id: binding.binding_id.clone(),
                        policy_id: binding.policy_id.clone(),
                        view_kind: binding.view_kind.clone(),
                        view_version: binding.view_version.clone(),
                        allowed_intents: binding.allowed_intents.clone(),
                        runtime_moment: binding.runtime_moment.clone(),
                    }),
            });
        }

        Ok(FpsProjectBundleLoadInput {
            project_bundle: request.project_bundle.clone(),
            definitions,
        })
    }

    fn fps_lifecycle_status(status: FpsLifecycleStatus) -> FpsBridgeLifecycleStatus {
        match status {
            FpsLifecycleStatus::Active => FpsBridgeLifecycleStatus::Active,
            FpsLifecycleStatus::EnemyDefeated { entity, tick } => {
                FpsBridgeLifecycleStatus::EnemyDefeated {
                    entity: entity.raw(),
                    tick,
                }
            }
        }
    }

    fn fps_read_sets() -> Vec<FpsReadSetEvidence> {
        vec![
            FpsReadSetEvidence {
                view_kind: "runtime_session.lifecycle.v0".to_string(),
                owner: "rule-lifecycle".to_string(),
                read_set: vec![
                    "EntityStore.lifecycle".to_string(),
                    "FpsRuntimeSessionState.lifecycle_status".to_string(),
                ],
            },
            FpsReadSetEvidence {
                view_kind: "runtime_session.health.v0".to_string(),
                owner: "svc-combat".to_string(),
                read_set: vec![
                    "CombatState.health".to_string(),
                    "CombatState.health_hash".to_string(),
                ],
            },
            FpsReadSetEvidence {
                view_kind: "runtime_session.policy_binding.v0".to_string(),
                owner: "rule-lifecycle".to_string(),
                read_set: vec!["FpsStoredEntityDefinition.policy_binding".to_string()],
            },
            FpsReadSetEvidence {
                view_kind: "runtime_session.replay.v0".to_string(),
                owner: "rule-lifecycle".to_string(),
                read_set: vec!["FpsRuntimeSessionState.replay_records".to_string()],
            },
        ]
    }

    fn fps_encounter_read_sets() -> Vec<FpsReadSetEvidence> {
        vec![
            FpsReadSetEvidence {
                view_kind: "runtime_session.encounter_director.v0".to_string(),
                owner: "rule-lifecycle".to_string(),
                read_set: vec![
                    "FpsRuntimeSessionState.encounter".to_string(),
                    "FpsRuntimeSessionState.lifecycle_status".to_string(),
                ],
            },
            FpsReadSetEvidence {
                view_kind: "runtime_session.encounter_replay.v0".to_string(),
                owner: "rule-lifecycle".to_string(),
                read_set: vec!["FpsRuntimeSessionState.replay_records".to_string()],
            },
        ]
    }

    fn bridge_encounter_lifecycle(
        lifecycle: FpsEncounterLifecycleInput,
    ) -> RuleFpsEncounterLifecycleInput {
        RuleFpsEncounterLifecycleInput {
            outcome_kind: lifecycle.outcome_kind,
            terminal: lifecycle.terminal,
            enemy_dead: lifecycle.enemy_dead,
            player_dead: lifecycle.player_dead,
            lifecycle_hash: lifecycle.lifecycle_hash,
        }
    }

    fn bridge_encounter_state(state: &FpsEncounterState) -> FpsEncounterStateReadout {
        FpsEncounterStateReadout {
            preset_id: state.preset_id.clone(),
            status: Self::encounter_status_label(state.status).to_string(),
            spawned_enemy_ids: state.spawned_enemy_ids.clone(),
            defeated_enemy_ids: state.defeated_enemy_ids.clone(),
            revision: state.revision,
            last_transition: Self::encounter_last_transition_label(state.last_transition)
                .to_string(),
        }
    }

    fn encounter_status_label(status: FpsEncounterStatus) -> &'static str {
        match status {
            FpsEncounterStatus::Pending => "pending",
            FpsEncounterStatus::Active => "active",
            FpsEncounterStatus::Cleared => "cleared",
            FpsEncounterStatus::Failed => "failed",
        }
    }

    fn encounter_last_transition_label(transition: FpsEncounterLastTransition) -> &'static str {
        match transition {
            FpsEncounterLastTransition::Initialized => "initialized",
            FpsEncounterLastTransition::Activated => "activated",
            FpsEncounterLastTransition::Cleared => "cleared",
            FpsEncounterLastTransition::Failed => "failed",
            FpsEncounterLastTransition::Reset => "reset",
        }
    }

    fn encounter_action(action: &str) -> BridgeResult<FpsEncounterTransitionAction> {
        match action {
            "activate" => Ok(FpsEncounterTransitionAction::Activate),
            "sync_lifecycle" => Ok(FpsEncounterTransitionAction::SyncLifecycle),
            "reset" => Ok(FpsEncounterTransitionAction::Reset),
            other => Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                format!("unknown FPS encounter transition action '{other}'"),
            )),
        }
    }

    fn encounter_hash(state: &FpsEncounterState, lifecycle: &FpsEncounterLifecycleInput) -> u64 {
        let key = format!(
            "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
            state.preset_id,
            Self::encounter_status_label(state.status),
            state.spawned_enemy_ids.join(","),
            state.defeated_enemy_ids.join(","),
            state.revision,
            Self::encounter_last_transition_label(state.last_transition),
            lifecycle.outcome_kind,
            lifecycle.terminal,
            lifecycle.enemy_dead,
            lifecycle.player_dead,
            lifecycle.lifecycle_hash
        );
        u64::from_str_radix(&Self::fnv1a64(&key), 16).expect("fnv1a64 emits hex")
    }

    fn encounter_snapshot(
        session: &FpsRuntimeSessionState,
        lifecycle: FpsEncounterLifecycleInput,
    ) -> FpsEncounterDirectorSnapshot {
        let latest = session.replay_records.last();
        let encounter_hash = Self::encounter_hash(&session.encounter, &lifecycle);
        FpsEncounterDirectorSnapshot {
            backend: "reference_bridge_rust".to_string(),
            authority_surface: "runtime_session.fps.encounter_director.v0".to_string(),
            mutation_owner: "rule-lifecycle".to_string(),
            workspace_trace: vec!["projected encounter state from rule-lifecycle".to_string()],
            state: Self::bridge_encounter_state(&session.encounter),
            lifecycle,
            read_sets: Self::fps_encounter_read_sets(),
            encounter_hash,
            replay_hash: latest
                .map(|record| record.record_hash)
                .unwrap_or(encounter_hash),
        }
    }

    fn encounter_transition_result(
        receipt: FpsEncounterTransitionReceipt,
        lifecycle: FpsEncounterLifecycleInput,
    ) -> FpsEncounterTransitionResult {
        FpsEncounterTransitionResult {
            backend: "reference_bridge_rust".to_string(),
            authority_surface: "runtime_session.fps.encounter_transition.v0".to_string(),
            mutation_owner: "rule-lifecycle".to_string(),
            workspace_trace: vec![
                "validated encounter transition against rule-lifecycle".to_string(),
                "serialized accepted encounter transition into replay evidence".to_string(),
            ],
            accepted: receipt.accepted,
            rejection_reason: receipt.rejection_reason.map(str::to_string),
            event_kind: receipt.event_kind.map(str::to_string),
            state: Self::bridge_encounter_state(&receipt.state),
            lifecycle,
            encounter_hash: receipt.encounter_hash,
            replay_hash: receipt.replay_hash,
        }
    }

    fn fps_snapshot(
        session: &FpsRuntimeSessionState,
        epoch: u64,
    ) -> BridgeResult<FpsRuntimeSessionSnapshot> {
        let player = session
            .role_entity(FpsRuntimeRole::Player)
            .map_err(Self::fps_runtime_error)?;
        let enemy = session
            .role_entity(FpsRuntimeRole::Enemy)
            .map_err(Self::fps_runtime_error)?;
        let mut health = Vec::new();
        let mut policy_bindings = Vec::new();
        for (entity, definition) in &session.definitions {
            if let Some(state) = session.health(*entity) {
                health.push(FpsEntityHealthReadout {
                    entity: entity.raw(),
                    current: state.current,
                    max: state.max,
                });
            }
            if let Some(binding) = &definition.policy_binding {
                policy_bindings.push(FpsPolicyBindingReadout {
                    entity: entity.raw(),
                    binding_id: binding.binding_id.clone(),
                    policy_id: binding.policy_id.clone(),
                    view_kind: binding.view_kind.clone(),
                    view_version: binding.view_version.clone(),
                    allowed_intents: binding.allowed_intents.clone(),
                    runtime_moment: binding.runtime_moment.clone(),
                });
            }
        }
        let replay_records = session
            .replay_records
            .iter()
            .map(|record| FpsReplayEvidence {
                replay_unit: record.kind.to_string(),
                entity_hash: record.entity_hash,
                health_hash: record.health_hash,
                record_hash: record.record_hash,
            })
            .collect::<Vec<_>>();
        let latest = session.replay_records.last();
        Ok(FpsRuntimeSessionSnapshot {
            backend: "reference_bridge_rust".to_string(),
            authority_surface: "runtime_session.fps.authority.v0".to_string(),
            project_bundle: session.project_bundle.clone(),
            session_epoch: epoch,
            lifecycle_status: Self::fps_lifecycle_status(session.lifecycle_status),
            player_entity: player.raw(),
            enemy_entity: enemy.raw(),
            health,
            policy_bindings,
            replay_records,
            read_sets: Self::fps_read_sets(),
            entity_hash: session.entities.hash().0,
            health_hash: session.combat.health_hash(),
            replay_hash: latest.map(|record| record.record_hash).unwrap_or(0),
        })
    }

    fn bridge_health(state: HealthState) -> FpsBridgeHealth {
        FpsBridgeHealth {
            current: state.current,
            max: state.max,
        }
    }

    fn primary_fire_result(receipt: FpsPrimaryFireReceipt) -> FpsPrimaryFireResult {
        FpsPrimaryFireResult {
            backend: "reference_bridge_rust".to_string(),
            authority_surface: "runtime_session.fps.primary_fire.v0".to_string(),
            mutation_owner: "rule-lifecycle + svc-combat".to_string(),
            workspace_trace: vec![
                "validated FireIntentCommand against svc-combat".to_string(),
                "serialized accepted combat/lifecycle outcome into replay evidence".to_string(),
            ],
            shooter: receipt.shooter.raw(),
            target: receipt.target.map(EntityId::raw),
            target_health_before: receipt.target_health_before.map(Self::bridge_health),
            target_health_after: receipt.target_health_after.map(Self::bridge_health),
            lifecycle_status: Self::fps_lifecycle_status(receipt.lifecycle_status),
            target_render_visible: receipt.target_render_visible,
            entity_hash: receipt.entity_hash,
            health_hash: receipt.health_hash,
            replay_hash: receipt.replay_hash,
        }
    }

    fn ray_from_primary_fire(request: FpsPrimaryFireRequest) -> BridgeResult<Ray> {
        if !request.origin.iter().all(|value| value.is_finite())
            || !request.direction.iter().all(|value| value.is_finite())
        {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                "primary fire origin/direction must be finite",
            ));
        }
        Ok(Ray::new(
            WorldPos::new(request.origin[0], request.origin[1], request.origin[2]),
            WorldVec::new(
                request.direction[0],
                request.direction[1],
                request.direction[2],
            ),
        ))
    }

    fn enemy_entity_id(raw: u64) -> BridgeResult<EntityId> {
        if raw == 0 {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                EnemyDirectNavMovementError::InvalidEntity.label(),
            ));
        }
        Ok(EntityId::new(raw))
    }

    fn seed_or_read_enemy_transform(
        entities: &mut EntityStore,
        entity: EntityId,
        seed_position: Vec3,
    ) -> BridgeResult<(EnemyDirectNavAuthoritySource, EntityTransform)> {
        if let Some(transform) = entities.transform(entity) {
            return Ok((
                EnemyDirectNavAuthoritySource::RustEntityStore,
                transform.transform,
            ));
        }
        entities
            .apply(EntityLifecycleCommand::Create {
                id: entity,
                source: EntitySource::RuntimeCreated { by: None },
                labels: Vec::new(),
            })
            .map_err(|err| {
                RuntimeBridgeError::new(
                    RuntimeBridgeErrorKind::InvalidInput,
                    format!("enemy direct-nav entity seed rejected: {err}"),
                )
            })?;
        let transform = EntityTransform::at(seed_position);
        let attached = entities.attach_transform(entity, transform);
        debug_assert!(attached);
        Ok((EnemyDirectNavAuthoritySource::SeededFromRequest, transform))
    }

    fn transform_hash(entity: EntityId, transform: EntityTransform) -> u64 {
        let key = format!(
            "{}|{:.3},{:.3},{:.3}|{:.3},{:.3},{:.3},{:.3}|{:.3},{:.3},{:.3}",
            entity.raw(),
            transform.translation.x,
            transform.translation.y,
            transform.translation.z,
            transform.rotation.x,
            transform.rotation.y,
            transform.rotation.z,
            transform.rotation.w,
            transform.scale.x,
            transform.scale.y,
            transform.scale.z
        );
        u64::from_str_radix(&Self::fnv1a64(&key), 16).expect("fnv1a64 emits hex")
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

    fn validate_camera_input(input: FirstPersonCameraInput) -> BridgeResult<()> {
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
                "camera input values must be finite; dt_seconds and move_speed_units_per_second must be non-negative",
            ));
        }
        Ok(())
    }

    fn integrate_camera_snapshot(
        prior: CameraSnapshot,
        input: FirstPersonCameraInput,
        tick: u64,
    ) -> CameraSnapshot {
        let distance = input.dt_seconds * input.move_speed_units_per_second;
        let basis = prior.basis;
        let pose = CameraPose {
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
        CameraSnapshot {
            tick,
            pose,
            basis: Self::basis_from_pose(pose),
            ..prior
        }
    }

    fn aabb_for_pose(pose: CameraPose, shape: CameraCollisionShape) -> (WorldPos, WorldPos) {
        let p = pose.position;
        let h = shape.half_extents;
        (
            WorldPos::new(
                (p[0] - h[0]) as f64,
                (p[1] - h[1]) as f64,
                (p[2] - h[2]) as f64,
            ),
            WorldPos::new(
                (p[0] + h[0]) as f64,
                (p[1] + h[1]) as f64,
                (p[2] + h[2]) as f64,
            ),
        )
    }

    fn validate_collision_shape(shape: CameraCollisionShape) -> BridgeResult<()> {
        if !shape.half_extents.iter().all(|v| v.is_finite() && *v > 0.0) {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                "collision shape half_extents must be finite positive values",
            ));
        }
        Ok(())
    }

    fn collision_projection_hash(world: &VoxelWorld, projection: &CollisionProjection) -> String {
        let chunks = projection
            .collider_chunks()
            .map(|coord| format!("{},{},{}", coord.x, coord.y, coord.z))
            .collect::<Vec<_>>()
            .join(";");
        let key = format!(
            "{}|v{}|n{}|{}",
            Self::world_hash(world),
            projection.version(),
            projection.collider_count(),
            chunks
        );
        format!("fnv1a64:{}", Self::fnv1a64(&key))
    }

    fn screen_point_to_normalized(
        point: ScreenPoint,
        viewport: ViewportSize,
    ) -> BridgeResult<(f32, f32)> {
        if !point.x.is_finite() || !point.y.is_finite() {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                "screen point coordinates must be finite",
            ));
        }
        match point.space {
            ScreenPointSpace::Normalized01 => Ok((point.x, point.y)),
            ScreenPointSpace::Pixel => Ok((
                point.x / viewport.width as f32,
                point.y / viewport.height as f32,
            )),
        }
    }

    fn pick_ray_snapshot(
        snapshot: CameraSnapshot,
        request: ScreenPointToPickRayRequest,
    ) -> BridgeResult<PickRaySnapshot> {
        let viewport = request.viewport.unwrap_or(snapshot.viewport);
        Self::validate_viewport(viewport)?;
        if !request.max_distance.is_finite() || request.max_distance <= 0.0 {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                "max_distance must be finite and positive",
            ));
        }
        let (sx, sy) = Self::screen_point_to_normalized(request.screen_point, viewport)?;
        if !(0.0..=1.0).contains(&sx) || !(0.0..=1.0).contains(&sy) {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                "screen point must be inside the viewport",
            ));
        }
        let ndc_x = sx * 2.0 - 1.0;
        let ndc_y = 1.0 - sy * 2.0;
        let aspect = viewport.width as f32 / viewport.height as f32;
        let tan_y = (snapshot.projection.fov_y_degrees.to_radians() / 2.0).tan();
        let tan_x = tan_y * aspect;
        let f = snapshot.basis.forward;
        let r = snapshot.basis.right;
        let u = snapshot.basis.up;
        let raw = [
            f[0] + r[0] * ndc_x * tan_x + u[0] * ndc_y * tan_y,
            f[1] + r[1] * ndc_x * tan_x + u[1] * ndc_y * tan_y,
            f[2] + r[2] * ndc_x * tan_x + u[2] * ndc_y * tan_y,
        ];
        let len = (raw[0] * raw[0] + raw[1] * raw[1] + raw[2] * raw[2]).sqrt();
        if !len.is_finite() || len <= 0.0 {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                "derived pick ray direction is invalid",
            ));
        }
        let dir = [raw[0] / len, raw[1] / len, raw[2] / len];
        let ray = PickRay {
            grid: request.grid,
            origin: [
                snapshot.pose.position[0] as f64,
                snapshot.pose.position[1] as f64,
                snapshot.pose.position[2] as f64,
            ],
            direction: [dir[0] as f64, dir[1] as f64, dir[2] as f64],
            max_distance: request.max_distance,
        };
        let projection_hash = Self::projection_snapshot(snapshot, viewport).projection_hash;
        let ray_hash = format!(
            "fnv1a64:{}",
            Self::fnv1a64(&format!(
                "{}|{}|{:.6},{:.6},{:.6}|{:.6},{:.6},{:.6}|{:.6}|{}",
                snapshot.camera.raw(),
                request.grid,
                ray.origin[0],
                ray.origin[1],
                ray.origin[2],
                ray.direction[0],
                ray.direction[1],
                ray.direction[2],
                ray.max_distance,
                projection_hash
            ))
        );
        Ok(PickRaySnapshot {
            camera: snapshot.camera,
            tick: snapshot.tick,
            grid: request.grid,
            screen_point: request.screen_point,
            origin: ray.origin,
            direction: ray.direction,
            max_distance: ray.max_distance,
            camera_projection_hash: projection_hash,
            ray_hash,
        })
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
        self.fps_session = None;
        self.fps_seed = None;
        self.fps_epoch = 0;
        self.voxel_conversion_plan = None;
        self.voxel_conversion_evidence.clear();

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

    fn apply_collision_constrained_camera_input(
        &mut self,
        envelope: CollisionConstrainedCameraInputEnvelope,
    ) -> BridgeResult<CameraCollisionSnapshot> {
        let world = self.voxel.as_ref().ok_or_else(|| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::NotInitialized,
                "apply_collision_constrained_camera_input called before initialize_engine",
            )
        })?;
        if envelope.grid != world.grid().id().raw() as u64 {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                "collision camera input targets an unknown grid",
            ));
        }
        Self::validate_camera_input(envelope.input)?;
        Self::validate_collision_shape(envelope.shape)?;
        if envelope.policy.mode != CameraCollisionPolicyMode::AxisSeparableSlide
            || envelope.policy.max_iterations == 0
            || envelope.policy.max_iterations > 3
        {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                "only axis_separable_slide with max_iterations in 1..=3 is supported",
            ));
        }
        let before = *self.cameras.get(&envelope.camera.raw()).ok_or_else(|| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::UnknownHandle,
                "unknown camera handle",
            )
        })?;
        let attempted = Self::integrate_camera_snapshot(before, envelope.input, envelope.tick);
        let projection = CollisionProjection::build(world);
        let mut after_pose = CameraPose {
            position: before.pose.position,
            yaw_degrees: attempted.pose.yaw_degrees,
            pitch_degrees: attempted.pose.pitch_degrees,
        };
        let delta = [
            attempted.pose.position[0] - before.pose.position[0],
            attempted.pose.position[1] - before.pose.position[1],
            attempted.pose.position[2] - before.pose.position[2],
        ];
        let mut blocked_axes = Vec::new();
        for (idx, axis) in [
            (0usize, CollisionAxis::X),
            (1, CollisionAxis::Y),
            (2, CollisionAxis::Z),
        ] {
            if delta[idx] == 0.0 {
                continue;
            }
            let mut candidate = after_pose;
            candidate.position[idx] += delta[idx];
            let (min, max) = Self::aabb_for_pose(candidate, envelope.shape);
            if projection.aabb_overlaps_solid(min, max) {
                blocked_axes.push(axis);
            } else {
                after_pose.position[idx] = candidate.position[idx];
            }
        }
        let after = CameraSnapshot {
            tick: envelope.tick,
            pose: after_pose,
            basis: Self::basis_from_pose(after_pose),
            ..before
        };
        self.cameras.insert(envelope.camera.raw(), after);
        let (min, max) = Self::aabb_for_pose(after.pose, envelope.shape);
        let collision_projection_hash = Self::collision_projection_hash(world, &projection);
        let world_hash = Self::world_hash(world);
        let correction = [
            after.pose.position[0] - attempted.pose.position[0],
            after.pose.position[1] - attempted.pose.position[1],
            after.pose.position[2] - attempted.pose.position[2],
        ];
        let movement_hash = format!(
            "fnv1a64:{}",
            Self::fnv1a64(&format!(
                "{}|{}|{:?}|{:?}|{:?}|{}|{}",
                envelope.camera.raw(),
                envelope.tick,
                before.pose,
                attempted.pose,
                after.pose,
                world_hash,
                collision_projection_hash
            ))
        );
        Ok(CameraCollisionSnapshot {
            camera: envelope.camera,
            tick: envelope.tick,
            before,
            attempted,
            after,
            collision: CameraCollisionEvidence {
                grid: envelope.grid,
                shape: envelope.shape,
                policy: envelope.policy,
                collided: !blocked_axes.is_empty(),
                blocked_axes,
                correction,
                queried_aabb: CollisionAabbEvidence {
                    min: [min.x as f32, min.y as f32, min.z as f32],
                    max: [max.x as f32, max.y as f32, max.z as f32],
                },
                world_hash,
                collision_projection_hash,
            },
            movement_hash,
        })
    }

    fn select_voxel(
        &self,
        request: ScreenPointToPickRayRequest,
    ) -> BridgeResult<VoxelSelectionSnapshot> {
        let snapshot = *self.cameras.get(&request.camera.raw()).ok_or_else(|| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::UnknownHandle,
                "unknown camera handle",
            )
        })?;
        let pick_ray = Self::pick_ray_snapshot(snapshot, request)?;
        let ray = PickRay {
            grid: pick_ray.grid,
            origin: pick_ray.origin,
            direction: pick_ray.direction,
            max_distance: pick_ray.max_distance,
        };
        let pick_result = self.pick_voxel(ray)?;
        let outcome = match pick_result {
            PickResult::Hit(_) => VoxelSelectionOutcome::Hit,
            PickResult::Miss(_) => VoxelSelectionOutcome::Miss,
        };
        let (selected_voxel, selected_face, edit_anchor) = match pick_result {
            PickResult::Hit(hit) => {
                let dir = match hit.face {
                    Face::PosX => Direction6::PosX,
                    Face::NegX => Direction6::NegX,
                    Face::PosY => Direction6::PosY,
                    Face::NegY => Direction6::NegY,
                    Face::PosZ => Direction6::PosZ,
                    Face::NegZ => Direction6::NegZ,
                };
                (
                    Some(hit.voxel),
                    Some(hit.face),
                    Some(hit.voxel.neighbor(dir)),
                )
            }
            PickResult::Miss(_) => (None, None, None),
        };
        let selection_hash = format!(
            "fnv1a64:{}",
            Self::fnv1a64(&format!(
                "{}|{:?}|{:?}|{:?}",
                pick_ray.ray_hash, pick_result, selected_voxel, edit_anchor
            ))
        );
        Ok(VoxelSelectionSnapshot {
            pick_ray,
            outcome,
            selected_voxel,
            selected_face,
            edit_anchor,
            selection_hash,
        })
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

    fn plan_voxel_conversion(
        &mut self,
        request: VoxelConversionPlanRequest,
    ) -> BridgeResult<VoxelConversionPlan> {
        self.require_initialized("plan_voxel_conversion")?;
        let source = Self::source_for_voxel_conversion(&request);
        let planned = svc_voxel_conversion::plan_conversion(&request, &source);
        let plan = planned.plan.clone();
        self.voxel_conversion_plan = Some(planned);
        self.voxel_conversion_evidence.clear();
        self.remember_voxel_conversion_evidence(plan.evidence.clone());
        Ok(plan)
    }

    fn preview_voxel_conversion(
        &mut self,
        request: VoxelConversionPreviewRequest,
    ) -> BridgeResult<VoxelConversionPreview> {
        self.require_initialized("preview_voxel_conversion")?;
        let planned = self.voxel_conversion_plan.as_ref().ok_or_else(|| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                "preview_voxel_conversion called before a conversion plan exists",
            )
        })?;
        let preview = svc_voxel_conversion::preview_conversion(&request, planned);
        self.remember_voxel_conversion_evidence(preview.evidence.clone());
        Ok(preview)
    }

    fn apply_voxel_conversion(
        &mut self,
        request: VoxelConversionApplyRequest,
    ) -> BridgeResult<VoxelConversionReceipt> {
        self.require_initialized("apply_voxel_conversion")?;
        let planned = self.voxel_conversion_plan.as_ref().ok_or_else(|| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                "apply_voxel_conversion called before a conversion plan exists",
            )
        })?;
        let mut receipt = svc_voxel_conversion::apply_conversion(&request, planned);
        if !receipt.applied {
            self.remember_voxel_conversion_evidence(receipt.evidence.clone());
            return Ok(receipt);
        }

        let hosted_grid = self.voxel.as_ref().ok_or_else(|| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::NotInitialized,
                "apply_voxel_conversion called before initialize_engine",
            )
        })?;
        if planned.plan.target.grid != hosted_grid.grid().id().raw() as u64 {
            return Ok(Self::rejected_voxel_conversion_receipt(
                request.plan_id,
                vec![Self::voxel_conversion_diagnostic(
                    VoxelConversionDiagnosticCode::ConversionReplayMismatch,
                    "target.grid",
                    "conversion target grid is not hosted by current authority state",
                )],
            ));
        }

        let Some(batch) = Self::conversion_commands(planned)? else {
            return Ok(Self::rejected_voxel_conversion_receipt(
                request.plan_id,
                vec![Self::voxel_conversion_diagnostic(
                    VoxelConversionDiagnosticCode::ConversionReplayMismatch,
                    "output",
                    "conversion apply had no authority output to commit",
                )],
            ));
        };
        let expected = batch.commands.len() as u32;
        let command_result = self.submit_commands(batch)?;
        if command_result.accepted != expected || command_result.rejected != 0 {
            receipt = Self::rejected_voxel_conversion_receipt(
                request.plan_id,
                vec![Self::voxel_conversion_diagnostic(
                    VoxelConversionDiagnosticCode::ConversionReplayMismatch,
                    "voxel_command_apply",
                    format!(
                        "conversion output command apply accepted {} of {} commands and rejected {}",
                        command_result.accepted, expected, command_result.rejected
                    ),
                )],
            );
        }
        self.remember_voxel_conversion_evidence(receipt.evidence.clone());
        Ok(receipt)
    }

    fn export_voxel_conversion_evidence(
        &self,
        evidence: Vec<VoxelConversionEvidenceRef>,
    ) -> BridgeResult<Vec<VoxelConversionEvidenceRef>> {
        self.require_initialized("export_voxel_conversion_evidence")?;
        for requested in &evidence {
            if !self.voxel_conversion_evidence.contains(requested) {
                return Err(RuntimeBridgeError::new(
                    RuntimeBridgeErrorKind::InvalidInput,
                    format!(
                        "voxel conversion evidence ref is not available from current authority state: {}",
                        requested.uri
                    ),
                ));
            }
        }
        Ok(evidence)
    }

    fn load_fps_runtime_session(
        &mut self,
        request: FpsRuntimeSessionLoadRequest,
    ) -> BridgeResult<FpsRuntimeSessionSnapshot> {
        self.require_initialized("load_fps_runtime_session")?;
        let input = Self::convert_fps_load_request(&request)?;
        let loaded = load_fps_project_bundle(input).map_err(Self::fps_runtime_error)?;
        // Commit only after the full authority bootstrap succeeds.
        self.fps_session = Some(loaded);
        self.fps_seed = Some(request);
        self.fps_epoch = self.fps_epoch.saturating_add(1);
        Self::fps_snapshot(
            self.fps_session.as_ref().expect("just committed"),
            self.fps_epoch,
        )
    }

    fn read_fps_runtime_session(&self) -> BridgeResult<FpsRuntimeSessionSnapshot> {
        self.require_initialized("read_fps_runtime_session")?;
        Self::fps_snapshot(
            self.fps_session("read_fps_runtime_session")?,
            self.fps_epoch,
        )
    }

    fn apply_fps_primary_fire(
        &mut self,
        request: FpsPrimaryFireRequest,
    ) -> BridgeResult<FpsPrimaryFireResult> {
        self.require_initialized("apply_fps_primary_fire")?;
        let ray = Self::ray_from_primary_fire(request)?;
        let world = self.voxel.as_ref().ok_or_else(|| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::NotInitialized,
                "apply_fps_primary_fire called before initialize_engine",
            )
        })?;
        let projection = CollisionProjection::build(world);
        let receipt = self
            .fps_session_mut("apply_fps_primary_fire")?
            .apply_primary_fire(&projection, ray, request.tick)
            .map_err(Self::fps_runtime_error)?;
        Ok(Self::primary_fire_result(receipt))
    }

    fn restart_fps_runtime_session(
        &mut self,
        request: FpsRuntimeSessionRestartRequest,
    ) -> BridgeResult<FpsRuntimeSessionSnapshot> {
        self.require_initialized("restart_fps_runtime_session")?;
        if request.expected_epoch != self.fps_epoch {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                format!(
                    "restart expected epoch {} but current epoch is {}",
                    request.expected_epoch, self.fps_epoch
                ),
            ));
        }
        let seed = self.fps_seed.clone().ok_or_else(|| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::NotInitialized,
                "restart_fps_runtime_session called before load_fps_runtime_session",
            )
        })?;
        let input = Self::convert_fps_load_request(&seed)?;
        let loaded = load_fps_project_bundle(input).map_err(Self::fps_runtime_error)?;
        self.fps_session = Some(loaded);
        self.fps_epoch = self.fps_epoch.saturating_add(1);
        Self::fps_snapshot(
            self.fps_session.as_ref().expect("just restarted"),
            self.fps_epoch,
        )
    }

    fn read_fps_encounter_director(
        &self,
        lifecycle: FpsEncounterLifecycleInput,
    ) -> BridgeResult<FpsEncounterDirectorSnapshot> {
        self.require_initialized("read_fps_encounter_director")?;
        Ok(Self::encounter_snapshot(
            self.fps_session("read_fps_encounter_director")?,
            lifecycle,
        ))
    }

    fn apply_fps_encounter_transition(
        &mut self,
        request: FpsEncounterTransitionRequest,
    ) -> BridgeResult<FpsEncounterTransitionResult> {
        self.require_initialized("apply_fps_encounter_transition")?;
        let action = Self::encounter_action(&request.action)?;
        let lifecycle = request.lifecycle;
        let rule_lifecycle = Self::bridge_encounter_lifecycle(lifecycle.clone());
        let receipt = self
            .fps_session_mut("apply_fps_encounter_transition")?
            .apply_encounter_transition(&request.preset_id, action, &rule_lifecycle)
            .map_err(Self::fps_runtime_error)?;
        Ok(Self::encounter_transition_result(receipt, lifecycle))
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
        Self::validate_camera_input(input)?;
        let snapshot = Self::integrate_camera_snapshot(prior, input, envelope.tick);
        self.cameras.insert(envelope.camera.raw(), snapshot);
        Ok(snapshot)
    }

    fn apply_enemy_direct_nav_movement(
        &mut self,
        request: EnemyDirectNavMovementRequest,
    ) -> BridgeResult<EnemyDirectNavMovementResult> {
        self.require_initialized("apply_enemy_direct_nav_movement")?;
        let entity = Self::enemy_entity_id(request.entity)?;
        let (authority_source, current_transform) =
            Self::seed_or_read_enemy_transform(&mut self.entities, entity, request.seed_position)?;
        let from = current_transform.translation;
        let nav = propose_direct_nav_movement(DirectNavMovementRequest {
            from,
            target: request.target,
            max_step_units: request.max_step_units,
        })
        .map_err(|err| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                format!(
                    "enemy direct-nav movement rejected by svc-pathfinding: {}",
                    EnemyDirectNavMovementError::Navigation(err).label()
                ),
            )
        })?;
        let next_transform = EntityTransform {
            translation: nav.next_waypoint,
            ..current_transform
        };
        let transform_event = self
            .entities
            .apply_transform(TransformCommand::Set {
                id: entity,
                transform: next_transform,
            })
            .map_err(|err| {
                RuntimeBridgeError::new(
                    RuntimeBridgeErrorKind::InvalidInput,
                    format!(
                        "enemy direct-nav movement rejected by core-entity: {}",
                        EnemyDirectNavMovementError::Transform(err).label()
                    ),
                )
            })?;
        Ok(EnemyDirectNavMovementResult {
            entity: entity.raw(),
            authority_source,
            from,
            target: nav.target,
            next_waypoint: nav.next_waypoint,
            distance_units: nav.distance_units,
            reached: nav.reached,
            path_hash: nav.path_hash,
            transform_hash: Self::transform_hash(entity, transform_event.transform),
            projection_changed: transform_event.projection_changed,
        })
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
    fn enemy_direct_nav_movement_routes_through_rust_entity_authority() {
        let mut bridge = ReferenceBridge::new();
        bridge.initialize_engine(EngineConfig { seed: 7 }).unwrap();

        let first = bridge
            .apply_enemy_direct_nav_movement(EnemyDirectNavMovementRequest {
                entity: 777,
                seed_position: Vec3::new(0.0, 0.5, -2.6),
                target: Vec3::new(0.0, 1.62, 1.25),
                max_step_units: 0.35,
            })
            .unwrap();
        assert_eq!(
            first.authority_source,
            EnemyDirectNavAuthoritySource::SeededFromRequest
        );
        assert_eq!(first.from, Vec3::new(0.0, 0.5, -2.6));
        assert_eq!(first.next_waypoint, Vec3::new(0.0, 0.598, -2.264));
        assert_eq!(first.path_hash, 0x69ed_74d6_9292_2db7);
        assert_ne!(first.transform_hash, 0);

        let second = bridge
            .apply_enemy_direct_nav_movement(EnemyDirectNavMovementRequest {
                entity: 777,
                seed_position: Vec3::new(99.0, 99.0, 99.0),
                target: Vec3::new(0.0, 1.62, 1.25),
                max_step_units: 0.35,
            })
            .unwrap();
        assert_eq!(
            second.authority_source,
            EnemyDirectNavAuthoritySource::RustEntityStore
        );
        assert_eq!(
            second.from, first.next_waypoint,
            "Rust store, not a stale TS seed, owns the next starting transform"
        );
        assert_ne!(second.next_waypoint, first.next_waypoint);
    }

    #[test]
    fn enemy_direct_nav_movement_fails_closed_on_invalid_request() {
        let mut bridge = ReferenceBridge::new();
        let before_init = bridge
            .apply_enemy_direct_nav_movement(EnemyDirectNavMovementRequest {
                entity: 1,
                seed_position: Vec3::ZERO,
                target: Vec3::ZERO,
                max_step_units: 0.35,
            })
            .unwrap_err();
        assert_eq!(before_init.kind, RuntimeBridgeErrorKind::NotInitialized);

        bridge.initialize_engine(EngineConfig { seed: 7 }).unwrap();
        let invalid_entity = bridge
            .apply_enemy_direct_nav_movement(EnemyDirectNavMovementRequest {
                entity: 0,
                seed_position: Vec3::ZERO,
                target: Vec3::ZERO,
                max_step_units: 0.35,
            })
            .unwrap_err();
        assert_eq!(invalid_entity.kind, RuntimeBridgeErrorKind::InvalidInput);

        let invalid_step = bridge
            .apply_enemy_direct_nav_movement(EnemyDirectNavMovementRequest {
                entity: 1,
                seed_position: Vec3::ZERO,
                target: Vec3::new(1.0, 0.0, 0.0),
                max_step_units: 0.0,
            })
            .unwrap_err();
        assert_eq!(invalid_step.kind, RuntimeBridgeErrorKind::InvalidInput);
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

    fn voxel_conversion_request(grid: u64) -> VoxelConversionPlanRequest {
        VoxelConversionPlanRequest {
            source: protocol_voxel_conversion::VoxelConversionSourceRef {
                asset_id: "mesh/quad".to_string(),
                asset_kind: "mesh".to_string(),
                asset_version: 1,
                source_hash: "sha256:quad".to_string(),
                mesh_primitive: None,
            },
            target: protocol_voxel_conversion::VoxelConversionTargetRef {
                grid,
                volume_asset_id: Some("voxel/generated".to_string()),
                origin: protocol_voxel_conversion::VoxelConversionCoord { x: 0, y: 0, z: 0 },
            },
            settings: protocol_voxel_conversion::VoxelConversionSettings {
                mode: protocol_voxel_conversion::VoxelConversionMode::Surface,
                fit_policy: protocol_voxel_conversion::VoxelConversionFitPolicy::Contain,
                origin_policy: protocol_voxel_conversion::VoxelConversionOriginPolicy::TargetMin,
                resolution: [4, 4, 1],
                voxel_size: 1.0,
                max_output_voxels: 16,
                transform: [
                    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
                ],
                material_map: protocol_voxel_conversion::VoxelConversionMaterialMap {
                    entries: vec![
                        protocol_voxel_conversion::VoxelConversionMaterialMapEntry {
                            source_material_slot: 0,
                            source_material_id: Some("mat/a".to_string()),
                            voxel_material: 3,
                        },
                        protocol_voxel_conversion::VoxelConversionMaterialMapEntry {
                            source_material_slot: 1,
                            source_material_id: Some("mat/b".to_string()),
                            voxel_material: 2,
                        },
                    ],
                    default_voxel_material: None,
                },
            },
        }
    }

    fn fps_load_request(enemy_health: u32) -> FpsRuntimeSessionLoadRequest {
        FpsRuntimeSessionLoadRequest {
            project_bundle: "custom-demo".to_string(),
            definitions: vec![
                FpsBridgeStoredEntityDefinition {
                    entity: 101,
                    stable_id: "actor/custom-player".to_string(),
                    display_name: "Custom Player".to_string(),
                    source_path: "catalogs/actors/player.entity.json".to_string(),
                    tags: vec!["player".to_string()],
                    role: FpsBridgeRole::Player,
                    transform: Some(FpsBridgeTransformCapability {
                        translation: [0.0, 1.5, 0.0],
                        rotation: [0.0, 0.0, 0.0, 1.0],
                        scale: [1.0, 1.0, 1.0],
                    }),
                    bounds: Some(FpsBridgeBoundsCapability {
                        min: [2.2, 1.0, 1.0],
                        max: [2.8, 2.0, 2.0],
                    }),
                    render_visible: Some(true),
                    static_collider: Some(false),
                    health: Some(FpsBridgeHealth {
                        current: 88,
                        max: 88,
                    }),
                    weapon: Some(FpsBridgeWeaponMount {
                        weapon_id: "weapon.custom.primary".to_string(),
                        damage: 75,
                        range_units: 16,
                        ammo: 3,
                        cooldown_ticks_after_fire: 4,
                    }),
                    policy_binding: None,
                },
                FpsBridgeStoredEntityDefinition {
                    entity: 777,
                    stable_id: "actor/custom-enemy".to_string(),
                    display_name: "Custom Enemy".to_string(),
                    source_path: "catalogs/actors/enemy.entity.json".to_string(),
                    tags: vec!["enemy".to_string()],
                    role: FpsBridgeRole::Enemy,
                    transform: Some(FpsBridgeTransformCapability {
                        translation: [0.0, 1.5, 5.2],
                        rotation: [0.0, 0.0, 0.0, 1.0],
                        scale: [1.0, 1.0, 1.0],
                    }),
                    bounds: Some(FpsBridgeBoundsCapability {
                        min: [2.2, 1.0, 5.0],
                        max: [2.8, 2.0, 5.8],
                    }),
                    render_visible: Some(true),
                    static_collider: Some(false),
                    health: Some(FpsBridgeHealth {
                        current: enemy_health,
                        max: enemy_health,
                    }),
                    weapon: None,
                    policy_binding: Some(FpsBridgePolicyBinding {
                        binding_id: "binding.enemy.custom.v0".to_string(),
                        policy_id: "policy.enemy.custom.v0".to_string(),
                        view_kind: "runtime_session.nav_policy_view.v0".to_string(),
                        view_version: "v0".to_string(),
                        allowed_intents: vec![
                            "runtime.intent.move_direct_nav.v0".to_string(),
                            "runtime.intent.primary_fire.v0".to_string(),
                        ],
                        runtime_moment: "runtime.tick.enemy_policy.v0".to_string(),
                    }),
                },
            ],
        }
    }

    #[test]
    fn fps_runtime_session_loads_project_bundle_through_rust_authority() {
        let mut bridge = init_bridge();
        let snapshot = bridge
            .load_fps_runtime_session(fps_load_request(75))
            .expect("fps session loads");

        assert_eq!(snapshot.backend, "reference_bridge_rust");
        assert_eq!(
            snapshot.authority_surface,
            "runtime_session.fps.authority.v0"
        );
        assert_eq!(snapshot.session_epoch, 1);
        assert_eq!(snapshot.player_entity, 101);
        assert_eq!(snapshot.enemy_entity, 777);
        assert_eq!(
            snapshot.health,
            vec![
                FpsEntityHealthReadout {
                    entity: 101,
                    current: 88,
                    max: 88,
                },
                FpsEntityHealthReadout {
                    entity: 777,
                    current: 75,
                    max: 75,
                },
            ]
        );
        assert_eq!(snapshot.policy_bindings.len(), 1);
        assert_eq!(snapshot.policy_bindings[0].entity, 777);
        assert_eq!(
            snapshot.replay_records[0].replay_unit,
            "runtime_session.fps.bootstrap.v0"
        );
        assert_ne!(snapshot.replay_hash, 0);
        assert!(snapshot
            .read_sets
            .iter()
            .any(|view| view.owner == "rule-lifecycle"));
    }

    #[test]
    fn fps_primary_fire_receipt_comes_from_rust_combat_lifecycle_and_replay() {
        let mut bridge = init_bridge();
        bridge
            .load_fps_runtime_session(fps_load_request(75))
            .unwrap();
        let receipt = bridge
            .apply_fps_primary_fire(FpsPrimaryFireRequest {
                tick: 9,
                origin: [2.5, 1.5, 1.5],
                direction: [0.0, 0.0, 1.0],
            })
            .expect("primary fire applies");

        assert_eq!(receipt.backend, "reference_bridge_rust");
        assert_eq!(receipt.mutation_owner, "rule-lifecycle + svc-combat");
        assert_eq!(receipt.shooter, 101);
        assert_eq!(receipt.target, Some(777));
        assert_eq!(
            receipt.target_health_before,
            Some(FpsBridgeHealth {
                current: 75,
                max: 75,
            })
        );
        assert_eq!(
            receipt.target_health_after,
            Some(FpsBridgeHealth {
                current: 0,
                max: 75,
            })
        );
        assert_eq!(
            receipt.lifecycle_status,
            FpsBridgeLifecycleStatus::EnemyDefeated {
                entity: 777,
                tick: 9,
            }
        );
        assert_eq!(receipt.target_render_visible, Some(false));
        assert_ne!(receipt.replay_hash, 0);

        let snapshot = bridge.read_fps_runtime_session().unwrap();
        assert_eq!(snapshot.replay_records.len(), 2);
        assert_eq!(snapshot.replay_hash, receipt.replay_hash);
    }

    #[test]
    fn fps_encounter_transition_is_rule_lifecycle_authority() {
        let mut bridge = init_bridge();
        bridge
            .load_fps_runtime_session(fps_load_request(75))
            .unwrap();
        let active_lifecycle = FpsEncounterLifecycleInput {
            outcome_kind: "in_progress".to_string(),
            terminal: false,
            enemy_dead: false,
            player_dead: false,
            lifecycle_hash: "fnv1a64:active".to_string(),
        };
        let pending = bridge
            .read_fps_encounter_director(active_lifecycle.clone())
            .unwrap();
        assert_eq!(pending.backend, "reference_bridge_rust");
        assert_eq!(
            pending.authority_surface,
            "runtime_session.fps.encounter_director.v0"
        );
        assert_eq!(pending.state.status, "pending");
        assert_eq!(pending.read_sets[0].owner, "rule-lifecycle");

        let activated = bridge
            .apply_fps_encounter_transition(FpsEncounterTransitionRequest {
                preset_id: "generated-tunnel-small-encounter".to_string(),
                action: "activate".to_string(),
                lifecycle: active_lifecycle,
            })
            .unwrap();
        assert!(activated.accepted);
        assert_eq!(
            activated.event_kind.as_deref(),
            Some("runtime_encounter.activated.v0")
        );
        assert_eq!(activated.state.status, "active");
        assert_eq!(
            activated.state.spawned_enemy_ids,
            vec!["encounter.generated_tunnel_small.wave_1.enemy_001".to_string()]
        );

        bridge
            .apply_fps_primary_fire(FpsPrimaryFireRequest {
                tick: 9,
                origin: [2.5, 1.5, 1.5],
                direction: [0.0, 0.0, 1.0],
            })
            .unwrap();
        let won_lifecycle = FpsEncounterLifecycleInput {
            outcome_kind: "won".to_string(),
            terminal: true,
            enemy_dead: true,
            player_dead: false,
            lifecycle_hash: "fnv1a64:won".to_string(),
        };
        let cleared = bridge
            .apply_fps_encounter_transition(FpsEncounterTransitionRequest {
                preset_id: "generated-tunnel-small-encounter".to_string(),
                action: "sync_lifecycle".to_string(),
                lifecycle: won_lifecycle,
            })
            .unwrap();
        assert!(cleared.accepted);
        assert_eq!(cleared.state.status, "cleared");
        assert_eq!(
            cleared.state.defeated_enemy_ids,
            vec!["encounter.generated_tunnel_small.wave_1.enemy_001".to_string()]
        );
        assert_ne!(cleared.replay_hash, 0);

        let rejected = bridge
            .apply_fps_encounter_transition(FpsEncounterTransitionRequest {
                preset_id: "generated-tunnel-small-encounter".to_string(),
                action: "activate".to_string(),
                lifecycle: FpsEncounterLifecycleInput {
                    outcome_kind: "in_progress".to_string(),
                    terminal: false,
                    enemy_dead: false,
                    player_dead: false,
                    lifecycle_hash: "fnv1a64:active-again".to_string(),
                },
            })
            .unwrap();
        assert!(!rejected.accepted);
        assert_eq!(
            rejected.rejection_reason.as_deref(),
            Some("encounter_not_pending")
        );

        let restarted = bridge
            .restart_fps_runtime_session(FpsRuntimeSessionRestartRequest { expected_epoch: 1 })
            .unwrap();
        assert_eq!(restarted.session_epoch, 2);
        let reset = bridge
            .read_fps_encounter_director(FpsEncounterLifecycleInput {
                outcome_kind: "in_progress".to_string(),
                terminal: false,
                enemy_dead: false,
                player_dead: false,
                lifecycle_hash: "fnv1a64:reset".to_string(),
            })
            .unwrap();
        assert_eq!(reset.state.status, "pending");
        assert_eq!(reset.state.revision, 0);
    }

    #[test]
    fn fps_runtime_session_restart_is_epoch_guarded_and_authority_owned() {
        let mut bridge = init_bridge();
        bridge
            .load_fps_runtime_session(fps_load_request(75))
            .unwrap();
        bridge
            .apply_fps_primary_fire(FpsPrimaryFireRequest {
                tick: 9,
                origin: [2.5, 1.5, 1.5],
                direction: [0.0, 0.0, 1.0],
            })
            .unwrap();

        let stale = bridge
            .restart_fps_runtime_session(FpsRuntimeSessionRestartRequest { expected_epoch: 0 })
            .unwrap_err();
        assert_eq!(stale.kind, RuntimeBridgeErrorKind::InvalidInput);

        let restarted = bridge
            .restart_fps_runtime_session(FpsRuntimeSessionRestartRequest { expected_epoch: 1 })
            .unwrap();
        assert_eq!(restarted.session_epoch, 2);
        assert_eq!(restarted.lifecycle_status, FpsBridgeLifecycleStatus::Active);
        assert_eq!(
            restarted
                .health
                .iter()
                .find(|health| health.entity == 777)
                .map(|health| (health.current, health.max)),
            Some((75, 75))
        );
        assert_eq!(restarted.replay_records.len(), 1);
    }

    #[test]
    fn invalid_fps_load_fails_closed_without_replacing_prior_session() {
        let mut bridge = init_bridge();
        bridge
            .load_fps_runtime_session(fps_load_request(75))
            .unwrap();
        let before = bridge.read_fps_runtime_session().unwrap();
        let mut invalid = fps_load_request(33);
        invalid.definitions[1].policy_binding = Some(FpsBridgePolicyBinding {
            binding_id: String::new(),
            policy_id: "policy.enemy.custom.v0".to_string(),
            view_kind: "runtime_session.nav_policy_view.v0".to_string(),
            view_version: "v0".to_string(),
            allowed_intents: vec!["runtime.intent.primary_fire.v0".to_string()],
            runtime_moment: "runtime.tick.enemy_policy.v0".to_string(),
        });

        let err = bridge.load_fps_runtime_session(invalid).unwrap_err();
        assert_eq!(err.kind, RuntimeBridgeErrorKind::InvalidInput);
        let after = bridge.read_fps_runtime_session().unwrap();
        assert_eq!(after.session_epoch, before.session_epoch);
        assert_eq!(after.health, before.health);
        assert_eq!(after.replay_hash, before.replay_hash);
    }

    #[test]
    fn voxel_conversion_plan_preview_apply_uses_rust_authority_and_commands() {
        let mut bridge = init_bridge();
        let request = voxel_conversion_request(1);
        let plan = bridge.plan_voxel_conversion(request).unwrap();
        assert_eq!(
            plan.authority_version,
            svc_voxel_conversion::AUTHORITY_VERSION
        );
        assert!(plan.diagnostics.is_empty());
        assert_eq!(plan.estimated_output_voxels, 4);

        let stale = bridge
            .preview_voxel_conversion(VoxelConversionPreviewRequest {
                plan_id: plan.plan_id.clone(),
                expected_plan_hash: "fnv1a64:stale".to_string(),
            })
            .unwrap();
        assert_eq!(
            stale.diagnostics[0].code,
            VoxelConversionDiagnosticCode::StaleAuthoritySnapshot
        );

        let preview = bridge
            .preview_voxel_conversion(VoxelConversionPreviewRequest {
                plan_id: plan.plan_id.clone(),
                expected_plan_hash: svc_voxel_conversion::plan_hash(&plan),
            })
            .unwrap();
        assert!(preview.diagnostics.is_empty());
        assert_eq!(preview.output_voxel_count, 4);

        let receipt = bridge
            .apply_voxel_conversion(VoxelConversionApplyRequest {
                plan_id: plan.plan_id.clone(),
                expected_plan_hash: svc_voxel_conversion::plan_hash(&plan),
                expected_preview_hash: Some(preview.output_hash.clone()),
            })
            .unwrap();
        assert!(receipt.applied);
        assert_eq!(receipt.output_voxel_count, 4);

        let world = bridge.voxel.as_ref().unwrap();
        let chunk = world.get(ChunkCoord::new(0, 0, 0)).unwrap();
        assert_eq!(
            chunk.get(LocalVoxelCoord::new(0, 0, 0)),
            Some(VoxelValue::solid_raw(2)),
            "conversion output applied through voxel command authority"
        );

        let exported = bridge
            .export_voxel_conversion_evidence(
                plan.evidence
                    .iter()
                    .chain(preview.evidence.iter())
                    .chain(receipt.evidence.iter())
                    .cloned()
                    .collect(),
            )
            .unwrap();
        assert_eq!(exported.len(), 3);
    }

    #[test]
    fn voxel_conversion_apply_to_unknown_grid_returns_diagnostic_receipt() {
        let mut bridge = init_bridge();
        let plan = bridge
            .plan_voxel_conversion(voxel_conversion_request(7))
            .unwrap();
        let preview = bridge
            .preview_voxel_conversion(VoxelConversionPreviewRequest {
                plan_id: plan.plan_id.clone(),
                expected_plan_hash: svc_voxel_conversion::plan_hash(&plan),
            })
            .unwrap();
        let receipt = bridge
            .apply_voxel_conversion(VoxelConversionApplyRequest {
                plan_id: plan.plan_id.clone(),
                expected_plan_hash: svc_voxel_conversion::plan_hash(&plan),
                expected_preview_hash: Some(preview.output_hash),
            })
            .unwrap();
        assert!(!receipt.applied);
        assert_eq!(
            receipt.diagnostics[0].code,
            VoxelConversionDiagnosticCode::ConversionReplayMismatch
        );
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
    fn collision_constrained_camera_blocks_terrain_and_allows_empty_space() {
        use protocol_view::{CameraPose, PerspectiveProjection, ViewportSize};

        let mut bridge = init_bridge();
        let camera = bridge
            .create_camera(CameraCreateRequest {
                initial_pose: CameraPose {
                    position: [1.5, 1.5, 1.3],
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
            })
            .unwrap();
        let shape = CameraCollisionShape {
            half_extents: [0.2, 0.2, 0.2],
        };
        let policy = CameraCollisionPolicy {
            mode: CameraCollisionPolicyMode::AxisSeparableSlide,
            max_iterations: 3,
        };
        let blocked = bridge
            .apply_collision_constrained_camera_input(CollisionConstrainedCameraInputEnvelope {
                camera: camera.camera,
                grid: 1,
                input: FirstPersonCameraInput {
                    move_forward: 1.0,
                    move_right: 0.0,
                    move_up: 0.0,
                    yaw_delta_degrees: 0.0,
                    pitch_delta_degrees: 0.0,
                    dt_seconds: 1.0,
                    move_speed_units_per_second: 1.0,
                },
                tick: 1,
                shape,
                policy,
            })
            .unwrap();
        assert!(blocked.collision.collided);
        assert_eq!(blocked.collision.blocked_axes, vec![CollisionAxis::Z]);
        assert_eq!(blocked.after.pose.position, camera.pose.position);
        assert!(blocked.movement_hash.starts_with("fnv1a64:"));

        let clear = bridge
            .apply_collision_constrained_camera_input(CollisionConstrainedCameraInputEnvelope {
                camera: camera.camera,
                grid: 1,
                input: FirstPersonCameraInput {
                    move_forward: -1.0,
                    move_right: 0.0,
                    move_up: 0.0,
                    yaw_delta_degrees: 0.0,
                    pitch_delta_degrees: 0.0,
                    dt_seconds: 1.0,
                    move_speed_units_per_second: 1.0,
                },
                tick: 2,
                shape,
                policy,
            })
            .unwrap();
        assert!(!clear.collision.collided);
        assert_eq!(clear.collision.blocked_axes, Vec::<CollisionAxis>::new());
        assert_eq!(clear.after.pose.position, [1.5, 1.5, 2.3]);
    }

    #[test]
    fn select_voxel_derives_center_ray_and_edit_anchor_from_camera() {
        use protocol_view::{CameraPose, PerspectiveProjection, ViewportSize};

        let mut bridge = init_bridge();
        let camera = bridge
            .create_camera(CameraCreateRequest {
                initial_pose: CameraPose {
                    position: [1.5, 1.5, 4.0],
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
            })
            .unwrap();
        let selection = bridge
            .select_voxel(ScreenPointToPickRayRequest {
                camera: camera.camera,
                grid: 1,
                viewport: None,
                screen_point: ScreenPoint {
                    x: 0.5,
                    y: 0.5,
                    space: ScreenPointSpace::Normalized01,
                },
                max_distance: 10.0,
            })
            .unwrap();
        assert_eq!(selection.pick_ray.direction, [0.0, 0.0, -1.0]);
        assert_eq!(selection.selected_voxel, Some(VoxelCoord::new(1, 1, 0)));
        assert_eq!(selection.selected_face, Some(Face::PosZ));
        assert_eq!(selection.edit_anchor, Some(VoxelCoord::new(1, 1, 1)));
        assert!(selection
            .pick_ray
            .camera_projection_hash
            .starts_with("fnv1a64:"));
        assert!(selection.selection_hash.starts_with("fnv1a64:"));
    }

    #[test]
    fn select_voxel_reports_miss_for_out_of_range_crosshair() {
        use protocol_view::{CameraPose, PerspectiveProjection, ViewportSize};

        let mut bridge = init_bridge();
        let camera = bridge
            .create_camera(CameraCreateRequest {
                initial_pose: CameraPose {
                    position: [1.5, 1.5, 4.0],
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
            })
            .unwrap();
        let selection = bridge
            .select_voxel(ScreenPointToPickRayRequest {
                camera: camera.camera,
                grid: 1,
                viewport: None,
                screen_point: ScreenPoint {
                    x: 0.5,
                    y: 0.5,
                    space: ScreenPointSpace::Normalized01,
                },
                max_distance: 1.0,
            })
            .unwrap();
        assert_eq!(selection.outcome, VoxelSelectionOutcome::Miss);
        assert_eq!(selection.selected_voxel, None);
        assert_eq!(selection.edit_anchor, None);
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
