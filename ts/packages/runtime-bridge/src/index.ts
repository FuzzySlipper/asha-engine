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
export type { BridgeOperation, BridgeSurface } from './generated/operations.js';

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
  ScreenPointToPickRayRequest,
  PickRaySnapshot,
  VoxelSelectionSnapshot,
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
  SceneNodeId,
  SceneNodeRecord,
  SceneObjectCommandRejection,
  SceneObjectCommandRequest,
  SceneObjectCommandResult,
  SceneObjectSnapshot,
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

export { RuntimeBridgeError, frameCursor } from './bridge.js';
export type {
  CompositionStatus,
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
  RuntimeBridgeErrorKind,
  RuntimeBufferHandle,
  RuntimeBufferView,
  StepInputEnvelope,
  StepResult,
  VoxelMeshBoundsEvidence,
  VoxelMeshChunkEvidence,
  VoxelMeshEvidenceRequest,
  VoxelMeshEvidenceSnapshot,
  VoxelMeshStatsEvidence,
  WorldLoadRequest,
  WorldSaveSummary,
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
export * from './browser-fps-input.js';
export * from './combat-feedback.js';
export * from './combat-readout.js';
export * from './encounter-director.js';
export * from './generated-tunnel.js';
export * from './nav-readout.js';
export * from './enemy-policy.js';
export * from './runtime-action.js';
export * from './runtime-session.js';
