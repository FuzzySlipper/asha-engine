import type { CameraCollisionSnapshot, CameraControllerReadRequest, CameraControllerState, CameraCreateRequest, CameraModeChangeReceipt, CameraModeCommand, CameraNavigationInputEnvelope, CameraNavigationReceipt, CameraProjectionRequest, CameraProjectionSnapshot, CameraSnapshot, CollisionConstrainedCameraInputEnvelope, CommandBatch, CommandResult, FirstPersonCameraInputEnvelope, GameRuleCatalog, GameRuleResolutionReceipt, InputActionReplayReceipt, InputContextChangeReceipt, InputContextCommand, InputContextStackState, InputResolutionReceipt, InputSessionConfigureRequest, InputSessionSnapshot, GeneratedTunnelRuntimeApplyReceipt, GeneratedTunnelRuntimeApplyRequest, ModelMaterialPreviewRequest, ModelMaterialPreviewSnapshot, PickRay, PickResult, RawInputSample, RecordedInputAction, RenderFrameDiff, RuntimeProjectionFrame, TimeControlCommand, TimeControlReceipt, TimeControlState, SceneObjectCommandRequest, SceneObjectCommandResult, SceneObjectSnapshot, ScreenPointToPickRayRequest, VoxelSelectionSnapshot, VoxelConversionApplyRequest, VoxelConversionEvidenceRef, VoxelConversionMeshAssetRegistrationRequest, VoxelConversionMeshSourceImportReceipt, VoxelConversionMeshSourceImportRequest, VoxelConversionPlan, VoxelConversionPlanRequest, VoxelConversionPreview, VoxelConversionPreviewRequest, VoxelConversionReceipt, VoxelConversionSourceMetadataReadout, VoxelConversionSourceMetadataRequest, VoxelConversionSourceRegistration, VoxelConversionSourceRegistrationRequest, VoxelModelInfoReadout, VoxelModelInfoRequest, VoxelModelWindowReadout, VoxelModelWindowRequest, VoxelAnnotationEditReceipt, VoxelAnnotationEditRequest, VoxelAnnotationLayerExportReceipt, VoxelAnnotationLayerExportRequest, VoxelAnnotationLayerLoadReceipt, VoxelAnnotationLayerLoadRequest, VoxelAnnotationLayerValidationReport, VoxelAnnotationLayerValidationRequest, VoxelAnnotationQueryReadout, VoxelAnnotationQueryRequest, VoxelEditHistoryReadRequest, VoxelEditHistoryRedoReceipt, VoxelEditHistoryRedoRequest, VoxelEditHistoryRevertReceipt, VoxelEditHistoryRevertRequest, VoxelEditHistorySummary, VoxelEditHistoryUndoReceipt, VoxelEditHistoryUndoRequest, VoxelVolumeAssetExportReceipt, VoxelVolumeAssetExportRequest, VoxelVolumeAssetLoadReceipt, VoxelVolumeAssetLoadRequest, VoxelVolumeAssetUnloadReceipt, VoxelVolumeAssetUnloadRequest, VoxelVolumeAssetPaletteUpdateReceipt, VoxelVolumeAssetPaletteUpdateRequest, VoxelVolumeAssetSaveReceipt, VoxelVolumeAssetSaveRequest, VoxelVolumeAuthoringInitializeReceipt, VoxelVolumeAuthoringInitializeRequest } from '@asha/contracts';
export type { GeneratedTunnelRuntimeApplyReceipt, GeneratedTunnelRuntimeApplyRequest, } from '@asha/contracts';
import type { CompositionStatus, EnemyDirectNavMovementRequest, EnemyDirectNavMovementResult, EngineHandle, FpsEncounterDirectorSnapshot, FpsEncounterLifecycleInput, FpsEncounterTransitionRequest, FpsEncounterTransitionResult, FpsPrimaryFireRequest, FpsPrimaryFireResult, FpsRuntimeSessionLoadRequest, FpsRuntimeSessionRestartRequest, FpsRuntimeSessionSnapshot, FrameCursor, GameExtensionWeaponEffectInvocationRequest, GameExtensionWeaponEffectInvocationResult, GameRuleCatalogValidationReceipt, GameRuleEffectIntentRequest, GameRuleRuntimeReadout, ProjectBundleLoadRequest, StepResult } from '@asha/runtime-session';
export type { BridgeVec3, CompositionStatus, EnemyDirectNavAuthoritySource, EnemyDirectNavAuthorityTransport, EnemyDirectNavMovementRequest, EnemyDirectNavMovementResult, EngineHandle, FpsBoundsCapability, FpsEncounterDirectorSnapshot, FpsEncounterLastTransition, FpsEncounterLifecycleInput, FpsEncounterStateReadout, FpsEncounterStatus, FpsEncounterTransitionAction, FpsEncounterTransitionRequest, FpsEncounterTransitionResult, FpsEntityHealthReadout, FpsHealth, FpsLifecycleStatus, FpsPolicyBinding, FpsPolicyBindingReadout, FpsPrimaryFireRequest, FpsPrimaryFireResult, FpsReadSetEvidence, FpsReplayEvidence, FpsRuntimeAuthorityTransport, FpsRuntimeRole, FpsRuntimeSessionLoadRequest, FpsRuntimeSessionRestartRequest, FpsRuntimeSessionSnapshot, FpsStoredEntityDefinition, FpsTransformCapability, FpsWeaponMount, FrameCursor, GameExtensionWeaponEffectInvocationRequest, GameExtensionWeaponEffectInvocationResult, GameRuleCatalogValidationReceipt, GameRuleEffectIntentRequest, GameRuleRuntimeReadout, ProjectBundleLoadRequest, StepResult, } from '@asha/runtime-session';
export type RuntimeBufferHandle = number & {
    readonly __brand: 'RuntimeBufferHandle';
};
export type ReplaySessionHandle = number & {
    readonly __brand: 'ReplaySessionHandle';
};
export declare const frameCursor: (frame: number) => FrameCursor;
export type RuntimeBridgeErrorKind = 'not_initialized' | 'invalid_input' | 'unknown_handle' | 'buffer_expired' | 'native_unavailable' | 'voxel_conversion_unavailable' | 'unsupported_source_asset' | 'source_hash_mismatch' | 'invalid_material_map' | 'output_limit_exceeded' | 'stale_authority_snapshot' | 'conversion_replay_mismatch' | 'operation_unimplemented' | 'internal';
/** Typed, classified error for every facade operation. No JSON error blobs. */
export declare class RuntimeBridgeError extends Error {
    readonly kind: RuntimeBridgeErrorKind;
    constructor(kind: RuntimeBridgeErrorKind, message: string);
}
export declare function nonNegativeSafeInteger(value: number, field: string): number;
export declare function u32(value: number, field: string): number;
export interface EngineConfig {
    readonly seed: number;
}
export interface StepInputEnvelope {
    readonly tick: number;
}
/** Borrowed, read-only view over bridge-owned bytes (large payloads, e.g. mesh). */
export interface RuntimeBufferView {
    readonly handle: RuntimeBufferHandle;
    readonly bytes: Uint8Array;
}
export interface ReplayFixture {
    readonly name: string;
    readonly steps: number;
}
export interface ReplayStepReport {
    readonly step: number;
    readonly hash: string;
    readonly diverged: boolean;
}
export interface ProjectBundleSaveSummary {
    readonly artifactsWritten: number;
    readonly compactedEdits: number;
    readonly retainedEdits: number;
}
export interface VoxelMeshEvidenceRequest {
    readonly grid: number;
    readonly chunks: readonly {
        readonly x: number;
        readonly y: number;
        readonly z: number;
    }[];
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
    readonly coord: {
        readonly x: number;
        readonly y: number;
        readonly z: number;
    };
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
    applyGeneratedTunnelToRuntimeWorld(request: GeneratedTunnelRuntimeApplyRequest): GeneratedTunnelRuntimeApplyReceipt;
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
    exportVoxelConversionEvidence(evidence: readonly VoxelConversionEvidenceRef[]): readonly VoxelConversionEvidenceRef[];
    loadReplayFixture(fixture: ReplayFixture): ReplaySessionHandle;
    runReplayStep(session: ReplaySessionHandle): ReplayStepReport;
}
/** Bounded verbs only — mirrors bridge-manifest.toml. No generic call(method, json). */
export interface RuntimeBridge extends RuntimeInputPort, RuntimeTimeSimulationPort, RuntimeSceneEntityPort, RuntimeVoxelAssetBufferPort, RuntimeCameraPort, RuntimeGameplayPort, RuntimeProjectionPort, RuntimeBundleLifecyclePort, RuntimeReplayEvidencePort {
}
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
export declare function runtimeBridgePorts(bridge: RuntimeBridge): RuntimeBridgePorts;
export type RuntimeBridgePortId = keyof RuntimeBridgePorts;
export interface RuntimeBridgePortContract {
    readonly initialization: 'requiresEngine' | 'createsEngine';
    readonly projectBundle: 'retainedAcrossLoadUnload' | 'ownsLoadUnload';
    readonly snapshotHash: 'inputEvidence' | 'timeState' | 'sceneDocument' | 'voxelStateAndResources' | 'cameraProjection' | 'gameplaySessionAndReplay' | 'projectionFrame' | 'compositionStatus' | 'replayEvidence';
    readonly resourceLifetime: 'session' | 'frame' | 'mixedExplicitAndSession';
}
/** Reviewable lifecycle rules for the fixed port set. */
export declare const RUNTIME_BRIDGE_PORT_CONTRACTS: Readonly<Record<RuntimeBridgePortId, RuntimeBridgePortContract>>;
//# sourceMappingURL=bridge.d.ts.map