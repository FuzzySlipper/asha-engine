import type { CameraCreateRequest, CameraProjectionRequest, CameraProjectionSnapshot, CameraSnapshot, CameraCollisionSnapshot, CollisionConstrainedCameraInputEnvelope, ScreenPointToPickRayRequest, VoxelSelectionSnapshot, CommandBatch, CommandResult, FirstPersonCameraInputEnvelope, PickRay, PickResult, RenderFrameDiff, ModelMaterialPreviewRequest, ModelMaterialPreviewSnapshot, SceneObjectCommandRequest, SceneObjectCommandResult, SceneObjectSnapshot } from '@asha/contracts';
import { type NativeAddon } from '@asha/native-bridge';
export { MANIFEST_OPERATIONS } from './generated/operations.js';
export type { BridgeOperation, BridgeSurface } from './generated/operations.js';
export type { CameraCreateRequest, CameraProjectionRequest, CameraProjectionSnapshot, CameraSnapshot, CameraCollisionSnapshot, CollisionConstrainedCameraInputEnvelope, ScreenPointToPickRayRequest, PickRaySnapshot, VoxelSelectionSnapshot, CommandBatch, CommandResult, FirstPersonCameraInputEnvelope, PickRay, PickResult, CatalogEntry, MaterialProjection, StaticMeshAsset, ModelMaterialPreviewRequest, ModelMaterialPreviewSnapshot, FlatSceneDocument, SceneNodeId, SceneNodeRecord, SceneObjectCommandRejection, SceneObjectCommandRequest, SceneObjectCommandResult, SceneObjectSnapshot, } from '@asha/contracts';
export { decodeRenderDiff, decodeRenderFrameDiff, RenderDecodeError, RenderDiffStream, FrameMemory, } from './render-decode.js';
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
export type GameRuntimeMode = 'reference' | 'native' | 'degraded';
export type GameRuntimeNonClaim = 'not_native_runtime' | 'not_hardware_gpu' | 'not_performance_evidence' | 'not_publish_artifact' | 'not_wasm_authority';
export type GameRuntimeDiagnosticCode = 'missing_compatibility' | 'missing_world_bundle' | 'unsupported_runtime_entry' | 'runtime_unavailable' | 'operation_unimplemented' | 'command_rejected' | 'stale_sequence' | 'stale_readback' | 'internal';
export interface GameRuntimeDiagnostic {
    readonly code: GameRuntimeDiagnosticCode;
    readonly severity: 'info' | 'warning' | 'error';
    readonly message: string;
}
export interface GameRuntimeCompatibility {
    readonly contractsPackageVersion: string;
    readonly runtimeBridgePackageVersion: string;
    readonly devtoolsProtocolVersion?: string;
    readonly publishArtifactVersion?: string;
}
export interface GameRuntimeProfile {
    readonly profileId: string;
    readonly runtimeMode: GameRuntimeMode;
    readonly launcherName: string;
    readonly bridgeCompatibility: GameRuntimeCompatibility;
    readonly nonClaims: readonly GameRuntimeNonClaim[];
}
export interface GameRuntimeResourceProfile {
    readonly profileId: string;
    readonly runtimeEntry: string;
    readonly worldBundleId: string;
    readonly resourceManifestHash?: string;
    readonly estimatedBytes?: number;
}
export interface GameRuntimeEvidenceRef {
    readonly kind: 'projection' | 'render_diff' | 'replay' | 'evidence_export' | 'telemetry' | 'diagnostic';
    readonly id: string;
    readonly path?: string;
    readonly sha256?: string;
    readonly sequenceId?: number;
}
export interface GameRuntimeConfig {
    readonly gameId: string;
    readonly workspaceId: string;
    readonly runtimeEntry: string;
    readonly compatibility: GameRuntimeCompatibility;
    readonly resourceProfile: GameRuntimeResourceProfile;
    readonly world: WorldLoadRequest;
    readonly startedAtIso?: string;
}
export interface GameRuntimeIdentity {
    readonly gameId: string;
    readonly workspaceId: string;
    readonly runtimeMode: GameRuntimeMode;
    readonly runtimeEntry: string;
    readonly startedAtIso: string;
    readonly compatibility: GameRuntimeCompatibility;
    readonly nonClaims: readonly GameRuntimeNonClaim[];
}
export interface GameRuntimeProjectionSummary {
    readonly sequenceId: number;
    readonly worldHash: string;
    readonly authorityHash: string;
    readonly loadedWorld: number | null;
    readonly fatalCount: number;
    readonly totalDiagnosticCount: number;
    readonly evidenceRefs: readonly GameRuntimeEvidenceRef[];
}
export interface GameRuntimeLaunchResult {
    readonly status: 'launched' | 'degraded' | 'failed';
    readonly identity: GameRuntimeIdentity;
    readonly runtimeProfile: GameRuntimeProfile;
    readonly resourceProfile: GameRuntimeResourceProfile;
    readonly projection: GameRuntimeProjectionSummary;
    readonly diagnostics: readonly GameRuntimeDiagnostic[];
    readonly evidenceRefs: readonly GameRuntimeEvidenceRef[];
}
export interface GameRuntimeCommandProposalResult {
    readonly sequenceId: number;
    readonly status: 'accepted' | 'rejected' | 'failed';
    readonly batch: CommandBatch;
    readonly result: CommandResult | null;
    readonly authorityHashBefore: string;
    readonly authorityHashAfter: string;
    readonly diagnostics: readonly GameRuntimeDiagnostic[];
    readonly evidenceRefs: readonly GameRuntimeEvidenceRef[];
}
export interface GameRuntimeRenderDiffSnapshot {
    readonly sequenceId: number;
    readonly cursor: FrameCursor;
    readonly frame: RenderFrameDiff;
    readonly evidenceRefs: readonly GameRuntimeEvidenceRef[];
}
export interface GameRuntimeTelemetrySnapshot {
    readonly sequenceId: number;
    readonly runtimeMode: GameRuntimeMode;
    readonly acceptedCommandCount: number;
    readonly rejectedCommandCount: number;
    readonly diagnostics: readonly GameRuntimeDiagnostic[];
    readonly evidenceRefs: readonly GameRuntimeEvidenceRef[];
}
export interface GameRuntimeReplayExportRequest {
    readonly replayId: string;
}
export interface GameRuntimeReplayExport {
    readonly replayId: string;
    readonly sequenceId: number;
    readonly authorityHash: string;
    readonly evidenceRefs: readonly GameRuntimeEvidenceRef[];
}
export interface GameRuntimeEvidenceExportRequest {
    readonly evidenceId: string;
}
export interface GameRuntimeEvidenceExport {
    readonly evidenceId: string;
    readonly sequenceId: number;
    readonly projection: GameRuntimeProjectionSummary;
    readonly nonClaims: readonly GameRuntimeNonClaim[];
    readonly evidenceRefs: readonly GameRuntimeEvidenceRef[];
}
export interface GameRuntimeSession {
    readonly launch: GameRuntimeLaunchResult;
    readonly identity: GameRuntimeIdentity;
    pullProjection(): Promise<GameRuntimeProjectionSummary>;
    pullRenderDiff(cursor?: FrameCursor): Promise<GameRuntimeRenderDiffSnapshot>;
    pullTelemetry(): Promise<GameRuntimeTelemetrySnapshot>;
    proposeCommands(batch: CommandBatch): Promise<GameRuntimeCommandProposalResult>;
    exportReplay(request: GameRuntimeReplayExportRequest): Promise<GameRuntimeReplayExport>;
    exportEvidence(request: GameRuntimeEvidenceExportRequest): Promise<GameRuntimeEvidenceExport>;
    shutdown(): Promise<void>;
}
export interface GameRuntimeLauncher {
    readonly mode: GameRuntimeMode;
    launch(config: GameRuntimeConfig): Promise<GameRuntimeSession>;
}
export declare class MockRuntimeBridge implements RuntimeBridge {
    #private;
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
export declare class ReferenceGameRuntimeLauncher implements GameRuntimeLauncher {
    readonly mode = "reference";
    launch(config: GameRuntimeConfig): Promise<GameRuntimeSession>;
}
export declare function createReferenceGameRuntimeLauncher(): GameRuntimeLauncher;
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
    readModelMaterialPreview(_request: ModelMaterialPreviewRequest): ModelMaterialPreviewSnapshot;
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
//# sourceMappingURL=index.d.ts.map