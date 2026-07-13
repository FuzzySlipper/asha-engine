import type {
  CameraCollisionSnapshot,
  CameraControllerReadRequest,
  CameraControllerState,
  CameraCreateRequest,
  CameraModeChangeReceipt,
  CameraModeCommand,
  CameraNavigationInputEnvelope,
  CameraNavigationReceipt,
  CameraProjectionRequest,
  CameraProjectionSnapshot,
  CameraSnapshot,
  CollisionConstrainedCameraInputEnvelope,
  CommandBatch,
  CommandResult,
  FirstPersonCameraInputEnvelope,
  GameRuleCatalog,
  GameRuleResolutionReceipt,
  InputActionReplayReceipt,
  InputContextChangeReceipt,
  InputContextCommand,
  InputContextStackState,
  InputResolutionReceipt,
  InputSessionConfigureRequest,
  InputSessionSnapshot,
  GeneratedTunnelRuntimeApplyReceipt,
  GeneratedTunnelRuntimeApplyRequest,
  ModelMaterialPreviewRequest,
  ModelMaterialPreviewSnapshot,
  PickRay,
  PickResult,
  RawInputSample,
  RecordedInputAction,
  RenderFrameDiff,
  RuntimeProjectionFrame,
  TimeControlCommand,
  TimeControlReceipt,
  TimeControlState,
  SceneObjectCommandRequest,
  SceneObjectCommandResult,
  SceneObjectSnapshot,
  ScreenPointToPickRayRequest,
  VoxelSelectionSnapshot,
  VoxelConversionApplyRequest,
  VoxelConversionEvidenceRef,
  VoxelConversionMeshAssetRegistrationRequest,
  VoxelConversionMeshSourceImportReceipt,
  VoxelConversionMeshSourceImportRequest,
  VoxelConversionPlan,
  VoxelConversionPlanRequest,
  VoxelConversionPreview,
  VoxelConversionPreviewRequest,
  VoxelConversionReceipt,
  VoxelConversionSourceMetadataReadout,
  VoxelConversionSourceMetadataRequest,
  VoxelConversionSourceRegistration,
  VoxelConversionSourceRegistrationRequest,
  VoxelModelInfoReadout,
  VoxelModelInfoRequest,
  VoxelModelWindowReadout,
  VoxelModelWindowRequest,
  VoxelAnnotationEditReceipt,
  VoxelAnnotationEditRequest,
  VoxelAnnotationLayerExportReceipt,
  VoxelAnnotationLayerExportRequest,
  VoxelAnnotationLayerLoadReceipt,
  VoxelAnnotationLayerLoadRequest,
  VoxelAnnotationLayerValidationReport,
  VoxelAnnotationLayerValidationRequest,
  VoxelAnnotationQueryReadout,
  VoxelAnnotationQueryRequest,
  VoxelEditHistoryReadRequest,
  VoxelEditHistoryRedoReceipt,
  VoxelEditHistoryRedoRequest,
  VoxelEditHistoryRevertReceipt,
  VoxelEditHistoryRevertRequest,
  VoxelEditHistorySummary,
  VoxelEditHistoryUndoReceipt,
  VoxelEditHistoryUndoRequest,
  VoxelVolumeAssetExportReceipt,
  VoxelVolumeAssetExportRequest,
  VoxelVolumeAssetLoadReceipt,
  VoxelVolumeAssetLoadRequest,
  VoxelVolumeAssetUnloadReceipt,
  VoxelVolumeAssetUnloadRequest,
  VoxelVolumeAssetPaletteUpdateReceipt,
  VoxelVolumeAssetPaletteUpdateRequest,
  VoxelVolumeAssetSaveReceipt,
  VoxelVolumeAssetSaveRequest,
  VoxelVolumeAuthoringInitializeReceipt,
  VoxelVolumeAuthoringInitializeRequest,
} from '@asha/contracts';
export type {
  GeneratedTunnelRuntimeApplyReceipt,
  GeneratedTunnelRuntimeApplyRequest,
} from '@asha/contracts';
import type {
  CompositionStatus,
  EnemyDirectNavMovementRequest,
  EnemyDirectNavMovementResult,
  EngineHandle,
  FpsEncounterDirectorSnapshot,
  FpsEncounterLifecycleInput,
  FpsEncounterTransitionRequest,
  FpsEncounterTransitionResult,
  FpsPrimaryFireRequest,
  FpsPrimaryFireResult,
  FpsRuntimeSessionLoadRequest,
  FpsRuntimeSessionRestartRequest,
  FpsRuntimeSessionSnapshot,
  FrameCursor,
  GameExtensionWeaponEffectInvocationRequest,
  GameExtensionWeaponEffectInvocationResult,
  GameRuleCatalogValidationReceipt,
  GameRuleEffectIntentRequest,
  GameRuleRuntimeReadout,
  ProjectBundleLoadRequest,
  StepResult,
} from '@asha/runtime-session';

