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
  type VoxelConversionApplyRequest,
  type VoxelConversionEvidenceRef,
  type VoxelConversionMeshAssetRegistrationRequest,
  type VoxelConversionPlan,
  type VoxelConversionPlanRequest,
  type VoxelConversionPreview,
  type VoxelConversionPreviewRequest,
  type VoxelConversionReceipt,
  type VoxelConversionSourceRegistration,
  type VoxelConversionSourceRegistrationRequest,
  type VoxelModelInfoReadout, type VoxelModelInfoRequest,
  type VoxelModelWindowReadout, type VoxelModelWindowRequest,
  type VoxelVolumeAssetExportReceipt,
  type VoxelVolumeAssetExportRequest,
  type VoxelVolumeAssetLoadReceipt,
  type VoxelVolumeAssetLoadRequest,
  type VoxelVolumeAssetSaveReceipt,
  type VoxelVolumeAssetSaveRequest,
  type GameRuleModuleManifest,
  type GameExtensionHookReceipt,
  type GameExtensionReplayEvidence,
  type GameRuleCatalog,
  type GameRuleResolutionReceipt,
  type GameRuleResolutionRequest,
  type WeaponEffectHookRequest,
} from '@asha/contracts';
import {
  RuntimeBridgeError,
  frameCursor,
  type CompositionStatus,
  type EnemyDirectNavMovementResult,
  type EngineHandle,
  type FrameCursor,
  type FpsPrimaryFireRequest,
  type FpsPrimaryFireResult,
  type GameRuleCatalogValidationReceipt,
  type GameRuleRuntimeReadout,
  type RuntimeBridge, type StepResult,
  type ProjectBundleLoadRequest,
} from './bridge.js';
import type { RuntimeSessionEcrpRenderTargetIdentity } from './ecrp-render-target.js';
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
import {
  buildRuntimeSessionEnemyNavPath,
  ecrpActorPosition,
  ecrpEntityTransform,
  runtimeTransformHashRecord,
} from './runtime-session-enemy-authority.js';
import {
  buildEcrpProjectState,
  buildEcrpRuntimeReadout,
  defaultRuntimeSessionEcrpProjectLoadInput,
  lifecycleStateFromEcrpProject,
  validateEcrpProjectLoadInput,
} from './runtime-session-ecrp.js';
import {
  acceptedAutonomousMovementReceipt,
  applyReferenceCombatReadoutToLifecycleState,
  buildReferenceRuntimeSessionPrimaryFireReadout,
  combatReadoutTick,
  generatedTunnelEnemyDefeatedLifecycleState,
  generatedTunnelPlayerDefeatedLifecycleState,
  initialRuntimeSessionLifecycleState,
  lifecycleStatusReadout,
  lifecycleStatusToEncounterLifecycle,
  rejectedAutonomousPolicyProposalReceipt,
  runtimeActionReceiptToAutonomousReceipt,
  type RuntimeSessionAutonomousPolicyCombatSummary,
  type RuntimeSessionAutonomousPolicyMovementSummary,
  type RuntimeSessionAutonomousPolicyProposalReceipt,
  type RuntimeSessionAutonomousPolicyProposalRejection,
  validateAutonomousPolicyProposal,
  validateAutonomousPolicyTickInput,
  validateGeneratedTunnelOperationRequest,
  validateGeneratedTunnelReadoutRequest,
  validateInitializeInput,
  validateLifecycleStatusRequest,
  validateRestartIntent,
  validateRuntimeActionIntentEnvelope,
} from './runtime-session-lifecycle.js';
import {
  compositionHashRecord,
  encounterStateHashRecord,
  identityHashRecord,
  lifecycleStateHashRecord,
  referenceRuntimeSessionNonClaims,
  renderFrameHashRecord,
  stableHash,
} from './runtime-session-hash.js';
import { RustBackedRuntimeSessionFacade } from './runtime-session-rust-facade.js';

export type {
  RuntimeSessionAutonomousPolicyCombatSummary,
  RuntimeSessionAutonomousPolicyMovementSummary,
  RuntimeSessionAutonomousPolicyProposalReceipt,
  RuntimeSessionAutonomousPolicyProposalRejection,
  RuntimeSessionAutonomousPolicyProposalRejectionReason,
  RuntimeSessionAutonomousPolicyProposalStatus,
} from './runtime-session-lifecycle.js';

