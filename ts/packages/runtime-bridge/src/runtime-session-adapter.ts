import {
  cameraHandle,
  type CameraCreateRequest,
  type CameraControllerReadRequest,
  type CameraControllerState,
  type CameraModeChangeReceipt,
  type CameraModeCommand,
  type CameraNavigationInputEnvelope,
  type CameraNavigationReceipt,
  type CameraProjectionRequest,
  type CollisionConstrainedCameraInputEnvelope,
  type CommandBatch,
  type FirstPersonCameraInputEnvelope,
  type VoxelConversionApplyRequest,
  type VoxelConversionEvidenceRef,
  type VoxelConversionMeshAssetRegistrationRequest,
  type VoxelConversionMeshSourceImportReceipt,
  type VoxelConversionMeshSourceImportRequest,
  type VoxelConversionPlan,
  type VoxelConversionPlanRequest,
  type VoxelConversionPreview,
  type VoxelConversionPreviewRequest,
  type VoxelConversionReceipt,
  type VoxelConversionSourceMetadataReadout,
  type VoxelConversionSourceMetadataRequest,
  type VoxelConversionSourceRegistration,
  type VoxelConversionSourceRegistrationRequest,
  type VoxelModelInfoReadout, type VoxelModelInfoRequest,
  type VoxelModelWindowReadout, type VoxelModelWindowRequest,
  type VoxelAnnotationEditReceipt,
  type VoxelAnnotationEditRequest,
  type VoxelAnnotationLayerExportReceipt,
  type VoxelAnnotationLayerExportRequest,
  type VoxelAnnotationLayerLoadReceipt,
  type VoxelAnnotationLayerLoadRequest,
  type VoxelAnnotationLayerValidationReport,
  type VoxelAnnotationLayerValidationRequest,
  type VoxelAnnotationQueryReadout,
  type VoxelAnnotationQueryRequest,
  type VoxelEditHistoryReadRequest, type VoxelEditHistoryRedoReceipt, type VoxelEditHistoryRedoRequest,
  type VoxelEditHistoryRevertReceipt, type VoxelEditHistoryRevertRequest, type VoxelEditHistorySummary,
  type VoxelEditHistoryUndoReceipt, type VoxelEditHistoryUndoRequest,
  type VoxelVolumeAssetExportReceipt, type VoxelVolumeAssetExportRequest,
  type VoxelVolumeAssetLoadReceipt, type VoxelVolumeAssetLoadRequest,
  type VoxelVolumeAssetUnloadReceipt, type VoxelVolumeAssetUnloadRequest,
  type VoxelVolumeAssetPaletteUpdateReceipt, type VoxelVolumeAssetPaletteUpdateRequest,
  type VoxelVolumeAssetSaveReceipt, type VoxelVolumeAssetSaveRequest,
  type VoxelVolumeAuthoringInitializeReceipt, type VoxelVolumeAuthoringInitializeRequest,
  type VoxelUpdateTelemetryReadout, type VoxelUpdateTelemetryRequest,
  type GameRuleCatalog,
  type GameRuleResolutionRequest,
  type InputActionReplayReceipt,
  type InputContextChangeReceipt,
  type InputContextCommand,
  type InputContextStackState,
  type InputResolutionReceipt,
  type InputSessionConfigureRequest,
  type InputSessionSnapshot,
  type RawInputSample,
  type RecordedInputAction,
  type TimeControlCommand,
  type TimeControlReceipt,
  type TimeControlState,
  type SceneDocumentCodecResult,
  type SceneDocumentAuthoringRequest,
  type SceneDocumentAuthoringResult,
  type SceneDocumentDecodeRequest,
  type SceneDocumentEncodeRequest,
  type WeaponEffectHookRequest,
} from '@asha/contracts';
import {
  RuntimeBridgeError,
  frameCursor,
  type EnemyDirectNavMovementResult,
  type EngineHandle,
  type FpsPrimaryFireRequest,
  type GameRuleRuntimeReadout,
  type RuntimeBridge,
} from './bridge.js';
import {
  GENERATED_TUNNEL_FIRE_HIT_READOUT,
  GENERATED_TUNNEL_FIRE_MISS_READOUT,
  type CombatRuntimeReadout,
} from '@asha/runtime-session';
import {
  buildCombatFeedbackProjection,
  defaultCombatFeedbackIntent,
  type CombatFeedbackProjection,
} from '@asha/runtime-session';
import {
  createGeneratedTunnelEnemyPolicyFixture,
  validateEnemyPolicySource,
  type EnemyPolicyProposal,
  type EnemyPolicyVec3,
} from '@asha/runtime-session';
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
} from '@asha/runtime-session';
import {
  GENERATED_TUNNEL_NAV_POLICY_VIEW,
  GENERATED_TUNNEL_NAV_PROJECTION,
  GENERATED_TUNNEL_NO_PATH,
  GENERATED_TUNNEL_REACHABLE_PATH,
  type NavPathQueryRequest,
  type NavPathReadout,
  type NavPolicyViewReadout,
  type NavProjectionReadout,
} from '@asha/runtime-session';
import type { RuntimeActionIntentEnvelope } from '@asha/runtime-session';
import {
  buildRuntimeSessionEnemyNavPath,
  ecrpActorPosition,
  ecrpEntityTransform,
  runtimeTransformHashRecord,
} from './runtime-session-enemy-authority.js';
import {
  buildEcrpProjectStateFromCanonical,
  buildEcrpRuntimeReadout,
  lifecycleStateFromEcrpProject,
  type RuntimeSessionEcrpProjectState,
  type RuntimeSessionEcrpTransformState,
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
  validateAutonomousPolicyProposal,
  validateAutonomousPolicyTickInput,
  validateInitializeInput,
  validateLifecycleStatusRequest,
  validateRestartIntent,
  validateRuntimeActionIntentEnvelope,
} from './runtime-session-lifecycle.js';
import {
  encounterStateHashRecord,
  identityHashRecord,
  lifecycleStateHashRecord,
  referenceRuntimeSessionNonClaims,
  renderFrameHashRecord,
  runtimeProjectionFrameHashRecord,
  stableHash,
} from './runtime-session-hash.js';
import { RustBackedRuntimeSessionFacade } from './runtime-session-rust-facade.js';
import { buildRuntimeSessionAnimationIntentReadout } from './runtime-session-animation.js';
import { loadRuntimeSessionProject } from './runtime-project-loader.js';

