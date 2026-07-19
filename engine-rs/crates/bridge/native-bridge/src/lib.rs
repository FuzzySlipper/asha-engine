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
    parse_voxel_command_batch_json, EnemyDirectNavMovementRequest, EngineBridge, EngineConfig,
    GameRuleCatalog, GameRuleModuleManifest, GameRuleResolutionRequest, PresentationOpMeta,
    PresentationOriginRef, ProjectBundleLoadRequest, RuntimeBridge, RuntimeBridgeError,
    RuntimeBridgeErrorKind, RuntimeProjectionFrame, StepInputEnvelope, VoxelAnnotationEditRequest,
    VoxelAnnotationLayerExportRequest, VoxelAnnotationLayerLoadRequest,
    VoxelAnnotationLayerValidationRequest, VoxelAnnotationQueryRequest,
    VoxelConversionApplyRequest, VoxelConversionEvidenceRef,
    VoxelConversionMeshAssetRegistrationRequest, VoxelConversionMeshSourceImportRequest,
    VoxelConversionPlanRequest, VoxelConversionPreviewRequest,
    VoxelConversionSourceMetadataRequest, VoxelConversionSourceRegistrationRequest,
    VoxelEditHistoryReadRequest, VoxelEditHistoryRedoRequest, VoxelEditHistoryRevertRequest,
    VoxelEditHistoryUndoRequest, VoxelModelInfoRequest, VoxelModelWindowRequest,
    VoxelVolumeAssetExportRequest, VoxelVolumeAssetLoadRequest,
    VoxelVolumeAssetPaletteUpdateRequest, VoxelVolumeAssetSaveRequest,
    VoxelVolumeAssetUnloadRequest, VoxelVolumeAuthoringInitializeRequest, WeaponEffectHookRequest,
    WorkspaceAuthoringCloseRequest, WorkspaceAuthoringOpenRequest,
    WorkspaceAuthoringProjectionRequest, WorkspaceAuthoringStoredConfirmationRequest,
    VOXEL_CONVERSION_MESH_IMPORT_MAX_REQUEST_BYTES, VOXEL_PALETTE_UPDATE_MAX_REQUEST_BYTES,
};
use serde::Serialize;

pub use napi::module_init as native_provider_module_init;

