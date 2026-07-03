import type { CameraCreateRequest, CameraProjectionRequest, CameraProjectionSnapshot, CameraSnapshot, CommandBatch, CommandResult, FirstPersonCameraInputEnvelope, RenderFrameDiff } from '@asha/contracts';
import { type CompositionStatus, type EngineHandle, type FrameCursor, type RuntimeBridge, type StepResult, type WorldLoadRequest } from './bridge.js';
export type RuntimeSessionMode = 'reference';
export interface RuntimeSessionProjectIdentity {
    readonly gameId: string;
    readonly workspaceId: string;
}
export interface RuntimeSessionInitializeInput {
    readonly sessionId: string;
    readonly seed: number;
    readonly project: RuntimeSessionProjectIdentity;
    readonly projectBundle: WorldLoadRequest;
}
export interface RuntimeSessionIdentity {
    readonly sessionId: string;
    readonly mode: RuntimeSessionMode;
    readonly seed: number;
    readonly project: RuntimeSessionProjectIdentity;
    readonly projectBundle: WorldLoadRequest;
    readonly nonClaims: readonly RuntimeSessionNonClaim[];
}
export type RuntimeSessionNonClaim = 'not_native_runtime' | 'not_raw_state_store' | 'not_arbitrary_json_bridge' | 'not_gameplay_loop' | 'not_renderer';
export interface RuntimeSessionStateSummary {
    readonly identity: RuntimeSessionIdentity;
    readonly engine: EngineHandle;
    readonly composition: CompositionStatus;
    readonly sequenceId: number;
    readonly tick: number;
    readonly sessionHash: string;
}
export interface RuntimeSessionTickInput {
    readonly tick?: number;
}
export interface RuntimeSessionTickResult {
    readonly sequenceId: number;
    readonly tick: number;
    readonly step: StepResult;
    readonly composition: CompositionStatus;
    readonly sessionHash: string;
}
export interface RuntimeSessionCommandReceipt {
    readonly sequenceId: number;
    readonly batch: CommandBatch;
    readonly result: CommandResult;
    readonly acceptedCommandCount: number;
    readonly rejectedCommandCount: number;
    readonly sessionHashBefore: string;
    readonly sessionHashAfter: string;
}
export interface RuntimeSessionProjectionSummary {
    readonly sequenceId: number;
    readonly cursor: FrameCursor;
    readonly frame: RenderFrameDiff;
    readonly composition: CompositionStatus;
    readonly renderDiffCount: number;
    readonly projectionHash: string;
}
export interface RuntimeSessionReplayRecord {
    readonly sequenceId: number;
    readonly kind: 'initialize' | 'submitCommands' | 'tick' | 'createCamera' | 'applyFirstPersonCameraInput' | 'restart';
    readonly recordHash: string;
}
export interface RuntimeSessionTelemetrySummary {
    readonly sequenceId: number;
    readonly tick: number;
    readonly composition: CompositionStatus;
    readonly acceptedCommandCount: number;
    readonly rejectedCommandCount: number;
    readonly restartCount: number;
    readonly sessionHash: string;
    readonly replayRecords: readonly RuntimeSessionReplayRecord[];
}
export interface RuntimeSessionRestartResult {
    readonly sequenceId: number;
    readonly tick: number;
    readonly composition: CompositionStatus;
    readonly restartCount: number;
    readonly sessionHash: string;
}
export interface RuntimeSessionCameraCreateReceipt {
    readonly sequenceId: number;
    readonly request: CameraCreateRequest;
    readonly snapshot: CameraSnapshot;
    readonly sessionHash: string;
}
export interface RuntimeSessionCameraInputReceipt {
    readonly sequenceId: number;
    readonly envelope: FirstPersonCameraInputEnvelope;
    readonly snapshot: CameraSnapshot;
    readonly sessionHashBefore: string;
    readonly sessionHashAfter: string;
}
export interface RuntimeSessionCameraProjectionReadout {
    readonly sequenceId: number;
    readonly request: CameraProjectionRequest;
    readonly snapshot: CameraProjectionSnapshot;
    readonly projectionHash: string;
}
export interface RuntimeSessionFacade {
    initialize(input: RuntimeSessionInitializeInput): RuntimeSessionStateSummary;
    submitCommands(batch: CommandBatch): RuntimeSessionCommandReceipt;
    tick(input?: RuntimeSessionTickInput): RuntimeSessionTickResult;
    createCamera(request: CameraCreateRequest): RuntimeSessionCameraCreateReceipt;
    applyFirstPersonCameraInput(envelope: FirstPersonCameraInputEnvelope): RuntimeSessionCameraInputReceipt;
    readCameraProjection(request: CameraProjectionRequest): RuntimeSessionCameraProjectionReadout;
    readProjection(): RuntimeSessionProjectionSummary;
    readTelemetry(): RuntimeSessionTelemetrySummary;
    restart(): RuntimeSessionRestartResult;
}
export interface RuntimeSessionFacadeOptions {
    readonly bridge?: RuntimeBridge;
}
export declare function createMockRuntimeSession(options?: RuntimeSessionFacadeOptions): RuntimeSessionFacade;
//# sourceMappingURL=runtime-session.d.ts.map