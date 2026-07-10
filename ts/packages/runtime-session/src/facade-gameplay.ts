import type {
  AnimatedMeshAsset,
  AnimatedMeshPlaybackCommand,
  CameraCollisionSnapshot,
  CameraCreateRequest,
  CameraHandle,
  CameraProjectionRequest,
  CameraProjectionSnapshot,
  CameraSnapshot,
  CollisionAxis,
  CollisionConstrainedCameraInputEnvelope,
  FirstPersonCameraInputEnvelope,
  GameExtensionHookReceipt,
  GameExtensionReplayEvidence,
  GameRuleCatalog,
  GameRuleResolutionReceipt,
  GameRuleResolutionRequest,
  RenderFrameDiff,
  WeaponEffectHookRequest,
} from '@asha/contracts';
import type { CombatReadoutScenario, CombatRuntimeReadout } from './combat-readout.js';
import type {
  EnemyPolicyActorView,
  EnemyPolicyCombatView,
  EnemyPolicyProposal,
  EnemyPolicyProposalFrame,
  EnemyPolicySourceDiagnostic,
  EnemyPolicyTargetView,
  EnemyPolicyVec3,
} from './enemy-policy.js';
import type {
  GeneratedTunnelOperationReceipt,
  GeneratedTunnelOperationRequest,
} from './generated-tunnel.js';
import type { NavPathReadout, NavPathScenario } from './nav-readout.js';
import type {
  RuntimeActionIntentEnvelope,
  RuntimeActionIntentRejection,
  RuntimeActionIntentStatus,
} from './runtime-action.js';
import type { RuntimeSessionLifecycleState } from './facade-lifecycle.js';
import type { RuntimeSessionReplayRecord, RuntimeSessionTickResult } from './facade-core.js';
import type {
  EnemyDirectNavMovementResult,
  FpsPrimaryFireRequest,
  FpsPrimaryFireResult,
  GameRuleCatalogValidationReceipt,
} from './transport-contracts.js';

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
  readonly transformHash: string | null;
  readonly authoritySource: EnemyDirectNavMovementResult['authoritySource'] | null;
  readonly authorityTransport: EnemyDirectNavMovementResult['authorityTransport'] | null;
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
  readonly collisionSourceHash: string;
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
  ];
}

export interface RuntimeSessionCombatReadoutRequest {
  readonly scenario?: CombatReadoutScenario;
}

export interface RuntimeSessionCombatFeedbackProjectionRequest extends RuntimeSessionCombatReadoutRequest {
  readonly camera?: CameraHandle;
  readonly viewport?: CameraProjectionRequest['viewport'];
}

export type RuntimeSessionGeneratedTunnelOperationReceipt = GeneratedTunnelOperationReceipt & {
  readonly sequenceId: number;
  readonly request: GeneratedTunnelOperationRequest;
  readonly sessionHashBefore: string;
  readonly sessionHashAfter: string;
};

export interface RuntimeSessionGameExtensionWeaponEffectReceipt {
  readonly sequenceId: number;
  readonly request: {
    readonly hook: WeaponEffectHookRequest;
    readonly primaryFire: FpsPrimaryFireRequest;
  };
  readonly hookReceipt: GameExtensionHookReceipt;
  readonly replayEvidence: GameExtensionReplayEvidence;
  readonly primaryFire: FpsPrimaryFireResult | null;
  readonly sessionHashBefore: string;
  readonly sessionHashAfter: string;
}

export interface RuntimeSessionGameRuleCatalogValidationReceipt extends GameRuleCatalogValidationReceipt {
  readonly sequenceId: number;
  readonly catalog: GameRuleCatalog;
  readonly sessionHashBefore: string;
  readonly sessionHashAfter: string;
}

export interface RuntimeSessionGameRuleEffectIntentReceipt extends GameRuleResolutionReceipt {
  readonly sequenceId: number;
  readonly catalog: GameRuleCatalog;
  readonly request: GameRuleResolutionRequest;
  readonly sessionHashBefore: string;
  readonly sessionHashAfter: string;
}

export interface RuntimeSessionAnimationIntentReadout {
  readonly kind: 'runtime_session.animation_intent.v0';
  readonly sequenceId: number;
  readonly tick: number;
  readonly asset: AnimatedMeshAsset;
  readonly instanceHandle: number;
  readonly selectedClipId: string;
  readonly selectionReason: RuntimeSessionAnimationSelectionReason;
  readonly playback: AnimatedMeshPlaybackCommand;
  readonly frame: RenderFrameDiff;
  readonly authority: RuntimeSessionAnimationIntentAuthority;
  readonly nonClaims: readonly RuntimeSessionAnimationIntentNonClaim[];
  readonly intentHash: string;
}

export type RuntimeSessionAnimationSelectionReason =
  | 'enemy_active_visual_run'
  | 'enemy_defeated_visual_idle'
  | 'player_defeated_visual_idle';

export type RuntimeSessionAnimationIntentNonClaim =
  | 'not_mixer_authority'
  | 'not_gameplay_outcome_authority'
  | 'not_collision_authority'
  | 'not_replay_authority';

export interface RuntimeSessionAnimationIntentAuthority {
  readonly source: 'runtime_session_lifecycle';
  readonly readSets: readonly ['lifecycle.player.health', 'lifecycle.enemy.health'];
  readonly projectionOnly: true;
}

export interface RuntimeSessionAnimationIntentInput {
  readonly sequenceId: number;
  readonly tick: number;
  readonly lifecycleState: RuntimeSessionLifecycleState;
}
