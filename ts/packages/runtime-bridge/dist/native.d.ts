import type { CameraCollisionSnapshot, CameraControllerReadRequest, CameraControllerState, CameraCreateRequest, CameraModeChangeReceipt, CameraModeCommand, CameraNavigationInputEnvelope, CameraNavigationReceipt, CameraProjectionRequest, CameraProjectionSnapshot, CameraSnapshot, CommandBatch, CommandResult, CollisionConstrainedCameraInputEnvelope, FirstPersonCameraInputEnvelope, ModelMaterialPreviewRequest, ModelMaterialPreviewSnapshot, PickResult, PickRay, RenderFrameDiff, RuntimeProjectionFrame, TimeControlCommand, TimeControlReceipt, TimeControlState, SceneObjectCommandResult, SceneObjectCommandRequest, SceneObjectSnapshot, VoxelConversionApplyRequest, VoxelConversionEvidenceRef, VoxelConversionMeshAssetRegistrationRequest, VoxelConversionMeshSourceImportReceipt, VoxelConversionMeshSourceImportRequest, VoxelConversionPlan, VoxelConversionPlanRequest, VoxelConversionPreview, VoxelConversionPreviewRequest, VoxelConversionReceipt, VoxelConversionSourceMetadataReadout, VoxelConversionSourceMetadataRequest, VoxelConversionSourceRegistration, VoxelConversionSourceRegistrationRequest, VoxelSelectionSnapshot, VoxelModelInfoReadout, VoxelModelInfoRequest, VoxelModelWindowReadout, VoxelModelWindowRequest, VoxelAnnotationEditReceipt, VoxelAnnotationEditRequest, VoxelAnnotationLayerExportReceipt, VoxelAnnotationLayerExportRequest, VoxelAnnotationLayerLoadReceipt, VoxelAnnotationLayerLoadRequest, VoxelAnnotationLayerValidationReport, VoxelAnnotationLayerValidationRequest, VoxelAnnotationQueryReadout, VoxelAnnotationQueryRequest, VoxelEditHistoryReadRequest, VoxelEditHistoryRedoReceipt, VoxelEditHistoryRedoRequest, VoxelEditHistoryRevertReceipt, VoxelEditHistoryRevertRequest, VoxelEditHistorySummary, VoxelEditHistoryUndoReceipt, VoxelEditHistoryUndoRequest, VoxelVolumeAssetExportReceipt, VoxelVolumeAssetExportRequest, VoxelVolumeAssetLoadReceipt, VoxelVolumeAssetLoadRequest, VoxelVolumeAssetUnloadReceipt, VoxelVolumeAssetUnloadRequest, VoxelVolumeAssetPaletteUpdateReceipt, VoxelVolumeAssetPaletteUpdateRequest, VoxelVolumeAssetSaveReceipt, VoxelVolumeAssetSaveRequest, VoxelVolumeAuthoringInitializeReceipt, VoxelVolumeAuthoringInitializeRequest, GameRuleCatalog, GameRuleResolutionReceipt, InputActionReplayReceipt, InputContextChangeReceipt, InputContextCommand, InputContextStackState, InputResolutionReceipt, InputSessionConfigureRequest, InputSessionSnapshot, RawInputSample, RecordedInputAction, ScreenPointToPickRayRequest } from '@asha/contracts';
import { type NativeAddon } from '@asha/native-bridge';
import { type CompositionStatus, type EnemyDirectNavMovementRequest, type EnemyDirectNavMovementResult, type EngineConfig, type EngineHandle, type FrameCursor, type FpsEncounterDirectorSnapshot, type FpsEncounterLifecycleInput, type FpsEncounterTransitionRequest, type FpsEncounterTransitionResult, type GameExtensionWeaponEffectInvocationRequest, type GameExtensionWeaponEffectInvocationResult, type GameRuleCatalogValidationReceipt, type GameRuleEffectIntentRequest, type GameRuleRuntimeReadout, type GeneratedTunnelRuntimeApplyReceipt, type GeneratedTunnelRuntimeApplyRequest, type FpsPrimaryFireRequest, type FpsPrimaryFireResult, type FpsRuntimeSessionLoadRequest, type FpsRuntimeSessionRestartRequest, type FpsRuntimeSessionSnapshot, type ReplaySessionHandle, type ReplayStepReport, type RuntimeBridge, type RuntimeBufferView, type RuntimeBufferHandle, type StepInputEnvelope, type StepResult, type VoxelMeshEvidenceSnapshot, type VoxelMeshEvidenceRequest, type ProjectBundleLoadRequest, type ProjectBundleSaveSummary } from './bridge.js';
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
    configureInputSession(request: InputSessionConfigureRequest): InputSessionSnapshot;
    applyInputContextCommand(command: InputContextCommand): InputContextChangeReceipt;
    submitRawInput(sample: RawInputSample): InputResolutionReceipt;
    replayResolvedInputAction(record: RecordedInputAction): InputActionReplayReceipt;
    readInputContextState(): InputContextStackState;
    applyTimeControlCommand(command: TimeControlCommand): TimeControlReceipt;
    readTimeControlState(): TimeControlState;
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
    applySceneObjectCommand(request: SceneObjectCommandRequest): SceneObjectCommandResult;
    readRenderDiffs(cursor: FrameCursor): RenderFrameDiff;
    readProjectionFrame(cursor: FrameCursor): RuntimeProjectionFrame;
    saveProjectBundle(): ProjectBundleSaveSummary;
    getProjectBundleCompositionStatus(): CompositionStatus;
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
    pickVoxel(ray: PickRay): PickResult;
    applyCollisionConstrainedCameraInput(envelope: CollisionConstrainedCameraInputEnvelope): CameraCollisionSnapshot;
    applyGeneratedTunnelToRuntimeWorld(request: GeneratedTunnelRuntimeApplyRequest): GeneratedTunnelRuntimeApplyReceipt;
    selectVoxel(request: ScreenPointToPickRayRequest): VoxelSelectionSnapshot;
    readVoxelMeshEvidence(request: VoxelMeshEvidenceRequest): VoxelMeshEvidenceSnapshot;
    readVoxelEditHistory(request: VoxelEditHistoryReadRequest): VoxelEditHistorySummary;
    previewVoxelEditRevert(request: VoxelEditHistoryRevertRequest): VoxelEditHistoryRevertReceipt;
    applyVoxelEditRevert(request: VoxelEditHistoryRevertRequest): VoxelEditHistoryRevertReceipt;
    undoVoxelEdit(request: VoxelEditHistoryUndoRequest): VoxelEditHistoryUndoReceipt;
    redoVoxelEdit(request: VoxelEditHistoryRedoRequest): VoxelEditHistoryRedoReceipt;
    createCamera(request: CameraCreateRequest): CameraSnapshot;
    applyCameraModeCommand(command: CameraModeCommand): CameraModeChangeReceipt;
    applyCameraNavigationInput(input: CameraNavigationInputEnvelope): CameraNavigationReceipt;
    readCameraControllerState(request: CameraControllerReadRequest): CameraControllerState;
    applyFirstPersonCameraInput(input: FirstPersonCameraInputEnvelope): CameraSnapshot;
    readCameraProjection(request: CameraProjectionRequest): CameraProjectionSnapshot;
    getBuffer(bufferHandle: RuntimeBufferHandle): RuntimeBufferView;
    releaseBuffer(bufferHandle: RuntimeBufferHandle): void;
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