export type {
  BridgeVec3,
  CompositionStatus,
  EnemyDirectNavAuthoritySource,
  EnemyDirectNavAuthorityTransport,
  EnemyDirectNavMovementRequest,
  EnemyDirectNavMovementResult,
  EngineHandle,
  FpsBoundsCapability,
  FpsEncounterDirectorSnapshot,
  FpsEncounterLastTransition,
  FpsEncounterLifecycleInput,
  FpsEncounterStateReadout,
  FpsEncounterStatus,
  FpsEncounterTransitionAction,
  FpsEncounterTransitionRequest,
  FpsEncounterTransitionResult,
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
  FrameCursor,
  GameExtensionWeaponEffectInvocationRequest,
  GameExtensionWeaponEffectInvocationResult,
  GameRuleCatalogValidationReceipt,
  GameRuleEffectIntentRequest,
  GameRuleRuntimeReadout,
  ProjectBundleLoadRequest,
  StepResult,
} from '@asha/runtime-session';

// ── Opaque handle types ───────────────────────────────────────────────────────
// Branded numbers so a buffer handle can't be passed where an engine handle is
// expected. They carry no transport detail and never expose a StateStore.

export type RuntimeBufferHandle = number & { readonly __brand: 'RuntimeBufferHandle' };
export type ReplaySessionHandle = number & { readonly __brand: 'ReplaySessionHandle' };

export const frameCursor = (frame: number): FrameCursor => frame as FrameCursor;

// ── Error taxonomy ────────────────────────────────────────────────────────────

export type RuntimeBridgeErrorKind =
  | 'not_initialized'
  | 'invalid_input'
  | 'unknown_handle'
  | 'buffer_expired'
  | 'native_unavailable'
  | 'voxel_conversion_unavailable'
  | 'unsupported_source_asset'
  | 'source_hash_mismatch'
  | 'invalid_material_map'
  | 'output_limit_exceeded'
  | 'stale_authority_snapshot'
  | 'conversion_replay_mismatch'
  // A stable operation exists on the facade but has no native implementation
  // wired yet. The native bridge throws this instead of silently falling back to
  // mock/reference behaviour — the seam is explicit and fail-closed.
  | 'operation_unimplemented'
  | 'internal';

/** Typed, classified error for every facade operation. No JSON error blobs. */
export class RuntimeBridgeError extends Error {
  constructor(readonly kind: RuntimeBridgeErrorKind, message: string) {
    super(`runtime bridge error [${kind}]: ${message}`);
    this.name = 'RuntimeBridgeError';
  }
}
export function nonNegativeSafeInteger(value: number, field: string): number {
  if (!Number.isSafeInteger(value) || value < 0) {
    throw new RuntimeBridgeError('invalid_input', `${field} must be a non-negative safe integer`);
  }
  return value;
}

export function u32(value: number, field: string): number {
  nonNegativeSafeInteger(value, field);
  if (value > 0xffffffff) {
    throw new RuntimeBridgeError('invalid_input', `${field} must fit in u32`);
  }
  return value;
}

