import {
  cameraHandle,
  type CameraCollisionSnapshot,
  type CameraCreateRequest,
  type CameraHandle,
  type CameraProjectionRequest,
  type CameraProjectionSnapshot,
  type CameraSnapshot,
  type CollisionAxis,
  type CollisionConstrainedCameraInputEnvelope,
  type CommandBatch,
  type CommandResult,
  type FirstPersonCameraInputEnvelope,
  type RenderFrameDiff,
} from '@asha/contracts';
import {
  RuntimeBridgeError,
  frameCursor,
  type CompositionStatus,
  type EngineHandle,
  type FrameCursor,
  type RuntimeBridge,
  type StepResult,
  type WorldLoadRequest,
} from './bridge.js';
import {
  GENERATED_TUNNEL_FIRE_HIT_READOUT,
  GENERATED_TUNNEL_FIRE_MISS_READOUT,
  type CombatReadoutScenario,
  type CombatRuntimeReadout,
} from './combat-readout.js';
import {
  buildCombatFeedbackProjection,
  defaultCombatFeedbackIntent,
  type CombatFeedbackProjection,
} from './combat-feedback.js';
import {
  TINY_GENERATED_TUNNEL_READOUT,
  type GeneratedTunnelOperationReceipt,
  type GeneratedTunnelOperationRequest,
  type GeneratedTunnelReadout,
  type GeneratedTunnelReadoutRequest,
} from './generated-tunnel.js';
import {
  createGeneratedTunnelEnemyPolicyFixture,
  validateEnemyPolicySource,
  type EnemyPolicyActorView,
  type EnemyPolicyCombatView,
  type EnemyPolicyProposal,
  type EnemyPolicyProposalFrame,
  type EnemyPolicySourceDiagnostic,
  type EnemyPolicyTargetView,
  type EnemyPolicyVec3,
} from './enemy-policy.js';
import {
  buildEncounterDirectorReadout,
  buildEncounterTransitionReceipt,
  initialEncounterDirectorState,
  transitionEncounterDirectorState,
  validateEncounterDirectorReadoutRequest,
  validateEncounterTransitionRequest,
  type EncounterDirectorReadout,
  type EncounterDirectorReadoutRequest,
  type EncounterDirectorState,
  type EncounterLifecycleInput,
  type EncounterLifecycleScenario,
  type EncounterTransitionRequest,
  type RuntimeSessionEncounterTransitionReceipt,
} from './encounter-director.js';
import { createMockRuntimeBridge } from './mock.js';
import {
  GENERATED_TUNNEL_NAV_POLICY_VIEW,
  GENERATED_TUNNEL_NAV_PROJECTION,
  GENERATED_TUNNEL_NO_PATH,
  GENERATED_TUNNEL_REACHABLE_PATH,
  type NavPathQueryRequest,
  type NavPathReadout,
  type NavPathScenario,
  type NavPolicyViewReadout,
  type NavProjectionReadout,
} from './nav-readout.js';
import type {
  RuntimeActionIntentEnvelope,
  RuntimeActionIntentRejection,
  RuntimeActionIntentStatus,
} from './runtime-action.js';

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

export type RuntimeSessionNonClaim =
  | 'not_native_runtime'
  | 'not_raw_state_store'
  | 'not_arbitrary_json_bridge'
  | 'not_gameplay_loop'
  | 'not_renderer';

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
  readonly kind:
    | 'initialize'
    | 'submitCommands'
    | 'tick'
    | 'createCamera'
    | 'applyFirstPersonCameraInput'
    | 'applyCollisionConstrainedCameraInput'
    | 'submitRuntimeActionIntent'
    | 'lifecycleDeath'
    | 'runAutonomousPolicyTick'
    | 'requestGeneratedTunnelOperation'
    | 'requestEncounterTransition'
    | 'requestSessionRestart'
    | 'restart';
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

export type RuntimeSessionLifecycleScenario =
  | 'current_session'
  | 'generated_tunnel_enemy_defeated'
  | 'generated_tunnel_player_defeated';
export type RuntimeSessionLifecycleRole = 'player' | 'enemy';
export type RuntimeSessionLifecycleOutcomeKind = 'in_progress' | 'won' | 'lost';
export type RuntimeSessionLifecycleEventKind =
  | 'runtime_lifecycle.enemy_defeated.v0'
  | 'runtime_lifecycle.player_defeated.v0';

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
    'not_demo_local_lifecycle',
  ];
}

export type RuntimeSessionRestartIntentSource = 'hud_menu' | 'programmatic';
export type RuntimeSessionRestartIntentStatus = 'accepted' | 'rejected';
export type RuntimeSessionRestartIntentRejectionReason =
  | 'session_not_terminal'
  | 'session_hash_mismatch'
  | 'invalid_restart_intent';

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
export type RuntimeSessionAutonomousPolicyProposalRejectionReason =
  | 'movement_authority_not_wired'
  | 'policy_source_forbidden_capability'
  | 'invalid_policy_proposal'
  | 'runtime_action_rejected';

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
    'movement_authority_not_wired',
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
  applyCollisionConstrainedCameraInput(
    envelope: CollisionConstrainedCameraInputEnvelope,
  ): RuntimeSessionCameraCollisionInputReceipt;
  submitRuntimeActionIntent(envelope: RuntimeActionIntentEnvelope): RuntimeSessionActionIntentReceipt;
  runAutonomousPolicyTick(input: RuntimeSessionAutonomousPolicyTickInput): RuntimeSessionAutonomousPolicyTickReadout;
  readLifecycleStatus(request?: RuntimeSessionLifecycleStatusRequest): RuntimeSessionLifecycleStatusReadout;
  requestSessionRestart(intent: RuntimeSessionRestartIntent): RuntimeSessionLifecycleRestartReceipt;
  readEncounterDirector(request?: EncounterDirectorReadoutRequest): EncounterDirectorReadout;
  requestEncounterTransition(
    request: EncounterTransitionRequest,
  ): RuntimeSessionEncounterTransitionReceipt;
  readCombatReadout(request?: RuntimeSessionCombatReadoutRequest): CombatRuntimeReadout;
  readCombatFeedbackProjection(
    request?: RuntimeSessionCombatFeedbackProjectionRequest,
  ): CombatFeedbackProjection;
  readGeneratedTunnelReadout(request?: GeneratedTunnelReadoutRequest): GeneratedTunnelReadout;
  readNavProjection(): NavProjectionReadout;
  queryNavPath(request?: NavPathQueryRequest): NavPathReadout;
  readNavPolicyView(): NavPolicyViewReadout;
  requestGeneratedTunnelOperation(
    request: GeneratedTunnelOperationRequest,
  ): RuntimeSessionGeneratedTunnelOperationReceipt;
  readCameraProjection(request: CameraProjectionRequest): RuntimeSessionCameraProjectionReadout;
  readProjection(): RuntimeSessionProjectionSummary;
  readTelemetry(): RuntimeSessionTelemetrySummary;
  restart(): RuntimeSessionRestartResult;
}

