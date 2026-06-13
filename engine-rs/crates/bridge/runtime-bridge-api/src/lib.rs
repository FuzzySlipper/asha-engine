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

use core_error::ErrorCategory;

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

// ── The bridge surface ────────────────────────────────────────────────────────

/// The bounded set of verbs every transport implements. There is no generic
/// `call(method, json)` — adding a verb here is a reviewed boundary change.
pub trait RuntimeBridge {
    fn initialize_engine(&mut self, config: EngineConfig) -> BridgeResult<EngineHandle>;
    fn step_simulation(&mut self, input: StepInputEnvelope) -> BridgeResult<StepResult>;
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

/// A minimal deterministic bridge used for boundary smoke tests.
#[derive(Debug, Default)]
pub struct ReferenceBridge {
    engine: Option<EngineHandle>,
    buffer: Vec<u8>,
    /// The currently-loaded world's scene identity (the staged/live world).
    loaded_world: Option<u64>,
}

/// The bundle schema / protocol versions this reference bridge understands.
const REFERENCE_SUPPORTED_VERSION: u32 = 1;

impl ReferenceBridge {
    pub fn new() -> Self {
        Self::default()
    }
}

impl RuntimeBridge for ReferenceBridge {
    fn initialize_engine(&mut self, config: EngineConfig) -> BridgeResult<EngineHandle> {
        let handle = EngineHandle::new(config.seed);
        self.engine = Some(handle);
        // Deterministic: buffer content derived from the seed.
        self.buffer = config.seed.to_le_bytes().to_vec();
        Ok(handle)
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
        if handle.raw() != 0 {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::UnknownHandle,
                format!("no buffer for handle {}", handle.raw()),
            ));
        }
        Ok(RuntimeBufferView {
            handle,
            bytes: &self.buffer,
        })
    }

    fn release_buffer(&mut self, handle: RuntimeBufferHandle) -> BridgeResult<()> {
        if handle.raw() != 0 {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::UnknownHandle,
                format!("no buffer for handle {}", handle.raw()),
            ));
        }
        self.buffer.clear();
        Ok(())
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