import type {
  RuntimeSessionActionIntentReceipt,
  RuntimeSessionAnimationIntentReadout,
  RuntimeSessionAutonomousPolicyProposalReceipt,
  RuntimeSessionAutonomousPolicyProposalRejection,
  RuntimeSessionAutonomousPolicyTickInput,
  RuntimeSessionAutonomousPolicyTickReadout,
  RuntimeSessionCameraCollisionInputReceipt,
  RuntimeSessionCameraCreateReceipt,
  RuntimeSessionCameraInputReceipt,
  RuntimeSessionCameraProjectionReadout,
  RuntimeSessionCombatFeedbackProjectionRequest,
  RuntimeSessionCombatReadoutRequest,
  RuntimeSessionCommandReceipt,
  RuntimeSessionEcrpReadout,
  RuntimeSessionFacade,
  RuntimeSessionGameExtensionWeaponEffectReceipt,
  RuntimeSessionGameRuleCatalogValidationReceipt,
  RuntimeSessionGameRuleEffectIntentReceipt,
  RuntimeSessionIdentity,
  RuntimeSessionInitializeInput,
  RuntimeSessionLifecycleRestartReceipt,
  RuntimeSessionLifecycleState,
  RuntimeSessionLifecycleStatusReadout,
  RuntimeSessionLifecycleStatusRequest,
  RuntimeSessionMode,
  RuntimeSessionProjectionSummary,
  RuntimeSessionReplayRecord,
  RuntimeSessionRestartIntent,
  RuntimeSessionRestartIntentRejection,
  RuntimeSessionRestartResult,
  RuntimeSessionStateSummary,
  RuntimeSessionTelemetrySummary,
  RuntimeSessionTickInput,
  RuntimeSessionTickResult,
  RuntimeSessionGameplayCheckpoint,
  RuntimeSessionGameplayCheckpointRestoreReceipt,
  RuntimeSessionGameplayCheckpointSaveReceipt,
  RuntimeSessionProjectCloseReceipt,
  RuntimeSessionProjectLoadInput,
  RuntimeSessionProjectLoadReceipt,
} from '@asha/runtime-session';

export interface RuntimeSessionFacadeOptions {
  readonly bridge: RuntimeBridge;
  readonly mode?: RuntimeSessionMode;
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
  #nextProjectionCursor = 0;
  #lifecycleState: RuntimeSessionLifecycleState = initialRuntimeSessionLifecycleState();
  #encounterState: EncounterDirectorState = initialEncounterDirectorState();
  #ecrpProjectState: RuntimeSessionEcrpProjectState | null = null;
  #runtimeTransforms = new Map<number, RuntimeSessionEcrpTransformState>();
  #replayRecords: RuntimeSessionReplayRecord[] = [];
  #runtimeProjectLifecycle = { generation: 0, revision: 0 };

