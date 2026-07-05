import { type CombatRuntimeReadout } from './combat-readout.js';
import { type EncounterLifecycleInput } from './encounter-director.js';
import type { EnemyPolicyProposal } from './enemy-policy.js';
import type { GeneratedTunnelOperationRequest, GeneratedTunnelReadoutRequest } from './generated-tunnel.js';
import type { RuntimeActionIntentEnvelope } from './runtime-action.js';
import type { RuntimeSessionActionIntentReceipt, RuntimeSessionAutonomousPolicyTickInput, RuntimeSessionAutonomousPolicyProposalReceipt, RuntimeSessionAutonomousPolicyProposalRejection, RuntimeSessionEcrpProjectState, RuntimeSessionIdentity, RuntimeSessionInitializeInput, RuntimeSessionLifecycleEventKind, RuntimeSessionLifecycleEventReadout, RuntimeSessionLifecycleHealthReadout, RuntimeSessionLifecycleScenario, RuntimeSessionLifecycleState, RuntimeSessionLifecycleStatusReadout, RuntimeSessionLifecycleStatusRequest, RuntimeSessionRestartIntent } from './runtime-session.js';
export declare function initialRuntimeSessionLifecycleState(): RuntimeSessionLifecycleState;
export declare function generatedTunnelEnemyDefeatedLifecycleState(): RuntimeSessionLifecycleState;
export declare function generatedTunnelPlayerDefeatedLifecycleState(): RuntimeSessionLifecycleState;
export declare function lifecycleHealth(entity: number, current: number, max: number, dead: boolean): RuntimeSessionLifecycleHealthReadout;
export declare function buildRuntimeSessionPrimaryFireReadout(input: {
    readonly projectState: RuntimeSessionEcrpProjectState | null;
    readonly lifecycleState: RuntimeSessionLifecycleState;
    readonly tick: number;
}): CombatRuntimeReadout;
export declare function lifecycleEvent(kind: RuntimeSessionLifecycleEventKind, entity: number, tick: number, reason: RuntimeSessionLifecycleEventReadout['reason']): RuntimeSessionLifecycleEventReadout;
export declare function lifecycleStatusReadout(input: {
    readonly scenario: RuntimeSessionLifecycleScenario;
    readonly state: RuntimeSessionLifecycleState;
    readonly identity: RuntimeSessionIdentity;
    readonly sequenceId: number;
    readonly tick: number;
    readonly restartCount: number;
    readonly sessionHash: string;
}): RuntimeSessionLifecycleStatusReadout;
export declare function lifecycleStatusToEncounterLifecycle(status: RuntimeSessionLifecycleStatusReadout): EncounterLifecycleInput;
export declare function validateLifecycleStatusRequest(request: RuntimeSessionLifecycleStatusRequest): void;
export declare function validateRestartIntent(intent: RuntimeSessionRestartIntent): void;
export declare function validateAutonomousPolicyTickInput(input: RuntimeSessionAutonomousPolicyTickInput): void;
export declare function validateAutonomousPolicyProposal(proposal: EnemyPolicyProposal, tick: number): RuntimeSessionAutonomousPolicyProposalRejection | null;
export declare function rejectedAutonomousPolicyProposalReceipt(proposal: EnemyPolicyProposal, rejection: RuntimeSessionAutonomousPolicyProposalRejection): RuntimeSessionAutonomousPolicyProposalReceipt;
export declare function unsupportedAutonomousMovementReceipt(proposal: Extract<EnemyPolicyProposal, {
    readonly kind: 'enemy_policy.move_toward_target.v0';
}>): RuntimeSessionAutonomousPolicyProposalReceipt;
export declare function runtimeActionReceiptToAutonomousReceipt(proposal: Extract<EnemyPolicyProposal, {
    readonly kind: 'enemy_policy.primary_fire_intent.v0';
}>, actionReceipt: RuntimeSessionActionIntentReceipt): RuntimeSessionAutonomousPolicyProposalReceipt;
export declare function validateInitializeInput(input: RuntimeSessionInitializeInput): void;
export declare function validateRuntimeActionIntentEnvelope(envelope: RuntimeActionIntentEnvelope): void;
export declare function combatReadoutTick(readout: CombatRuntimeReadout): number;
export declare function validateGeneratedTunnelReadoutRequest(request: GeneratedTunnelReadoutRequest): void;
export declare function validateGeneratedTunnelOperationRequest(request: GeneratedTunnelOperationRequest): void;
//# sourceMappingURL=runtime-session-lifecycle.d.ts.map