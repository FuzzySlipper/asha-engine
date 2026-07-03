import type { CameraCollisionSnapshot, CameraCreateRequest, CameraProjectionRequest, CameraProjectionSnapshot, CameraSnapshot, CollisionAxis, CollisionConstrainedCameraInputEnvelope, CommandBatch, CommandResult, FirstPersonCameraInputEnvelope, RenderFrameDiff } from '@asha/contracts';
import { type CompositionStatus, type EngineHandle, type FrameCursor, type RuntimeBridge, type StepResult, type WorldLoadRequest } from './bridge.js';
import { type CombatReadoutScenario, type CombatRuntimeReadout } from './combat-readout.js';
import { type GeneratedTunnelOperationReceipt, type GeneratedTunnelOperationRequest, type GeneratedTunnelReadout, type GeneratedTunnelReadoutRequest } from './generated-tunnel.js';
import { type NavPathQueryRequest, type NavPathReadout, type NavPolicyViewReadout, type NavProjectionReadout } from './nav-readout.js';
import type { RuntimeActionIntentEnvelope, RuntimeActionIntentRejection, RuntimeActionIntentStatus } from './runtime-action.js';
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
    readonly kind: 'initialize' | 'submitCommands' | 'tick' | 'createCamera' | 'applyFirstPersonCameraInput' | 'applyCollisionConstrainedCameraInput' | 'submitRuntimeActionIntent' | 'requestGeneratedTunnelOperation' | 'restart';
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
export interface RuntimeSessionCameraCollisionInputReceipt {
    readonly sequenceId: number;
    readonly envelope: CollisionConstrainedCameraInputEnvelope;
    readonly snapshot: CameraCollisionSnapshot;
    readonly collided: boolean;
    readonly blockedAxes: readonly CollisionAxis[];
    readonly worldHash: string;
    readonly collisionProjectionHash: string;
    readonly movementHash: string;
    readonly sessionHashBefore: string;
    readonly sessionHashAfter: string;
}
export interface RuntimeSessionCameraProjectionReadout {
    readonly sequenceId: number;
    readonly request: CameraProjectionRequest;
    readonly snapshot: CameraProjectionSnapshot;
    readonly projectionHash: string;
}
export interface RuntimeSessionActionIntentReceipt {
    readonly sequenceId: number;
    readonly envelope: RuntimeActionIntentEnvelope;
    readonly accepted: boolean;
    readonly status: RuntimeActionIntentStatus;
    readonly rejection: RuntimeActionIntentRejection | null;
    readonly combatReadout: CombatRuntimeReadout | null;
    readonly sessionHashBefore: string;
    readonly sessionHashAfter: string;
}
export interface RuntimeSessionCombatReadoutRequest {
    readonly scenario?: CombatReadoutScenario;
}
export interface RuntimeSessionGeneratedTunnelOperationReceipt extends GeneratedTunnelOperationReceipt {
    readonly sequenceId: number;
    readonly request: GeneratedTunnelOperationRequest;
    readonly sessionHashBefore: string;
    readonly sessionHashAfter: string;
}
export interface RuntimeSessionFacade {
    initialize(input: RuntimeSessionInitializeInput): RuntimeSessionStateSummary;
    submitCommands(batch: CommandBatch): RuntimeSessionCommandReceipt;
    tick(input?: RuntimeSessionTickInput): RuntimeSessionTickResult;
    createCamera(request: CameraCreateRequest): RuntimeSessionCameraCreateReceipt;
    applyFirstPersonCameraInput(envelope: FirstPersonCameraInputEnvelope): RuntimeSessionCameraInputReceipt;
    applyCollisionConstrainedCameraInput(envelope: CollisionConstrainedCameraInputEnvelope): RuntimeSessionCameraCollisionInputReceipt;
    submitRuntimeActionIntent(envelope: RuntimeActionIntentEnvelope): RuntimeSessionActionIntentReceipt;
    readCombatReadout(request?: RuntimeSessionCombatReadoutRequest): CombatRuntimeReadout;
    readGeneratedTunnelReadout(request?: GeneratedTunnelReadoutRequest): GeneratedTunnelReadout;
    readNavProjection(): NavProjectionReadout;
    queryNavPath(request?: NavPathQueryRequest): NavPathReadout;
    readNavPolicyView(): NavPolicyViewReadout;
    requestGeneratedTunnelOperation(request: GeneratedTunnelOperationRequest): RuntimeSessionGeneratedTunnelOperationReceipt;
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