// ── Prototype operation payloads ──────────────────────────────────────────────
// PROTOTYPE: replaced by generated protocol_runtime / protocol_script contracts
// once the codegen emitter lands. The facade *shape* is the stable part.
//
// The simplified ProjectBundle DTOs below are deliberate subsets of the generated
// bundle/diagnostic contracts (@asha/contracts manifest / SaveSummary /
// DiagnosticReportSet). `world-dto-conformance.test.ts` is a compile-time guard
// that fails when a shared field's type drifts in the generated contract, keeping
// this prototype debt visible until the DTOs are replaced outright.

export interface EngineConfig {
  readonly seed: number;
}
export interface StepInputEnvelope {
  readonly tick: number;
}


// `CommandBatch` / `CommandResult` are NOT prototype DTOs: they are the generated
// voxel command border (imported from `@asha/contracts`). `submitCommands` carries
// the real `VoxelCommand` union — there is no `{ kind: 'smoke-edit' }` placeholder
// command tunnel; an ad-hoc command shape fails to type-check at the call site.
/** Borrowed, read-only view over bridge-owned bytes (large payloads, e.g. mesh). */
export interface RuntimeBufferView {
  readonly handle: RuntimeBufferHandle;
  readonly bytes: Uint8Array;
}
// Quarantined replay payloads.
export interface ReplayFixture {
  readonly name: string;
  readonly steps: number;
}
export interface ReplayStepReport {
  readonly step: number;
  readonly hash: string;
  readonly diverged: boolean;
}
// ProjectBundle load/save composition payloads (#2363). PROTOTYPE: replaced by
// generated bundle/diagnostic contracts once the emitter wires them.

export interface ProjectBundleSaveSummary {
  readonly artifactsWritten: number;
  readonly compactedEdits: number;
  readonly retainedEdits: number;
}
// Compact voxel mesh/remesh evidence (#2646). Prototype DTOs until generated
// protocol_render contracts grow the same shapes.
export interface VoxelMeshEvidenceRequest {
  readonly grid: number;
  readonly chunks: readonly { readonly x: number; readonly y: number; readonly z: number }[];
}
export interface VoxelMeshStatsEvidence {
  readonly vertices: number;
  readonly indices: number;
  readonly quads: number;
  readonly facesEmitted: number;
  readonly facesCulled: number;
}
export interface VoxelMeshBoundsEvidence {
  readonly min: readonly [number, number, number];
  readonly max: readonly [number, number, number];
}
export interface VoxelMeshChunkEvidence {
  readonly coord: { readonly x: number; readonly y: number; readonly z: number };
  readonly resident: boolean;
  readonly visible: boolean;
  readonly contentHash: string | null;
  readonly meshHash: string | null;
  readonly stats: VoxelMeshStatsEvidence | null;
  readonly bounds: VoxelMeshBoundsEvidence | null;
  readonly materialSlots: readonly number[];
}
export interface VoxelMeshEvidenceSnapshot {
  readonly grid: number;
  readonly fixtureId: string;
  readonly voxelStateHash: string;
  readonly meshingStrategy: string;
  readonly chunks: readonly VoxelMeshChunkEvidence[];
  readonly diagnostics: readonly string[];
}

// ── Fixed capability ports ───────────────────────────────────────────────────
// These are compile-time subsets of one bridge cell, never service names or a
// runtime lookup table. The public RuntimeBridge root composes every port below.

export interface RuntimeInputPort {
  configureInputSession(request: InputSessionConfigureRequest): InputSessionSnapshot;
  applyInputContextCommand(command: InputContextCommand): InputContextChangeReceipt;
  submitRawInput(sample: RawInputSample): InputResolutionReceipt;
  replayResolvedInputAction(record: RecordedInputAction): InputActionReplayReceipt;
  readInputContextState(): InputContextStackState;
}

export interface RuntimeTimeSimulationPort {
  applyTimeControlCommand(command: TimeControlCommand): TimeControlReceipt;
  readTimeControlState(): TimeControlState;
  stepSimulation(input: StepInputEnvelope): StepResult;
}

