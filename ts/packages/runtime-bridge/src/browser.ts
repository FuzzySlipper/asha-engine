// Browser-safe package-root condition for @asha/runtime-bridge.
//
// Browser consumers still import `@asha/runtime-bridge`; package.json selects
// this entry under the `browser` condition so Vite/Webpack do not evaluate the
// native transport module or its Node-only dependency chain.

export { MANIFEST_OPERATIONS } from './generated/operations.js';
export type { BridgeOperation, BridgeSurface } from './generated/operations.js';

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

export {
  decodeRenderDiff,
  decodeRenderFrameDiff,
  RenderDecodeError,
  RenderDiffStream,
  FrameMemory,
} from './render-decode.js';

export { RuntimeBridgeError, frameCursor } from './bridge.js';
export type {
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
} from './bridge.js';
export * from './browser-input-host.js';
export * from './browser-fps-resolved-actions.js';
export * from './resolved-time-control.js';
export { buildRuntimeSessionAnimationControllerTargetFrame } from './runtime-session-animation.js';
export * from './native-runtime-provider.js';
export * from './playable-encounter-tick.js';
export * from './playable-loop-state.js';
export {
  createRuntimeSessionFacade,
  type RuntimeSessionFacadeOptions,
} from './runtime-session-adapter.js';
export {
  createWorkspaceAuthoringFacade,
  RustBackedWorkspaceAuthoringFacade,
} from './workspace-authoring-rust-facade.js';
export type { WorkspaceAuthoringFacadeOptions } from './workspace-authoring-rust-facade.js';
