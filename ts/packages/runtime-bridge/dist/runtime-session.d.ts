import { type CameraCollisionSnapshot, type CameraCreateRequest, type CameraHandle, type CameraProjectionRequest, type CameraProjectionSnapshot, type CameraSnapshot, type CollisionAxis, type CollisionConstrainedCameraInputEnvelope, type CommandBatch, type CommandResult, type FirstPersonCameraInputEnvelope, type RenderFrameDiff, type VoxelConversionApplyRequest, type VoxelConversionEvidenceRef, type VoxelConversionMeshAssetRegistrationRequest, type VoxelConversionPlan, type VoxelConversionPlanRequest, type VoxelConversionPreview, type VoxelConversionPreviewRequest, type VoxelConversionReceipt, type VoxelConversionSourceRegistration, type VoxelConversionSourceRegistrationRequest, type VoxelModelInfoReadout, type VoxelModelInfoRequest, type VoxelVolumeAssetExportReceipt, type VoxelVolumeAssetExportRequest, type VoxelVolumeAssetLoadReceipt, type VoxelVolumeAssetLoadRequest, type VoxelVolumeAssetSaveReceipt, type VoxelVolumeAssetSaveRequest, type GameRuleModuleManifest, type GameExtensionHookReceipt, type GameExtensionReplayEvidence, type GameRuleCatalog, type GameRuleResolutionReceipt, type GameRuleResolutionRequest, type WeaponEffectHookRequest } from '@asha/contracts';
import { type CompositionStatus, type EngineHandle, type FrameCursor, type FpsPrimaryFireRequest, type FpsPrimaryFireResult, type GameRuleCatalogValidationReceipt, type GameRuleRuntimeReadout, type RuntimeBridge, type StepResult, type ProjectBundleLoadRequest } from './bridge.js';
import type { RuntimeSessionEcrpRenderTargetIdentity } from './ecrp-render-target.js';
import { type CombatReadoutScenario, type CombatRuntimeReadout } from './combat-readout.js';
import { type CombatFeedbackProjection } from './combat-feedback.js';
import { type GeneratedTunnelOperationReceipt, type GeneratedTunnelOperationRequest, type GeneratedTunnelReadout, type GeneratedTunnelReadoutRequest } from './generated-tunnel.js';
import { type EnemyPolicyActorView, type EnemyPolicyCombatView, type EnemyPolicyProposalFrame, type EnemyPolicySourceDiagnostic, type EnemyPolicyTargetView } from './enemy-policy.js';
import { type EncounterDirectorReadout, type EncounterDirectorReadoutRequest, type EncounterTransitionRequest, type RuntimeSessionEncounterTransitionReceipt } from './encounter-director.js';
import { type NavPathQueryRequest, type NavPathReadout, type NavPathScenario, type NavPolicyViewReadout, type NavProjectionReadout } from './nav-readout.js';
import type { RuntimeActionIntentEnvelope, RuntimeActionIntentRejection, RuntimeActionIntentStatus } from './runtime-action.js';
import { type RuntimeSessionAutonomousPolicyCombatSummary, type RuntimeSessionAutonomousPolicyMovementSummary, type RuntimeSessionAutonomousPolicyProposalReceipt, type RuntimeSessionAutonomousPolicyProposalRejection } from './runtime-session-lifecycle.js';
export type { RuntimeSessionAutonomousPolicyCombatSummary, RuntimeSessionAutonomousPolicyMovementSummary, RuntimeSessionAutonomousPolicyProposalReceipt, RuntimeSessionAutonomousPolicyProposalRejection, RuntimeSessionAutonomousPolicyProposalRejectionReason, RuntimeSessionAutonomousPolicyProposalStatus, } from './runtime-session-lifecycle.js';
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
export type RuntimeSessionNonClaim = 'not_native_runtime' | 'not_raw_state_store' | 'not_arbitrary_json_bridge' | 'not_product_authority' | 'not_gameplay_loop' | 'not_renderer';
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
    readonly kind: 'initialize' | 'submitCommands' | 'tick' | 'createCamera' | 'applyFirstPersonCameraInput' | 'applyCollisionConstrainedCameraInput' | 'loadEcrpProject' | 'submitRuntimeActionIntent' | 'submitGameExtensionWeaponEffect' | 'validateGameRuleCatalog' | 'submitGameRuleEffectIntent' | 'lifecycleDeath' | 'runAutonomousPolicyTick' | 'requestGeneratedTunnelOperation' | 'requestEncounterTransition' | 'requestSessionRestart' | 'restart';
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
export type RuntimeSessionEcrpCapabilityKind = 'transform' | 'collisionBody' | 'controller' | 'health' | 'weaponMount' | 'renderProjection' | 'policyBinding' | 'spawnMarker' | 'faction';
export type RuntimeSessionEcrpCapabilityState = {
    readonly kind: 'transform';
    readonly position: readonly [number, number, number];
    readonly yawDegrees: number;
    readonly pitchDegrees: number;
    readonly stateHash: string;
} | {
    readonly kind: 'collisionBody';
    readonly staticCollider: boolean;
    readonly bounds: readonly [number, number, number];
    readonly stateHash: string;
} | {
    readonly kind: 'controller';
    readonly controller: 'player_input' | 'enemy_policy';
    readonly stateHash: string;
} | {
    readonly kind: 'health';
    readonly current: number;
    readonly max: number;
    readonly dead: boolean;
    readonly stateHash: string;
} | {
    readonly kind: 'weaponMount';
    readonly weaponId: string;
    readonly stateHash: string;
} | {
    readonly kind: 'renderProjection';
    readonly visible: boolean;
    readonly projection: 'first_person_camera' | 'target_cube' | 'spawn_marker';
    readonly target: RuntimeSessionEcrpRenderTargetIdentity;
    readonly stateHash: string;
} | {
    readonly kind: 'policyBinding';
    readonly policyId: string;
    readonly stateHash: string;
} | {
    readonly kind: 'spawnMarker';
    readonly markerId: string;
    readonly stateHash: string;
} | {
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
        'not_demo_local_authority'
    ];
}
export type RuntimeSessionEcrpProjectDiagnosticCode = 'duplicateEntityDefinition' | 'duplicatePlacement' | 'emptyEntityDefinitionList' | 'invalidGameRuleModuleManifest' | 'invalidCapability' | 'missingCapability' | 'missingEntityDefinition' | 'missingPlacement' | 'missingProjectBundle' | 'unknownEntityDefinition';
export interface RuntimeSessionEcrpProjectDiagnostic {
    readonly code: RuntimeSessionEcrpProjectDiagnosticCode;
    readonly path: string;
    readonly detail: string;
}
export type RuntimeSessionEcrpProjectCapabilityDefinition = {
    readonly kind: 'transform';
    readonly initial: {
        readonly position: readonly [number, number, number];
        readonly yawDegrees: number;
        readonly pitchDegrees: number;
    };
} | {
    readonly kind: 'collisionBody';
    readonly halfExtents: readonly [number, number, number];
    readonly staticCollider?: boolean;
    readonly policy?: object;
} | {
    readonly kind: 'controller';
    readonly controller: 'player_input' | 'enemy_policy';
    readonly tuning?: object;
} | {
    readonly kind: 'health';
    readonly current: number;
    readonly max: number;
} | {
    readonly kind: 'weaponMount';
    readonly weaponId: string;
    readonly tuning?: object;
} | {
    readonly kind: 'renderProjection';
    readonly projection: 'first_person_camera' | 'target_cube' | 'spawn_marker';
    readonly visible?: boolean;
} | {
    readonly kind: 'policyBinding';
    readonly policyId: string;
    readonly policyLoopRef?: string;
} | {
    readonly kind: 'spawnMarker';
    readonly markerId: string;
} | {
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
        'not_demo_local_authority'
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
    applyCollisionConstrainedCameraInput(envelope: CollisionConstrainedCameraInputEnvelope): RuntimeSessionCameraCollisionInputReceipt;
    submitRuntimeActionIntent(envelope: RuntimeActionIntentEnvelope): RuntimeSessionActionIntentReceipt;
    submitGameExtensionWeaponEffect(hook: WeaponEffectHookRequest, primaryFire: FpsPrimaryFireRequest): RuntimeSessionGameExtensionWeaponEffectReceipt;
    validateGameRuleCatalog(catalog: GameRuleCatalog): RuntimeSessionGameRuleCatalogValidationReceipt;
    submitGameRuleEffectIntent(catalog: GameRuleCatalog, request: GameRuleResolutionRequest): RuntimeSessionGameRuleEffectIntentReceipt;
    readGameRuleRuntimeReadout(): GameRuleRuntimeReadout;
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
    registerVoxelConversionSource(request: VoxelConversionSourceRegistrationRequest): VoxelConversionSourceRegistration;
    registerVoxelConversionMeshAsset(request: VoxelConversionMeshAssetRegistrationRequest): VoxelConversionSourceRegistration;
    planVoxelConversion(request: VoxelConversionPlanRequest): VoxelConversionPlan;
    previewVoxelConversion(request: VoxelConversionPreviewRequest): VoxelConversionPreview;
    applyVoxelConversion(request: VoxelConversionApplyRequest): VoxelConversionReceipt;
    exportVoxelConversionEvidence(evidence: readonly VoxelConversionEvidenceRef[]): readonly VoxelConversionEvidenceRef[];
    readVoxelModelInfo(request: VoxelModelInfoRequest): VoxelModelInfoReadout;
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
export type RuntimeSessionHashValue = RuntimeSessionHashPrimitive | readonly RuntimeSessionHashValue[] | RuntimeSessionHashRecord;
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
export declare function createRuntimeSessionFacade(options: RuntimeSessionFacadeOptions): RuntimeSessionFacade;
//# sourceMappingURL=runtime-session.d.ts.map