mod animation_projection;
mod audio_projection;
mod billboard_projection;
mod camera;
mod fps;
mod generated;
mod generated_tunnel;
mod input_session;
mod particle_projection;
mod presentation_operation;
mod procedural_environment;
mod project_content;
mod project_source;
mod render_projection;
#[cfg(test)]
mod resource_limit_tests;
mod scene_preview;
mod telemetry_overlay_projection;
mod time_control;
mod voxel_assets;
mod voxel_readout;
#[cfg(test)]
mod voxel_rejection_tests;
mod voxel_rejections;
mod wire;
pub use camera::{
    apply_camera_mode_command, apply_camera_navigation_input,
    apply_collision_constrained_camera_input, apply_first_person_camera_input, create_camera,
    read_camera_controller_state, read_camera_projection, NativeCameraBasis,
    NativeCameraCollisionEvidence, NativeCameraCollisionPolicy, NativeCameraCollisionShape,
    NativeCameraCollisionSnapshot, NativeCameraCreateRequest, NativeCameraPose,
    NativeCameraSnapshot, NativeCollisionAabbEvidence,
    NativeCollisionConstrainedCameraInputEnvelope, NativeFirstPersonCameraInput,
    NativeFirstPersonCameraInputEnvelope, NativePerspectiveProjection, NativeViewportSize,
};
pub use fps::{
    apply_fps_encounter_transition, apply_fps_primary_fire, apply_gameplay_prefab_part_interaction,
    invoke_game_extension_weapon_effect, load_fps_runtime_session, read_composed_runtime_session,
    read_fps_encounter_director, read_fps_runtime_session, read_game_rule_runtime_readout,
    read_gameplay_module_view, restart_fps_runtime_session, submit_game_rule_effect_intent,
    validate_game_rule_catalog, NativeComposedGameplayReadout, NativeComposedRuntimeSessionReadout,
    NativeFpsBoundsCapability, NativeFpsEncounterDirectorSnapshot,
    NativeFpsEncounterLifecycleInput, NativeFpsEncounterStateReadout,
    NativeFpsEncounterTransitionRequest, NativeFpsEncounterTransitionResult,
    NativeFpsEntityHealthReadout, NativeFpsHealth, NativeFpsLifecycleStatus,
    NativeFpsPolicyBinding, NativeFpsPolicyBindingReadout, NativeFpsPrimaryFireResult,
    NativeFpsReadSetEvidence, NativeFpsReplayEvidence, NativeFpsRuntimeSessionSnapshot,
    NativeFpsStoredEntityDefinition, NativeFpsTransformCapability, NativeFpsWeaponMount,
    NativeGameExtensionWeaponEffectInvocationResult, NativeGameplayContractRef,
    NativeGameplayModuleViewSnapshot, NativeGameplayPrefabPartInteractionReceipt,
};
pub use generated_tunnel::apply_generated_tunnel_to_runtime_world;
pub use input_session::{
    apply_input_context_command, configure_input_session, read_input_context_state,
    replay_resolved_input_action, submit_raw_input,
};
use presentation_operation::NativePresentationOp;
pub use procedural_environment::{apply_procedural_environment, preview_procedural_environment};
pub use project_content::{
    apply_project_content_authoring, decode_project_content, encode_project_content,
};
pub use project_source::{
    admit_runtime_project_source_batch, begin_runtime_project_source_resources,
    close_runtime_project, load_runtime_project, stage_runtime_project_source_resource,
    NativeProjectResourceTransaction, NativeStagedProjectResource,
};
pub use render_projection::read_render_diffs;
pub use scene_preview::{
    apply_scene_document_authoring, apply_scene_object_command, decode_scene_document,
    encode_scene_document, read_model_material_preview, read_scene_object_snapshot,
};
pub use time_control::{apply_time_control_command, read_time_control_state};
pub use voxel_assets::{
    export_voxel_volume_asset, import_voxel_conversion_mesh_source,
    initialize_voxel_volume_authoring, load_voxel_volume_asset, save_voxel_volume_asset,
    unload_voxel_volume_asset, update_voxel_volume_asset_palette,
};
pub use voxel_readout::{
    configure_voxel_projection_instances, pick_voxel, pick_voxel_instance,
    read_voxel_mesh_evidence, select_voxel,
};
pub use voxel_rejections::NativeCommandResult;

#[derive(Default)]
struct NativeSessions {
    next_handle: u64,
    bridges: BTreeMap<u64, EngineBridge>,
}

static SESSIONS: OnceLock<Mutex<NativeSessions>> = OnceLock::new();

/// Statically linked bridge constructor installed by a downstream native
/// provider when its addon is loaded. The provider returns the same
/// `EngineBridge` root consumed by the generated N-API operation table; no
/// semantic callback or per-operation hook crosses the transport boundary.
pub type NativeEngineBridgeFactory = fn() -> Result<EngineBridge, RuntimeBridgeError>;
pub type NativeProjectAuthoringBridgeFactory = fn() -> Result<EngineBridge, RuntimeBridgeError>;

static ENGINE_BRIDGE_FACTORY: OnceLock<NativeEngineBridgeFactory> = OnceLock::new();
static PROJECT_AUTHORING_BRIDGE_FACTORY: OnceLock<NativeProjectAuthoringBridgeFactory> =
    OnceLock::new();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeEngineBridgeFactoryInstallError;

impl core::fmt::Display for NativeEngineBridgeFactoryInstallError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str("a native EngineBridge factory is already installed")
    }
}

impl std::error::Error for NativeEngineBridgeFactoryInstallError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeProjectAuthoringBridgeFactoryInstallError;

impl core::fmt::Display for NativeProjectAuthoringBridgeFactoryInstallError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str("a native project-authoring EngineBridge factory is already installed")
    }
}

impl std::error::Error for NativeProjectAuthoringBridgeFactoryInstallError {}

