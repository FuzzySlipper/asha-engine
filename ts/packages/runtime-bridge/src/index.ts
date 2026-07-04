// @asha/runtime-bridge — the public, transport-agnostic runtime facade (ADR 0006).
//
// App / UI / renderer / devtools couple ONLY to this package root for runtime.
// The implementation is split by concern behind this barrel; package.json exports
// only "." so consumers cannot couple to native/mock/launcher internals.
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
  FrameCursor,
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
export * from './launcher.js';
export * from './mock.js';
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
