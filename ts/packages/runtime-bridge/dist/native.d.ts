import type { CameraCollisionSnapshot, CameraProjectionSnapshot, CameraSnapshot, CommandBatch, CommandResult, ModelMaterialPreviewRequest, ModelMaterialPreviewSnapshot, PickResult, RenderFrameDiff, SceneObjectCommandResult, SceneObjectSnapshot, VoxelSelectionSnapshot } from '@asha/contracts';
import { type NativeAddon } from '@asha/native-bridge';
import { type CompositionStatus, type EnemyDirectNavMovementRequest, type EnemyDirectNavMovementResult, type EngineConfig, type EngineHandle, type FrameCursor, type FpsEncounterDirectorSnapshot, type FpsEncounterLifecycleInput, type FpsEncounterTransitionRequest, type FpsEncounterTransitionResult, type FpsPrimaryFireRequest, type FpsPrimaryFireResult, type FpsRuntimeSessionLoadRequest, type FpsRuntimeSessionRestartRequest, type FpsRuntimeSessionSnapshot, type ReplaySessionHandle, type ReplayStepReport, type RuntimeBridge, type RuntimeBufferView, type StepInputEnvelope, type StepResult, type VoxelMeshEvidenceSnapshot, type WorldLoadRequest, type WorldSaveSummary } from './bridge.js';
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
    loadWorldBundle(request: WorldLoadRequest): CompositionStatus;
    submitCommands(batch: CommandBatch): CommandResult;
    stepSimulation(input: StepInputEnvelope): StepResult;
    applyEnemyDirectNavMovement(request: EnemyDirectNavMovementRequest): EnemyDirectNavMovementResult;
    loadFpsRuntimeSession(request: FpsRuntimeSessionLoadRequest): FpsRuntimeSessionSnapshot;
    readFpsRuntimeSession(): FpsRuntimeSessionSnapshot;
    applyFpsPrimaryFire(request: FpsPrimaryFireRequest): FpsPrimaryFireResult;
    restartFpsRuntimeSession(request: FpsRuntimeSessionRestartRequest): FpsRuntimeSessionSnapshot;
    readFpsEncounterDirector(lifecycle: FpsEncounterLifecycleInput): FpsEncounterDirectorSnapshot;
    applyFpsEncounterTransition(request: FpsEncounterTransitionRequest): FpsEncounterTransitionResult;
    readModelMaterialPreview(request: ModelMaterialPreviewRequest): ModelMaterialPreviewSnapshot;
    readSceneObjectSnapshot(): SceneObjectSnapshot;
    applySceneObjectCommand(): SceneObjectCommandResult;
    readRenderDiffs(cursor: FrameCursor): RenderFrameDiff;
    saveCurrentWorld(): WorldSaveSummary;
    getCompositionStatus(): CompositionStatus;
    pickVoxel(): PickResult;
    applyCollisionConstrainedCameraInput(): CameraCollisionSnapshot;
    selectVoxel(): VoxelSelectionSnapshot;
    readVoxelMeshEvidence(): VoxelMeshEvidenceSnapshot;
    createCamera(): CameraSnapshot;
    applyFirstPersonCameraInput(): CameraSnapshot;
    readCameraProjection(): CameraProjectionSnapshot;
    getBuffer(): RuntimeBufferView;
    releaseBuffer(): void;
    unloadWorld(): void;
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