/// Install the one bridge constructor for this native addon image.
///
/// Downstream addons call this from [`native_provider_module_init`] so factory
/// selection completes before JavaScript can invoke `initializeEngine`.
pub fn install_native_engine_bridge_factory(
    factory: NativeEngineBridgeFactory,
) -> Result<(), NativeEngineBridgeFactoryInstallError> {
    ENGINE_BRIDGE_FACTORY
        .set(factory)
        .map_err(|_| NativeEngineBridgeFactoryInstallError)
}

/// Install the authoring-only constructor for this native addon image.
///
/// This factory is intentionally distinct from the runtime constructor: it
/// must return a bridge containing only immutable provider schema/codec
/// authority and no activated gameplay RuntimeSession.
pub fn install_native_project_authoring_bridge_factory(
    factory: NativeProjectAuthoringBridgeFactory,
) -> Result<(), NativeProjectAuthoringBridgeFactoryInstallError> {
    PROJECT_AUTHORING_BRIDGE_FACTORY
        .set(factory)
        .map_err(|_| NativeProjectAuthoringBridgeFactoryInstallError)
}

fn create_engine_bridge() -> Result<EngineBridge, RuntimeBridgeError> {
    match ENGINE_BRIDGE_FACTORY.get() {
        Some(factory) => factory(),
        None => Ok(EngineBridge::new()),
    }
}

fn create_project_authoring_bridge() -> Result<EngineBridge, RuntimeBridgeError> {
    match PROJECT_AUTHORING_BRIDGE_FACTORY.get() {
        Some(factory) => factory(),
        None => Ok(EngineBridge::new()),
    }
}

fn sessions() -> &'static Mutex<NativeSessions> {
    SESSIONS.get_or_init(|| {
        Mutex::new(NativeSessions {
            next_handle: 1,
            bridges: BTreeMap::new(),
        })
    })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct NativeBridgeErrorEnvelope<'a> {
    schema_version: u32,
    code: &'static str,
    operation: &'a str,
    path: String,
    retryable: bool,
    message: String,
    details: Vec<String>,
    provenance: &'static str,
}

fn bounded_text(value: &str, max_chars: usize) -> String {
    value.chars().take(max_chars).collect()
}

