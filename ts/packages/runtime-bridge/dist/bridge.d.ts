import type { CameraCollisionSnapshot, CameraCreateRequest, CameraProjectionRequest, CameraProjectionSnapshot, CameraSnapshot, CollisionConstrainedCameraInputEnvelope, CommandBatch, CommandResult, FirstPersonCameraInputEnvelope, ModelMaterialPreviewRequest, ModelMaterialPreviewSnapshot, PickRay, PickResult, RenderFrameDiff, SceneObjectCommandRequest, SceneObjectCommandResult, SceneObjectSnapshot, ScreenPointToPickRayRequest, VoxelSelectionSnapshot } from '@asha/contracts';
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
export type RuntimeBridgeErrorKind = 'not_initialized' | 'invalid_input' | 'unknown_handle' | 'buffer_expired' | 'native_unavailable' | 'operation_unimplemented' | 'internal';
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