export interface RuntimeSessionFacadeOptions {
  readonly bridge?: RuntimeBridge;
}

type RuntimeSessionHashPrimitive = string | number | boolean | null;
type RuntimeSessionHashValue =
  | RuntimeSessionHashPrimitive
  | readonly RuntimeSessionHashValue[]
  | RuntimeSessionHashRecord;
interface RuntimeSessionHashRecord {
  readonly [key: string]: RuntimeSessionHashValue | undefined;
}

interface RuntimeSessionLifecycleState {
  readonly player: RuntimeSessionLifecycleHealthReadout;
  readonly enemy: RuntimeSessionLifecycleHealthReadout;
  readonly terminalEvent: RuntimeSessionLifecycleEventReadout | null;
  readonly revision: number;
}

export function createMockRuntimeSession(options: RuntimeSessionFacadeOptions = {}): RuntimeSessionFacade {
  return new ReferenceRuntimeSessionFacade(options.bridge ?? createMockRuntimeBridge());
}

class ReferenceRuntimeSessionFacade implements RuntimeSessionFacade {
  readonly #bridge: RuntimeBridge;
  #identity: RuntimeSessionIdentity | null = null;
  #engine: EngineHandle | null = null;
  #sequenceId = 0;
  #tick = 0;
  #acceptedCommandCount = 0;
  #rejectedCommandCount = 0;
  #restartCount = 0;
  #lifecycleState: RuntimeSessionLifecycleState = initialRuntimeSessionLifecycleState();
  #encounterState: EncounterDirectorState = initialEncounterDirectorState();
  #replayRecords: RuntimeSessionReplayRecord[] = [];

  constructor(bridge: RuntimeBridge) {
    this.#bridge = bridge;
  }

