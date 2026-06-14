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

use core_commands::VoxelCommand;
use core_error::ErrorCategory;
use core_space::{ChunkCoord, ChunkDims, GridId, VoxelGridSpec};
use core_voxel::{MaterialCatalog, VoxelMaterialId};
use rule_voxel_edit::VoxelEditRejection;
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
        let spec = Self::launch_grid();
        let mut world = VoxelWorld::new(spec);
        world.insert(ChunkCoord::new(0, 0, 0), VoxelChunk::from_spec(&spec));
        let _ = world.drain_dirty();
        self.voxel = Some(world);
        self.materials = MaterialCatalog::new([1, 2, 3].into_iter().map(VoxelMaterialId::new));

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
