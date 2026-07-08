import type { CameraCollisionSnapshot, CameraCreateRequest, CameraProjectionRequest, CameraProjectionSnapshot, CameraSnapshot, CollisionConstrainedCameraInputEnvelope, CommandBatch, CommandResult, FirstPersonCameraInputEnvelope, ModelMaterialPreviewRequest, ModelMaterialPreviewSnapshot, PickRay, PickResult, RenderFrameDiff, SceneObjectCommandRequest, SceneObjectCommandResult, SceneObjectSnapshot, ScreenPointToPickRayRequest, VoxelConversionApplyRequest, VoxelConversionEvidenceRef, VoxelConversionPlan, VoxelConversionPlanRequest, VoxelConversionPreview, VoxelConversionPreviewRequest, VoxelConversionReceipt, VoxelConversionSourceRegistration, VoxelConversionSourceRegistrationRequest, VoxelSelectionSnapshot, VoxelModelInfoReadout, VoxelModelInfoRequest, VoxelVolumeAssetExportReceipt, VoxelVolumeAssetExportRequest, GameRuleCatalog, GameRuleResolutionReceipt } from '@asha/contracts';
import { type CompositionStatus, type EnemyDirectNavMovementRequest, type EnemyDirectNavMovementResult, type EngineConfig, type EngineHandle, type FrameCursor, type FpsEncounterDirectorSnapshot, type FpsEncounterLifecycleInput, type FpsEncounterTransitionRequest, type FpsEncounterTransitionResult, type FpsPrimaryFireRequest, type FpsPrimaryFireResult, type GameExtensionWeaponEffectInvocationRequest, type GameExtensionWeaponEffectInvocationResult, type GameRuleCatalogValidationReceipt, type GameRuleEffectIntentRequest, type GameRuleRuntimeReadout, type FpsRuntimeSessionLoadRequest, type FpsRuntimeSessionRestartRequest, type FpsRuntimeSessionSnapshot, type ReplayFixture, type ReplaySessionHandle, type ReplayStepReport, type RuntimeBridge, type RuntimeBufferHandle, type RuntimeBufferView, type StepInputEnvelope, type StepResult, type VoxelMeshEvidenceRequest, type VoxelMeshEvidenceSnapshot, type WorldLoadRequest, type WorldSaveSummary } from './bridge.js';
export declare class MockRuntimeBridge implements RuntimeBridge {
    #private;
    initializeEngine(config: EngineConfig): EngineHandle;
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
    submitCommands(batch: CommandBatch): CommandResult;
    pickVoxel(ray: PickRay): PickResult;
    applyCollisionConstrainedCameraInput(input: CollisionConstrainedCameraInputEnvelope): CameraCollisionSnapshot;
    selectVoxel(request: ScreenPointToPickRayRequest): VoxelSelectionSnapshot;
    readVoxelMeshEvidence(request: VoxelMeshEvidenceRequest): VoxelMeshEvidenceSnapshot;
    planVoxelConversion(_request: VoxelConversionPlanRequest): VoxelConversionPlan;
    registerVoxelConversionSource(_request: VoxelConversionSourceRegistrationRequest): VoxelConversionSourceRegistration;
    previewVoxelConversion(_request: VoxelConversionPreviewRequest): VoxelConversionPreview;
    applyVoxelConversion(_request: VoxelConversionApplyRequest): VoxelConversionReceipt;
    exportVoxelConversionEvidence(_evidence: readonly VoxelConversionEvidenceRef[]): readonly VoxelConversionEvidenceRef[];
    readVoxelModelInfo(_request: VoxelModelInfoRequest): VoxelModelInfoReadout;
    exportVoxelVolumeAsset(_request: VoxelVolumeAssetExportRequest): VoxelVolumeAssetExportReceipt;
    readModelMaterialPreview(request: ModelMaterialPreviewRequest): ModelMaterialPreviewSnapshot;
    readSceneObjectSnapshot(): SceneObjectSnapshot;
    applySceneObjectCommand(request: SceneObjectCommandRequest): SceneObjectCommandResult;
    readRenderDiffs(cursor: FrameCursor): RenderFrameDiff;
    createCamera(request: CameraCreateRequest): CameraSnapshot;
    applyFirstPersonCameraInput(envelope: FirstPersonCameraInputEnvelope): CameraSnapshot;
    readCameraProjection(request: CameraProjectionRequest): CameraProjectionSnapshot;
    getBuffer(handle: RuntimeBufferHandle): RuntimeBufferView;
    releaseBuffer(handle: RuntimeBufferHandle): void;
    loadWorldBundle(request: WorldLoadRequest): CompositionStatus;
    saveCurrentWorld(): WorldSaveSummary;
    getCompositionStatus(): CompositionStatus;
    unloadWorld(): void;
    loadReplayFixture(fixture: ReplayFixture): ReplaySessionHandle;
    runReplayStep(session: ReplaySessionHandle): ReplayStepReport;
}
/** Construct the default mock bridge. */
export declare function createMockRuntimeBridge(): RuntimeBridge;
//# sourceMappingURL=mock.d.ts.map