  initialize(input: RuntimeSessionInitializeInput): RuntimeSessionStateSummary {
    validateInitializeInput(input);
    const engine = this.#bridge.initializeEngine({ seed: input.seed });
    const composition = this.#bridge.loadWorldBundle(input.projectBundle);
    this.#engine = engine;
    this.#identity = {
      sessionId: input.sessionId,
      mode: 'reference',
      seed: input.seed,
      project: input.project,
      projectBundle: input.projectBundle,
      nonClaims: referenceRuntimeSessionNonClaims(),
    };
    this.#sequenceId = 0;
    this.#tick = 0;
    this.#acceptedCommandCount = 0;
    this.#rejectedCommandCount = 0;
    this.#lifecycleState = initialRuntimeSessionLifecycleState();
    this.#encounterState = initialEncounterDirectorState();
    this.#replayRecords = [];
    this.#record('initialize');
    return this.#stateSummary(composition);
  }

  submitCommands(batch: CommandBatch): RuntimeSessionCommandReceipt {
    this.#requireInitialized('submitCommands');
    const before = this.#sessionHash();
    const result = this.#bridge.submitCommands(batch);
    this.#acceptedCommandCount += result.accepted;
    this.#rejectedCommandCount += result.rejected;
    this.#sequenceId += 1;
    this.#record('submitCommands');
    return {
      sequenceId: this.#sequenceId,
      batch,
      result,
      acceptedCommandCount: this.#acceptedCommandCount,
      rejectedCommandCount: this.#rejectedCommandCount,
      sessionHashBefore: before,
      sessionHashAfter: this.#sessionHash(),
    };
  }

  tick(input: RuntimeSessionTickInput = {}): RuntimeSessionTickResult {
    this.#requireInitialized('tick');
    const nextTick = input.tick ?? this.#tick + 1;
    const step = this.#bridge.stepSimulation({ tick: nextTick });
    this.#tick = step.tick;
    this.#sequenceId += 1;
    this.#record('tick');
    return {
      sequenceId: this.#sequenceId,
      tick: this.#tick,
      step,
      composition: this.#bridge.getCompositionStatus(),
      sessionHash: this.#sessionHash(),
    };
  }

  createCamera(request: CameraCreateRequest): RuntimeSessionCameraCreateReceipt {
    this.#requireInitialized('createCamera');
    const snapshot = this.#bridge.createCamera(request);
    this.#sequenceId += 1;
    this.#record('createCamera');
    return {
      sequenceId: this.#sequenceId,
      request,
      snapshot,
      sessionHash: this.#sessionHash(),
    };
  }

  applyFirstPersonCameraInput(envelope: FirstPersonCameraInputEnvelope): RuntimeSessionCameraInputReceipt {
    this.#requireInitialized('applyFirstPersonCameraInput');
    const before = this.#sessionHash();
    const snapshot = this.#bridge.applyFirstPersonCameraInput(envelope);
    this.#sequenceId += 1;
    this.#record('applyFirstPersonCameraInput');
    return {
      sequenceId: this.#sequenceId,
      envelope,
      snapshot,
      sessionHashBefore: before,
      sessionHashAfter: this.#sessionHash(),
    };
  }

  applyCollisionConstrainedCameraInput(
    envelope: CollisionConstrainedCameraInputEnvelope,
  ): RuntimeSessionCameraCollisionInputReceipt {
    this.#requireInitialized('applyCollisionConstrainedCameraInput');
    const before = this.#sessionHash();
    const snapshot = this.#bridge.applyCollisionConstrainedCameraInput(envelope);
    this.#sequenceId += 1;
    this.#record('applyCollisionConstrainedCameraInput');
    return {
      sequenceId: this.#sequenceId,
      envelope,
      snapshot,
      collided: snapshot.collision.collided,
      blockedAxes: snapshot.collision.blockedAxes,
      worldHash: snapshot.collision.worldHash,
      collisionProjectionHash: snapshot.collision.collisionProjectionHash,
      movementHash: snapshot.movementHash,
      sessionHashBefore: before,
      sessionHashAfter: this.#sessionHash(),
    };
  }

  submitRuntimeActionIntent(envelope: RuntimeActionIntentEnvelope): RuntimeSessionActionIntentReceipt {
    this.#requireInitialized('submitRuntimeActionIntent');
    validateRuntimeActionIntentEnvelope(envelope);
    const before = this.#sessionHash();
    this.#sequenceId += 1;
    this.#record('submitRuntimeActionIntent');
    const combatReadout =
      envelope.action === 'primary_fire' && envelope.phase === 'pressed'
        ? GENERATED_TUNNEL_FIRE_HIT_READOUT
        : null;
    const accepted = combatReadout !== null || (envelope.action === 'primary_fire' && envelope.phase === 'released');
    if (combatReadout !== null) {
      this.#applyCombatLifecycleReadout(combatReadout, envelope.tick);
    }
    return {
      sequenceId: this.#sequenceId,
      envelope,
      accepted,
      status: accepted ? 'accepted' : 'unsupported',
      rejection: accepted
        ? null
        : {
            reason: 'combat_runtime_not_wired',
            detail: 'Only primary_fire press/release is wired in the #4051 reference combat slice.',
          },
      combatReadout,
      sessionHashBefore: before,
      sessionHashAfter: this.#sessionHash(),
    };
  }

  runAutonomousPolicyTick(input: RuntimeSessionAutonomousPolicyTickInput): RuntimeSessionAutonomousPolicyTickReadout {
    this.#requireInitialized('runAutonomousPolicyTick');
    validateAutonomousPolicyTickInput(input);

    const sequenceIdBefore = this.#sequenceId;
    const sessionHashBefore = this.#sessionHash();
    const step = this.tick(input.tick === undefined ? {} : { tick: input.tick });
    const navPath = this.queryNavPath({ scenario: input.navScenario ?? 'generated_tunnel_reachable' });
    const navPolicyView: NavPolicyViewReadout = {
      ...this.readNavPolicyView(),
      latestPath: navPath,
    };
    const sourceDiagnostics =
      input.policySource === undefined ? [] : validateEnemyPolicySource(input.policySource);
    const fixture = createGeneratedTunnelEnemyPolicyFixture({
      tick: step.tick,
      nav: navPolicyView,
      target: {
        ...(input.target ?? {}),
        camera: input.targetCamera,
      },
      ...(input.enemy === undefined ? {} : { enemy: input.enemy }),
      ...(input.combat === undefined ? {} : { combat: input.combat }),
    });

    const proposalValidationDiagnostics: RuntimeSessionAutonomousPolicyProposalRejection[] = [];
    const proposalReceipts: RuntimeSessionAutonomousPolicyProposalReceipt[] = [];
    for (const proposal of fixture.frame.proposals) {
      const validation = validateAutonomousPolicyProposal(proposal, step.tick);
      if (validation !== null) {
        proposalValidationDiagnostics.push(validation);
        proposalReceipts.push(rejectedAutonomousPolicyProposalReceipt(proposal, validation));
        continue;
      }

      if (sourceDiagnostics.length > 0) {
        proposalReceipts.push(
          rejectedAutonomousPolicyProposalReceipt(proposal, {
            reason: 'policy_source_forbidden_capability',
            detail: `policy source referenced ${sourceDiagnostics.map((diagnostic) => diagnostic.token).join(', ')}`,
          }),
        );
        continue;
      }

      if (proposal.kind === 'enemy_policy.move_toward_target.v0') {
        proposalReceipts.push(unsupportedAutonomousMovementReceipt(proposal));
        continue;
      }

      const actionReceipt = this.submitRuntimeActionIntent(proposal.intent);
      proposalReceipts.push(runtimeActionReceiptToAutonomousReceipt(proposal, actionReceipt));
    }

    this.#sequenceId += 1;
    this.#record('runAutonomousPolicyTick');

    const telemetry = this.readTelemetry();
    const movementSummary = proposalReceipts.find((receipt) => receipt.movement !== null)?.movement ?? null;
    const combatSummary = proposalReceipts.find((receipt) => receipt.combat !== null)?.combat ?? null;
    const acceptedRuntimeActionCount = proposalReceipts.filter(
      (receipt) => receipt.actionReceipt?.accepted === true,
    ).length;
    const rejectedRuntimeActionCount = proposalReceipts.filter(
      (receipt) => receipt.actionReceipt !== null && receipt.actionReceipt.accepted === false,
    ).length;
    const recordHashes = telemetry.replayRecords.map((record) => record.recordHash);
    const tickHash = stableHash({
      loopId: 'generated_tunnel_enemy_policy_loop.v0',
      tick: step.tick,
      proposalFrameHash: fixture.frame.proposalHash,
      receiptStatuses: proposalReceipts.map((receipt) => receipt.status),
      receiptRejections: proposalReceipts.map((receipt) => receipt.rejection?.reason ?? null),
      navPathHash: navPath.pathHash,
      replayRecordHashes: recordHashes,
      sequenceIdAfter: telemetry.sequenceId,
    });

    return {
      kind: 'runtime_session.autonomous_policy_tick.v0',
      loopId: 'generated_tunnel_enemy_policy_loop.v0',
      sequenceIdBefore,
      sequenceIdAfter: telemetry.sequenceId,
      sessionHashBefore,
      sessionHashAfter: telemetry.sessionHash,
      tick: step.tick,
      step,
      policy: {
        fixtureKind: fixture.kind,
        proposalFrame: fixture.frame,
        sourceChecked: input.policySource !== undefined,
        sourceDiagnostics,
        proposalValidationDiagnostics,
      },
      nav: {
        projectionHash: navPath.projection.projectionHash,
        pathHash: navPath.pathHash,
        outcome: navPath.outcome,
        visited: navPath.visited,
        pathLength: navPath.path.length,
      },
      proposalReceipts,
      proposalSummary: {
        acceptedProposalCount: proposalReceipts.filter((receipt) => receipt.status === 'accepted').length,
        rejectedProposalCount: proposalReceipts.filter((receipt) => receipt.status === 'rejected').length,
        unsupportedProposalCount: proposalReceipts.filter((receipt) => receipt.status === 'unsupported').length,
      },
      commandSummary: {
        acceptedCommandCount: telemetry.acceptedCommandCount,
        rejectedCommandCount: telemetry.rejectedCommandCount,
        acceptedRuntimeActionCount,
        rejectedRuntimeActionCount,
      },
      movementSummary,
      combatSummary,
      replay: {
        recordCount: telemetry.replayRecords.length,
        lastRecordKind: telemetry.replayRecords.at(-1)?.kind ?? null,
        recordHashes,
      },
      tickHash,
      nonClaims: [
        'not_generic_event_bus',
        'not_behavior_tree',
        'not_demo_local_authority',
        'movement_authority_not_wired',
      ],
    };
  }

  readLifecycleStatus(request: RuntimeSessionLifecycleStatusRequest = {}): RuntimeSessionLifecycleStatusReadout {
    const identity = this.#requireInitialized('readLifecycleStatus');
    validateLifecycleStatusRequest(request);
    const scenario = request.scenario ?? 'current_session';
    const state =
      scenario === 'generated_tunnel_enemy_defeated'
        ? generatedTunnelEnemyDefeatedLifecycleState()
        : scenario === 'generated_tunnel_player_defeated'
          ? generatedTunnelPlayerDefeatedLifecycleState()
          : this.#lifecycleState;
    return lifecycleStatusReadout({
      scenario,
      state,
      identity,
      sequenceId: this.#sequenceId,
      tick: this.#tick,
      restartCount: this.#restartCount,
      sessionHash: this.#sessionHash(),
    });
  }

  requestSessionRestart(intent: RuntimeSessionRestartIntent): RuntimeSessionLifecycleRestartReceipt {
    this.#requireInitialized('requestSessionRestart');
    validateRestartIntent(intent);
    const statusBefore = this.readLifecycleStatus();
    const sessionHashBefore = this.#sessionHash();

    if (intent.expectedSessionHash !== undefined && intent.expectedSessionHash !== sessionHashBefore) {
      return this.#rejectSessionRestart(intent, statusBefore, sessionHashBefore, {
        reason: 'session_hash_mismatch',
        detail: 'Restart intent expectedSessionHash did not match the current RuntimeSession hash.',
      });
    }
    if (intent.requireTerminal === true && !statusBefore.outcome.terminal) {
      return this.#rejectSessionRestart(intent, statusBefore, sessionHashBefore, {
        reason: 'session_not_terminal',
        detail: 'Restart intent required a terminal win/loss lifecycle state.',
      });
    }

    const restart = this.restart();
    const statusAfter = this.readLifecycleStatus();
    return {
      kind: 'runtime_session.restart_receipt.v0',
      sequenceId: restart.sequenceId,
      intent,
      accepted: true,
      status: 'accepted',
      rejection: null,
      statusBefore,
      statusAfter,
      restart,
      sessionHashBefore,
      sessionHashAfter: restart.sessionHash,
      resetHash: statusAfter.fixture.resetHash,
    };
  }

  readEncounterDirector(request: EncounterDirectorReadoutRequest = {}): EncounterDirectorReadout {
    const identity = this.#requireInitialized('readEncounterDirector');
    validateEncounterDirectorReadoutRequest(request);
    const lifecycle = this.#encounterLifecycleFromScenario(request.lifecycleScenario);
    return buildEncounterDirectorReadout({
      state: this.#encounterState,
      sequenceId: this.#sequenceId,
      tick: this.#tick,
      sessionSeed: identity.seed,
      sessionHash: this.#sessionHash(),
      lifecycle,
    });
  }

  requestEncounterTransition(
    request: EncounterTransitionRequest,
  ): RuntimeSessionEncounterTransitionReceipt {
    this.#requireInitialized('requestEncounterTransition');
    const sessionHashBefore = this.#sessionHash();
    const validationRejection = validateEncounterTransitionRequest(request);
    const lifecycle =
      validationRejection === undefined
        ? this.#encounterLifecycleFromScenario(request.lifecycleScenario)
        : this.#encounterLifecycleFromScenario();
    const identity = this.#requireInitialized('requestEncounterTransition');
    const before = buildEncounterDirectorReadout({
      state: this.#encounterState,
      sequenceId: this.#sequenceId,
      tick: this.#tick,
      sessionSeed: identity.seed,
      sessionHash: sessionHashBefore,
      lifecycle,
    });
    const result =
      validationRejection === undefined
        ? transitionEncounterDirectorState({
            state: this.#encounterState,
            action: request.action,
            lifecycle,
          })
        : {
            accepted: false,
            state: this.#encounterState,
            rejectionReason: validationRejection,
          };

    if (result.accepted) {
      this.#encounterState = result.state;
    }

    this.#sequenceId += 1;
    this.#record('requestEncounterTransition');

    const after = buildEncounterDirectorReadout({
      state: this.#encounterState,
      sequenceId: this.#sequenceId,
      tick: this.#tick,
      sessionSeed: identity.seed,
      sessionHash: this.#sessionHash(),
      lifecycle,
    });

    return buildEncounterTransitionReceipt({
      request,
      sequenceId: this.#sequenceId,
      before,
      after,
      result,
      sessionHashBefore,
      sessionHashAfter: this.#sessionHash(),
    });
  }

  readCombatReadout(request: RuntimeSessionCombatReadoutRequest = {}): CombatRuntimeReadout {
    this.#requireInitialized('readCombatReadout');
    const scenario = request.scenario ?? 'generated_tunnel_fire_hit';
    switch (scenario) {
      case 'generated_tunnel_fire_hit':
        return GENERATED_TUNNEL_FIRE_HIT_READOUT;
      case 'generated_tunnel_geometry_blocked_miss':
        return GENERATED_TUNNEL_FIRE_MISS_READOUT;
      default:
        throw new RuntimeBridgeError('invalid_input', 'unknown combat readout scenario');
    }
  }

  readCombatFeedbackProjection(
    request: RuntimeSessionCombatFeedbackProjectionRequest = {},
  ): CombatFeedbackProjection {
    this.#requireInitialized('readCombatFeedbackProjection');
    const combatReadout = this.readCombatReadout(request);
    const cameraProjection =
      request.camera === undefined
        ? null
        : this.readCameraProjection({
            camera: request.camera,
            viewport: request.viewport ?? null,
          }).snapshot;
    return buildCombatFeedbackProjection({
      sequenceId: this.#sequenceId,
      ...defaultCombatFeedbackIntent({
        camera: request.camera ?? cameraHandle(0),
        tick: combatReadoutTick(combatReadout),
      }),
      combatReadout,
      camera: cameraProjection,
    });
  }

  readNavProjection(): NavProjectionReadout {
    this.#requireInitialized('readNavProjection');
    return GENERATED_TUNNEL_NAV_PROJECTION;
  }

  queryNavPath(request: NavPathQueryRequest = {}): NavPathReadout {
    this.#requireInitialized('queryNavPath');
    validateNavPathQueryRequest(request);
    return request.scenario === 'generated_tunnel_no_path' ? GENERATED_TUNNEL_NO_PATH : GENERATED_TUNNEL_REACHABLE_PATH;
  }

  readNavPolicyView(): NavPolicyViewReadout {
    this.#requireInitialized('readNavPolicyView');
    return GENERATED_TUNNEL_NAV_POLICY_VIEW;
  }

  readGeneratedTunnelReadout(request: GeneratedTunnelReadoutRequest = {}): GeneratedTunnelReadout {
    this.#requireInitialized('readGeneratedTunnelReadout');
    validateGeneratedTunnelReadoutRequest(request);
    return TINY_GENERATED_TUNNEL_READOUT;
  }

  requestGeneratedTunnelOperation(
    request: GeneratedTunnelOperationRequest,
  ): RuntimeSessionGeneratedTunnelOperationReceipt {
    this.#requireInitialized('requestGeneratedTunnelOperation');
    validateGeneratedTunnelOperationRequest(request);
    const before = this.#sessionHash();
    this.#sequenceId += 1;
    this.#record('requestGeneratedTunnelOperation');
    return {
      sequenceId: this.#sequenceId,
      request,
      operation: request.operation,
      status: 'unsupported',
      reason: 'generated_tunnel_operation_not_wired',
      detail: 'Generated tunnel regenerate/apply operations are not runtime commands in this slice.',
      sessionHashBefore: before,
      sessionHashAfter: this.#sessionHash(),
    };
  }

  readCameraProjection(request: CameraProjectionRequest): RuntimeSessionCameraProjectionReadout {
    this.#requireInitialized('readCameraProjection');
    const snapshot = this.#bridge.readCameraProjection(request);
    return {
      sequenceId: this.#sequenceId,
      request,
      snapshot,
      projectionHash: snapshot.projectionHash,
    };
  }

  readProjection(): RuntimeSessionProjectionSummary {
    this.#requireInitialized('readProjection');
    const cursor = frameCursor(this.#sequenceId);
    const frame = this.#bridge.readRenderDiffs(cursor);
    const composition = this.#bridge.getCompositionStatus();
    return {
      sequenceId: this.#sequenceId,
      cursor,
      frame,
      composition,
      renderDiffCount: frame.ops.length,
      projectionHash: stableHash({
        sequenceId: this.#sequenceId,
        composition: compositionHashRecord(composition),
        frame: renderFrameHashRecord(frame),
      }),
    };
  }

  readTelemetry(): RuntimeSessionTelemetrySummary {
    this.#requireInitialized('readTelemetry');
    return {
      sequenceId: this.#sequenceId,
      tick: this.#tick,
      composition: this.#bridge.getCompositionStatus(),
      acceptedCommandCount: this.#acceptedCommandCount,
      rejectedCommandCount: this.#rejectedCommandCount,
      restartCount: this.#restartCount,
      sessionHash: this.#sessionHash(),
      replayRecords: [...this.#replayRecords],
    };
  }

  restart(): RuntimeSessionRestartResult {
    const identity = this.#requireInitialized('restart');
    this.#bridge.unloadWorld();
    this.#bridge.initializeEngine({ seed: identity.seed });
    const composition = this.#bridge.loadWorldBundle(identity.projectBundle);
    this.#sequenceId += 1;
    this.#tick = 0;
    this.#acceptedCommandCount = 0;
    this.#rejectedCommandCount = 0;
    this.#lifecycleState = initialRuntimeSessionLifecycleState();
    this.#encounterState = initialEncounterDirectorState();
    this.#restartCount += 1;
    this.#record('restart');
    return {
      sequenceId: this.#sequenceId,
      tick: this.#tick,
      composition,
      restartCount: this.#restartCount,
      sessionHash: this.#sessionHash(),
    };
  }

  #rejectSessionRestart(
    intent: RuntimeSessionRestartIntent,
    statusBefore: RuntimeSessionLifecycleStatusReadout,
    sessionHashBefore: string,
    rejection: RuntimeSessionRestartIntentRejection,
  ): RuntimeSessionLifecycleRestartReceipt {
    this.#sequenceId += 1;
    this.#record('requestSessionRestart');
    const statusAfter = this.readLifecycleStatus();
    return {
      kind: 'runtime_session.restart_receipt.v0',
      sequenceId: this.#sequenceId,
      intent,
      accepted: false,
      status: 'rejected',
      rejection,
      statusBefore,
      statusAfter,
      restart: null,
      sessionHashBefore,
      sessionHashAfter: this.#sessionHash(),
      resetHash: statusAfter.fixture.resetHash,
    };
  }

  #applyCombatLifecycleReadout(readout: CombatRuntimeReadout, tick: number): void {
    const defeated = readout.health.find((health) => health.dead);
    if (defeated === undefined || this.#lifecycleState.enemy.dead) {
      return;
    }
    const enemy = lifecycleHealth(defeated.entity, defeated.current, defeated.max, defeated.dead);
    const event = lifecycleEvent('runtime_lifecycle.enemy_defeated.v0', enemy.entity, tick, 'combat_health_zero');
    this.#lifecycleState = {
      player: this.#lifecycleState.player,
      enemy,
      terminalEvent: event,
      revision: this.#lifecycleState.revision + 1,
    };
    this.#record('lifecycleDeath');
  }

  #encounterLifecycleFromScenario(scenario?: EncounterLifecycleScenario): EncounterLifecycleInput {
    const lifecycleScenario =
      scenario === undefined || scenario === 'active' ? 'current_session' : scenario;
    return lifecycleStatusToEncounterLifecycle(
      this.readLifecycleStatus({ scenario: lifecycleScenario }),
    );
  }

  #requireInitialized(operation: string): RuntimeSessionIdentity {
    if (this.#identity === null || this.#engine === null) {
      throw new RuntimeBridgeError('not_initialized', `${operation} before RuntimeSession initialize`);
    }
    return this.#identity;
  }

  #stateSummary(composition: CompositionStatus): RuntimeSessionStateSummary {
    const identity = this.#requireInitialized('stateSummary');
    return {
      identity,
      engine: this.#engine as EngineHandle,
      composition,
      sequenceId: this.#sequenceId,
      tick: this.#tick,
      sessionHash: this.#sessionHash(),
    };
  }

  #record(kind: RuntimeSessionReplayRecord['kind']): void {
    this.#replayRecords.push({
      sequenceId: this.#sequenceId,
      kind,
      recordHash: stableHash({
        kind,
        sequenceId: this.#sequenceId,
        tick: this.#tick,
        acceptedCommandCount: this.#acceptedCommandCount,
        rejectedCommandCount: this.#rejectedCommandCount,
        restartCount: this.#restartCount,
        lifecycle: lifecycleStateHashRecord(this.#lifecycleState),
        encounter: encounterStateHashRecord(this.#encounterState),
        composition: compositionHashRecord(this.#bridge.getCompositionStatus()),
      }),
    });
  }

  #sessionHash(): string {
    return stableHash({
      identity: this.#identity === null ? null : identityHashRecord(this.#identity),
      sequenceId: this.#sequenceId,
      tick: this.#tick,
      acceptedCommandCount: this.#acceptedCommandCount,
      rejectedCommandCount: this.#rejectedCommandCount,
      restartCount: this.#restartCount,
      lifecycle: this.#identity === null ? null : lifecycleStateHashRecord(this.#lifecycleState),
      encounter: this.#identity === null ? null : encounterStateHashRecord(this.#encounterState),
      composition: this.#identity === null ? null : compositionHashRecord(this.#bridge.getCompositionStatus()),
    });
  }
}

