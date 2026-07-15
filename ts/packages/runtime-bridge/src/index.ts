// @asha/runtime-bridge — the public, transport-agnostic runtime facade (ADR 0006).
//
// App / UI / renderer / devtools couple ONLY to this package root for runtime.
// The implementation is split by concern behind this barrel. Reference/mock
// helpers live at @asha/runtime-bridge/reference so production consumers do not
// casually couple to the deterministic fixture backend.
//
// The facade exports generated-compatible contract types and explicit
// buffer-handle APIs — never raw addon exports, WASM memory, or JSON escape
// hatches. The manifest-derived conformance tests keep these re-exports stable.

export { MANIFEST_OPERATIONS } from './generated/operations.js';
export type {
  BridgeErrorFamily,
  BridgeOperation,
  BridgeOperationDescriptor,
  BridgeSurface,
} from './generated/operations.js';
export {
  ResolvedTimeControlConsumer,
  TIME_CONTROL_INPUT_ACTIONS,
  timeControlCommandFromResolvedAction,
} from './resolved-time-control.js';
export { buildRuntimeSessionAnimationControllerTargetFrame } from './runtime-session-animation.js';
export {
  createWorkspaceAuthoringFacade,
  RustBackedWorkspaceAuthoringFacade,
} from './workspace-authoring-rust-facade.js';
export type { WorkspaceAuthoringFacadeOptions } from './workspace-authoring-rust-facade.js';

// `submit_commands` / `pick_voxel` carry the generated voxel border (manifest
// `protocol_voxel::{CommandBatch, CommandResult, PickRay, PickResult}`). Re-exported
// so consumers still couple only to this facade package for the runtime surface
// (ADR 0006).
export type {
  CameraCreateRequest,
  CameraProjectionRequest,
  CameraProjectionSnapshot,
  CameraSnapshot,
  CameraCollisionSnapshot,
  CollisionConstrainedCameraInputEnvelope,
  FirstPersonMovementMode,
  ScreenPointToPickRayRequest,
  PickRaySnapshot,
  VoxelSelectionSnapshot,
  VoxelConversionApplyRequest,
  VoxelConversionEvidenceRef,
  VoxelConversionMeshAsset,
  VoxelConversionMeshAssetGroup,
  VoxelConversionMeshAssetRegistrationRequest,
  VoxelConversionMeshSourceFormat,
  VoxelConversionMeshSourceImportReceipt,
  VoxelConversionMeshSourceImportRequest,
  VoxelConversionPlan,
  VoxelConversionPlanRequest,
  VoxelConversionPreview,
  VoxelConversionPreviewRequest,
  VoxelConversionReceipt,
  VoxelConversionSourceBounds,
  VoxelConversionSourceGroupMetadata,
  VoxelConversionSourceMaterialSlot,
  VoxelConversionSourceMetadataReadout,
  VoxelConversionSourceMetadataRequest,
  VoxelConversionSourceRegistration,
  VoxelConversionSourceRegistrationRequest,
  VoxelConversionSourceTriangle,
  VoxelModelInfoReadout,
  VoxelModelInfoRequest,
  VoxelModelMaterialCount,
  VoxelModelWindowReadout,
  VoxelModelWindowRequest,
  VoxelModelWindowSample,
  VoxelEditHistoryReadRequest,
  VoxelEditHistoryRedoReceipt,
  VoxelEditHistoryRedoRequest,
  VoxelEditHistoryRevertReceipt,
  VoxelEditHistoryRevertRequest,
  VoxelEditHistorySummary,
  VoxelEditHistoryUndoReceipt,
  VoxelEditHistoryUndoRequest,
  VoxelVolumeAsset,
  VoxelVolumeAssetExportReceipt,
  VoxelVolumeAssetExportRequest,
  VoxelVolumeAssetLoadReceipt,
  VoxelVolumeAssetLoadRequest,
  VoxelVolumeAssetUnloadReceipt,
  VoxelVolumeAssetUnloadRequest,
  VoxelVolumeAssetSaveReceipt,
  VoxelVolumeAssetSaveRequest,
  VoxelVolumeAuthoringInitializeReceipt,
  VoxelVolumeAuthoringInitializeRequest,
  VoxelVolumeAssetStoredDiff,
  CommandBatch,
  CommandResult,
  FirstPersonCameraInputEnvelope,
  PickRay,
  PickResult,
  CatalogEntry,
  MaterialProjection,
  StaticMeshAsset,
  ModelMaterialPreviewRequest,
  ModelMaterialPreviewSnapshot,
  FlatSceneDocument,
  SceneDocumentCodecDiagnostic,
  SceneDocumentCodecDiagnosticCode,
  SceneDocumentCodecResult,
  SceneDocumentDecodeRequest,
  SceneDocumentEncodeRequest,
  SceneNodeId,
  SceneNodeRecord,
  SceneObjectCommandRejection,
  SceneObjectCommandRequest,
  SceneObjectCommandResult,
  SceneObjectSnapshot,
  TimeControlCommand,
  TimeControlMode,
  TimeControlReceipt,
  TimeControlRejection,
  TimeControlState,
} from '@asha/contracts';