export type RuntimeSessionMode = 'reference' | 'rust';

export interface RuntimeSessionProjectIdentity {
  readonly gameId: string;
  readonly workspaceId: string;
}

export interface RuntimeSessionInitializeInput {
  readonly sessionId: string;
  readonly seed: number;
  readonly project: RuntimeSessionProjectIdentity;
  readonly projectBundle: ProjectBundleLoadRequest;
}

export interface RuntimeSessionIdentity {
  readonly sessionId: string;
  readonly mode: RuntimeSessionMode;
  readonly seed: number;
  readonly project: RuntimeSessionProjectIdentity;
  readonly projectBundle: ProjectBundleLoadRequest;
  readonly nonClaims: readonly RuntimeSessionNonClaim[];
}

export type RuntimeSessionNonClaim =
  | 'not_native_runtime'
  | 'not_raw_state_store'
  | 'not_arbitrary_json_bridge'
  | 'not_product_authority'
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
    | 'loadEcrpProject'
    | 'submitRuntimeActionIntent'
    | 'submitGameExtensionWeaponEffect'
    | 'validateGameRuleCatalog'
    | 'submitGameRuleEffectIntent'
    | 'lifecycleDeath'
    | 'runAutonomousPolicyTick'
    | 'requestGeneratedTunnelOperation'
    | 'requestEncounterTransition'
    | 'requestSessionRestart'
    | 'restart';
  readonly actionSource?: RuntimeActionIntentEnvelope['source'];
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

export type RuntimeSessionEcrpCapabilityKind =
  | 'transform'
  | 'collisionBody'
  | 'controller'
  | 'health'
  | 'weaponMount'
  | 'renderProjection'
  | 'policyBinding'
  | 'spawnMarker'
  | 'faction';

export type RuntimeSessionEcrpCapabilityState =
  | {
      readonly kind: 'transform';
      readonly position: readonly [number, number, number];
      readonly yawDegrees: number;
      readonly pitchDegrees: number;
      readonly stateHash: string;
    }
  | {
      readonly kind: 'collisionBody';
      readonly staticCollider: boolean;
      readonly bounds: readonly [number, number, number];
      readonly stateHash: string;
    }
  | {
      readonly kind: 'controller';
      readonly controller: 'player_input' | 'enemy_policy';
      readonly stateHash: string;
    }
  | {
      readonly kind: 'health';
      readonly current: number;
      readonly max: number;
      readonly dead: boolean;
      readonly stateHash: string;
    }
  | {
      readonly kind: 'weaponMount';
      readonly weaponId: string;
      readonly stateHash: string;
    }
  | {
      readonly kind: 'renderProjection';
      readonly visible: boolean;
      readonly projection: 'first_person_camera' | 'target_cube' | 'spawn_marker';
      readonly target: RuntimeSessionEcrpRenderTargetIdentity;
      readonly stateHash: string;
    }
  | {
      readonly kind: 'policyBinding';
      readonly policyId: string;
      readonly stateHash: string;
    }
  | {
      readonly kind: 'spawnMarker';
      readonly markerId: string;
      readonly stateHash: string;
    }
  | {
      readonly kind: 'faction';
      readonly factionId: string;
      readonly stateHash: string;
    };

export interface RuntimeSessionEcrpEntityEventReadout {
  readonly kind: RuntimeSessionLifecycleEventKind | 'runtime_session.bootstrap_entity.v0';
  readonly entity: number;
  readonly tick: number;
  readonly eventHash: string;
}

export type { RuntimeSessionEcrpRenderTargetIdentity } from './ecrp-render-target.js';

export interface RuntimeSessionEcrpEntityReadout {
  readonly entity: number;
  readonly lifecycle: 'active' | 'tombstoned';
  readonly definitionStableId: string;
  readonly displayName: string;
  readonly source: {
    readonly projectBundle: string;
    readonly relativePath: string;
  };
  readonly capabilityKinds: readonly RuntimeSessionEcrpCapabilityKind[];
  readonly capabilities: readonly RuntimeSessionEcrpCapabilityState[];
  readonly recentEvents: readonly RuntimeSessionEcrpEntityEventReadout[];
  readonly entityHash: string;
}