function initialRuntimeSessionLifecycleState(): RuntimeSessionLifecycleState {
  return {
    player: lifecycleHealth(10, 100, 100, false),
    enemy: lifecycleHealth(20, 40, 40, false),
    terminalEvent: null,
    revision: 0,
  };
}

function generatedTunnelEnemyDefeatedLifecycleState(): RuntimeSessionLifecycleState {
  const enemy = lifecycleHealth(20, 0, 40, true);
  return {
    player: lifecycleHealth(10, 100, 100, false),
    enemy,
    terminalEvent: lifecycleEvent('runtime_lifecycle.enemy_defeated.v0', enemy.entity, 7, 'combat_health_zero'),
    revision: 1,
  };
}

function generatedTunnelPlayerDefeatedLifecycleState(): RuntimeSessionLifecycleState {
  const player = lifecycleHealth(10, 0, 100, true);
  return {
    player,
    enemy: lifecycleHealth(20, 40, 40, false),
    terminalEvent: lifecycleEvent('runtime_lifecycle.player_defeated.v0', player.entity, 11, 'fixture_player_damage'),
    revision: 1,
  };
}

function lifecycleHealth(
  entity: number,
  current: number,
  max: number,
  dead: boolean,
): RuntimeSessionLifecycleHealthReadout {
  const healthRecord = {
    entity,
    current,
    max,
    dead,
  };
  return {
    ...healthRecord,
    healthHash: stableHash(healthRecord),
  };
}

