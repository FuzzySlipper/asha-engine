import type { CameraCollisionSnapshot, CameraCreateRequest, CameraProjectionRequest, CameraProjectionSnapshot, CameraSnapshot, CollisionConstrainedCameraInputEnvelope, CommandBatch, CommandResult, FirstPersonCameraInputEnvelope, ModelMaterialPreviewRequest, ModelMaterialPreviewSnapshot, PickRay, PickResult, RenderFrameDiff, SceneObjectCommandRequest, SceneObjectCommandResult, SceneObjectSnapshot, ScreenPointToPickRayRequest, VoxelSelectionSnapshot } from '@asha/contracts';
import { type CompositionStatus, type EnemyDirectNavMovementRequest, type EnemyDirectNavMovementResult, type EngineConfig, type EngineHandle, type FrameCursor, type FpsPrimaryFireRequest, type FpsPrimaryFireResult, type FpsRuntimeSessionLoadRequest, type FpsRuntimeSessionRestartRequest, type FpsRuntimeSessionSnapshot, type ReplayFixture, type ReplaySessionHandle, type ReplayStepReport, type RuntimeBridge, type RuntimeBufferHandle, type RuntimeBufferView, type StepInputEnvelope, type StepResult, type VoxelMeshEvidenceRequest, type VoxelMeshEvidenceSnapshot, type WorldLoadRequest, type WorldSaveSummary } from './bridge.js';
export declare class MockRuntimeBridge implements RuntimeBridge {
    #private;
    initializeEngine(config: EngineConfig): EngineHandle;
    stepSimulation(input: StepInputEnvelope): StepResult;
    applyEnemyDirectNavMovement(request: EnemyDirectNavMovementRequest): EnemyDirectNavMovementResult;
    loadFpsRuntimeSession(request: FpsRuntimeSessionLoadRequest): FpsRuntimeSessionSnapshot;
    readFpsRuntimeSession(): FpsRuntimeSessionSnapshot;
    applyFpsPrimaryFire(request: FpsPrimaryFireRequest): FpsPrimaryFireResult;
    restartFpsRuntimeSession(request: FpsRuntimeSessionRestartRequest): FpsRuntimeSessionSnapshot;
    submitCommands(batch: CommandBatch): CommandResult;
    pickVoxel(ray: PickRay): PickResult;
    applyCollisionConstrainedCameraInput(input: CollisionConstrainedCameraInputEnvelope): CameraCollisionSnapshot;
    selectVoxel(request: ScreenPointToPickRayRequest): VoxelSelectionSnapshot;
    readVoxelMeshEvidence(request: VoxelMeshEvidenceRequest): VoxelMeshEvidenceSnapshot;
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