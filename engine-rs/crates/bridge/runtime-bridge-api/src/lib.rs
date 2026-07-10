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

pub(crate) use std::collections::{BTreeMap, BTreeSet};

pub(crate) use core_commands::VoxelCommand;
pub(crate) use core_entity::{
    EntityLifecycleCommand, EntitySource, EntityStore, EntityTransform, TransformCommand,
    TransformError,
};
pub(crate) use core_error::ErrorCategory;
pub(crate) use core_ids::EntityId;
pub(crate) use core_math::Vec3;
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
pub(crate) use protocol_diagnostics::DiagnosticSeverity;
pub(crate) use protocol_entity_authoring::{
    AuthoringTransform, EntityDefinition, EntityDefinitionCapability, EntityDefinitionSourceTrace,
};
pub use protocol_game_rules::{
    GameRuleCatalog, GameRuleDiagnostic, GameRuleEvidenceKind, GameRuleEvidenceRef,
    GameRuleModifierState, GameRuleResolutionReceipt, GameRuleResolutionRequest,
    GameRuleTraceEntry,
};
pub(crate) use protocol_render::{
    MeshAttribute, MeshAttributeKind, MeshAttributeName, MeshBoundsDescriptor, MeshBufferLayout,
    MeshCollisionPolicy, MeshGroupDescriptor, MeshIndexWidth, MeshMaterialSlot,
    MeshPayloadDescriptor, MeshPayloadSource, MeshProvenance, StaticMeshAsset,
};
#[cfg(test)]
pub(crate) use protocol_view::CameraCollisionPolicy;
pub use protocol_view::{
    CameraBasis, CameraCreateRequest, CameraPose, CameraSnapshot, PerspectiveProjection,
    ViewportSize,
};
pub(crate) use protocol_view::{
    CameraCollisionEvidence, CameraCollisionPolicyMode, CameraCollisionShape,
    CameraCollisionSnapshot, CameraProjectionRequest, CameraProjectionSnapshot,
    CollisionAabbEvidence, CollisionAxis, CollisionConstrainedCameraInputEnvelope,
    FirstPersonCameraInput, FirstPersonCameraInputEnvelope, PickRaySnapshot, ScreenPoint,
    ScreenPointSpace, ScreenPointToPickRayRequest, VoxelSelectionOutcome, VoxelSelectionSnapshot,
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
};
pub use protocol_voxel_conversion::{
    VoxelConversionApplyRequest, VoxelConversionDiagnostic, VoxelConversionDiagnosticCode,
    VoxelConversionEvidenceRef, VoxelConversionMeshAssetRegistrationRequest, VoxelConversionPlan,
    VoxelConversionPlanRequest, VoxelConversionPreview, VoxelConversionPreviewRequest,
    VoxelConversionReceipt, VoxelConversionSourceBounds, VoxelConversionSourceGroupMetadata,
    VoxelConversionSourceMaterialSlot, VoxelConversionSourceMetadataReadout,
    VoxelConversionSourceMetadataRequest, VoxelConversionSourceRegistration,
    VoxelConversionSourceRegistrationRequest, VoxelModelInfoReadout, VoxelModelInfoRequest,
    VoxelModelMaterialCount, VoxelModelWindowReadout, VoxelModelWindowRequest,
    VoxelModelWindowSample,
};
pub use protocol_voxel_edit_history::{
    VoxelEditHistoryReadRequest, VoxelEditHistoryRedoReceipt, VoxelEditHistoryRedoRequest,
    VoxelEditHistoryRevertReceipt, VoxelEditHistoryRevertRequest, VoxelEditHistorySummary,
    VoxelEditHistoryUndoReceipt, VoxelEditHistoryUndoRequest,
};
pub(crate) use rule_lifecycle::{
    load_fps_project_bundle, FpsEncounterLastTransition,
    FpsEncounterLifecycleInput as RuleFpsEncounterLifecycleInput, FpsEncounterState,
    FpsEncounterStatus, FpsEncounterTransitionAction, FpsEncounterTransitionReceipt,
    FpsLifecycleStatus, FpsPolicyBinding, FpsPrimaryFireReceipt, FpsProjectBundleLoadInput,
    FpsRenderProjectionState, FpsRuntimeError, FpsRuntimeRole, FpsRuntimeSessionState,
    FpsStoredEntityDefinition, FpsWeaponMount,
};
pub(crate) use rule_voxel_edit::VoxelEditRejection;
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
mod handles;
mod payloads;

pub use authority::EngineBridge;
pub use bridge::RuntimeBridge;
pub use buffer_provider::{
    fixtures, BufferKind, BufferLifetime, BufferMetadata, RuntimeBufferProvider,
};
pub use errors::{BridgeResult, RuntimeBridgeError, RuntimeBridgeErrorKind};
pub use handles::{
    EngineHandle, FrameCursor, ReplaySessionHandle, RuntimeBufferHandle, RuntimeBufferView,
};
pub use payloads::*;