function lifecycleEvent(
  kind: RuntimeSessionLifecycleEventKind,
  entity: number,
  tick: number,
  reason: RuntimeSessionLifecycleEventReadout['reason'],
): RuntimeSessionLifecycleEventReadout {
  return {
    kind,
    entity,
    tick,
    reason,
    eventHash: stableHash({
      kind,
      entity,
      tick,
      reason,
    }),
  };
}

function lifecycleStatusReadout(input: {
  readonly scenario: RuntimeSessionLifecycleScenario;
  readonly state: RuntimeSessionLifecycleState;
  readonly identity: RuntimeSessionIdentity;
  readonly sequenceId: number;
  readonly tick: number;
  readonly restartCount: number;
  readonly sessionHash: string;
}): RuntimeSessionLifecycleStatusReadout {
  const outcome = lifecycleOutcome(input.state);
  const lifecycleHash = stableHash(lifecycleStateHashRecord(input.state));
  const resetHash = runtimeSessionResetHash(input.identity);
  return {
    kind: 'runtime_session.lifecycle_status.v0',
    scenario: input.scenario,
    sequenceId: input.sequenceId,
    tick: input.tick,
    sessionHash: input.sessionHash,
    player: {
      role: 'player',
      health: input.state.player,
      dead: input.state.player.dead,
    },
    enemy: {
      role: 'enemy',
      health: input.state.enemy,
      dead: input.state.enemy.dead,
    },
    outcome,
    restart: {
      eligible: true,
      intentKind: 'runtime.restart_session_intent',
      reason: 'always_resettable_reference_fixture',
    },
    events: input.state.terminalEvent === null ? [] : [input.state.terminalEvent],
    fixture: {
      seed: input.identity.seed,
      sceneId: input.identity.projectBundle.sceneId,
      bundleSchemaVersion: input.identity.projectBundle.bundleSchemaVersion,
      protocolVersion: input.identity.projectBundle.protocolVersion,
      resetHash,
    },
    hashes: {
      lifecycleHash,
      playerHealthHash: input.state.player.healthHash,
      enemyHealthHash: input.state.enemy.healthHash,
      replayHash: stableHash({
        lifecycleHash,
        resetHash,
        restartCount: input.restartCount,
        eventHash: input.state.terminalEvent?.eventHash ?? null,
      }),
    },
    nonClaims: [
      'not_save_load_persistence',
      'not_ui_authority',
      'not_demo_local_lifecycle',
    ],
  };
}