export interface RuntimeSessionEcrpReadout {
  readonly kind: 'runtime_session.ecrp_readout.v0';
  readonly sequenceId: number;
  readonly tick: number;
  readonly sessionHash: string;
  readonly authority: {
    readonly mode: RuntimeSessionMode;
    readonly source: 'reference_fixture' | 'rust_bridge';
    readonly surface: string;
    readonly readSets: readonly {
      readonly viewKind: string;
      readonly owner: string;
      readonly readSet: readonly string[];
    }[];
  };
  readonly project: RuntimeSessionProjectIdentity;
  readonly projectBundle: ProjectBundleLoadRequest;
  readonly entities: readonly RuntimeSessionEcrpEntityReadout[];
  readonly entityCount: number;
  readonly hashes: {
    readonly entityReadoutHash: string;
    readonly capabilityStateHash: string;
    readonly eventReadoutHash: string;
  };
  readonly nonClaims: readonly [
    'not_raw_state_store',
    'not_authoring_mode',
    'not_demo_local_authority',
  ];
}

export type RuntimeSessionEcrpProjectDiagnosticCode =
  | 'duplicateEntityDefinition'
  | 'duplicatePlacement'
  | 'emptyEntityDefinitionList'
  | 'invalidGameRuleModuleManifest'
  | 'invalidCapability'
  | 'missingCapability'
  | 'missingEntityDefinition'
  | 'missingPlacement'
  | 'missingProjectBundle'
  | 'unknownEntityDefinition';

export interface RuntimeSessionEcrpProjectDiagnostic {
  readonly code: RuntimeSessionEcrpProjectDiagnosticCode;
  readonly path: string;
  readonly detail: string;
}

export type RuntimeSessionEcrpProjectCapabilityDefinition =
  | {
      readonly kind: 'transform';
      readonly initial: {
        readonly position: readonly [number, number, number];
        readonly yawDegrees: number;
        readonly pitchDegrees: number;
      };
    }
  | {
      readonly kind: 'collisionBody';
      readonly halfExtents: readonly [number, number, number];
      readonly staticCollider?: boolean;
      readonly policy?: object;
    }
  | {
      readonly kind: 'controller';
      readonly controller: 'player_input' | 'enemy_policy';
      readonly tuning?: object;
    }
  | {
      readonly kind: 'health';
      readonly current: number;
      readonly max: number;
    }
  | {
      readonly kind: 'weaponMount';
      readonly weaponId: string;
      readonly tuning?: object;
    }
  | {
      readonly kind: 'renderProjection';
      readonly projection: 'first_person_camera' | 'target_cube' | 'spawn_marker';
      readonly visible?: boolean;
    }
  | {
      readonly kind: 'policyBinding';
      readonly policyId: string;
      readonly policyLoopRef?: string;
    }
  | {
      readonly kind: 'spawnMarker';
      readonly markerId: string;
    }
  | {
      readonly kind: 'faction';
      readonly factionId: string;
    };

export interface RuntimeSessionEcrpEntityDefinition {
  readonly kind: 'EntityDefinition';
  readonly stableId: string;
  readonly displayName: string;
  readonly source: {
    readonly projectBundle: string;
    readonly relativePath: string;
  };
  readonly capabilities: readonly RuntimeSessionEcrpProjectCapabilityDefinition[];
}

export interface RuntimeSessionEcrpScenePlacement {
  readonly entityDefinitionId: string;
  readonly spawnMarkerId?: string;
  readonly runtimeEntityId?: number;
}

export interface RuntimeSessionEcrpSceneDocument {
  readonly kind: 'SceneDocument';
  readonly sceneId: string;
  readonly placements: readonly RuntimeSessionEcrpScenePlacement[];
}

export interface RuntimeSessionEcrpProjectLoadInput {
  readonly kind: 'runtime_session.load_ecrp_project.v0';
  readonly projectBundle: {
    readonly kind: 'ProjectBundle';
    readonly project: RuntimeSessionProjectIdentity;
    readonly runtimeRequest: ProjectBundleLoadRequest;
  };
  readonly entityDefinitions: readonly RuntimeSessionEcrpEntityDefinition[];
  readonly sceneDocument: RuntimeSessionEcrpSceneDocument;
  readonly gameRuleModules?: readonly GameRuleModuleManifest[];
}