// Render-diff decode (moved from the former @asha/wasm-bridge). Transport-neutral
// payload -> contract types; backs `readRenderDiffs`. See render-decode.ts.
export {
  decodeRenderDiff,
  decodeRenderFrameDiff,
  RenderDecodeError,
  RenderDiffStream,
  FrameMemory,
} from './render-decode.js';

export {
  RUNTIME_BRIDGE_PORT_CONTRACTS,
  RuntimeBridgeError,
  frameCursor,
  runtimeBridgePorts,
} from './bridge.js';
export type {
  CompositionStatus,
  EnemyDirectNavAuthoritySource,
  EnemyDirectNavAuthorityTransport,
  EnemyDirectNavMovementRequest,
  EnemyDirectNavMovementResult,
  EngineConfig,
  EngineHandle,
  FpsEncounterDirectorSnapshot,
  FpsEncounterLastTransition,
  FpsEncounterLifecycleInput,
  FpsEncounterStateReadout,
  FpsEncounterStatus,
  FpsEncounterTransitionAction,
  FpsEncounterTransitionRequest,
  FpsEncounterTransitionResult,
  FrameCursor,
  FpsBoundsCapability,
  FpsEntityHealthReadout,
  FpsHealth,
  FpsLifecycleStatus,
  FpsPolicyBinding,
  FpsPolicyBindingReadout,
  FpsPrimaryFireRequest,
  FpsPrimaryFireResult,
  FpsReadSetEvidence,
  FpsReplayEvidence,
  FpsRuntimeAuthorityTransport,
  FpsRuntimeRole,
  FpsRuntimeSessionLoadRequest,
  FpsRuntimeSessionRestartRequest,
  FpsRuntimeSessionSnapshot,
  FpsStoredEntityDefinition,
  FpsTransformCapability,
  FpsWeaponMount,
  ReplayFixture,
  ReplaySessionHandle,
  ReplayStepReport,
  RuntimeBridge,
  RuntimeBridgePortContract,
  RuntimeBridgePortId,
  RuntimeBridgePorts,
  RuntimeBundleLifecyclePort,
  RuntimeCameraPort,
  RuntimeBridgeErrorKind,
  RuntimeGameplayPort,
  RuntimeInputPort,
  RuntimeProjectionPort,
  RuntimeReplayEvidencePort,
  RuntimeSceneEntityPort,
  RuntimeTimeSimulationPort,
  RuntimeVoxelAssetBufferPort,
  RuntimeBufferHandle,
  RuntimeBufferView,
  StepInputEnvelope,
  StepResult,
  VoxelMeshBoundsEvidence,
  VoxelMeshChunkEvidence,
  VoxelMeshEvidenceRequest,
  VoxelMeshEvidenceSnapshot,
  VoxelMeshStatsEvidence,
  ProjectBundleLoadRequest,
  ProjectBundleSaveSummary,
} from './bridge.js';
export {
  SelectedBackendGameRuntimeLauncher,
  createNativeGameRuntimeLauncher,
  createSelectedBackendGameRuntimeLauncher,
  nativeBackendProfile,
  validateGameRuntimeBackendProfile,
} from './launcher.js';
export type {
  GameRuntimeBackendMode,
  GameRuntimeBackendProfile,
  GameRuntimeBackendProfileValidation,
  GameRuntimeBackendTransport,
  GameRuntimeCommandProposalResult,
  GameRuntimeCompatibility,
  GameRuntimeConfig,
  GameRuntimeDiagnostic,
  GameRuntimeDiagnosticCode,
  GameRuntimeEvidenceExport,
  GameRuntimeEvidenceExportRequest,
  GameRuntimeEvidenceRef,
  GameRuntimeLaunchResult,
  GameRuntimeLauncher,
  GameRuntimeMode,
  GameRuntimeNonClaim,
  GameRuntimeProfile,
  GameRuntimeProjectionSummary,
  GameRuntimeRenderDiffSnapshot,
  GameRuntimeReplayExport,
  GameRuntimeReplayExportRequest,
  GameRuntimeResourceProfile,
  GameRuntimeSession,
  GameRuntimeTelemetrySnapshot,
  SelectedBackendLauncherOptions,
} from './launcher.js';
export * from './native.js';
export * from './browser-input-host.js';
export * from './browser-fps-resolved-actions.js';
export * from './resolved-time-control.js';
export * from './resolved-camera-navigation.js';
export * from './native-runtime-provider.js';
export * from './playable-encounter-tick.js';
export * from './playable-loop-state.js';
export {
  createRuntimeSessionFacade,
  type RuntimeSessionFacadeOptions,
} from './runtime-session-adapter.js';