function lifecycleOutcome(state: RuntimeSessionLifecycleState): RuntimeSessionLifecycleStatusReadout['outcome'] {
  if (state.player.dead) {
    return {
      kind: 'lost',
      terminal: true,
      reason: 'player_defeated',
      label: 'Player defeated',
    };
  }
  if (state.enemy.dead) {
    return {
      kind: 'won',
      terminal: true,
      reason: 'enemy_defeated',
      label: 'Enemy defeated',
    };
  }
  return {
    kind: 'in_progress',
    terminal: false,
    reason: 'none',
    label: 'In progress',
  };
}

function lifecycleStatusToEncounterLifecycle(
  status: RuntimeSessionLifecycleStatusReadout,
): EncounterLifecycleInput {
  return {
    outcomeKind: status.outcome.kind,
    terminal: status.outcome.terminal,
    enemyDead: status.enemy.dead,
    playerDead: status.player.dead,
    lifecycleHash: status.hashes.lifecycleHash,
  };
}

function validateLifecycleStatusRequest(request: RuntimeSessionLifecycleStatusRequest): void {
  if (
    request.scenario !== undefined &&
    request.scenario !== 'current_session' &&
    request.scenario !== 'generated_tunnel_enemy_defeated' &&
    request.scenario !== 'generated_tunnel_player_defeated'
  ) {
    throw new RuntimeBridgeError('invalid_input', 'unknown lifecycle status scenario');
  }
}

function validateRestartIntent(intent: RuntimeSessionRestartIntent): void {
  if (intent === null || typeof intent !== 'object') {
    throw new RuntimeBridgeError('invalid_input', 'restart intent must be an object');
  }
  if (intent.kind !== 'runtime.restart_session_intent') {
    throw new RuntimeBridgeError('invalid_input', 'restart intent kind must be runtime.restart_session_intent');
  }
  if (intent.source !== 'hud_menu' && intent.source !== 'programmatic') {
    throw new RuntimeBridgeError('invalid_input', 'restart intent source is unsupported');
  }
  if (intent.requireTerminal !== undefined && typeof intent.requireTerminal !== 'boolean') {
    throw new RuntimeBridgeError('invalid_input', 'restart intent requireTerminal must be boolean');
  }
  if (intent.expectedSessionHash !== undefined && intent.expectedSessionHash.trim().length === 0) {
    throw new RuntimeBridgeError('invalid_input', 'restart intent expectedSessionHash must be non-empty when provided');
  }
}

function validateAutonomousPolicyTickInput(input: RuntimeSessionAutonomousPolicyTickInput): void {
  if (input === null || typeof input !== 'object') {
    throw new RuntimeBridgeError('invalid_input', 'autonomous policy tick input must be an object');
  }
  if (!Number.isSafeInteger(input.targetCamera) || input.targetCamera < 0) {
    throw new RuntimeBridgeError('invalid_input', 'autonomous policy targetCamera must be a non-negative camera handle');
  }
  if (input.tick !== undefined && (!Number.isSafeInteger(input.tick) || input.tick < 0)) {
    throw new RuntimeBridgeError('invalid_input', 'autonomous policy tick must be a non-negative safe integer');
  }
  if (input.policySource !== undefined && typeof input.policySource !== 'string') {
    throw new RuntimeBridgeError('invalid_input', 'autonomous policy source must be a string');
  }
  if (
    input.navScenario !== undefined &&
    input.navScenario !== 'generated_tunnel_reachable' &&
    input.navScenario !== 'generated_tunnel_no_path'
  ) {
    throw new RuntimeBridgeError('invalid_input', 'unknown autonomous policy nav scenario');
  }
}