/// Mirror of the typed boundary error as a versioned, bounded wire envelope.
fn to_napi(err: RuntimeBridgeError) -> napi::Error {
    let envelope = NativeBridgeErrorEnvelope {
        schema_version: 1,
        code: err.kind.code(),
        operation: "native_bridge",
        path: bounded_text(&err.path, 256),
        retryable: err.kind.retryable(),
        message: bounded_text(&err.message, 512),
        details: err
            .details
            .iter()
            .take(8)
            .map(|detail| bounded_text(detail, 128))
            .collect(),
        provenance: "native_rust",
    };
    let reason = serde_json::to_string(&envelope).unwrap_or_else(|_| {
        r#"{"schemaVersion":1,"code":"internal","operation":"native_bridge","path":"$","retryable":false,"message":"failed to encode native error","details":["error_envelope_encoding_failed"],"provenance":"native_rust"}"#.to_owned()
    });
    napi::Error::new(napi::Status::GenericFailure, reason)
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
    f: impl FnOnce(&mut EngineBridge) -> napi::Result<T>,
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

#[derive(Debug, PartialEq, Eq)]
#[napi(object)]
pub struct NativeStepResult {
    pub tick: i64,
    pub diff_count: u32,
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
pub struct NativeRenderFrameDiff {
    pub ops: Vec<String>,
}

#[napi(object)]
pub struct NativePresentationOriginRef {
    pub kind: String,
    pub id: String,
    pub authority_tick: i64,
    pub causation_id: Option<String>,
    pub correlation_id: Option<String>,
}

impl From<PresentationOriginRef> for NativePresentationOriginRef {
    fn from(value: PresentationOriginRef) -> Self {
        Self {
            kind: match value.kind {
                runtime_bridge_api::PresentationOriginKind::OwnerFact => "ownerFact",
                runtime_bridge_api::PresentationOriginKind::GameplayEvent => "gameplayEvent",
                runtime_bridge_api::PresentationOriginKind::DecisionOutcome => "decisionOutcome",
                runtime_bridge_api::PresentationOriginKind::CapabilityState => "capabilityState",
            }
            .to_string(),
            id: value.id,
            authority_tick: value.authority_tick as i64,
            causation_id: value.causation_id,
            correlation_id: value.correlation_id,
        }
    }
}

#[napi(object)]
pub struct NativePresentationOpMeta {
    pub sequence: u32,
    pub origin: Option<NativePresentationOriginRef>,
}

impl From<PresentationOpMeta> for NativePresentationOpMeta {
    fn from(value: PresentationOpMeta) -> Self {
        Self {
            sequence: value.sequence,
            origin: value.origin.map(NativePresentationOriginRef::from),
        }
    }
}

#[napi(object)]
pub struct NativePresentationFrameDiff {
    pub replay_scope: String,
    pub ops: Vec<NativePresentationOp>,
}

#[napi(object)]
pub struct NativeRuntimeProjectionFrame {
    pub schema_version: u32,
    pub authority_tick: i64,
    pub scene: NativeRenderFrameDiff,
    pub presentation: NativePresentationFrameDiff,
}

#[napi(object)]
pub struct NativeDeveloperConsoleDetail {
    pub code: String,
    pub operation: Option<String>,
    pub resource_kind: Option<String>,
    pub resource_id: Option<String>,
    pub reason: Option<String>,
}

#[napi(object)]
pub struct NativeDeveloperConsoleRecord {
    pub sequence: i64,
    pub severity: String,
    pub category: String,
    pub source: String,
    pub message: String,
    pub correlation: Option<String>,
    pub authority_tick: Option<i64>,
    pub session: Option<String>,
    pub detail: NativeDeveloperConsoleDetail,
}

#[napi(object)]
pub struct NativeDeveloperConsoleSnapshot {
    pub schema_version: u32,
    pub records: Vec<NativeDeveloperConsoleRecord>,
    pub dropped_record_count: i64,
    pub first_sequence: Option<i64>,
    pub next_sequence: i64,
    pub snapshot_hash: String,
}

impl From<runtime_bridge_api::DeveloperConsoleSnapshot> for NativeDeveloperConsoleSnapshot {
    fn from(value: runtime_bridge_api::DeveloperConsoleSnapshot) -> Self {
        Self {
            schema_version: value.schema_version,
            records: value
                .records
                .into_iter()
                .map(|record| NativeDeveloperConsoleRecord {
                    sequence: record.sequence as i64,
                    severity: record.severity.as_str().to_owned(),
                    category: serde_json::to_value(record.category)
                        .expect("console category serializes")
                        .as_str()
                        .expect("console category is a string")
                        .to_owned(),
                    source: serde_json::to_value(record.source)
                        .expect("console source serializes")
                        .as_str()
                        .expect("console source is a string")
                        .to_owned(),
                    message: record.message,
                    correlation: record.correlation,
                    authority_tick: record.authority_tick.map(|tick| tick as i64),
                    session: record.session,
                    detail: NativeDeveloperConsoleDetail {
                        code: record.detail.code,
                        operation: record.detail.operation,
                        resource_kind: record.detail.resource_kind,
                        resource_id: record.detail.resource_id,
                        reason: record.detail.reason,
                    },
                })
                .collect(),
            dropped_record_count: value.dropped_record_count as i64,
            first_sequence: value.first_sequence.map(|sequence| sequence as i64),
            next_sequence: value.next_sequence as i64,
            snapshot_hash: value.snapshot_hash,
        }
    }
}

impl From<RuntimeProjectionFrame> for NativeRuntimeProjectionFrame {
    fn from(value: RuntimeProjectionFrame) -> Self {
        debug_assert!(
            value.scene.ops.is_empty(),
            "native scene compatibility is empty today"
        );
        Self {
            schema_version: u32::from(value.schema_version),
            authority_tick: value.authority_tick as i64,
            scene: NativeRenderFrameDiff { ops: Vec::new() },
            presentation: NativePresentationFrameDiff {
                replay_scope: "excludedFromReplayTruth".to_string(),
                ops: value
                    .presentation
                    .ops
                    .into_iter()
                    .map(NativePresentationOp::from)
                    .collect(),
            },
        }
    }
}

#[napi(object)]
pub struct NativeProjectBundleSaveSummary {
    pub artifacts_written: u32,
    pub compacted_edits: u32,
    pub retained_edits: u32,
}

#[napi(object)]
pub struct NativeRuntimeBufferView {
    pub handle: i64,
    pub bytes: Vec<u8>,
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

macro_rules! wire_parser {
    ($name:ident, $output:ty, $operation:literal) => {
        fn $name(payload: &str) -> napi::Result<$output> {
            crate::wire::parse_wire_json($operation, payload)
        }
    };
}

wire_parser!(
    parse_voxel_conversion_plan_request,
    VoxelConversionPlanRequest,
    "plan_voxel_conversion"
);
wire_parser!(
    parse_voxel_conversion_source_registration_request,
    VoxelConversionSourceRegistrationRequest,
    "register_voxel_conversion_source"
);
wire_parser!(
    parse_voxel_conversion_mesh_asset_registration_request,
    VoxelConversionMeshAssetRegistrationRequest,
    "register_voxel_conversion_mesh_asset"
);
wire_parser!(
    parse_voxel_conversion_source_metadata_request,
    VoxelConversionSourceMetadataRequest,
    "read_voxel_conversion_source_metadata"
);
wire_parser!(
    parse_voxel_conversion_preview_request,
    VoxelConversionPreviewRequest,
    "preview_voxel_conversion"
);
wire_parser!(
    parse_voxel_conversion_apply_request,
    VoxelConversionApplyRequest,
    "apply_voxel_conversion"
);
wire_parser!(
    parse_voxel_conversion_evidence,
    Vec<VoxelConversionEvidenceRef>,
    "export_voxel_conversion_evidence"
);
wire_parser!(
    parse_voxel_model_info_request,
    VoxelModelInfoRequest,
    "read_voxel_model_info"
);
wire_parser!(
    parse_voxel_model_window_request,
    VoxelModelWindowRequest,
    "read_voxel_model_window"
);
wire_parser!(
    parse_voxel_annotation_validation_request,
    VoxelAnnotationLayerValidationRequest,
    "validate_voxel_annotation_layer"
);
wire_parser!(
    parse_voxel_annotation_load_request,
    VoxelAnnotationLayerLoadRequest,
    "load_voxel_annotation_layer"
);
wire_parser!(
    parse_voxel_annotation_query_request,
    VoxelAnnotationQueryRequest,
    "read_voxel_annotation_query"
);
wire_parser!(
    parse_voxel_annotation_edit_request,
    VoxelAnnotationEditRequest,
    "apply_voxel_annotation_edit"
);
wire_parser!(
    parse_voxel_annotation_export_request,
    VoxelAnnotationLayerExportRequest,
    "export_voxel_annotation_layer"
);
wire_parser!(
    parse_voxel_edit_history_read_request,
    VoxelEditHistoryReadRequest,
    "read_voxel_edit_history"
);
wire_parser!(
    parse_voxel_edit_history_revert_request,
    VoxelEditHistoryRevertRequest,
    "apply_voxel_edit_revert"
);
wire_parser!(
    parse_voxel_edit_history_undo_request,
    VoxelEditHistoryUndoRequest,
    "undo_voxel_edit"
);
wire_parser!(
    parse_voxel_edit_history_redo_request,
    VoxelEditHistoryRedoRequest,
    "redo_voxel_edit"
);
wire_parser!(
    parse_game_rule_module_manifests,
    Vec<GameRuleModuleManifest>,
    "load_fps_runtime_session"
);
wire_parser!(
    parse_weapon_effect_hook_request,
    WeaponEffectHookRequest,
    "invoke_game_extension_weapon_effect"
);
wire_parser!(
    parse_game_rule_catalog,
    GameRuleCatalog,
    "validate_game_rule_catalog"
);
wire_parser!(
    parse_game_rule_resolution_request,
    GameRuleResolutionRequest,
    "submit_game_rule_effect_intent"
);

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

/// Construct a stateful native engine bridge from a deterministic seed and
/// return the opaque handle used by subsequent native operations.
#[napi]
pub fn initialize_engine(seed: i64) -> napi::Result<i64> {
    if seed < 0 {
        return Err(to_napi(RuntimeBridgeError::new(
            RuntimeBridgeErrorKind::InvalidInput,
            "seed must be non-negative",
        )));
    }
    let mut bridge = create_engine_bridge().map_err(to_napi)?;
    bridge
        .initialize_engine(EngineConfig { seed: seed as u64 })
        .map_err(to_napi)?;

    insert_native_bridge(bridge)
}

fn insert_native_bridge(bridge: EngineBridge) -> napi::Result<i64> {
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

fn workspace_authoring_json<T: Serialize>(value: &T) -> napi::Result<String> {
    serde_json::to_string(value).map_err(|error| {
        to_napi(RuntimeBridgeError::new(
            RuntimeBridgeErrorKind::Internal,
            format!("failed to serialize workspace-authoring receipt: {error}"),
        ))
    })
}

/// Construct the authoring-only authority cell and return its independent
/// native handle plus Rust-authored lifecycle state.
#[napi]
pub fn open_workspace_authoring(existing_handle: i64, request_json: String) -> napi::Result<i64> {
    let request = wire::parse_wire_json::<WorkspaceAuthoringOpenRequest>(
        "open_workspace_authoring",
        &request_json,
    )?;
    if existing_handle >= 0 {
        return with_bridge(existing_handle, |bridge| {
            bridge.open_workspace_authoring(request).map_err(to_napi)?;
            Ok(existing_handle)
        });
    }
    let mut bridge = create_project_authoring_bridge().map_err(to_napi)?;
    bridge.open_workspace_authoring(request).map_err(to_napi)?;
    insert_native_bridge(bridge)
}

#[napi]
pub fn read_workspace_authoring_state(handle: i64) -> napi::Result<String> {
    with_bridge(handle, |bridge| {
        bridge
            .read_workspace_authoring_state()
            .map_err(to_napi)
            .and_then(|state| workspace_authoring_json(&state))
    })
}

#[napi]
pub fn read_workspace_authoring_projection(
    handle: i64,
    request_json: String,
) -> napi::Result<String> {
    let request = wire::parse_wire_json::<WorkspaceAuthoringProjectionRequest>(
        "read_workspace_authoring_projection",
        &request_json,
    )?;
    with_bridge(handle, |bridge| {
        bridge
            .read_workspace_authoring_projection(request)
            .map_err(to_napi)
            .and_then(|receipt| workspace_authoring_json(&receipt))
    })
}

#[napi]
pub fn confirm_workspace_authoring_stored(
    handle: i64,
    request_json: String,
) -> napi::Result<String> {
    let request = wire::parse_wire_json::<WorkspaceAuthoringStoredConfirmationRequest>(
        "confirm_workspace_authoring_stored",
        &request_json,
    )?;
    with_bridge(handle, |bridge| {
        bridge
            .confirm_workspace_authoring_stored(request)
            .map_err(to_napi)
            .and_then(|receipt| workspace_authoring_json(&receipt))
    })
}

#[napi]
pub fn close_workspace_authoring(handle: i64, request_json: String) -> napi::Result<String> {
    let request = wire::parse_wire_json::<WorkspaceAuthoringCloseRequest>(
        "close_workspace_authoring",
        &request_json,
    )?;
    with_bridge(handle, |bridge| {
        bridge
            .close_workspace_authoring(request)
            .map_err(to_napi)
            .and_then(|receipt| workspace_authoring_json(&receipt))
    })
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
    let batch = parse_voxel_command_batch_json(&commands_json).map_err(to_napi)?;
    with_bridge(handle, |bridge| {
        bridge
            .submit_commands(batch)
            .map(NativeCommandResult::from)
            .map_err(to_napi)
    })
}

#[napi]
pub fn step_simulation(handle: i64, tick: i64) -> napi::Result<NativeStepResult> {
    let tick = u64_input(tick, "tick")?;
    with_bridge(handle, |bridge| {
        bridge
            .step_simulation(StepInputEnvelope { tick })
            .map(|result| NativeStepResult {
                tick: result.tick as i64,
                diff_count: result.diff_count,
            })
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
pub fn read_projection_frame(
    handle: i64,
    cursor: i64,
) -> napi::Result<NativeRuntimeProjectionFrame> {
    let cursor = u64_input(cursor, "cursor")?;
    with_bridge(handle, |bridge| {
        bridge
            .read_projection_frame(cursor)
            .map(NativeRuntimeProjectionFrame::from)
            .map_err(to_napi)
    })
}

#[napi]
pub fn read_developer_console(handle: i64) -> napi::Result<NativeDeveloperConsoleSnapshot> {
    with_bridge(handle, |bridge| {
        bridge
            .read_developer_console()
            .map(NativeDeveloperConsoleSnapshot::from)
            .map_err(to_napi)
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
pub fn get_buffer(handle: i64, buffer_handle: i64) -> napi::Result<NativeRuntimeBufferView> {
    let buffer_handle = u64_input(buffer_handle, "buffer_handle")?;
    with_bridge(handle, |bridge| {
        let view = bridge
            .get_buffer(runtime_bridge_api::RuntimeBufferHandle::new(buffer_handle))
            .map_err(to_napi)?;
        Ok(NativeRuntimeBufferView {
            handle: view.handle.raw() as i64,
            bytes: view.bytes.to_vec(),
        })
    })
}

#[napi]
pub fn release_buffer(handle: i64, buffer_handle: i64) -> napi::Result<()> {
    let buffer_handle = u64_input(buffer_handle, "buffer_handle")?;
    with_bridge(handle, |bridge| {
        bridge
            .release_buffer(runtime_bridge_api::RuntimeBufferHandle::new(buffer_handle))
            .map_err(to_napi)
    })
}

#[napi]
pub fn unload_project_bundle(handle: i64) -> napi::Result<()> {
    with_bridge(handle, |bridge| {
        bridge.unload_project_bundle().map_err(to_napi)
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
pub fn read_voxel_conversion_source_metadata(
    handle: i64,
    request_json: String,
) -> napi::Result<String> {
    let request = parse_voxel_conversion_source_metadata_request(&request_json)?;
    with_bridge(handle, |bridge| {
        let readout = bridge
            .read_voxel_conversion_source_metadata(request)
            .map_err(to_napi)?;
        voxel_conversion_json(&readout)
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

#[napi]
pub fn read_voxel_edit_history(handle: i64, request_json: String) -> napi::Result<String> {
    let request = parse_voxel_edit_history_read_request(&request_json)?;
    with_bridge(handle, |bridge| {
        let summary = bridge.read_voxel_edit_history(request).map_err(to_napi)?;
        voxel_conversion_json(&summary)
    })
}

#[napi]
pub fn preview_voxel_edit_revert(handle: i64, request_json: String) -> napi::Result<String> {
    let request = parse_voxel_edit_history_revert_request(&request_json)?;
    with_bridge(handle, |bridge| {
        let receipt = bridge.preview_voxel_edit_revert(request).map_err(to_napi)?;
        voxel_conversion_json(&receipt)
    })
}

#[napi]
pub fn apply_voxel_edit_revert(handle: i64, request_json: String) -> napi::Result<String> {
    let request = parse_voxel_edit_history_revert_request(&request_json)?;
    with_bridge(handle, |bridge| {
        let receipt = bridge.apply_voxel_edit_revert(request).map_err(to_napi)?;
        voxel_conversion_json(&receipt)
    })
}

#[napi]
pub fn undo_voxel_edit(handle: i64, request_json: String) -> napi::Result<String> {
    let request = parse_voxel_edit_history_undo_request(&request_json)?;
    with_bridge(handle, |bridge| {
        let receipt = bridge.undo_voxel_edit(request).map_err(to_napi)?;
        voxel_conversion_json(&receipt)
    })
}

#[napi]
pub fn redo_voxel_edit(handle: i64, request_json: String) -> napi::Result<String> {
    let request = parse_voxel_edit_history_redo_request(&request_json)?;
    with_bridge(handle, |bridge| {
        let receipt = bridge.redo_voxel_edit(request).map_err(to_napi)?;
        voxel_conversion_json(&receipt)
    })
}

#[cfg(test)]
mod tests;