export interface RuntimeSceneEntityPort {
  readModelMaterialPreview(request: ModelMaterialPreviewRequest): ModelMaterialPreviewSnapshot;
  readSceneObjectSnapshot(): SceneObjectSnapshot;
  applySceneObjectCommand(request: SceneObjectCommandRequest): SceneObjectCommandResult;
  applyEnemyDirectNavMovement(request: EnemyDirectNavMovementRequest): EnemyDirectNavMovementResult;
}

export interface RuntimeVoxelAssetBufferPort {
  submitCommands(batch: CommandBatch): CommandResult;
  pickVoxel(ray: PickRay): PickResult;
  selectVoxel(request: ScreenPointToPickRayRequest): VoxelSelectionSnapshot;
  readVoxelMeshEvidence(request: VoxelMeshEvidenceRequest): VoxelMeshEvidenceSnapshot;
  planVoxelConversion(request: VoxelConversionPlanRequest): VoxelConversionPlan;
  registerVoxelConversionSource(request: VoxelConversionSourceRegistrationRequest): VoxelConversionSourceRegistration;
  registerVoxelConversionMeshAsset(request: VoxelConversionMeshAssetRegistrationRequest): VoxelConversionSourceRegistration;
  importVoxelConversionMeshSource(request: VoxelConversionMeshSourceImportRequest): VoxelConversionMeshSourceImportReceipt;
  readVoxelConversionSourceMetadata(request: VoxelConversionSourceMetadataRequest): VoxelConversionSourceMetadataReadout;
  previewVoxelConversion(request: VoxelConversionPreviewRequest): VoxelConversionPreview;
  applyVoxelConversion(request: VoxelConversionApplyRequest): VoxelConversionReceipt;
  readVoxelModelInfo(request: VoxelModelInfoRequest): VoxelModelInfoReadout;
  readVoxelModelWindow(request: VoxelModelWindowRequest): VoxelModelWindowReadout;
  exportVoxelVolumeAsset(request: VoxelVolumeAssetExportRequest): VoxelVolumeAssetExportReceipt;
  saveVoxelVolumeAsset(request: VoxelVolumeAssetSaveRequest): VoxelVolumeAssetSaveReceipt;
  updateVoxelVolumeAssetPalette(request: VoxelVolumeAssetPaletteUpdateRequest): VoxelVolumeAssetPaletteUpdateReceipt;
  initializeVoxelVolumeAuthoring(request: VoxelVolumeAuthoringInitializeRequest): VoxelVolumeAuthoringInitializeReceipt;
  loadVoxelVolumeAsset(request: VoxelVolumeAssetLoadRequest): VoxelVolumeAssetLoadReceipt;
  unloadVoxelVolumeAsset(request: VoxelVolumeAssetUnloadRequest): VoxelVolumeAssetUnloadReceipt;
  validateVoxelAnnotationLayer(request: VoxelAnnotationLayerValidationRequest): VoxelAnnotationLayerValidationReport;
  loadVoxelAnnotationLayer(request: VoxelAnnotationLayerLoadRequest): VoxelAnnotationLayerLoadReceipt;
  readVoxelAnnotationQuery(request: VoxelAnnotationQueryRequest): VoxelAnnotationQueryReadout;
  applyVoxelAnnotationEdit(request: VoxelAnnotationEditRequest): VoxelAnnotationEditReceipt;
  exportVoxelAnnotationLayer(request: VoxelAnnotationLayerExportRequest): VoxelAnnotationLayerExportReceipt;
  readVoxelEditHistory(request: VoxelEditHistoryReadRequest): VoxelEditHistorySummary;
  previewVoxelEditRevert(request: VoxelEditHistoryRevertRequest): VoxelEditHistoryRevertReceipt;
  applyVoxelEditRevert(request: VoxelEditHistoryRevertRequest): VoxelEditHistoryRevertReceipt;
  undoVoxelEdit(request: VoxelEditHistoryUndoRequest): VoxelEditHistoryUndoReceipt;
  redoVoxelEdit(request: VoxelEditHistoryRedoRequest): VoxelEditHistoryRedoReceipt;
  getBuffer(handle: RuntimeBufferHandle): RuntimeBufferView;
  releaseBuffer(handle: RuntimeBufferHandle): void;
}

