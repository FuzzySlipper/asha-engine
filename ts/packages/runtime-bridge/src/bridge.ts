import type {
  CameraCollisionSnapshot,
  CameraCreateRequest,
  CameraProjectionRequest,
  CameraProjectionSnapshot,
  CameraSnapshot,
  CollisionConstrainedCameraInputEnvelope,
  CommandBatch,
  CommandResult,
  FirstPersonCameraInputEnvelope,
  GameRuleCatalog,
  GameRuleResolutionReceipt,
  GeneratedTunnelRuntimeApplyReceipt,
  GeneratedTunnelRuntimeApplyRequest,
  ModelMaterialPreviewRequest,
  ModelMaterialPreviewSnapshot,
  PickRay,
  PickResult,
  RenderFrameDiff,
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

// ── The facade surface ────────────────────────────────────────────────────────
// Bounded verbs only — mirrors bridge-manifest.toml. No generic call(method, json).

export interface RuntimeBridge {
  initializeEngine(config: EngineConfig): EngineHandle;
  stepSimulation(input: StepInputEnvelope): StepResult;
  submitCommands(batch: CommandBatch): CommandResult;
  pickVoxel(ray: PickRay): PickResult;
  applyCollisionConstrainedCameraInput(input: CollisionConstrainedCameraInputEnvelope): CameraCollisionSnapshot;
  applyGeneratedTunnelToRuntimeWorld(
    request: GeneratedTunnelRuntimeApplyRequest,
  ): GeneratedTunnelRuntimeApplyReceipt;
  selectVoxel(request: ScreenPointToPickRayRequest): VoxelSelectionSnapshot;
  readVoxelMeshEvidence(request: VoxelMeshEvidenceRequest): VoxelMeshEvidenceSnapshot;
  planVoxelConversion(request: VoxelConversionPlanRequest): VoxelConversionPlan;
  registerVoxelConversionSource(request: VoxelConversionSourceRegistrationRequest): VoxelConversionSourceRegistration;
  registerVoxelConversionMeshAsset(request: VoxelConversionMeshAssetRegistrationRequest): VoxelConversionSourceRegistration;
  importVoxelConversionMeshSource(request: VoxelConversionMeshSourceImportRequest): VoxelConversionMeshSourceImportReceipt;
  readVoxelConversionSourceMetadata(request: VoxelConversionSourceMetadataRequest): VoxelConversionSourceMetadataReadout;
  previewVoxelConversion(request: VoxelConversionPreviewRequest): VoxelConversionPreview;
  applyVoxelConversion(request: VoxelConversionApplyRequest): VoxelConversionReceipt;
  exportVoxelConversionEvidence(evidence: readonly VoxelConversionEvidenceRef[]): readonly VoxelConversionEvidenceRef[];
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
  readModelMaterialPreview(request: ModelMaterialPreviewRequest): ModelMaterialPreviewSnapshot;
  readSceneObjectSnapshot(): SceneObjectSnapshot;
  applySceneObjectCommand(request: SceneObjectCommandRequest): SceneObjectCommandResult;
  readRenderDiffs(cursor: FrameCursor): RenderFrameDiff;
  createCamera(request: CameraCreateRequest): CameraSnapshot;
  applyFirstPersonCameraInput(input: FirstPersonCameraInputEnvelope): CameraSnapshot;
  applyEnemyDirectNavMovement(request: EnemyDirectNavMovementRequest): EnemyDirectNavMovementResult;
  readCameraProjection(request: CameraProjectionRequest): CameraProjectionSnapshot;
  getBuffer(handle: RuntimeBufferHandle): RuntimeBufferView;
  releaseBuffer(handle: RuntimeBufferHandle): void;
  // ProjectBundle load/save composition (operational; not a replay-verification replacement).
  loadProjectBundle(request: ProjectBundleLoadRequest): CompositionStatus;
  saveProjectBundle(): ProjectBundleSaveSummary;
  getProjectBundleCompositionStatus(): CompositionStatus;
  unloadProjectBundle(): void;
  // Quarantined: replay/golden harness, not the production renderer path.
  loadReplayFixture(fixture: ReplayFixture): ReplaySessionHandle;
  runReplayStep(session: ReplaySessionHandle): ReplayStepReport;
}