function validateAutonomousPolicyProposal(
  proposal: EnemyPolicyProposal,
  tick: number,
): RuntimeSessionAutonomousPolicyProposalRejection | null {
  if (proposal.authority !== 'rust_runtime_must_validate') {
    return invalidAutonomousPolicyProposal('policy proposal authority must require Rust runtime validation');
  }
  if (proposal.actor.trim().length === 0 || proposal.target.trim().length === 0) {
    return invalidAutonomousPolicyProposal('policy proposal actor and target must be non-empty');
  }

  if (proposal.kind === 'enemy_policy.move_toward_target.v0') {
    if (!isEnemyPolicyVec3(proposal.from)) {
      return invalidAutonomousPolicyProposal('movement proposal from position must be a finite vec3');
    }
    if (proposal.nextWaypoint === null || !isEnemyPolicyVec3(proposal.nextWaypoint)) {
      return invalidAutonomousPolicyProposal('movement proposal must include a finite next waypoint');
    }
    if (proposal.pathHash.trim().length === 0) {
      return invalidAutonomousPolicyProposal('movement proposal path hash must be non-empty');
    }
    return null;
  }

  if (proposal.intent.kind !== 'runtime_action_intent.v0') {
    return invalidAutonomousPolicyProposal('fire proposal intent kind must be runtime_action_intent.v0');
  }
  if (proposal.intent.action !== 'primary_fire') {
    return invalidAutonomousPolicyProposal('fire proposal intent action must be primary_fire');
  }
  if (proposal.intent.phase !== 'pressed' || !proposal.intent.pressed) {
    return invalidAutonomousPolicyProposal('fire proposal intent must be a pressed primary fire action');
  }
  if (proposal.intent.source !== 'enemy_policy') {
    return invalidAutonomousPolicyProposal('fire proposal intent source must be enemy_policy');
  }
  if (proposal.intent.tick !== tick) {
    return invalidAutonomousPolicyProposal('fire proposal intent tick must match the autonomous policy tick');
  }
  if (!Number.isSafeInteger(proposal.intent.camera) || proposal.intent.camera < 0) {
    return invalidAutonomousPolicyProposal('fire proposal intent camera must be a non-negative camera handle');
  }
  if (!Number.isFinite(proposal.distanceUnits) || proposal.distanceUnits < 0) {
    return invalidAutonomousPolicyProposal('fire proposal distance must be finite and non-negative');
  }
  return null;
}

function invalidAutonomousPolicyProposal(detail: string): RuntimeSessionAutonomousPolicyProposalRejection {
  return {
    reason: 'invalid_policy_proposal',
    detail,
  };
}

function isEnemyPolicyVec3(value: EnemyPolicyVec3): boolean {
  return value.length === 3 && value.every((component) => Number.isFinite(component));
}

function rejectedAutonomousPolicyProposalReceipt(
  proposal: EnemyPolicyProposal,
  rejection: RuntimeSessionAutonomousPolicyProposalRejection,
): RuntimeSessionAutonomousPolicyProposalReceipt {
  return {
    proposalKind: proposal.kind,
    actor: proposal.actor,
    target: proposal.target,
    accepted: false,
    status: 'rejected',
    rejection,
    movement: null,
    actionReceipt: null,
    combat: null,
  };
}

function unsupportedAutonomousMovementReceipt(
  proposal: Extract<EnemyPolicyProposal, { readonly kind: 'enemy_policy.move_toward_target.v0' }>,
): RuntimeSessionAutonomousPolicyProposalReceipt {
  const rejection: RuntimeSessionAutonomousPolicyProposalRejection = {
    reason: 'movement_authority_not_wired',
    detail: 'Enemy movement proposals are exposed for Rust runtime validation; movement authority is not wired yet.',
  };
  return {
    proposalKind: proposal.kind,
    actor: proposal.actor,
    target: proposal.target,
    accepted: false,
    status: 'unsupported',
    rejection,
    movement: {
      status: 'unsupported',
      actor: proposal.actor,
      target: proposal.target,
      from: proposal.from,
      nextWaypoint: proposal.nextWaypoint,
      pathHash: proposal.pathHash,
      reason: 'movement_authority_not_wired',
    },
    actionReceipt: null,
    combat: null,
  };
}

function runtimeActionReceiptToAutonomousReceipt(
  proposal: Extract<EnemyPolicyProposal, { readonly kind: 'enemy_policy.primary_fire_intent.v0' }>,
  actionReceipt: RuntimeSessionActionIntentReceipt,
): RuntimeSessionAutonomousPolicyProposalReceipt {
  const status: RuntimeSessionAutonomousPolicyProposalStatus = actionReceipt.accepted ? 'accepted' : 'rejected';
  const rejection: RuntimeSessionAutonomousPolicyProposalRejection | null = actionReceipt.accepted
    ? null
    : {
        reason: 'runtime_action_rejected',
        detail: actionReceipt.rejection?.detail ?? 'Runtime action intent was not accepted.',
      };
  return {
    proposalKind: proposal.kind,
    actor: proposal.actor,
    target: proposal.target,
    accepted: actionReceipt.accepted,
    status,
    rejection,
    movement: null,
    actionReceipt,
    combat: {
      status,
      action: actionReceipt.envelope.action,
      outcome: actionReceipt.combatReadout?.outcome ?? null,
      healthHash: actionReceipt.combatReadout?.healthHash ?? null,
      replayHash: actionReceipt.combatReadout?.replayHash ?? null,
    },
  };
}

function validateInitializeInput(input: RuntimeSessionInitializeInput): void {
  if (input.sessionId.trim().length === 0) {
    throw new RuntimeBridgeError('invalid_input', 'sessionId must be non-empty');
  }
  if (input.project.gameId.trim().length === 0) {
    throw new RuntimeBridgeError('invalid_input', 'project.gameId must be non-empty');
  }
  if (input.project.workspaceId.trim().length === 0) {
    throw new RuntimeBridgeError('invalid_input', 'project.workspaceId must be non-empty');
  }
  if (!Number.isSafeInteger(input.seed) || input.seed < 0) {
    throw new RuntimeBridgeError('invalid_input', 'seed must be a non-negative safe integer');
  }
}