export interface RuntimeCameraPort {
  applyCollisionConstrainedCameraInput(input: CollisionConstrainedCameraInputEnvelope): CameraCollisionSnapshot;
  createCamera(request: CameraCreateRequest): CameraSnapshot;
  applyCameraModeCommand(command: CameraModeCommand): CameraModeChangeReceipt;
  applyCameraNavigationInput(input: CameraNavigationInputEnvelope): CameraNavigationReceipt;
  readCameraControllerState(request: CameraControllerReadRequest): CameraControllerState;
  applyFirstPersonCameraInput(input: FirstPersonCameraInputEnvelope): CameraSnapshot;
  readCameraProjection(request: CameraProjectionRequest): CameraProjectionSnapshot;
}

export interface RuntimeGameplayPort {
  applyGeneratedTunnelToRuntimeWorld(
    request: GeneratedTunnelRuntimeApplyRequest,
  ): GeneratedTunnelRuntimeApplyReceipt;
  loadFpsRuntimeSession(request: FpsRuntimeSessionLoadRequest): FpsRuntimeSessionSnapshot;
  readFpsRuntimeSession(): FpsRuntimeSessionSnapshot;
  applyFpsPrimaryFire(request: FpsPrimaryFireRequest): FpsPrimaryFireResult;
  invokeGameExtensionWeaponEffect(
    request: GameExtensionWeaponEffectInvocationRequest,
  ): GameExtensionWeaponEffectInvocationResult;
  validateGameRuleCatalog(catalog: GameRuleCatalog): GameRuleCatalogValidationReceipt;
  submitGameRuleEffectIntent(input: GameRuleEffectIntentRequest): GameRuleResolutionReceipt;
  readGameRuleRuntimeReadout(): GameRuleRuntimeReadout;
  restartFpsRuntimeSession(request: FpsRuntimeSessionRestartRequest): FpsRuntimeSessionSnapshot;
  readFpsEncounterDirector(lifecycle: FpsEncounterLifecycleInput): FpsEncounterDirectorSnapshot;
  applyFpsEncounterTransition(request: FpsEncounterTransitionRequest): FpsEncounterTransitionResult;
}

export interface RuntimeProjectionPort {
  readRenderDiffs(cursor: FrameCursor): RenderFrameDiff;
  readProjectionFrame(cursor: FrameCursor): RuntimeProjectionFrame;
}

export interface RuntimeBundleLifecyclePort {
  initializeEngine(config: EngineConfig): EngineHandle;
  loadProjectBundle(request: ProjectBundleLoadRequest): CompositionStatus;
  saveProjectBundle(): ProjectBundleSaveSummary;
  getProjectBundleCompositionStatus(): CompositionStatus;
  unloadProjectBundle(): void;
}

export interface RuntimeReplayEvidencePort {
  exportVoxelConversionEvidence(
    evidence: readonly VoxelConversionEvidenceRef[],
  ): readonly VoxelConversionEvidenceRef[];
  loadReplayFixture(fixture: ReplayFixture): ReplaySessionHandle;
  runReplayStep(session: ReplaySessionHandle): ReplayStepReport;
}

/** Bounded verbs only — mirrors bridge-manifest.toml. No generic call(method, json). */
export interface RuntimeBridge
  extends RuntimeInputPort,
    RuntimeTimeSimulationPort,
    RuntimeSceneEntityPort,
    RuntimeVoxelAssetBufferPort,
    RuntimeCameraPort,
    RuntimeGameplayPort,
    RuntimeProjectionPort,
    RuntimeBundleLifecyclePort,
    RuntimeReplayEvidencePort {}

