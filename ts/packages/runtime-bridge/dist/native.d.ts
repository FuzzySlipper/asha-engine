import type { CameraCollisionSnapshot, CameraCreateRequest, CameraProjectionSnapshot, CameraSnapshot, CommandBatch, CommandResult, ModelMaterialPreviewRequest, ModelMaterialPreviewSnapshot, PickResult, RenderFrameDiff, SceneObjectCommandResult, SceneObjectSnapshot, VoxelConversionApplyRequest, VoxelConversionEvidenceRef, VoxelConversionMeshAssetRegistrationRequest, VoxelConversionPlan, VoxelConversionPlanRequest, VoxelConversionPreview, VoxelConversionPreviewRequest, VoxelConversionReceipt, VoxelConversionSourceRegistration, VoxelConversionSourceRegistrationRequest, VoxelSelectionSnapshot, VoxelModelInfoReadout, VoxelModelInfoRequest, VoxelModelWindowReadout, VoxelModelWindowRequest, VoxelAnnotationEditReceipt, VoxelAnnotationEditRequest, VoxelAnnotationLayerExportReceipt, VoxelAnnotationLayerExportRequest, VoxelAnnotationLayerLoadReceipt, VoxelAnnotationLayerLoadRequest, VoxelAnnotationLayerValidationReport, VoxelAnnotationLayerValidationRequest, VoxelAnnotationQueryReadout, VoxelAnnotationQueryRequest, VoxelEditHistoryReadRequest, VoxelEditHistoryRedoReceipt, VoxelEditHistoryRedoRequest, VoxelEditHistoryRevertReceipt, VoxelEditHistoryRevertRequest, VoxelEditHistorySummary, VoxelEditHistoryUndoReceipt, VoxelEditHistoryUndoRequest, VoxelVolumeAssetExportReceipt, VoxelVolumeAssetExportRequest, VoxelVolumeAssetLoadReceipt, VoxelVolumeAssetLoadRequest, VoxelVolumeAssetSaveReceipt, VoxelVolumeAssetSaveRequest, GameRuleCatalog, GameRuleResolutionReceipt } from '@asha/contracts';
import { type NativeAddon } from '@asha/native-bridge';
import { type CompositionStatus, type EnemyDirectNavMovementRequest, type EnemyDirectNavMovementResult, type EngineConfig, type EngineHandle, type FrameCursor, type FpsEncounterDirectorSnapshot, type FpsEncounterLifecycleInput, type FpsEncounterTransitionRequest, type FpsEncounterTransitionResult, type GameExtensionWeaponEffectInvocationRequest, type GameExtensionWeaponEffectInvocationResult, type GameRuleCatalogValidationReceipt, type GameRuleEffectIntentRequest, type GameRuleRuntimeReadout, type FpsPrimaryFireRequest, type FpsPrimaryFireResult, type FpsRuntimeSessionLoadRequest, type FpsRuntimeSessionRestartRequest, type FpsRuntimeSessionSnapshot, type ReplaySessionHandle, type ReplayStepReport, type RuntimeBridge, type RuntimeBufferView, type StepInputEnvelope, type StepResult, type VoxelMeshEvidenceSnapshot, type ProjectBundleLoadRequest, type ProjectBundleSaveSummary } from './bridge.js';
/**
 * Manifest names of operations whose native (`#[napi]`) implementation is actually
 * wired. Everything else on {@link NativeRuntimeBridge} fail-closes with
 * `operation_unimplemented`. Adding a name here is the explicit signal that a
 * native implementation landed; the native conformance test keeps this set and the
 * routed methods in lockstep with the bridge manifest.
 */
