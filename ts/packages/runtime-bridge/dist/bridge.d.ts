import type { CameraCollisionSnapshot, CameraCreateRequest, CameraProjectionRequest, CameraProjectionSnapshot, CameraSnapshot, CollisionConstrainedCameraInputEnvelope, CommandBatch, CommandResult, FirstPersonCameraInputEnvelope, GameExtensionHookReceipt, GameExtensionReplayEvidence, GameRuleCatalog, GameRuleDiagnostic, GameRuleEvidenceRef, GameRuleModifierState, GameRuleResolutionReceipt, GameRuleResolutionRequest, GameRuleTraceEntry, GameRuleModuleManifest, ModelMaterialPreviewRequest, ModelMaterialPreviewSnapshot, PickRay, PickResult, RenderFrameDiff, SceneObjectCommandRequest, SceneObjectCommandResult, SceneObjectSnapshot, ScreenPointToPickRayRequest, VoxelSelectionSnapshot, VoxelConversionApplyRequest, VoxelConversionEvidenceRef, VoxelConversionMeshAssetRegistrationRequest, VoxelConversionPlan, VoxelConversionPlanRequest, VoxelConversionPreview, VoxelConversionPreviewRequest, VoxelConversionReceipt, VoxelConversionSourceRegistration, VoxelConversionSourceRegistrationRequest, VoxelModelInfoReadout, VoxelModelInfoRequest, VoxelVolumeAssetExportReceipt, VoxelVolumeAssetExportRequest, VoxelVolumeAssetLoadReceipt, VoxelVolumeAssetLoadRequest, WeaponEffectHookRequest } from '@asha/contracts';
export type EngineHandle = number & {
    readonly __brand: 'EngineHandle';
};
export type RuntimeBufferHandle = number & {
    readonly __brand: 'RuntimeBufferHandle';
};
export type FrameCursor = number & {
    readonly __brand: 'FrameCursor';
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
export interface StepResult {
    readonly tick: number;
    readonly diffCount: number;
}
export type BridgeVec3 = readonly [number, number, number];
export type EnemyDirectNavAuthoritySource = 'seeded_from_request' | 'rust_entity_store';
export type EnemyDirectNavAuthorityTransport = 'native_rust' | 'reference_bridge';
export interface EnemyDirectNavMovementRequest {
    readonly entity: number;
    readonly seedPosition: BridgeVec3;
    readonly target: BridgeVec3;
    readonly maxStepUnits: number;
}
export interface EnemyDirectNavMovementResult {
    readonly entity: number;
    readonly authoritySource: EnemyDirectNavAuthoritySource;
    readonly authorityTransport: EnemyDirectNavAuthorityTransport;
    readonly from: BridgeVec3;
    readonly target: BridgeVec3;
    readonly nextWaypoint: BridgeVec3;
    readonly distanceUnits: number;
    readonly reached: boolean;
    readonly pathHash: string;
    readonly transformHash: string;
    readonly projectionChanged: boolean;
}
export type FpsRuntimeRole = 'player' | 'enemy' | 'neutral';
export type FpsRuntimeAuthorityTransport = 'native_rust' | 'reference_bridge';
export interface FpsTransformCapability {
    readonly translation: BridgeVec3;
    readonly rotation: readonly [number, number, number, number];
    readonly scale: BridgeVec3;
}
export interface FpsBoundsCapability {
    readonly min: BridgeVec3;
    readonly max: BridgeVec3;
}
export interface FpsHealth {
    readonly current: number;
    readonly max: number;
}
export interface FpsWeaponMount {
    readonly weaponId: string;
    readonly damage: number;
    readonly rangeUnits: number;
    readonly ammo: number;
    readonly cooldownTicksAfterFire: number;
}
export interface FpsPolicyBinding {
    readonly bindingId: string;
    readonly policyId: string;
    readonly viewKind: string;
    readonly viewVersion: string;
    readonly allowedIntents: readonly string[];
    readonly runtimeMoment: string;
}
export interface FpsStoredEntityDefinition {
    readonly entity: number;
    readonly stableId: string;
    readonly displayName: string;
    readonly sourcePath: string;
    readonly tags: readonly string[];
    readonly role: FpsRuntimeRole;
    readonly transform: FpsTransformCapability | null;
    readonly bounds: FpsBoundsCapability | null;
    readonly renderVisible: boolean | null;
    readonly staticCollider: boolean | null;
    readonly health: FpsHealth | null;
    readonly weapon: FpsWeaponMount | null;
    readonly policyBinding: FpsPolicyBinding | null;
}
export interface FpsRuntimeSessionLoadRequest {
    readonly projectBundle: string;
    readonly definitions: readonly FpsStoredEntityDefinition[];
    readonly gameRuleModules: readonly GameRuleModuleManifest[];
}
export interface FpsRuntimeSessionRestartRequest {
    readonly expectedEpoch: number;
}
export interface FpsPrimaryFireRequest {
    readonly tick: number;
    readonly origin: BridgeVec3;
    readonly direction: BridgeVec3;
}
export type FpsLifecycleStatus = {
    readonly state: 'active';
} | {
    readonly state: 'enemy_defeated';
    readonly entity: number;
    readonly tick: number;
};
export interface FpsReadSetEvidence {
    readonly viewKind: string;
    readonly owner: string;
    readonly readSet: readonly string[];
}
export interface FpsReplayEvidence {
    readonly replayUnit: string;
    readonly entityHash: string;
    readonly healthHash: string;
    readonly recordHash: string;
}
export interface FpsEntityHealthReadout {
    readonly entity: number;
    readonly current: number;
    readonly max: number;
}
export interface FpsPolicyBindingReadout extends FpsPolicyBinding {
    readonly entity: number;
}
export interface FpsRuntimeSessionSnapshot {
    readonly backend: FpsRuntimeAuthorityTransport;
    readonly authoritySurface: string;
    readonly projectBundle: string;
    readonly sessionEpoch: number;
    readonly lifecycleStatus: FpsLifecycleStatus;
    readonly playerEntity: number;
    readonly enemyEntity: number;
    readonly health: readonly FpsEntityHealthReadout[];
    readonly policyBindings: readonly FpsPolicyBindingReadout[];
    readonly replayRecords: readonly FpsReplayEvidence[];
    readonly readSets: readonly FpsReadSetEvidence[];
    readonly entityHash: string;
    readonly healthHash: string;
    readonly replayHash: string;
}
export interface FpsPrimaryFireResult {
    readonly backend: FpsRuntimeAuthorityTransport;
    readonly authoritySurface: string;
    readonly mutationOwner: string;
    readonly workspaceTrace: readonly string[];
    readonly shooter: number;
    readonly target: number | null;
    readonly targetHealthBefore: FpsHealth | null;
    readonly targetHealthAfter: FpsHealth | null;
    readonly lifecycleStatus: FpsLifecycleStatus;
    readonly targetRenderVisible: boolean | null;
    readonly entityHash: string;
    readonly healthHash: string;
    readonly replayHash: string;
}
export interface GameExtensionWeaponEffectInvocationRequest {
    readonly hook: WeaponEffectHookRequest;
    readonly primaryFire: FpsPrimaryFireRequest;
}
export interface GameExtensionWeaponEffectInvocationResult {
    readonly hookReceipt: GameExtensionHookReceipt;
    readonly replayEvidence: GameExtensionReplayEvidence;
    readonly primaryFire: FpsPrimaryFireResult | null;
}
export interface GameRuleCatalogValidationReceipt {
    readonly accepted: boolean;
    readonly catalogHash: string;
    readonly diagnostics: readonly GameRuleDiagnostic[];
    readonly trace: readonly GameRuleTraceEntry[];
    readonly evidence: readonly GameRuleEvidenceRef[];
}
export interface GameRuleEffectIntentRequest {
    readonly catalog: GameRuleCatalog;
    readonly request: GameRuleResolutionRequest;
}
export interface GameRuleRuntimeReadout {
    readonly backend: FpsRuntimeAuthorityTransport;
    readonly authoritySurface: string;
    readonly activeModifiers: readonly GameRuleModifierState[];
    readonly recentTrace: readonly GameRuleTraceEntry[];
    readonly recentReplayHashes: readonly string[];
    readonly latestReplayHash: string | null;
}
export type FpsEncounterStatus = 'pending' | 'active' | 'cleared' | 'failed';
export type FpsEncounterLastTransition = 'initialized' | 'activated' | 'cleared' | 'failed' | 'reset';
export type FpsEncounterTransitionAction = 'activate' | 'sync_lifecycle' | 'reset';
export interface FpsEncounterLifecycleInput {
    readonly outcomeKind: 'in_progress' | 'won' | 'lost';
    readonly terminal: boolean;
    readonly enemyDead: boolean;
    readonly playerDead: boolean;
    readonly lifecycleHash: string;
}
export interface FpsEncounterTransitionRequest {
    readonly presetId: string;
    readonly action: FpsEncounterTransitionAction;
    readonly lifecycle: FpsEncounterLifecycleInput;
}
export interface FpsEncounterStateReadout {
    readonly presetId: string;
    readonly status: FpsEncounterStatus;
    readonly spawnedEnemyIds: readonly string[];
    readonly defeatedEnemyIds: readonly string[];
    readonly revision: number;
    readonly lastTransition: FpsEncounterLastTransition;
}
export interface FpsEncounterDirectorSnapshot {
    readonly backend: FpsRuntimeAuthorityTransport;
    readonly authoritySurface: string;
    readonly mutationOwner: string;
    readonly workspaceTrace: readonly string[];
    readonly state: FpsEncounterStateReadout;
    readonly lifecycle: FpsEncounterLifecycleInput;
    readonly readSets: readonly FpsReadSetEvidence[];
    readonly encounterHash: string;
    readonly replayHash: string;
}
export interface FpsEncounterTransitionResult {
    readonly backend: FpsRuntimeAuthorityTransport;
    readonly authoritySurface: string;
    readonly mutationOwner: string;
    readonly workspaceTrace: readonly string[];
    readonly accepted: boolean;
    readonly rejectionReason: 'encounter_not_pending' | 'invalid_encounter_transition' | 'unknown_encounter_preset' | null;
    readonly eventKind: 'runtime_encounter.activated.v0' | 'runtime_encounter.lifecycle_synced.v0' | 'runtime_encounter.reset.v0' | null;
    readonly state: FpsEncounterStateReadout;
    readonly lifecycle: FpsEncounterLifecycleInput;
    readonly encounterHash: string;
    readonly replayHash: string;
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
export interface WorldLoadRequest {
    readonly bundleSchemaVersion: number;
    readonly protocolVersion: number;
    readonly sceneId: number;
}
export interface CompositionStatus {
    readonly loadedWorld: number | null;
    readonly fatalCount: number;
    readonly totalCount: number;
    readonly blocksLoad: boolean;
}
export interface WorldSaveSummary {
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
    readonly worldHash: string;
    readonly meshingStrategy: string;
    readonly chunks: readonly VoxelMeshChunkEvidence[];
    readonly diagnostics: readonly string[];
}
export interface RuntimeBridge {
    initializeEngine(config: EngineConfig): EngineHandle;
    stepSimulation(input: StepInputEnvelope): StepResult;
    submitCommands(batch: CommandBatch): CommandResult;
    pickVoxel(ray: PickRay): PickResult;
    applyCollisionConstrainedCameraInput(input: CollisionConstrainedCameraInputEnvelope): CameraCollisionSnapshot;
    selectVoxel(request: ScreenPointToPickRayRequest): VoxelSelectionSnapshot;
    readVoxelMeshEvidence(request: VoxelMeshEvidenceRequest): VoxelMeshEvidenceSnapshot;
    planVoxelConversion(request: VoxelConversionPlanRequest): VoxelConversionPlan;
    registerVoxelConversionSource(request: VoxelConversionSourceRegistrationRequest): VoxelConversionSourceRegistration;
    registerVoxelConversionMeshAsset(request: VoxelConversionMeshAssetRegistrationRequest): VoxelConversionSourceRegistration;
    previewVoxelConversion(request: VoxelConversionPreviewRequest): VoxelConversionPreview;
    applyVoxelConversion(request: VoxelConversionApplyRequest): VoxelConversionReceipt;
    exportVoxelConversionEvidence(evidence: readonly VoxelConversionEvidenceRef[]): readonly VoxelConversionEvidenceRef[];
    readVoxelModelInfo(request: VoxelModelInfoRequest): VoxelModelInfoReadout;
    exportVoxelVolumeAsset(request: VoxelVolumeAssetExportRequest): VoxelVolumeAssetExportReceipt;
    loadVoxelVolumeAsset(request: VoxelVolumeAssetLoadRequest): VoxelVolumeAssetLoadReceipt;
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
    createCamera(request: CameraCreateRequest): CameraSnapshot;
    applyFirstPersonCameraInput(input: FirstPersonCameraInputEnvelope): CameraSnapshot;
    applyEnemyDirectNavMovement(request: EnemyDirectNavMovementRequest): EnemyDirectNavMovementResult;
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
//# sourceMappingURL=bridge.d.ts.map