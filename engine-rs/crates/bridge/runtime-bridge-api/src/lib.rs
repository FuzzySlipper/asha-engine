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

pub(crate) use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
};

pub(crate) use core_assets::{AssetHash, AssetId, AssetReference, AssetVersionReq};
pub(crate) use core_catalog::{Catalog, CatalogEntry};
#[cfg(test)]
pub(crate) use core_commands::CommandEnvelope;
pub(crate) use core_commands::VoxelCommand;
pub(crate) use core_entity::{
    EntityLifecycleCommand, EntitySource, EntityStore, EntityTransform, TransformCommand,
    TransformError,
};
pub(crate) use core_error::ErrorCategory;
pub(crate) use core_ids::{EntityId, SceneId, SceneNodeId};
pub use core_math::Vec3;
pub(crate) use core_space::{
    ChunkCoord, ChunkDims, Direction6, Face, GridId, VoxelCoord, VoxelGridSpec, WorldPos, WorldVec,
};
pub(crate) use core_voxel::{MaterialCatalog, VoxelMaterialId, VoxelValue};
pub(crate) use game_rule_extension::{
    proposed_receipt, rejected_receipt, unsupported_hook_diagnostic, GameExtensionDiagnostic,
    GameRuleExtensionResult, GameRuleModule,
};
pub use game_rule_extension::{
    GameExtensionDiagnosticCode, GameExtensionHookKind, GameExtensionHookReceipt,
    GameExtensionProposal, GameExtensionReceiptStatus, GameExtensionReplayEvidence,
    GameExtensionTraceEntry, GameRuleHookDeclaration, GameRuleModuleManifest, GameRuleModuleRef,
    WeaponEffectHookRequest,
};
pub use protocol_diagnostics::{
    DeveloperConsoleCategory, DeveloperConsoleDetail, DeveloperConsoleRecord,
    DeveloperConsoleSnapshot, DeveloperConsoleSource, DiagnosticSeverity,
    DEVELOPER_CONSOLE_MAX_RECORDS, DEVELOPER_CONSOLE_MAX_RECORDS_PER_TICK,
    DEVELOPER_CONSOLE_SCHEMA_VERSION,
};
pub(crate) use protocol_entity_authoring::{
    AuthoringTransform, EntityDefinition, EntityDefinitionCapability, EntityDefinitionSourceTrace,
};
pub use protocol_game_extension::GameplayContractRef;
pub use protocol_game_rules::{
    GameRuleCatalog, GameRuleDiagnostic, GameRuleEvidenceKind, GameRuleEvidenceRef,
    GameRuleModifierState, GameRuleResolutionReceipt, GameRuleResolutionRequest,
    GameRuleTraceEntry,
};
pub use protocol_input::{
    InputActionReplayReceipt, InputBindingCatalog, InputContextChangeReceipt, InputContextCommand,
    InputContextStackState, InputResolutionReceipt, InputSessionConfigureRequest,
    InputSessionSnapshot, RawInputSample, RecordedInputAction,
};
pub use protocol_presentation::{
    AnimationControllerProjectionState, AnimationProjectionDescriptor,
    AnimationProjectionDiagnostic, AnimationProjectionDiagnosticCode, AnimationProjectionHandle,
    AnimationProjectionOp, AnimationProjectionReadout, AnimationResolvedMotion,
    AnimationTransitionFactMoment, AnimationTransitionFactRef, AnimationTransitionProjection,
    AudioBus, AudioClipRef, AudioEmitter, AudioHandle, AudioProjectionDiagnostic,
    AudioProjectionDiagnosticCode, AudioProjectionOp, AudioProjectionReadout,
    AudioSourceDescriptor, AudioSourcePatch, BillboardAnchor, BillboardContent,
    BillboardDescriptor, BillboardFontRef, BillboardHandle, BillboardLayer, BillboardPatch,
    BillboardProjectionDiagnostic, BillboardProjectionDiagnosticCode, BillboardProjectionOp,
    BillboardProjectionReadout, BillboardTemplateArgument, BillboardTextureRef, ParticleAnchor,
    ParticleColorKey, ParticleEmitterDescriptor, ParticleEmitterHandle, ParticleEmitterPatch,
    ParticleProjectionDiagnostic, ParticleProjectionDiagnosticCode, ParticleProjectionOp,
    ParticleProjectionReadout, ParticleScalarKey, ParticleSpriteRef, PresentationFrameDiff,
    PresentationOp, PresentationOpMeta, PresentationOriginKind, PresentationOriginRef,
    ProjectionReplayScope, RuntimeProjectionFrame, TelemetryOverlayCorner,
    TelemetryOverlayDescriptor, TelemetryOverlayDiagnostic, TelemetryOverlayDiagnosticCode,
    TelemetryOverlayHandle, TelemetryOverlayPatch, TelemetryOverlayProjectionOp,
    TelemetryOverlayReadout, RUNTIME_PROJECTION_SCHEMA_VERSION,
};
pub(crate) use protocol_render::{
    MeshAttribute, MeshAttributeKind, MeshAttributeName, MeshBoundsDescriptor, MeshBufferLayout,
    MeshCollisionPolicy, MeshGroupDescriptor, MeshIndexWidth, MeshMaterialSlot,
    MeshPayloadDescriptor, MeshPayloadSource, MeshProvenance, StaticMeshAsset,
};
pub use protocol_render::{ModelMaterialPreviewRequest, ModelMaterialPreviewSnapshot};
pub use protocol_scene::{
    AssetReferenceDto, AssetVersionReqDto, FlatSceneDocumentDto, SceneDocumentCodecDiagnosticCode,
    SceneDocumentCodecDiagnosticDto, SceneDocumentCodecResultDto, SceneDocumentDecodeRequestDto,
    SceneDocumentEncodeRequestDto, SceneMetadataDto, SceneNodeKindDto, SceneNodeRecordDto,
    SceneObjectCommandDto, SceneObjectCommandOutcomeDto, SceneObjectCommandRejectionCode,
    SceneObjectCommandRejectionDto, SceneObjectCommandRequestDto, SceneObjectCommandResultDto,
    SceneObjectRecordDto, SceneObjectSnapshotDto, SceneTransformDto, SceneValidationCode,
    SceneValidationErrorDto, SceneValidationReportDto,
};
pub use protocol_time_control::{
    TimeControlCommand, TimeControlMode, TimeControlReceipt, TimeControlRejection,
    TimeControlState, TIME_CONTROL_STATE_SCHEMA_VERSION,
};
#[cfg(test)]
pub(crate) use protocol_view::CameraCollisionPolicy;
pub use protocol_view::{
    CameraBasis, CameraControllerReadRequest, CameraControllerRejection, CameraControllerState,
    CameraCreateRequest, CameraHandle, CameraMode, CameraModeChangeReceipt, CameraModeCommand,
    CameraModeTarget, CameraNavigationInput, CameraNavigationInputEnvelope,
    CameraNavigationReceipt, CameraPose, CameraSnapshot, CameraTransitionEasing,
    CameraTransitionReadout, CameraTransitionSpec, GeneratedTunnelPreset,
    GeneratedTunnelRuntimeApplyReceipt, GeneratedTunnelRuntimeApplyRequest,
    GeneratedTunnelRuntimeFrame, PerspectiveProjection, ViewportSize,
    CAMERA_CONTROLLER_STATE_SCHEMA_VERSION,
};
pub(crate) use protocol_view::{
    CameraCollisionEvidence, CameraCollisionPolicyMode, CameraCollisionShape,
    CameraCollisionSnapshot, CameraProjectionRequest, CameraProjectionSnapshot,
    CollisionAabbEvidence, CollisionAxis, CollisionConstrainedCameraInputEnvelope,
    FirstPersonCameraInput, FirstPersonCameraInputEnvelope, FirstPersonMovementMode,
    PickRaySnapshot, ScreenPoint, ScreenPointSpace, ScreenPointToPickRayRequest,
    VoxelSelectionOutcome, VoxelSelectionSnapshot,
};
pub(crate) use protocol_voxel_annotation::{
    VoxelAnnotationDiagnostic, VoxelAnnotationDiagnosticCode, VoxelAnnotationEditOperation,
};
pub use protocol_voxel_annotation::{
    VoxelAnnotationEditReceipt, VoxelAnnotationEditRequest, VoxelAnnotationLayer,
    VoxelAnnotationLayerDraft, VoxelAnnotationLayerExportReceipt,
    VoxelAnnotationLayerExportRequest, VoxelAnnotationLayerLoadReceipt,
    VoxelAnnotationLayerLoadRequest, VoxelAnnotationLayerValidationInput,
    VoxelAnnotationLayerValidationReport, VoxelAnnotationLayerValidationRequest,
    VoxelAnnotationQueryReadout, VoxelAnnotationQueryRequest, VoxelAnnotationRegion,
    VoxelAnnotationSelection, VoxelAnnotationSparseRun,
};
pub(crate) use protocol_voxel_asset::{
    VoxelAssetAuthoringMetadata, VoxelAssetBounds, VoxelAssetContentHashes, VoxelAssetCoord,
    VoxelAssetDiagnostic, VoxelAssetDiagnosticCode, VoxelAssetGrid, VoxelAssetMaterialBinding,
    VoxelAssetMaterialCount, VoxelAssetProvenanceKind, VoxelAssetProvenanceRef,
    VoxelAssetRepresentation, VoxelAssetRepresentationKind, VoxelAssetSparseRun, VoxelVolumeAsset,
    VOXEL_ASSET_EXTENSION, VOXEL_ASSET_MEDIA_TYPE, VOXEL_ASSET_SCHEMA_VERSION,
};
pub use protocol_voxel_asset::{
    VoxelVolumeAssetExportReceipt, VoxelVolumeAssetExportRequest, VoxelVolumeAssetLoadReceipt,
    VoxelVolumeAssetLoadRequest, VoxelVolumeAssetPaletteStoredDiff,
    VoxelVolumeAssetPaletteUpdateReceipt, VoxelVolumeAssetPaletteUpdateRequest,
    VoxelVolumeAssetSaveReceipt, VoxelVolumeAssetSaveRequest, VoxelVolumeAssetStoredDiff,
    VoxelVolumeAssetUnloadReceipt, VoxelVolumeAssetUnloadRequest,
    VoxelVolumeAuthoringInitializeReceipt, VoxelVolumeAuthoringInitializeRequest,
    VOXEL_PALETTE_UPDATE_MAX_EMBEDDED_DIAGNOSTICS, VOXEL_PALETTE_UPDATE_MAX_MATERIAL_BINDINGS,
    VOXEL_PALETTE_UPDATE_MAX_PROVENANCE_REFS, VOXEL_PALETTE_UPDATE_MAX_REPRESENTED_VOXELS,
    VOXEL_PALETTE_UPDATE_MAX_REQUEST_BYTES, VOXEL_PALETTE_UPDATE_MAX_SPARSE_RUNS,
    VOXEL_PALETTE_UPDATE_MAX_STRING_BYTES,
};
pub use protocol_voxel_conversion::{
    VoxelConversionApplyRequest, VoxelConversionDiagnostic, VoxelConversionDiagnosticCode,
    VoxelConversionEvidenceRef, VoxelConversionMeshAssetRegistrationRequest,
    VoxelConversionMeshSourceFormat, VoxelConversionMeshSourceImportReceipt,
    VoxelConversionMeshSourceImportRequest, VoxelConversionPlan, VoxelConversionPlanRequest,
    VoxelConversionPreview, VoxelConversionPreviewRequest, VoxelConversionReceipt,
    VoxelConversionSourceBounds, VoxelConversionSourceGroupMetadata,
    VoxelConversionSourceMaterialSlot, VoxelConversionSourceMetadataReadout,
    VoxelConversionSourceMetadataRequest, VoxelConversionSourceRegistration,
    VoxelConversionSourceRegistrationRequest, VoxelModelInfoReadout, VoxelModelInfoRequest,
    VoxelModelMaterialCount, VoxelModelWindowReadout, VoxelModelWindowRequest,
    VoxelModelWindowSample, VOXEL_CONVERSION_MESH_IMPORT_MAX_ASSET_ID_BYTES,
    VOXEL_CONVERSION_MESH_IMPORT_MAX_INDICES, VOXEL_CONVERSION_MESH_IMPORT_MAX_PRIMITIVE_BYTES,
    VOXEL_CONVERSION_MESH_IMPORT_MAX_REQUEST_BYTES, VOXEL_CONVERSION_MESH_IMPORT_MAX_SOURCE_BYTES,
    VOXEL_CONVERSION_MESH_IMPORT_MAX_SOURCE_PATH_BYTES, VOXEL_CONVERSION_MESH_IMPORT_MAX_VERTICES,
};
pub use protocol_voxel_edit_history::{
    VoxelEditHistoryReadRequest, VoxelEditHistoryRedoReceipt, VoxelEditHistoryRedoRequest,
    VoxelEditHistoryRevertReceipt, VoxelEditHistoryRevertRequest, VoxelEditHistorySummary,
    VoxelEditHistoryUndoReceipt, VoxelEditHistoryUndoRequest,
};
pub(crate) use render_audio::AudioProjector;
pub(crate) use render_billboard::BillboardProjector;
pub(crate) use render_particle::{ParticleProjectionLimits, ParticleProjector};
pub(crate) use render_telemetry_overlay::TelemetryOverlayProjector;
pub(crate) use rule_input::InputSessionResolver;
pub(crate) use rule_lifecycle::{
    load_fps_project_bundle_into, FpsEncounterLastTransition,
    FpsEncounterLifecycleInput as RuleFpsEncounterLifecycleInput, FpsEncounterState,
    FpsEncounterStatus, FpsEncounterTransitionAction, FpsEncounterTransitionReceipt,
    FpsLifecycleStatus, FpsPolicyBinding, FpsPrimaryFireAuthorityInput, FpsPrimaryFireReceipt,
    FpsProjectBundleLoadInput, FpsRenderProjectionState, FpsRuntimeError, FpsRuntimeRole,
    FpsRuntimeSessionState, FpsStoredEntityDefinition, FpsWeaponMount,
};
pub use rule_voxel_edit::VoxelEditRejection;
pub(crate) use sim_runner::{SimulationAuthority, TimeController};
pub(crate) use svc_collision::{CollisionProjection, Ray};
pub(crate) use svc_combat::HealthState;
pub(crate) use svc_game_rules::{resolve_protocol_request, validate_catalog};
pub(crate) use svc_mesh::mesh_chunk_in_world;
pub(crate) use svc_pathfinding::{
    propose_direct_nav_movement, DirectNavMovementError, DirectNavMovementRequest,
};
pub(crate) use svc_serialization::BundleHash;
pub(crate) use svc_spatial::VoxelWorld;
pub(crate) use svc_volume::VoxelChunk;
pub(crate) use svc_voxel_conversion::{MeshTriangle, PlannedConversion, StaticMeshSource};

mod authority;
mod bridge;
pub mod buffer_provider;
mod errors;
mod generated;
mod handles;
mod payloads;

pub use authority::EngineBridge;
pub use authority::{
    ComposedGameplayOwner, ComposedGameplayOwnerCheckpoint, ComposedGameplayOwnerOutput,
    ComposedGameplayOwnerReadout, ComposedGameplayOwnerTransactionReceipt, ComposedGameplayRuntime,
    ComposedGameplayRuntimeBuilder, ComposedRuntimeSessionCheckpoint,
    ComposedRuntimeSessionReadout, StaticRuntimeSessionBuilder,
    StaticRuntimeSessionCompositionError,
};
pub use bridge::RuntimeBridge;
pub use buffer_provider::{
    fixtures, BufferKind, BufferLifetime, BufferMetadata, RuntimeBufferProvider,
};
pub use errors::{BridgeResult, RuntimeBridgeError, RuntimeBridgeErrorKind};
pub use handles::{
    EngineHandle, FrameCursor, ReplaySessionHandle, RuntimeBufferHandle, RuntimeBufferView,
};
pub use payloads::*;