export declare const NATIVE_WIRED_OPERATIONS: ReadonlySet<string>;
export declare class NativeRuntimeBridge implements RuntimeBridge {
    #private;
    constructor(addon: NativeAddon);
    initializeEngine(config: EngineConfig): EngineHandle;
    loadProjectBundle(request: ProjectBundleLoadRequest): CompositionStatus;
    submitCommands(batch: CommandBatch): CommandResult;
    stepSimulation(input: StepInputEnvelope): StepResult;
    applyEnemyDirectNavMovement(request: EnemyDirectNavMovementRequest): EnemyDirectNavMovementResult;
    loadFpsRuntimeSession(request: FpsRuntimeSessionLoadRequest): FpsRuntimeSessionSnapshot;
    readFpsRuntimeSession(): FpsRuntimeSessionSnapshot;
    applyFpsPrimaryFire(request: FpsPrimaryFireRequest): FpsPrimaryFireResult;
    invokeGameExtensionWeaponEffect(request: GameExtensionWeaponEffectInvocationRequest): GameExtensionWeaponEffectInvocationResult;
    validateGameRuleCatalog(catalog: GameRuleCatalog): GameRuleCatalogValidationReceipt;
    submitGameRuleEffectIntent(input: GameRuleEffectIntentRequest): GameRuleResolutionReceipt;
    readGameRuleRuntimeReadout(): GameRuleRuntimeReadout;
    restartFpsRuntimeSession(request: FpsRuntimeSessionRestartRequest): FpsRuntimeSessionSnapshot;
    readFpsEncounterDirector(lifecycle: FpsEncounterLifecycleInput): FpsEncounterDirectorSnapshot;
    applyFpsEncounterTransition(request: FpsEncounterTransitionRequest): FpsEncounterTransitionResult;
    readModelMaterialPreview(request: ModelMaterialPreviewRequest): ModelMaterialPreviewSnapshot;
    readSceneObjectSnapshot(): SceneObjectSnapshot;
    applySceneObjectCommand(): SceneObjectCommandResult;
    readRenderDiffs(cursor: FrameCursor): RenderFrameDiff;
    saveProjectBundle(): ProjectBundleSaveSummary;
    getProjectBundleCompositionStatus(): CompositionStatus;
    planVoxelConversion(request: VoxelConversionPlanRequest): VoxelConversionPlan;
    registerVoxelConversionSource(request: VoxelConversionSourceRegistrationRequest): VoxelConversionSourceRegistration;
    registerVoxelConversionMeshAsset(request: VoxelConversionMeshAssetRegistrationRequest): VoxelConversionSourceRegistration;
    previewVoxelConversion(request: VoxelConversionPreviewRequest): VoxelConversionPreview;
    applyVoxelConversion(request: VoxelConversionApplyRequest): VoxelConversionReceipt;
    exportVoxelConversionEvidence(evidence: readonly VoxelConversionEvidenceRef[]): readonly VoxelConversionEvidenceRef[];
    readVoxelModelInfo(request: VoxelModelInfoRequest): VoxelModelInfoReadout;
    readVoxelModelWindow(request: VoxelModelWindowRequest): VoxelModelWindowReadout;
    exportVoxelVolumeAsset(request: VoxelVolumeAssetExportRequest): VoxelVolumeAssetExportReceipt;
    saveVoxelVolumeAsset(request: VoxelVolumeAssetSaveRequest): VoxelVolumeAssetSaveReceipt;
    loadVoxelVolumeAsset(request: VoxelVolumeAssetLoadRequest): VoxelVolumeAssetLoadReceipt;
    validateVoxelAnnotationLayer(request: VoxelAnnotationLayerValidationRequest): VoxelAnnotationLayerValidationReport;
    loadVoxelAnnotationLayer(request: VoxelAnnotationLayerLoadRequest): VoxelAnnotationLayerLoadReceipt;
    readVoxelAnnotationQuery(request: VoxelAnnotationQueryRequest): VoxelAnnotationQueryReadout;
    applyVoxelAnnotationEdit(request: VoxelAnnotationEditRequest): VoxelAnnotationEditReceipt;
    exportVoxelAnnotationLayer(request: VoxelAnnotationLayerExportRequest): VoxelAnnotationLayerExportReceipt;
    pickVoxel(): PickResult;
    applyCollisionConstrainedCameraInput(): CameraCollisionSnapshot;
    selectVoxel(): VoxelSelectionSnapshot;
    readVoxelMeshEvidence(): VoxelMeshEvidenceSnapshot;
    readVoxelEditHistory(_request: VoxelEditHistoryReadRequest): VoxelEditHistorySummary;
    previewVoxelEditRevert(_request: VoxelEditHistoryRevertRequest): VoxelEditHistoryRevertReceipt;
    applyVoxelEditRevert(_request: VoxelEditHistoryRevertRequest): VoxelEditHistoryRevertReceipt;
    undoVoxelEdit(_request: VoxelEditHistoryUndoRequest): VoxelEditHistoryUndoReceipt;
    redoVoxelEdit(_request: VoxelEditHistoryRedoRequest): VoxelEditHistoryRedoReceipt;
    createCamera(request: CameraCreateRequest): CameraSnapshot;
    applyFirstPersonCameraInput(): CameraSnapshot;
    readCameraProjection(): CameraProjectionSnapshot;
    getBuffer(): RuntimeBufferView;
    releaseBuffer(): void;
    unloadProjectBundle(): void;
    loadReplayFixture(): ReplaySessionHandle;
    runReplayStep(): ReplayStepReport;
}
/**
 * Construct the native (napi-rs) bridge. Throws a classified
 * {@link RuntimeBridgeError} of kind `native_unavailable` if the addon is not built
 * — callers can fall back to the mock for tests/dev.
 */
export declare function createNativeRuntimeBridge(modulePath?: string): RuntimeBridge;
/** Operation count for quick sanity in consumers/tests. */
export declare const STABLE_OPERATION_COUNT: number;
//# sourceMappingURL=native.d.ts.map