function validateRuntimeActionIntentEnvelope(envelope: RuntimeActionIntentEnvelope): void {
  if (envelope.kind !== 'runtime_action_intent.v0') {
    throw new RuntimeBridgeError('invalid_input', 'runtime action intent kind must be runtime_action_intent.v0');
  }
  if (envelope.action !== 'primary_fire' && envelope.action !== 'use') {
    throw new RuntimeBridgeError('invalid_input', 'runtime action intent action is unsupported');
  }
  if (envelope.phase !== 'pressed' && envelope.phase !== 'released') {
    throw new RuntimeBridgeError('invalid_input', 'runtime action intent phase is unsupported');
  }
  if (
    envelope.source !== 'browser_fps_pointer' &&
    envelope.source !== 'programmatic' &&
    envelope.source !== 'enemy_policy'
  ) {
    throw new RuntimeBridgeError('invalid_input', 'runtime action intent source is unsupported');
  }
  if (!Number.isSafeInteger(envelope.tick) || envelope.tick < 0) {
    throw new RuntimeBridgeError('invalid_input', 'runtime action intent tick must be a non-negative safe integer');
  }
  if (envelope.phase === 'pressed' && !envelope.pressed) {
    throw new RuntimeBridgeError('invalid_input', 'pressed runtime action intent must report pressed=true');
  }
  if (envelope.phase === 'released' && envelope.pressed) {
    throw new RuntimeBridgeError('invalid_input', 'released runtime action intent must report pressed=false');
  }
}

function combatReadoutTick(readout: CombatRuntimeReadout): number {
  const fireEvent = readout.events.find(
    (event) => event.kind === 'fire_hit' || event.kind === 'fire_missed',
  );
  return fireEvent?.tick ?? 0;
}

function validateGeneratedTunnelReadoutRequest(request: GeneratedTunnelReadoutRequest): void {
  if (request.presetId !== undefined && request.presetId !== 'tiny-enclosed') {
    throw new RuntimeBridgeError('invalid_input', 'only the tiny-enclosed generated tunnel readout is available');
  }
  if (request.seed !== undefined && request.seed !== 17) {
    throw new RuntimeBridgeError('invalid_input', 'only seed 17 generated tunnel fixture readout is available');
  }
}

function validateGeneratedTunnelOperationRequest(request: GeneratedTunnelOperationRequest): void {
  if (request.operation !== 'regenerate' && request.operation !== 'apply_to_runtime_world') {
    throw new RuntimeBridgeError('invalid_input', 'generated tunnel operation is unsupported');
  }
  validateGeneratedTunnelReadoutRequest(request);
}

function validateNavPathQueryRequest(request: NavPathQueryRequest): void {
  if (
    request.scenario !== undefined &&
    request.scenario !== 'generated_tunnel_reachable' &&
    request.scenario !== 'generated_tunnel_no_path'
  ) {
    throw new RuntimeBridgeError('invalid_input', 'unknown nav path scenario');
  }
  if (request.maxVisited !== undefined && (!Number.isSafeInteger(request.maxVisited) || request.maxVisited <= 0)) {
    throw new RuntimeBridgeError('invalid_input', 'nav path maxVisited must be a positive safe integer');
  }
}

function referenceRuntimeSessionNonClaims(): readonly RuntimeSessionNonClaim[] {
  return [
    'not_native_runtime',
    'not_raw_state_store',
    'not_arbitrary_json_bridge',
    'not_gameplay_loop',
    'not_renderer',
  ];
}

function identityHashRecord(identity: RuntimeSessionIdentity): RuntimeSessionHashRecord {
  return {
    sessionId: identity.sessionId,
    mode: identity.mode,
    seed: identity.seed,
    project: {
      gameId: identity.project.gameId,
      workspaceId: identity.project.workspaceId,
    },
    projectBundle: projectBundleHashRecord(identity.projectBundle),
    nonClaims: identity.nonClaims,
  };
}

function runtimeSessionResetHash(identity: RuntimeSessionIdentity): string {
  return stableHash({
    seed: identity.seed,
    projectBundle: projectBundleHashRecord(identity.projectBundle),
    lifecycle: lifecycleStateHashRecord(initialRuntimeSessionLifecycleState()),
    encounter: encounterStateHashRecord(initialEncounterDirectorState()),
  });
}

function encounterStateHashRecord(state: EncounterDirectorState): RuntimeSessionHashRecord {
  return {
    presetId: state.presetId,
    status: state.status,
    spawnedEnemyIds: state.spawnedEnemyIds,
    defeatedEnemyIds: state.defeatedEnemyIds,
    revision: state.revision,
    lastTransition: state.lastTransition,
  };
}

function lifecycleStateHashRecord(state: RuntimeSessionLifecycleState): RuntimeSessionHashRecord {
  return {
    player: lifecycleHealthHashRecord(state.player),
    enemy: lifecycleHealthHashRecord(state.enemy),
    terminalEventHash: state.terminalEvent?.eventHash ?? null,
    revision: state.revision,
  };
}

function lifecycleHealthHashRecord(health: RuntimeSessionLifecycleHealthReadout): RuntimeSessionHashRecord {
  return {
    entity: health.entity,
    current: health.current,
    max: health.max,
    dead: health.dead,
  };
}

function projectBundleHashRecord(projectBundle: WorldLoadRequest): RuntimeSessionHashRecord {
  return {
    bundleSchemaVersion: projectBundle.bundleSchemaVersion,
    protocolVersion: projectBundle.protocolVersion,
    sceneId: projectBundle.sceneId,
  };
}

function compositionHashRecord(composition: CompositionStatus): RuntimeSessionHashRecord {
  return {
    loadedWorld: composition.loadedWorld,
    fatalCount: composition.fatalCount,
    totalCount: composition.totalCount,
    blocksLoad: composition.blocksLoad,
  };
}

function renderFrameHashRecord(frame: RenderFrameDiff): RuntimeSessionHashRecord {
  return {
    opCount: frame.ops.length,
    opKinds: frame.ops.map((op) => op.op),
  };
}

function stableHash(value: RuntimeSessionHashValue | undefined): string {
  return `fnv1a64:${fnv1a64(stableStringify(value))}`;
}

function stableStringify(value: RuntimeSessionHashValue | undefined): string {
  if (value === undefined) {
    return 'undefined';
  }
  if (value === null || typeof value !== 'object') {
    return JSON.stringify(value);
  }
  if (Array.isArray(value)) {
    return `[${value.map((entry) => stableStringify(entry)).join(',')}]`;
  }
  const record = value as RuntimeSessionHashRecord;
  return `{${Object.keys(record)
    .sort()
    .map((key) => `${JSON.stringify(key)}:${stableStringify(record[key])}`)
    .join(',')}}`;
}

function fnv1a64(text: string): string {
  let hash = 0xcbf29ce484222325n;
  const prime = 0x100000001b3n;
  const mask = 0xffffffffffffffffn;
  for (let index = 0; index < text.length; index += 1) {
    hash ^= BigInt(text.charCodeAt(index));
    hash = (hash * prime) & mask;
  }
  return hash.toString(16).padStart(16, '0');
}