export interface RuntimeBridgePorts {
  readonly input: RuntimeInputPort;
  readonly timeSimulation: RuntimeTimeSimulationPort;
  readonly sceneEntities: RuntimeSceneEntityPort;
  readonly voxelAssetsBuffers: RuntimeVoxelAssetBufferPort;
  readonly camera: RuntimeCameraPort;
  readonly gameplay: RuntimeGameplayPort;
  readonly projection: RuntimeProjectionPort;
  readonly bundleLifecycle: RuntimeBundleLifecyclePort;
  readonly replayEvidence: RuntimeReplayEvidencePort;
}

/**
 * Produce fixed typed views over one root. Every property is statically named;
 * callers cannot request arbitrary capabilities or discover mutable state.
 */
export function runtimeBridgePorts(bridge: RuntimeBridge): RuntimeBridgePorts {
  return {
    input: bridge,
    timeSimulation: bridge,
    sceneEntities: bridge,
    voxelAssetsBuffers: bridge,
    camera: bridge,
    gameplay: bridge,
    projection: bridge,
    bundleLifecycle: bridge,
    replayEvidence: bridge,
  };
}

export type RuntimeBridgePortId = keyof RuntimeBridgePorts;

export interface RuntimeBridgePortContract {
  readonly initialization: 'requiresEngine' | 'createsEngine';
  readonly projectBundle: 'retainedAcrossLoadUnload' | 'ownsLoadUnload';
  readonly snapshotHash:
    | 'inputEvidence'
    | 'timeState'
    | 'sceneDocument'
    | 'voxelStateAndResources'
    | 'cameraProjection'
    | 'gameplaySessionAndReplay'
    | 'projectionFrame'
    | 'compositionStatus'
    | 'replayEvidence';
  readonly resourceLifetime: 'session' | 'frame' | 'mixedExplicitAndSession';
}

/** Reviewable lifecycle rules for the fixed port set. */
export const RUNTIME_BRIDGE_PORT_CONTRACTS: Readonly<
  Record<RuntimeBridgePortId, RuntimeBridgePortContract>
> = {
  input: {
    initialization: 'requiresEngine',
    projectBundle: 'retainedAcrossLoadUnload',
    snapshotHash: 'inputEvidence',
    resourceLifetime: 'session',
  },
  timeSimulation: {
    initialization: 'requiresEngine',
    projectBundle: 'retainedAcrossLoadUnload',
    snapshotHash: 'timeState',
    resourceLifetime: 'session',
  },
  sceneEntities: {
    initialization: 'requiresEngine',
    projectBundle: 'retainedAcrossLoadUnload',
    snapshotHash: 'sceneDocument',
    resourceLifetime: 'session',
  },
  voxelAssetsBuffers: {
    initialization: 'requiresEngine',
    projectBundle: 'retainedAcrossLoadUnload',
    snapshotHash: 'voxelStateAndResources',
    resourceLifetime: 'mixedExplicitAndSession',
  },
  camera: {
    initialization: 'requiresEngine',
    projectBundle: 'retainedAcrossLoadUnload',
    snapshotHash: 'cameraProjection',
    resourceLifetime: 'session',
  },
  gameplay: {
    initialization: 'requiresEngine',
    projectBundle: 'retainedAcrossLoadUnload',
    snapshotHash: 'gameplaySessionAndReplay',
    resourceLifetime: 'session',
  },
  projection: {
    initialization: 'requiresEngine',
    projectBundle: 'retainedAcrossLoadUnload',
    snapshotHash: 'projectionFrame',
    resourceLifetime: 'frame',
  },
  bundleLifecycle: {
    initialization: 'createsEngine',
    projectBundle: 'ownsLoadUnload',
    snapshotHash: 'compositionStatus',
    resourceLifetime: 'session',
  },
  replayEvidence: {
    initialization: 'requiresEngine',
    projectBundle: 'retainedAcrossLoadUnload',
    snapshotHash: 'replayEvidence',
    resourceLifetime: 'session',
  },
};
