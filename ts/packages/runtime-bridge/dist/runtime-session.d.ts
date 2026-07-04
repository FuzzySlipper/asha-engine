import { type CameraCollisionSnapshot, type CameraCreateRequest, type CameraHandle, type CameraProjectionRequest, type CameraProjectionSnapshot, type CameraSnapshot, type CollisionAxis, type CollisionConstrainedCameraInputEnvelope, type CommandBatch, type CommandResult, type FirstPersonCameraInputEnvelope, type RenderFrameDiff } from '@asha/contracts';
import { type CompositionStatus, type EngineHandle, type FrameCursor, type RuntimeBridge, type StepResult, type WorldLoadRequest } from './bridge.js';
import { type CombatReadoutScenario, type CombatRuntimeReadout } from './combat-readout.js';
import { type CombatFeedbackProjection } from './combat-feedback.js';
import { type GeneratedTunnelOperationReceipt, type GeneratedTunnelOperationRequest, type GeneratedTunnelReadout, type GeneratedTunnelReadoutRequest } from './generated-tunnel.js';
import { type EnemyPolicyActorView, type EnemyPolicyCombatView, type EnemyPolicyProposal, type EnemyPolicyProposalFrame, type EnemyPolicySourceDiagnostic, type EnemyPolicyTargetView, type EnemyPolicyVec3 } from './enemy-policy.js';
import { type EncounterDirectorReadout, type EncounterDirectorReadoutRequest, type EncounterTransitionRequest, type RuntimeSessionEncounterTransitionReceipt } from './encounter-director.js';
import { type NavPathQueryRequest, type NavPathReadout, type NavPathScenario, type NavPolicyViewReadout, type NavProjectionReadout } from './nav-readout.js';
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
    readonly kind: 'initialize' | 'submitCommands' | 'tick' | 'createCamera' | 'applyFirstPersonCameraInput' | 'applyCollisionConstrainedCameraInput' | 'submitRuntimeActionIntent' | 'lifecycleDeath' | 'runAutonomousPolicyTick' | 'requestGeneratedTunnelOperation' | 'requestEncounterTransition' | 'requestSessionRestart' | 'restart';
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
export type RuntimeSessionLifecycleScenario = 'current_session' | 'generated_tunnel_enemy_defeated' | 'generated_tunnel_player_defeated';
export type RuntimeSessionLifecycleRole = 'player' | 'enemy';
export type RuntimeSessionLifecycleOutcomeKind = 'in_progress' | 'won' | 'lost';
export type RuntimeSessionLifecycleEventKind = 'runtime_lifecycle.enemy_defeated.v0' | 'runtime_lifecycle.player_defeated.v0';
export interface RuntimeSessionLifecycleStatusRequest {
    readonly scenario?: RuntimeSessionLifecycleScenario;
}
export interface RuntimeSessionLifecycleHealthReadout {
    readonly entity: number;
    readonly current: number;
    readonly max: number;
    readonly dead: boolean;
    readonly healthHash: string;
}
export interface RuntimeSessionLifecycleParticipantReadout {
    readonly role: RuntimeSessionLifecycleRole;
    readonly health: RuntimeSessionLifecycleHealthReadout;
    readonly dead: boolean;
}
export interface RuntimeSessionLifecycleEventReadout {
    readonly kind: RuntimeSessionLifecycleEventKind;
    readonly entity: number;
    readonly tick: number;
    readonly reason: 'combat_health_zero' | 'fixture_player_damage';
    readonly eventHash: string;
}
export interface RuntimeSessionLifecycleStatusReadout {
    readonly kind: 'runtime_session.lifecycle_status.v0';
    readonly scenario: RuntimeSessionLifecycleScenario;
    readonly sequenceId: number;
    readonly tick: number;
    readonly sessionHash: string;
    readonly player: RuntimeSessionLifecycleParticipantReadout;
    readonly enemy: RuntimeSessionLifecycleParticipantReadout;
    readonly outcome: {
        readonly kind: RuntimeSessionLifecycleOutcomeKind;
        readonly terminal: boolean;
        readonly reason: 'none' | 'enemy_defeated' | 'player_defeated';
        readonly label: string;
    };
    readonly restart: {
        readonly eligible: boolean;
        readonly intentKind: 'runtime.restart_session_intent';
        readonly reason: 'always_resettable_reference_fixture';
    };
    readonly events: readonly RuntimeSessionLifecycleEventReadout[];
    readonly fixture: {
        readonly seed: number;
        readonly sceneId: number;
        readonly bundleSchemaVersion: number;
        readonly protocolVersion: number;
        readonly resetHash: string;
    };
    readonly hashes: {
        readonly lifecycleHash: string;
        readonly playerHealthHash: string;
        readonly enemyHealthHash: string;
        readonly replayHash: string;
    };
    readonly nonClaims: readonly [
        'not_save_load_persistence',
        'not_ui_authority',
        'not_demo_local_lifecycle'
    ];
}
export type RuntimeSessionRestartIntentSource = 'hud_menu' | 'programmatic';
export type RuntimeSessionRestartIntentStatus = 'accepted' | 'rejected';
export type RuntimeSessionRestartIntentRejectionReason = 'session_not_terminal' | 'session_hash_mismatch' | 'invalid_restart_intent';
export interface RuntimeSessionRestartIntent {
    readonly kind: 'runtime.restart_session_intent';
    readonly source: RuntimeSessionRestartIntentSource;
    readonly requireTerminal?: boolean;
    readonly expectedSessionHash?: string;
}
export interface RuntimeSessionRestartIntentRejection {
    readonly reason: RuntimeSessionRestartIntentRejectionReason;
    readonly detail: string;
}
export interface RuntimeSessionLifecycleRestartReceipt {
    readonly kind: 'runtime_session.restart_receipt.v0';
    readonly sequenceId: number;
    readonly intent: RuntimeSessionRestartIntent;
    readonly accepted: boolean;
    readonly status: RuntimeSessionRestartIntentStatus;
    readonly rejection: RuntimeSessionRestartIntentRejection | null;
    readonly statusBefore: RuntimeSessionLifecycleStatusReadout;
    readonly statusAfter: RuntimeSessionLifecycleStatusReadout;
    readonly restart: RuntimeSessionRestartResult | null;
    readonly sessionHashBefore: string;
    readonly sessionHashAfter: string;
    readonly resetHash: string;
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
export interface RuntimeSessionAutonomousPolicyTickInput {
    readonly targetCamera: CameraHandle;
    readonly tick?: number;
    readonly policySource?: string;
    readonly navScenario?: NavPathScenario;
    readonly enemy?: Partial<EnemyPolicyActorView>;
    readonly target?: Omit<Partial<EnemyPolicyTargetView>, 'camera'>;
    readonly combat?: Partial<EnemyPolicyCombatView>;
}
export type RuntimeSessionAutonomousPolicyProposalStatus = 'accepted' | 'unsupported' | 'rejected';
export type RuntimeSessionAutonomousPolicyProposalRejectionReason = 'movement_authority_not_wired' | 'policy_source_forbidden_capability' | 'invalid_policy_proposal' | 'runtime_action_rejected';
export interface RuntimeSessionAutonomousPolicyProposalRejection {
    readonly reason: RuntimeSessionAutonomousPolicyProposalRejectionReason;
    readonly detail: string;
}
export interface RuntimeSessionAutonomousPolicyMovementSummary {
    readonly status: RuntimeSessionAutonomousPolicyProposalStatus;
    readonly actor: string;
    readonly target: string;
    readonly from: EnemyPolicyVec3;
    readonly nextWaypoint: EnemyPolicyVec3 | null;
    readonly pathHash: string;
    readonly reason: RuntimeSessionAutonomousPolicyProposalRejectionReason | null;
}
export interface RuntimeSessionAutonomousPolicyCombatSummary {
    readonly status: RuntimeSessionAutonomousPolicyProposalStatus;
    readonly action: RuntimeActionIntentEnvelope['action'];
    readonly outcome: CombatRuntimeReadout['outcome'] | null;
    readonly healthHash: string | null;
    readonly replayHash: string | null;
}
export interface RuntimeSessionAutonomousPolicyProposalReceipt {
    readonly proposalKind: EnemyPolicyProposal['kind'];
    readonly actor: string;
    readonly target: string;
    readonly accepted: boolean;
    readonly status: RuntimeSessionAutonomousPolicyProposalStatus;
    readonly rejection: RuntimeSessionAutonomousPolicyProposalRejection | null;
    readonly movement: RuntimeSessionAutonomousPolicyMovementSummary | null;
    readonly actionReceipt: RuntimeSessionActionIntentReceipt | null;
    readonly combat: RuntimeSessionAutonomousPolicyCombatSummary | null;
}
export interface RuntimeSessionAutonomousPolicyTickReadout {
    readonly kind: 'runtime_session.autonomous_policy_tick.v0';
    readonly loopId: 'generated_tunnel_enemy_policy_loop.v0';
    readonly sequenceIdBefore: number;
    readonly sequenceIdAfter: number;
    readonly sessionHashBefore: string;
    readonly sessionHashAfter: string;
    readonly tick: number;
    readonly step: RuntimeSessionTickResult;
    readonly policy: {
        readonly fixtureKind: 'generated_tunnel_enemy_policy_fixture.v0';
        readonly proposalFrame: EnemyPolicyProposalFrame;
        readonly sourceChecked: boolean;
        readonly sourceDiagnostics: readonly EnemyPolicySourceDiagnostic[];
        readonly proposalValidationDiagnostics: readonly RuntimeSessionAutonomousPolicyProposalRejection[];
    };
    readonly nav: {
        readonly projectionHash: string;
        readonly pathHash: string;
        readonly outcome: NavPathReadout['outcome'];
        readonly visited: number;
        readonly pathLength: number;
    };
    readonly proposalReceipts: readonly RuntimeSessionAutonomousPolicyProposalReceipt[];
    readonly proposalSummary: {
        readonly acceptedProposalCount: number;
        readonly rejectedProposalCount: number;
        readonly unsupportedProposalCount: number;
    };
    readonly commandSummary: {
        readonly acceptedCommandCount: number;
        readonly rejectedCommandCount: number;
        readonly acceptedRuntimeActionCount: number;
        readonly rejectedRuntimeActionCount: number;
    };
    readonly movementSummary: RuntimeSessionAutonomousPolicyMovementSummary | null;
    readonly combatSummary: RuntimeSessionAutonomousPolicyCombatSummary | null;
    readonly replay: {
        readonly recordCount: number;
        readonly lastRecordKind: RuntimeSessionReplayRecord['kind'] | null;
        readonly recordHashes: readonly string[];
    };
    readonly tickHash: string;
    readonly nonClaims: readonly [
        'not_generic_event_bus',
        'not_behavior_tree',
        'not_demo_local_authority',
        'movement_authority_not_wired'
    ];
}
export interface RuntimeSessionCombatReadoutRequest {
    readonly scenario?: CombatReadoutScenario;
}
export interface RuntimeSessionCombatFeedbackProjectionRequest extends RuntimeSessionCombatReadoutRequest {
    readonly camera?: CameraHandle;
    readonly viewport?: CameraProjectionRequest['viewport'];
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
    runAutonomousPolicyTick(input: RuntimeSessionAutonomousPolicyTickInput): RuntimeSessionAutonomousPolicyTickReadout;
    readLifecycleStatus(request?: RuntimeSessionLifecycleStatusRequest): RuntimeSessionLifecycleStatusReadout;
    requestSessionRestart(intent: RuntimeSessionRestartIntent): RuntimeSessionLifecycleRestartReceipt;
    readEncounterDirector(request?: EncounterDirectorReadoutRequest): EncounterDirectorReadout;
    requestEncounterTransition(request: EncounterTransitionRequest): RuntimeSessionEncounterTransitionReceipt;
    readCombatReadout(request?: RuntimeSessionCombatReadoutRequest): CombatRuntimeReadout;
    readCombatFeedbackProjection(request?: RuntimeSessionCombatFeedbackProjectionRequest): CombatFeedbackProjection;
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