  constructor(bridge: RuntimeBridge) {
    this.#bridge = bridge;
  }

  #unsupportedOperation(method: string, message: string): never {
    this.#requireInitialized(method);
    throw new RuntimeBridgeError('operation_unimplemented', message);
  }

  initialize(input: RuntimeSessionInitializeInput): RuntimeSessionStateSummary {
    validateInitializeInput(input);
    const engine = this.#bridge.initializeEngine({ seed: input.seed });
    this.#engine = engine;
    this.#identity = {
      sessionId: input.sessionId,
      mode: 'reference',
      seed: input.seed,
      project: input.project,
      nonClaims: referenceRuntimeSessionNonClaims(),
    };
    this.#sequenceId = 0;
    this.#tick = 0;
    this.#acceptedCommandCount = 0;
    this.#rejectedCommandCount = 0;
    this.#nextProjectionCursor = 0;
    this.#ecrpProjectState = null;
    this.#lifecycleState = initialRuntimeSessionLifecycleState();
    this.#runtimeTransforms = new Map();
    this.#encounterState = initialEncounterDirectorState();
    this.#replayRecords = [];
    this.#runtimeProjectLifecycle = { generation: 0, revision: 0 };
    this.#record('initialize');
    return this.#stateSummary();
  }

  async loadProject(input: RuntimeSessionProjectLoadInput): Promise<RuntimeSessionProjectLoadReceipt> {
    this.#requireInitialized('loadProject');
    const receipt = await loadRuntimeSessionProject(
      this.#bridge,
      input,
      this.#runtimeProjectLifecycle,
    );
    this.#runtimeProjectLifecycle = receipt.lifecycle;
    if (receipt.accepted) {
      this.#ecrpProjectState = buildEcrpProjectStateFromCanonical(
        this.#bridge.readActiveRuntimeProjectContent(),
      );
      this.#lifecycleState = lifecycleStateFromEcrpProject(this.#ecrpProjectState);
      this.#runtimeTransforms = new Map();
    }
    this.#sequenceId += 1;
    this.#record('loadProject');
    return receipt;
  }

  closeProject(): RuntimeSessionProjectCloseReceipt {
    this.#requireInitialized('closeProject');
    const receipt = this.#bridge.closeRuntimeProject({
      expectedLifecycle: this.#runtimeProjectLifecycle,
    });
    this.#runtimeProjectLifecycle = receipt.lifecycle;
    if (receipt.accepted) {
      this.#ecrpProjectState = null;
      this.#runtimeTransforms = new Map();
    }
    this.#sequenceId += 1;
    this.#record('closeProject');
    return receipt;
  }

  saveGameplayCheckpoint(): RuntimeSessionGameplayCheckpointSaveReceipt {
    this.#requireInitialized('saveGameplayCheckpoint');
    return this.#bridge.saveRuntimeProjectGameplayCheckpoint({
      expectedLifecycle: this.#runtimeProjectLifecycle,
    });
  }

  restoreGameplayCheckpoint(
    checkpoint: RuntimeSessionGameplayCheckpoint,
  ): RuntimeSessionGameplayCheckpointRestoreReceipt {
    this.#requireInitialized('restoreGameplayCheckpoint');
    const receipt = this.#bridge.restoreRuntimeProjectGameplayCheckpoint({
      expectedLifecycle: this.#runtimeProjectLifecycle,
      checkpoint,
    });
    this.#runtimeProjectLifecycle = receipt.lifecycle;
    if (receipt.accepted) {
      this.#tick = checkpoint.authorityTick;
      this.#ecrpProjectState = buildEcrpProjectStateFromCanonical(
        this.#bridge.readActiveRuntimeProjectContent(),
      );
      this.#lifecycleState = lifecycleStateFromEcrpProject(this.#ecrpProjectState);
      this.#runtimeTransforms = new Map();
    }
    this.#sequenceId += 1;
    this.#record('restoreGameplayCheckpoint');
    return receipt;
  }

  configureInputSession(request: InputSessionConfigureRequest): InputSessionSnapshot {
    this.#requireInitialized('configureInputSession');
    return this.#bridge.configureInputSession(request);
  }

  applyInputContextCommand(command: InputContextCommand): InputContextChangeReceipt {
    this.#requireInitialized('applyInputContextCommand');
    return this.#bridge.applyInputContextCommand(command);
  }

  submitRawInput(sample: RawInputSample): InputResolutionReceipt {
    this.#requireInitialized('submitRawInput');
    return this.#bridge.submitRawInput(sample);
  }

  replayResolvedInputAction(record: RecordedInputAction): InputActionReplayReceipt {
    this.#requireInitialized('replayResolvedInputAction');
    return this.#bridge.replayResolvedInputAction(record);
  }

  readInputContextState(): InputContextStackState {
    this.#requireInitialized('readInputContextState');
    return this.#bridge.readInputContextState();
  }

  applyTimeControlCommand(command: TimeControlCommand): TimeControlReceipt {
    this.#requireInitialized('applyTimeControlCommand');
    return this.#bridge.applyTimeControlCommand(command);
  }

  readTimeControlState(): TimeControlState {
    this.#requireInitialized('readTimeControlState');
    return this.#bridge.readTimeControlState();
  }

  decodeSceneDocument(request: SceneDocumentDecodeRequest): SceneDocumentCodecResult {
    this.#requireInitialized('decodeSceneDocument');
    return this.#bridge.decodeSceneDocument(request);
  }

  encodeSceneDocument(request: SceneDocumentEncodeRequest): SceneDocumentCodecResult {
    this.#requireInitialized('encodeSceneDocument');
    return this.#bridge.encodeSceneDocument(request);
  }

  applySceneDocumentAuthoring(request: SceneDocumentAuthoringRequest): SceneDocumentAuthoringResult {
    this.#requireInitialized('applySceneDocumentAuthoring');
    return this.#bridge.applySceneDocumentAuthoring(request);
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

  readVoxelUpdateTelemetry(request: VoxelUpdateTelemetryRequest): VoxelUpdateTelemetryReadout {
    this.#requireInitialized('readVoxelUpdateTelemetry');
    return this.#bridge.readVoxelUpdateTelemetry(request);
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

  applyCameraModeCommand(command: CameraModeCommand): CameraModeChangeReceipt {
    this.#requireInitialized('applyCameraModeCommand');
    const receipt = this.#bridge.applyCameraModeCommand(command);
    this.#sequenceId += 1;
    this.#record('applyCameraModeCommand');
    return receipt;
  }

  applyCameraNavigationInput(input: CameraNavigationInputEnvelope): CameraNavigationReceipt {
    this.#requireInitialized('applyCameraNavigationInput');
    const receipt = this.#bridge.applyCameraNavigationInput(input);
    this.#sequenceId += 1;
    this.#record('applyCameraNavigationInput');
    return receipt;
  }

  readCameraControllerState(request: CameraControllerReadRequest): CameraControllerState {
    this.#requireInitialized('readCameraControllerState');
    return this.#bridge.readCameraControllerState(request);
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
      resetHash: statusAfter.reset.resetHash,
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

  planVoxelConversion(_request: VoxelConversionPlanRequest): VoxelConversionPlan { void _request; return this.#unsupportedOperation('planVoxelConversion', 'Voxel conversion authority is not wired into the reference RuntimeSession'); }

  registerVoxelConversionSource(_request: VoxelConversionSourceRegistrationRequest): VoxelConversionSourceRegistration { void _request; return this.#unsupportedOperation('registerVoxelConversionSource', 'Voxel conversion source registration is not wired into the reference RuntimeSession'); }

  registerVoxelConversionMeshAsset(_request: VoxelConversionMeshAssetRegistrationRequest): VoxelConversionSourceRegistration { void _request; return this.#unsupportedOperation('registerVoxelConversionMeshAsset', 'Voxel conversion mesh asset registration is not wired into the reference RuntimeSession'); }

  importVoxelConversionMeshSource(_request: VoxelConversionMeshSourceImportRequest): VoxelConversionMeshSourceImportReceipt { void _request; return this.#unsupportedOperation('importVoxelConversionMeshSource', 'Voxel conversion mesh source import is not wired into the reference RuntimeSession'); }

  readVoxelConversionSourceMetadata(_request: VoxelConversionSourceMetadataRequest): VoxelConversionSourceMetadataReadout { void _request; return this.#unsupportedOperation('readVoxelConversionSourceMetadata', 'Voxel conversion source metadata is not wired into the reference RuntimeSession'); }

  previewVoxelConversion(_request: VoxelConversionPreviewRequest): VoxelConversionPreview { void _request; return this.#unsupportedOperation('previewVoxelConversion', 'Voxel conversion preview is not wired into the reference RuntimeSession'); }

  applyVoxelConversion(_request: VoxelConversionApplyRequest): VoxelConversionReceipt { void _request; return this.#unsupportedOperation('applyVoxelConversion', 'Voxel conversion apply is not wired into the reference RuntimeSession'); }

  exportVoxelConversionEvidence(_evidence: readonly VoxelConversionEvidenceRef[]): readonly VoxelConversionEvidenceRef[] { void _evidence; return this.#unsupportedOperation('exportVoxelConversionEvidence', 'Voxel conversion evidence export is not wired into the reference RuntimeSession'); }

  readVoxelModelInfo(_request: VoxelModelInfoRequest): VoxelModelInfoReadout { void _request; return this.#unsupportedOperation('readVoxelModelInfo', 'Voxel model info is not wired into the reference RuntimeSession'); }

  readVoxelModelWindow(_request: VoxelModelWindowRequest): VoxelModelWindowReadout { void _request; return this.#unsupportedOperation('readVoxelModelWindow', 'Voxel model window is not wired into the reference RuntimeSession'); }

  exportVoxelVolumeAsset(_request: VoxelVolumeAssetExportRequest): VoxelVolumeAssetExportReceipt { void _request; return this.#unsupportedOperation('exportVoxelVolumeAsset', 'Voxel volume asset export is not wired into the reference RuntimeSession'); }

  saveVoxelVolumeAsset(_request: VoxelVolumeAssetSaveRequest): VoxelVolumeAssetSaveReceipt { void _request; return this.#unsupportedOperation('saveVoxelVolumeAsset', 'Voxel volume asset save is not wired into the reference RuntimeSession'); }

  updateVoxelVolumeAssetPalette(_request: VoxelVolumeAssetPaletteUpdateRequest): VoxelVolumeAssetPaletteUpdateReceipt { void _request; return this.#unsupportedOperation('updateVoxelVolumeAssetPalette', 'Durable voxel palette updates are not wired into the reference RuntimeSession'); }

  initializeVoxelVolumeAuthoring(_request: VoxelVolumeAuthoringInitializeRequest): VoxelVolumeAuthoringInitializeReceipt { void _request; return this.#unsupportedOperation('initializeVoxelVolumeAuthoring', 'Voxel volume authoring initialization is not wired into the reference RuntimeSession'); }

  loadVoxelVolumeAsset(_request: VoxelVolumeAssetLoadRequest): VoxelVolumeAssetLoadReceipt { void _request; return this.#unsupportedOperation('loadVoxelVolumeAsset', 'Voxel volume asset load is not wired into the reference RuntimeSession'); }

  unloadVoxelVolumeAsset(_request: VoxelVolumeAssetUnloadRequest): VoxelVolumeAssetUnloadReceipt { void _request; return this.#unsupportedOperation('unloadVoxelVolumeAsset', 'Voxel volume asset unload is not wired into the reference RuntimeSession'); }

  validateVoxelAnnotationLayer(_request: VoxelAnnotationLayerValidationRequest): VoxelAnnotationLayerValidationReport { void _request; return this.#unsupportedOperation('validateVoxelAnnotationLayer', 'Voxel annotation validation is not wired into the reference RuntimeSession'); }

  loadVoxelAnnotationLayer(_request: VoxelAnnotationLayerLoadRequest): VoxelAnnotationLayerLoadReceipt { void _request; return this.#unsupportedOperation('loadVoxelAnnotationLayer', 'Voxel annotation load is not wired into the reference RuntimeSession'); }

  readVoxelAnnotationQuery(_request: VoxelAnnotationQueryRequest): VoxelAnnotationQueryReadout { void _request; return this.#unsupportedOperation('readVoxelAnnotationQuery', 'Voxel annotation query is not wired into the reference RuntimeSession'); }

  applyVoxelAnnotationEdit(_request: VoxelAnnotationEditRequest): VoxelAnnotationEditReceipt { void _request; return this.#unsupportedOperation('applyVoxelAnnotationEdit', 'Voxel annotation edit is not wired into the reference RuntimeSession'); }

  exportVoxelAnnotationLayer(_request: VoxelAnnotationLayerExportRequest): VoxelAnnotationLayerExportReceipt { void _request; return this.#unsupportedOperation('exportVoxelAnnotationLayer', 'Voxel annotation export is not wired into the reference RuntimeSession'); }

  readVoxelEditHistory(_request: VoxelEditHistoryReadRequest): VoxelEditHistorySummary { void _request; return this.#unsupportedOperation('readVoxelEditHistory', 'Voxel edit history authority is not wired into the reference RuntimeSession'); }
  previewVoxelEditRevert(_request: VoxelEditHistoryRevertRequest): VoxelEditHistoryRevertReceipt { void _request; return this.#unsupportedOperation('previewVoxelEditRevert', 'Voxel edit history authority is not wired into the reference RuntimeSession'); }
  applyVoxelEditRevert(_request: VoxelEditHistoryRevertRequest): VoxelEditHistoryRevertReceipt { void _request; return this.#unsupportedOperation('applyVoxelEditRevert', 'Voxel edit history authority is not wired into the reference RuntimeSession'); }
  undoVoxelEdit(_request: VoxelEditHistoryUndoRequest): VoxelEditHistoryUndoReceipt { void _request; return this.#unsupportedOperation('undoVoxelEdit', 'Voxel edit history authority is not wired into the reference RuntimeSession'); }
  redoVoxelEdit(_request: VoxelEditHistoryRedoRequest): VoxelEditHistoryRedoReceipt { void _request; return this.#unsupportedOperation('redoVoxelEdit', 'Voxel edit history authority is not wired into the reference RuntimeSession'); }

  readEcrpRuntimeReadout(): RuntimeSessionEcrpReadout {
    const identity = this.#requireInitialized('readEcrpRuntimeReadout');
    return buildEcrpRuntimeReadout({
      identity,
      projectState: this.#requireEcrpProjectState(),
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
  readAnimationIntent(): RuntimeSessionAnimationIntentReadout { this.#requireInitialized('readAnimationIntent'); return buildRuntimeSessionAnimationIntentReadout({ sequenceId: this.#sequenceId, tick: this.#tick, lifecycleState: this.#lifecycleState }); }
  readProjection(): RuntimeSessionProjectionSummary {
    this.#requireInitialized('readProjection');
    const authorityCursor = frameCursor(this.#tick);
    const cursor = frameCursor(this.#nextProjectionCursor);
    const projectedRuntimeFrame = this.#bridge.readProjectionFrame(authorityCursor);
    const retainedScene = this.#bridge.readRenderDiffs(cursor);
    this.#nextProjectionCursor += 1;
    const frame = {
      ops: [...projectedRuntimeFrame.scene.ops, ...retainedScene.ops],
    };
    const runtimeFrame = {
      ...projectedRuntimeFrame,
      scene: frame,
    };
    return {
      sequenceId: this.#sequenceId,
      cursor,
      frame,
      runtimeFrame,
      renderDiffCount: frame.ops.length,
      presentationOpCount: runtimeFrame.presentation.ops.length,
      projectionHash: stableHash({
        sequenceId: this.#sequenceId,
        cursor,
        frame: renderFrameHashRecord(frame),
        runtimeFrame: runtimeProjectionFrameHashRecord(runtimeFrame),
      }),
    };
  }

  readDeveloperConsole() {
    this.#requireInitialized('readDeveloperConsole');
    return this.#bridge.readDeveloperConsole();
  }

  readTelemetry(): RuntimeSessionTelemetrySummary {
    this.#requireInitialized('readTelemetry');
    return {
      sequenceId: this.#sequenceId,
      tick: this.#tick,
      acceptedCommandCount: this.#acceptedCommandCount,
      rejectedCommandCount: this.#rejectedCommandCount,
      restartCount: this.#restartCount,
      sessionHash: this.#sessionHash(),
      replayRecords: [...this.#replayRecords],
    };
  }

  restart(): RuntimeSessionRestartResult {
    this.#requireInitialized('restart');
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
      resetHash: statusAfter.reset.resetHash,
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

  #requireEcrpProjectState(): RuntimeSessionEcrpProjectState {
    if (this.#ecrpProjectState === null) {
      throw new RuntimeBridgeError(
        'not_initialized',
        'ECRP runtime readout is unavailable before an admitted project is active',
      );
    }
    return this.#ecrpProjectState;
  }

  #stateSummary(): RuntimeSessionStateSummary {
    const identity = this.#requireInitialized('stateSummary');
    return {
      identity,
      engine: this.#engine as EngineHandle,
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
        activeProjectContent: this.#ecrpProjectState?.contentHash ?? null,
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
      activeProjectContent: this.#ecrpProjectState?.contentHash ?? null,
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