export interface RuntimeSessionEcrpProjectLoadReceipt {
  readonly kind: 'runtime_session.ecrp_project_load_receipt.v0';
  readonly sequenceId: number;
  readonly accepted: boolean;
  readonly diagnostics: readonly RuntimeSessionEcrpProjectDiagnostic[];
  readonly entityCount: number;
  readonly bootstrapHash: string | null;
  readonly sessionHashBefore: string;
  readonly sessionHashAfter: string;
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
    readonly reason: 'always_resettable_reference_fixture' | 'rust_epoch_restart';
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

export interface RuntimeSessionGeneratedTunnelOperationReceipt extends GeneratedTunnelOperationReceipt {
  readonly sequenceId: number;
  readonly request: GeneratedTunnelOperationRequest;
  readonly sessionHashBefore: string;
  readonly sessionHashAfter: string;
}

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

export interface RuntimeSessionFacade {
  initialize(input: RuntimeSessionInitializeInput): RuntimeSessionStateSummary;
  loadEcrpProject(input: RuntimeSessionEcrpProjectLoadInput): RuntimeSessionEcrpProjectLoadReceipt;
  submitCommands(batch: CommandBatch): RuntimeSessionCommandReceipt;
  tick(input?: RuntimeSessionTickInput): RuntimeSessionTickResult;
  createCamera(request: CameraCreateRequest): RuntimeSessionCameraCreateReceipt;
  applyFirstPersonCameraInput(envelope: FirstPersonCameraInputEnvelope): RuntimeSessionCameraInputReceipt;
  applyCollisionConstrainedCameraInput(
    envelope: CollisionConstrainedCameraInputEnvelope,
  ): RuntimeSessionCameraCollisionInputReceipt;
  submitRuntimeActionIntent(envelope: RuntimeActionIntentEnvelope): RuntimeSessionActionIntentReceipt;
  submitGameExtensionWeaponEffect(
    hook: WeaponEffectHookRequest,
    primaryFire: FpsPrimaryFireRequest,
  ): RuntimeSessionGameExtensionWeaponEffectReceipt;
  validateGameRuleCatalog(catalog: GameRuleCatalog): RuntimeSessionGameRuleCatalogValidationReceipt;
  submitGameRuleEffectIntent(
    catalog: GameRuleCatalog,
    request: GameRuleResolutionRequest,
  ): RuntimeSessionGameRuleEffectIntentReceipt;
  readGameRuleRuntimeReadout(): GameRuleRuntimeReadout;
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
  registerVoxelConversionSource(request: VoxelConversionSourceRegistrationRequest): VoxelConversionSourceRegistration;
  registerVoxelConversionMeshAsset(request: VoxelConversionMeshAssetRegistrationRequest): VoxelConversionSourceRegistration;
  planVoxelConversion(request: VoxelConversionPlanRequest): VoxelConversionPlan;
  previewVoxelConversion(request: VoxelConversionPreviewRequest): VoxelConversionPreview;
  applyVoxelConversion(request: VoxelConversionApplyRequest): VoxelConversionReceipt;
  exportVoxelConversionEvidence(evidence: readonly VoxelConversionEvidenceRef[]): readonly VoxelConversionEvidenceRef[];
  readVoxelModelInfo(request: VoxelModelInfoRequest): VoxelModelInfoReadout;
  readVoxelModelWindow(request: VoxelModelWindowRequest): VoxelModelWindowReadout;
  exportVoxelVolumeAsset(request: VoxelVolumeAssetExportRequest): VoxelVolumeAssetExportReceipt;
  saveVoxelVolumeAsset(request: VoxelVolumeAssetSaveRequest): VoxelVolumeAssetSaveReceipt;
  loadVoxelVolumeAsset(request: VoxelVolumeAssetLoadRequest): VoxelVolumeAssetLoadReceipt;
  readEcrpRuntimeReadout(): RuntimeSessionEcrpReadout;
  readCameraProjection(request: CameraProjectionRequest): RuntimeSessionCameraProjectionReadout;
  readProjection(): RuntimeSessionProjectionSummary;
  readTelemetry(): RuntimeSessionTelemetrySummary;
  restart(): RuntimeSessionRestartResult;
}

export interface RuntimeSessionFacadeOptions {
  readonly bridge: RuntimeBridge;
  readonly mode?: RuntimeSessionMode;
}

export type RuntimeSessionHashPrimitive = string | number | boolean | null;
export type RuntimeSessionHashValue =
  | RuntimeSessionHashPrimitive
  | readonly RuntimeSessionHashValue[]
  | RuntimeSessionHashRecord;
export interface RuntimeSessionHashRecord {
  readonly [key: string]: RuntimeSessionHashValue | undefined;
}

export interface RuntimeSessionLifecycleState {
  readonly player: RuntimeSessionLifecycleHealthReadout;
  readonly enemy: RuntimeSessionLifecycleHealthReadout;
  readonly terminalEvent: RuntimeSessionLifecycleEventReadout | null;
  readonly revision: number;
}

export interface RuntimeSessionEcrpEntityState {
  readonly entity: number;
  readonly definition: RuntimeSessionEcrpEntityDefinition;
  readonly role: RuntimeSessionLifecycleRole | 'neutral';
}

export interface RuntimeSessionEcrpTransformState {
  readonly position: readonly [number, number, number];
  readonly yawDegrees: number;
  readonly pitchDegrees: number;
}

export interface RuntimeSessionEcrpProjectState {
  readonly input: RuntimeSessionEcrpProjectLoadInput;
  readonly entities: readonly RuntimeSessionEcrpEntityState[];
  readonly bootstrapHash: string;
}

export function createRuntimeSessionFacade(options: RuntimeSessionFacadeOptions): RuntimeSessionFacade {
  if (options.mode === 'reference') {
    return new ReferenceRuntimeSessionFacade(options.bridge);
  }
  return new RustBackedRuntimeSessionFacade(options.bridge);
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
  #ecrpProjectState: RuntimeSessionEcrpProjectState | null = null;
  #runtimeTransforms = new Map<number, RuntimeSessionEcrpTransformState>();
  #replayRecords: RuntimeSessionReplayRecord[] = [];

  constructor(bridge: RuntimeBridge) {
    this.#bridge = bridge;
  }

  initialize(input: RuntimeSessionInitializeInput): RuntimeSessionStateSummary {
    validateInitializeInput(input);
    const engine = this.#bridge.initializeEngine({ seed: input.seed });
    const composition = this.#bridge.loadProjectBundle(input.projectBundle);
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
    this.#ecrpProjectState = buildEcrpProjectState(defaultRuntimeSessionEcrpProjectLoadInput(input));
    this.#lifecycleState = lifecycleStateFromEcrpProject(this.#ecrpProjectState);
    this.#runtimeTransforms = new Map();
    this.#encounterState = initialEncounterDirectorState();
    this.#replayRecords = [];
    this.#record('initialize');
    return this.#stateSummary(composition);
  }

  loadEcrpProject(input: RuntimeSessionEcrpProjectLoadInput): RuntimeSessionEcrpProjectLoadReceipt {
    const identity = this.#requireInitialized('loadEcrpProject');
    const before = this.#sessionHash();
    const diagnostics = validateEcrpProjectLoadInput(input);
    this.#sequenceId += 1;

    if (diagnostics.length > 0) {
      this.#record('loadEcrpProject');
      return {
        kind: 'runtime_session.ecrp_project_load_receipt.v0',
        sequenceId: this.#sequenceId,
        accepted: false,
        diagnostics,
        entityCount: 0,
        bootstrapHash: null,
        sessionHashBefore: before,
        sessionHashAfter: this.#sessionHash(),
      };
    }

    const state = buildEcrpProjectState(input);
    this.#bridge.loadProjectBundle(input.projectBundle.runtimeRequest);
    this.#identity = {
      ...identity,
      project: input.projectBundle.project,
      projectBundle: input.projectBundle.runtimeRequest,
    };
    this.#ecrpProjectState = state;
    this.#lifecycleState = lifecycleStateFromEcrpProject(state);
    this.#runtimeTransforms = new Map();
    this.#record('loadEcrpProject');
    return {
      kind: 'runtime_session.ecrp_project_load_receipt.v0',
      sequenceId: this.#sequenceId,
      accepted: true,
      diagnostics: [],
      entityCount: state.entities.length,
      bootstrapHash: state.bootstrapHash,
      sessionHashBefore: before,
      sessionHashAfter: this.#sessionHash(),
    };
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
      composition: this.#bridge.getProjectBundleCompositionStatus(),
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
      collisionSourceHash: snapshot.collision.collisionSourceHash,
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
    this.#record('submitRuntimeActionIntent', envelope.source);
    const combatReadout =
      envelope.action === 'primary_fire' && envelope.phase === 'pressed'
        ? buildReferenceRuntimeSessionPrimaryFireReadout({
            projectState: this.#ecrpProjectState,
            lifecycleState: this.#lifecycleState,
            source: envelope.source,
            tick: envelope.tick,
          })
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

  submitGameExtensionWeaponEffect(
    hook: WeaponEffectHookRequest,
    primaryFire: FpsPrimaryFireRequest,
  ): RuntimeSessionGameExtensionWeaponEffectReceipt {
    this.#requireInitialized('submitGameExtensionWeaponEffect');
    const before = this.#sessionHash();
    const result = this.#bridge.invokeGameExtensionWeaponEffect({ hook, primaryFire });
    this.#sequenceId += 1;
    this.#record('submitGameExtensionWeaponEffect');
    return {
      sequenceId: this.#sequenceId,
      request: { hook, primaryFire },
      hookReceipt: result.hookReceipt,
      replayEvidence: result.replayEvidence,
      primaryFire: result.primaryFire,
      sessionHashBefore: before,
      sessionHashAfter: this.#sessionHash(),
    };
  }

  validateGameRuleCatalog(catalog: GameRuleCatalog): RuntimeSessionGameRuleCatalogValidationReceipt {
    this.#requireInitialized('validateGameRuleCatalog');
    const before = this.#sessionHash();
    const receipt = this.#bridge.validateGameRuleCatalog(catalog);
    this.#sequenceId += 1;
    this.#record('validateGameRuleCatalog');
    return {
      ...receipt,
      sequenceId: this.#sequenceId,
      catalog,
      sessionHashBefore: before,
      sessionHashAfter: this.#sessionHash(),
    };
  }

  submitGameRuleEffectIntent(
    catalog: GameRuleCatalog,
    request: GameRuleResolutionRequest,
  ): RuntimeSessionGameRuleEffectIntentReceipt {
    this.#requireInitialized('submitGameRuleEffectIntent');
    const before = this.#sessionHash();
    const receipt = this.#bridge.submitGameRuleEffectIntent({ catalog, request });
    this.#sequenceId += 1;
    this.#record('submitGameRuleEffectIntent');
    return {
      ...receipt,
      sequenceId: this.#sequenceId,
      catalog,
      request,
      sessionHashBefore: before,
      sessionHashAfter: this.#sessionHash(),
    };
  }

  readGameRuleRuntimeReadout(): GameRuleRuntimeReadout {
    this.#requireInitialized('readGameRuleRuntimeReadout');
    return this.#bridge.readGameRuleRuntimeReadout();
  }

  runAutonomousPolicyTick(input: RuntimeSessionAutonomousPolicyTickInput): RuntimeSessionAutonomousPolicyTickReadout {
    this.#requireInitialized('runAutonomousPolicyTick');
    validateAutonomousPolicyTickInput(input);

    const sequenceIdBefore = this.#sequenceId;
    const sessionHashBefore = this.#sessionHash();
    const step = this.tick(input.tick === undefined ? {} : { tick: input.tick });
    const usesLivePolicyPositions = input.enemy?.position !== undefined || input.target?.position !== undefined;
    const enemyPolicyPosition =
      input.enemy?.position ??
      ecrpActorPosition({
        projectState: this.#ecrpProjectState,
        runtimeTransforms: this.#runtimeTransforms,
        role: 'enemy',
      }) ??
      undefined;
    const targetPolicyPosition =
      input.target?.position ??
      ecrpActorPosition({
        projectState: this.#ecrpProjectState,
        runtimeTransforms: this.#runtimeTransforms,
        role: 'player',
      }) ??
      undefined;
    const navPath = buildRuntimeSessionEnemyNavPath({
      ...(input.navScenario === undefined ? {} : { scenario: input.navScenario }),
      ...(!usesLivePolicyPositions || enemyPolicyPosition === undefined ? {} : { enemyPosition: enemyPolicyPosition }),
      ...(!usesLivePolicyPositions || targetPolicyPosition === undefined ? {} : { targetPosition: targetPolicyPosition }),
      queryFixturePath: (scenario) => this.queryNavPath(scenario === undefined ? {} : { scenario }),
    });
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
        const movement = this.#applyAutonomousMovementProposal(proposal, targetPolicyPosition);
        proposalReceipts.push(acceptedAutonomousMovementReceipt(proposal, movement));
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

  planVoxelConversion(_request: VoxelConversionPlanRequest): VoxelConversionPlan {
    void _request;
    this.#requireInitialized('planVoxelConversion');
    throw new RuntimeBridgeError('operation_unimplemented', 'Voxel conversion authority is not wired into the reference RuntimeSession');
  }

  registerVoxelConversionSource(_request: VoxelConversionSourceRegistrationRequest): VoxelConversionSourceRegistration {
    void _request;
    this.#requireInitialized('registerVoxelConversionSource');
    throw new RuntimeBridgeError('operation_unimplemented', 'Voxel conversion source registration is not wired into the reference RuntimeSession');
  }

  registerVoxelConversionMeshAsset(_request: VoxelConversionMeshAssetRegistrationRequest): VoxelConversionSourceRegistration {
    void _request;
    this.#requireInitialized('registerVoxelConversionMeshAsset');
    throw new RuntimeBridgeError('operation_unimplemented', 'Voxel conversion mesh asset registration is not wired into the reference RuntimeSession');
  }

  previewVoxelConversion(_request: VoxelConversionPreviewRequest): VoxelConversionPreview {
    void _request;
    this.#requireInitialized('previewVoxelConversion');
    throw new RuntimeBridgeError('operation_unimplemented', 'Voxel conversion preview is not wired into the reference RuntimeSession');
  }

  applyVoxelConversion(_request: VoxelConversionApplyRequest): VoxelConversionReceipt {
    void _request;
    this.#requireInitialized('applyVoxelConversion');
    throw new RuntimeBridgeError('operation_unimplemented', 'Voxel conversion apply is not wired into the reference RuntimeSession');
  }

  exportVoxelConversionEvidence(_evidence: readonly VoxelConversionEvidenceRef[]): readonly VoxelConversionEvidenceRef[] {
    void _evidence;
    this.#requireInitialized('exportVoxelConversionEvidence');
    throw new RuntimeBridgeError('operation_unimplemented', 'Voxel conversion evidence export is not wired into the reference RuntimeSession');
  }

  readVoxelModelInfo(_request: VoxelModelInfoRequest): VoxelModelInfoReadout {
    void _request;
    this.#requireInitialized('readVoxelModelInfo');
    throw new RuntimeBridgeError('operation_unimplemented', 'Voxel model info is not wired into the reference RuntimeSession');
  }

  readVoxelModelWindow(_request: VoxelModelWindowRequest): VoxelModelWindowReadout { void _request; this.#requireInitialized('readVoxelModelWindow'); throw new RuntimeBridgeError('operation_unimplemented', 'Voxel model window is not wired into the reference RuntimeSession'); }

  exportVoxelVolumeAsset(_request: VoxelVolumeAssetExportRequest): VoxelVolumeAssetExportReceipt {
    void _request;
    this.#requireInitialized('exportVoxelVolumeAsset');
    throw new RuntimeBridgeError('operation_unimplemented', 'Voxel volume asset export is not wired into the reference RuntimeSession');
  }

  saveVoxelVolumeAsset(_request: VoxelVolumeAssetSaveRequest): VoxelVolumeAssetSaveReceipt {
    void _request;
    this.#requireInitialized('saveVoxelVolumeAsset');
    throw new RuntimeBridgeError('operation_unimplemented', 'Voxel volume asset save is not wired into the reference RuntimeSession');
  }

  loadVoxelVolumeAsset(_request: VoxelVolumeAssetLoadRequest): VoxelVolumeAssetLoadReceipt {
    void _request;
    this.#requireInitialized('loadVoxelVolumeAsset');
    throw new RuntimeBridgeError('operation_unimplemented', 'Voxel volume asset load is not wired into the reference RuntimeSession');
  }

  readEcrpRuntimeReadout(): RuntimeSessionEcrpReadout {
    const identity = this.#requireInitialized('readEcrpRuntimeReadout');
    return buildEcrpRuntimeReadout({
      identity,
      projectState: this.#ecrpProjectState,
      lifecycleState: this.#lifecycleState,
      runtimeTransforms: this.#runtimeTransforms,
      sequenceId: this.#sequenceId,
      tick: this.#tick,
      sessionHash: this.#sessionHash(),
    });
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
    const composition = this.#bridge.getProjectBundleCompositionStatus();
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
      composition: this.#bridge.getProjectBundleCompositionStatus(),
      acceptedCommandCount: this.#acceptedCommandCount,
      rejectedCommandCount: this.#rejectedCommandCount,
      restartCount: this.#restartCount,
      sessionHash: this.#sessionHash(),
      replayRecords: [...this.#replayRecords],
    };
  }

  restart(): RuntimeSessionRestartResult {
    const identity = this.#requireInitialized('restart');
    this.#bridge.unloadProjectBundle();
    this.#bridge.initializeEngine({ seed: identity.seed });
    const composition = this.#bridge.loadProjectBundle(identity.projectBundle);
    this.#sequenceId += 1;
    this.#tick = 0;
    this.#acceptedCommandCount = 0;
    this.#rejectedCommandCount = 0;
    if (this.#ecrpProjectState !== null) {
      this.#lifecycleState = lifecycleStateFromEcrpProject(this.#ecrpProjectState);
    } else {
      this.#lifecycleState = initialRuntimeSessionLifecycleState();
    }
    this.#runtimeTransforms = new Map();
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

  #applyAutonomousMovementProposal(
    proposal: Extract<EnemyPolicyProposal, { readonly kind: 'enemy_policy.move_toward_target.v0' }>,
    targetPosition: EnemyPolicyVec3 | undefined,
  ): EnemyDirectNavMovementResult {
    const enemy = this.#ecrpProjectState?.entities.find((entity) => entity.role === 'enemy');
    if (enemy === undefined || proposal.nextWaypoint === null || this.#lifecycleState.enemy.dead) {
      throw new RuntimeBridgeError('invalid_input', 'enemy movement proposal cannot be applied without a live ECRP enemy');
    }
    const movement = this.#bridge.applyEnemyDirectNavMovement({
      entity: enemy.entity,
      seedPosition: proposal.from,
      target: targetPosition ?? proposal.nextWaypoint,
      maxStepUnits: 0.35,
    });
    const current = ecrpEntityTransform({
      entity: enemy,
      runtimeTransforms: this.#runtimeTransforms,
    });
    this.#runtimeTransforms.set(enemy.entity, {
      position: movement.nextWaypoint,
      yawDegrees: current?.yawDegrees ?? 0,
      pitchDegrees: current?.pitchDegrees ?? 0,
    });
    return movement;
  }

  #applyCombatLifecycleReadout(readout: CombatRuntimeReadout, tick: number): void {
    const applied = applyReferenceCombatReadoutToLifecycleState({
      state: this.#lifecycleState,
      readout,
      tick,
    });
    this.#lifecycleState = applied.state;
    if (applied.recordLifecycleDeath) {
      this.#record('lifecycleDeath');
    }
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
  #record(kind: RuntimeSessionReplayRecord['kind'], actionSource?: RuntimeActionIntentEnvelope['source']): void {
    this.#replayRecords.push({
      sequenceId: this.#sequenceId,
      kind,
      ...(actionSource === undefined ? {} : { actionSource }),
      recordHash: stableHash({
        kind,
        ...(actionSource === undefined ? {} : { actionSource }),
        sequenceId: this.#sequenceId,
        tick: this.#tick,
        acceptedCommandCount: this.#acceptedCommandCount,
        rejectedCommandCount: this.#rejectedCommandCount,
        restartCount: this.#restartCount,
        lifecycle: lifecycleStateHashRecord(this.#lifecycleState),
        ...(this.#runtimeTransforms.size === 0
          ? {}
          : { runtimeTransforms: runtimeTransformHashRecord(this.#runtimeTransforms) }),
        encounter: encounterStateHashRecord(this.#encounterState),
        composition: compositionHashRecord(this.#bridge.getProjectBundleCompositionStatus()),
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
      ...(this.#identity === null || this.#runtimeTransforms.size === 0
        ? {}
        : { runtimeTransforms: runtimeTransformHashRecord(this.#runtimeTransforms) }),
      encounter: this.#identity === null ? null : encounterStateHashRecord(this.#encounterState),
      composition: this.#identity === null ? null : compositionHashRecord(this.#bridge.getProjectBundleCompositionStatus()),
    });
  }
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
