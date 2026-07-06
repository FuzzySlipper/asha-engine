import type { CameraCreateRequest, CameraProjectionRequest, CollisionConstrainedCameraInputEnvelope, CommandBatch, FirstPersonCameraInputEnvelope } from '@asha/contracts';
import { type RuntimeBridge } from './bridge.js';
import type { CombatRuntimeReadout } from './combat-readout.js';
import type { CombatFeedbackProjection } from './combat-feedback.js';
import type { GeneratedTunnelOperationRequest, GeneratedTunnelReadout, GeneratedTunnelReadoutRequest } from './generated-tunnel.js';
import type { EncounterDirectorReadout, EncounterDirectorReadoutRequest, EncounterTransitionRequest, RuntimeSessionEncounterTransitionReceipt } from './encounter-director.js';
import type { NavPathQueryRequest, NavPathReadout, NavPolicyViewReadout, NavProjectionReadout } from './nav-readout.js';
import type { RuntimeActionIntentEnvelope } from './runtime-action.js';
import type { RuntimeSessionActionIntentReceipt, RuntimeSessionAutonomousPolicyTickInput, RuntimeSessionAutonomousPolicyTickReadout, RuntimeSessionCameraCollisionInputReceipt, RuntimeSessionCameraCreateReceipt, RuntimeSessionCameraInputReceipt, RuntimeSessionCameraProjectionReadout, RuntimeSessionCommandReceipt, RuntimeSessionCombatFeedbackProjectionRequest, RuntimeSessionCombatReadoutRequest, RuntimeSessionEcrpProjectLoadInput, RuntimeSessionEcrpProjectLoadReceipt, RuntimeSessionEcrpReadout, RuntimeSessionFacade, RuntimeSessionGeneratedTunnelOperationReceipt, RuntimeSessionInitializeInput, RuntimeSessionLifecycleRestartReceipt, RuntimeSessionLifecycleStatusReadout, RuntimeSessionLifecycleStatusRequest, RuntimeSessionProjectionSummary, RuntimeSessionRestartIntent, RuntimeSessionRestartResult, RuntimeSessionStateSummary, RuntimeSessionTelemetrySummary, RuntimeSessionTickInput, RuntimeSessionTickResult } from './runtime-session.js';
export declare class RustBackedRuntimeSessionFacade implements RuntimeSessionFacade {
    #private;
    constructor(bridge: RuntimeBridge);
    initialize(input: RuntimeSessionInitializeInput): RuntimeSessionStateSummary;
    loadEcrpProject(input: RuntimeSessionEcrpProjectLoadInput): RuntimeSessionEcrpProjectLoadReceipt;
    submitCommands(batch: CommandBatch): RuntimeSessionCommandReceipt;
    tick(input?: RuntimeSessionTickInput): RuntimeSessionTickResult;
    createCamera(request: CameraCreateRequest): RuntimeSessionCameraCreateReceipt;
    applyFirstPersonCameraInput(envelope: FirstPersonCameraInputEnvelope): RuntimeSessionCameraInputReceipt;
    applyCollisionConstrainedCameraInput(envelope: CollisionConstrainedCameraInputEnvelope): RuntimeSessionCameraCollisionInputReceipt;
    submitRuntimeActionIntent(envelope: RuntimeActionIntentEnvelope): RuntimeSessionActionIntentReceipt;
    runAutonomousPolicyTick(_input: RuntimeSessionAutonomousPolicyTickInput): RuntimeSessionAutonomousPolicyTickReadout;
    readLifecycleStatus(request?: RuntimeSessionLifecycleStatusRequest): RuntimeSessionLifecycleStatusReadout;
    requestSessionRestart(intent: RuntimeSessionRestartIntent): RuntimeSessionLifecycleRestartReceipt;
    readEncounterDirector(request?: EncounterDirectorReadoutRequest): EncounterDirectorReadout;
    requestEncounterTransition(request: EncounterTransitionRequest): RuntimeSessionEncounterTransitionReceipt;
    readCombatReadout(_request?: RuntimeSessionCombatReadoutRequest): CombatRuntimeReadout;
    readCombatFeedbackProjection(_request?: RuntimeSessionCombatFeedbackProjectionRequest): CombatFeedbackProjection;
    readGeneratedTunnelReadout(_request?: GeneratedTunnelReadoutRequest): GeneratedTunnelReadout;
    readNavProjection(): NavProjectionReadout;
    queryNavPath(_request?: NavPathQueryRequest): NavPathReadout;
    readNavPolicyView(): NavPolicyViewReadout;
    requestGeneratedTunnelOperation(_request: GeneratedTunnelOperationRequest): RuntimeSessionGeneratedTunnelOperationReceipt;
    readEcrpRuntimeReadout(): RuntimeSessionEcrpReadout;
    readCameraProjection(request: CameraProjectionRequest): RuntimeSessionCameraProjectionReadout;
    readProjection(): RuntimeSessionProjectionSummary;
    readTelemetry(): RuntimeSessionTelemetrySummary;
    restart(): RuntimeSessionRestartResult;
}
//# sourceMappingURL=runtime-session-rust-facade.d.ts.map