//! Generated-border shapes for disposable non-scene presentation.
//!
//! The scene remains in `protocol-render`. This crate owns the shared ordered
//! frame and closed host-capability union used by audio and later presentation
//! domains. None of these types is authority or replay truth.

#![forbid(unsafe_code)]

use protocol_render::{RenderFrameDiff, RenderHandle};
use serde::{Deserialize, Serialize};

pub const RUNTIME_PROJECTION_SCHEMA_VERSION: u16 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct AudioHandle(pub u64);

impl AudioHandle {
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct BillboardHandle(pub u64);

impl BillboardHandle {
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ParticleEmitterHandle(pub u64);

impl ParticleEmitterHandle {
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TelemetryOverlayHandle(pub u64);

impl TelemetryOverlayHandle {
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct AnimationProjectionHandle(pub u64);

impl AnimationProjectionHandle {
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ProjectionReplayScope {
    ExcludedFromReplayTruth,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PresentationOriginKind {
    OwnerFact,
    GameplayEvent,
    DecisionOutcome,
    CapabilityState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresentationOriginRef {
    pub kind: PresentationOriginKind,
    pub id: String,
    pub authority_tick: u64,
    pub causation_id: Option<String>,
    pub correlation_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresentationOpMeta {
    pub sequence: u32,
    pub origin: Option<PresentationOriginRef>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AudioBus {
    Sfx,
    Ambient,
    Ui,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum AudioEmitter {
    Global2d,
    World3d { position: [f32; 3] },
    EntityAttached { entity: u64, offset: [f32; 3] },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioClipRef {
    pub asset: String,
    pub content_hash: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioSourceDescriptor {
    pub clip: AudioClipRef,
    pub bus: AudioBus,
    pub volume: f32,
    pub pitch: f32,
    pub looping: bool,
    pub spatial_blend: f32,
    pub attenuation: f32,
    pub pan: f32,
    pub emitter: AudioEmitter,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioSourcePatch {
    pub volume: Option<f32>,
    pub pitch: Option<f32>,
    pub looping: Option<bool>,
    pub spatial_blend: Option<f32>,
    pub attenuation: Option<f32>,
    pub pan: Option<f32>,
    pub emitter: Option<AudioEmitter>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum AudioProjectionOp {
    Emit {
        signal_id: String,
        descriptor: AudioSourceDescriptor,
    },
    Create {
        handle: AudioHandle,
        descriptor: AudioSourceDescriptor,
    },
    Update {
        handle: AudioHandle,
        patch: AudioSourcePatch,
    },
    Destroy {
        handle: AudioHandle,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AudioProjectionDiagnosticCode {
    InvalidDescriptor,
    AssetMissing,
    AssetKindMismatch,
    ContentHashMismatch,
    DuplicateSignal,
    DuplicateHandle,
    UnknownHandle,
    UnavailableHost,
    AudioContextBlocked,
    DecodeFailed,
    HostFailure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioProjectionDiagnostic {
    pub code: AudioProjectionDiagnosticCode,
    pub sequence: u32,
    pub handle: Option<AudioHandle>,
    pub message: String,
    pub origin: Option<PresentationOriginRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioProjectionReadout {
    pub active_sources: u32,
    pub cached_clips: u32,
    pub emitted_signals: u64,
    pub diagnostics: Vec<AudioProjectionDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum BillboardAnchor {
    World { position: [f32; 3] },
    EntityAttached { entity: u64, offset: [f32; 3] },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BillboardTemplateArgument {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BillboardTextureRef {
    pub asset: String,
    pub content_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum BillboardContent {
    Text {
        localization_key: String,
        fallback_text: String,
        arguments: Vec<BillboardTemplateArgument>,
    },
    Value {
        label_key: String,
        fallback_label: String,
        value: String,
        unit_key: Option<String>,
        fallback_unit: Option<String>,
    },
    Icon {
        texture: BillboardTextureRef,
        alt_key: String,
        fallback_alt: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum BillboardFontRef {
    System {
        family: String,
    },
    Asset {
        asset: String,
        content_hash: String,
        family: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BillboardLayer {
    AlwaysOnTop,
    DepthTested,
    Occluded,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BillboardDescriptor {
    pub anchor: BillboardAnchor,
    pub content: BillboardContent,
    pub font: BillboardFontRef,
    pub height_pixels: f32,
    pub color: [f32; 4],
    pub background: [f32; 4],
    pub max_distance: f32,
    pub layer: BillboardLayer,
    pub visible: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BillboardPatch {
    pub anchor: Option<BillboardAnchor>,
    pub content: Option<BillboardContent>,
    pub font: Option<BillboardFontRef>,
    pub height_pixels: Option<f32>,
    pub color: Option<[f32; 4]>,
    pub background: Option<[f32; 4]>,
    pub max_distance: Option<f32>,
    pub layer: Option<BillboardLayer>,
    pub visible: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum BillboardProjectionOp {
    Create {
        handle: BillboardHandle,
        descriptor: BillboardDescriptor,
    },
    Update {
        handle: BillboardHandle,
        patch: BillboardPatch,
    },
    Destroy {
        handle: BillboardHandle,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BillboardProjectionDiagnosticCode {
    InvalidDescriptor,
    AssetMissing,
    AssetKindMismatch,
    ContentHashMismatch,
    DuplicateHandle,
    UnknownHandle,
    AnchorMissing,
    UnavailableHost,
    FontLoadFailed,
    IconLoadFailed,
    HostFailure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BillboardProjectionDiagnostic {
    pub code: BillboardProjectionDiagnosticCode,
    pub sequence: u32,
    pub handle: Option<BillboardHandle>,
    pub message: String,
    pub origin: Option<PresentationOriginRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BillboardProjectionReadout {
    pub active_billboards: u32,
    pub loaded_fonts: u32,
    pub loaded_icons: u32,
    pub culled_billboards: u32,
    pub diagnostics: Vec<BillboardProjectionDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ParticleAnchor {
    World { position: [f32; 3] },
    EntityAttached { entity: u64, offset: [f32; 3] },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParticleSpriteRef {
    pub asset: String,
    pub content_hash: String,
    pub frame_count: u16,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParticleScalarKey {
    pub age: f32,
    pub value: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParticleColorKey {
    pub age: f32,
    pub color: [f32; 4],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParticleEmitterDescriptor {
    pub anchor: ParticleAnchor,
    pub sprite: ParticleSpriteRef,
    pub rate_per_second: f32,
    pub burst_count: u32,
    pub lifetime_seconds: [f32; 2],
    pub velocity_min: [f32; 3],
    pub velocity_max: [f32; 3],
    pub acceleration: [f32; 3],
    pub size_curve: Vec<ParticleScalarKey>,
    pub color_curve: Vec<ParticleColorKey>,
    pub flipbook_frames_per_second: f32,
    pub seed: u64,
    pub max_particles: u32,
    pub visible: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParticleEmitterPatch {
    pub anchor: Option<ParticleAnchor>,
    pub sprite: Option<ParticleSpriteRef>,
    pub rate_per_second: Option<f32>,
    pub burst_count: Option<u32>,
    pub lifetime_seconds: Option<[f32; 2]>,
    pub velocity_min: Option<[f32; 3]>,
    pub velocity_max: Option<[f32; 3]>,
    pub acceleration: Option<[f32; 3]>,
    pub size_curve: Option<Vec<ParticleScalarKey>>,
    pub color_curve: Option<Vec<ParticleColorKey>>,
    pub flipbook_frames_per_second: Option<f32>,
    pub max_particles: Option<u32>,
    pub visible: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum ParticleProjectionOp {
    Emit {
        signal_id: String,
        descriptor: ParticleEmitterDescriptor,
    },
    Create {
        handle: ParticleEmitterHandle,
        descriptor: ParticleEmitterDescriptor,
    },
    Update {
        handle: ParticleEmitterHandle,
        patch: ParticleEmitterPatch,
    },
    Destroy {
        handle: ParticleEmitterHandle,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ParticleProjectionDiagnosticCode {
    InvalidDescriptor,
    AssetMissing,
    AssetKindMismatch,
    ContentHashMismatch,
    DuplicateSignal,
    DuplicateHandle,
    UnknownHandle,
    AnchorMissing,
    BudgetExceeded,
    UnavailableHost,
    SpriteLoadFailed,
    HostFailure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParticleProjectionDiagnostic {
    pub code: ParticleProjectionDiagnosticCode,
    pub sequence: u32,
    pub handle: Option<ParticleEmitterHandle>,
    pub message: String,
    pub origin: Option<PresentationOriginRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParticleProjectionReadout {
    pub active_emitters: u32,
    pub active_particles: u32,
    pub loaded_sprites: u32,
    pub emitted_bursts: u64,
    pub dropped_particles: u64,
    pub diagnostics: Vec<ParticleProjectionDiagnostic>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TelemetryOverlayCorner {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TelemetryOverlayDescriptor {
    pub title: String,
    pub corner: TelemetryOverlayCorner,
    pub refresh_interval_ms: u32,
    pub max_frame_time_samples: u16,
    pub visible: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TelemetryOverlayPatch {
    pub title: Option<String>,
    pub corner: Option<TelemetryOverlayCorner>,
    pub refresh_interval_ms: Option<u32>,
    pub max_frame_time_samples: Option<u16>,
    pub visible: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum TelemetryOverlayProjectionOp {
    Create {
        handle: TelemetryOverlayHandle,
        descriptor: TelemetryOverlayDescriptor,
    },
    Update {
        handle: TelemetryOverlayHandle,
        patch: TelemetryOverlayPatch,
    },
    Destroy {
        handle: TelemetryOverlayHandle,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TelemetryOverlayDiagnosticCode {
    InvalidDescriptor,
    DuplicateHandle,
    UnknownHandle,
    UnavailableHost,
    SnapshotUnavailable,
    HostFailure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TelemetryOverlayDiagnostic {
    pub code: TelemetryOverlayDiagnosticCode,
    pub sequence: u32,
    pub handle: Option<TelemetryOverlayHandle>,
    pub message: String,
    pub origin: Option<PresentationOriginRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TelemetryOverlayReadout {
    pub active_overlays: u32,
    pub rendered_snapshots: u64,
    pub diagnostics: Vec<TelemetryOverlayDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimationResolvedMotion {
    pub clip_a: String,
    pub clip_b: Option<String>,
    pub blend_weight_milli: i32,
    pub speed_milli: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimationTransitionProjection {
    pub transition_id: String,
    pub from_state_id: String,
    pub to_state_id: String,
    pub elapsed_ticks: u32,
    pub duration_ticks: u32,
    pub target_motion: AnimationResolvedMotion,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AnimationTransitionFactMoment {
    Started,
    Completed,
}

/// Inspectable trace from controller projection back to one durable authority
/// transition fact. This is copied evidence, never a presentation-owned event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimationTransitionFactRef {
    pub fact_id: String,
    pub source_fact_id: String,
    pub authority_tick: u64,
    pub controller_input_sequence: u64,
    pub controller_tick: u64,
    pub causation_id: String,
    pub correlation_id: String,
    pub transition_id: String,
    pub from_state_id: String,
    pub to_state_id: String,
    pub moment: AnimationTransitionFactMoment,
    pub duration_ticks: u32,
    pub fact_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimationControllerProjectionState {
    pub graph_id: String,
    pub graph_version: u32,
    pub graph_hash: String,
    pub state_id: String,
    pub revision: u64,
    pub state_hash: String,
    pub motion: AnimationResolvedMotion,
    pub transition: Option<AnimationTransitionProjection>,
    pub timing_fact: Option<Box<AnimationTransitionFactRef>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimationProjectionDescriptor {
    pub target: RenderHandle,
    pub asset: String,
    pub tick_duration_millis: u32,
    pub controller: AnimationControllerProjectionState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum AnimationProjectionOp {
    Create {
        handle: AnimationProjectionHandle,
        descriptor: AnimationProjectionDescriptor,
    },
    Update {
        handle: AnimationProjectionHandle,
        controller: AnimationControllerProjectionState,
    },
    Destroy {
        handle: AnimationProjectionHandle,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AnimationProjectionDiagnosticCode {
    InvalidDescriptor,
    DuplicateHandle,
    UnknownHandle,
    UnknownTarget,
    AssetMissing,
    ClipMissing,
    IncompatibleRig,
    InvalidBlendWeight,
    InvalidTransition,
    StaleRevision,
    UnavailableHost,
    CompatibilityFallback,
    HostFailure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimationProjectionDiagnostic {
    pub code: AnimationProjectionDiagnosticCode,
    pub sequence: u32,
    pub handle: Option<AnimationProjectionHandle>,
    pub target: Option<RenderHandle>,
    pub message: String,
    pub origin: Option<PresentationOriginRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimationProjectionReadout {
    pub active_controllers: u32,
    pub sampled_frames: u64,
    pub compatibility_fallbacks: u64,
    pub diagnostics: Vec<AnimationProjectionDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "domain", rename_all = "camelCase")]
pub enum PresentationOp {
    Audio {
        meta: PresentationOpMeta,
        op: AudioProjectionOp,
    },
    Billboard {
        meta: PresentationOpMeta,
        op: BillboardProjectionOp,
    },
    Particle {
        meta: PresentationOpMeta,
        op: ParticleProjectionOp,
    },
    TelemetryOverlay {
        meta: PresentationOpMeta,
        op: TelemetryOverlayProjectionOp,
    },
    Animation {
        meta: PresentationOpMeta,
        op: AnimationProjectionOp,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresentationFrameDiff {
    pub replay_scope: ProjectionReplayScope,
    pub ops: Vec<PresentationOp>,
}

impl Default for PresentationFrameDiff {
    fn default() -> Self {
        Self {
            replay_scope: ProjectionReplayScope::ExcludedFromReplayTruth,
            ops: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeProjectionFrame {
    pub schema_version: u16,
    pub authority_tick: u64,
    pub scene: RenderFrameDiff,
    pub presentation: PresentationFrameDiff,
}

impl RuntimeProjectionFrame {
    pub fn empty(authority_tick: u64) -> Self {
        Self {
            schema_version: RUNTIME_PROJECTION_SCHEMA_VERSION,
            authority_tick,
            scene: RenderFrameDiff::new(),
            presentation: PresentationFrameDiff::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_frame_is_scene_plus_ordered_replay_excluded_presentation() {
        let frame = RuntimeProjectionFrame::empty(41);
        assert_eq!(frame.schema_version, RUNTIME_PROJECTION_SCHEMA_VERSION);
        assert_eq!(frame.authority_tick, 41);
        assert!(frame.scene.is_empty());
        assert!(frame.presentation.ops.is_empty());
        assert_eq!(
            frame.presentation.replay_scope,
            ProjectionReplayScope::ExcludedFromReplayTruth
        );
    }

    #[test]
    fn audio_handle_is_domain_branded_and_stable() {
        assert_eq!(AudioHandle::new(7).raw(), 7);
        assert_eq!(BillboardHandle::new(8).raw(), 8);
        assert_eq!(ParticleEmitterHandle::new(9).raw(), 9);
        assert_eq!(AnimationProjectionHandle::new(10).raw(), 10);
    }
}
