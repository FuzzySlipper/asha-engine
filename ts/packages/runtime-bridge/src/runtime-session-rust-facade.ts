import type {
  CameraCreateRequest,
  CameraControllerReadRequest,
  CameraControllerState,
  CameraModeChangeReceipt,
  CameraModeCommand,
  CameraNavigationInputEnvelope,
  CameraNavigationReceipt,
  CameraProjectionRequest,
  CollisionConstrainedCameraInputEnvelope,
  CommandBatch,
  DeveloperConsoleSnapshot,
  FirstPersonCameraInputEnvelope,
  VoxelConversionApplyRequest,
  VoxelConversionEvidenceRef,
  VoxelConversionMeshAssetRegistrationRequest,
  VoxelConversionMeshSourceImportReceipt,
  VoxelConversionMeshSourceImportRequest,
  VoxelConversionPlan,
  VoxelConversionPlanRequest,
  VoxelConversionPreview,
  VoxelConversionPreviewRequest,
  VoxelConversionReceipt,
  VoxelConversionSourceMetadataReadout,
  VoxelConversionSourceMetadataRequest,
  VoxelConversionSourceRegistration,
  VoxelConversionSourceRegistrationRequest,
  VoxelModelInfoReadout,
  VoxelModelInfoRequest,
  VoxelModelWindowReadout,
  VoxelModelWindowRequest,
  VoxelAnnotationEditReceipt,
  VoxelAnnotationEditRequest,
  VoxelAnnotationLayerExportReceipt,
  VoxelAnnotationLayerExportRequest,
  VoxelAnnotationLayerLoadReceipt,
  VoxelAnnotationLayerLoadRequest,
  VoxelAnnotationLayerValidationReport,
  VoxelAnnotationLayerValidationRequest,
  VoxelAnnotationQueryReadout,
  VoxelAnnotationQueryRequest,
  VoxelEditHistoryReadRequest,
  VoxelEditHistoryRedoReceipt,
  VoxelEditHistoryRedoRequest,
  VoxelEditHistoryRevertReceipt,
  VoxelEditHistoryRevertRequest,
  VoxelEditHistorySummary,
  VoxelEditHistoryUndoReceipt,
  VoxelEditHistoryUndoRequest,
  VoxelVolumeAssetExportReceipt,
  VoxelVolumeAssetExportRequest,
  VoxelVolumeAssetLoadReceipt,
  VoxelVolumeAssetLoadRequest,
  VoxelVolumeAssetUnloadReceipt,
  VoxelVolumeAssetUnloadRequest,
  VoxelVolumeAssetPaletteUpdateReceipt,
  VoxelVolumeAssetPaletteUpdateRequest,
  VoxelVolumeAssetSaveReceipt,
  VoxelVolumeAssetSaveRequest,
  VoxelVolumeAuthoringInitializeReceipt,
  VoxelVolumeAuthoringInitializeRequest,
  WeaponEffectHookRequest,
  GameRuleCatalog,
  GameRuleResolutionRequest,
  InputActionReplayReceipt,
  InputContextChangeReceipt,
  InputContextCommand,
  InputContextStackState,
  InputResolutionReceipt,
  InputSessionConfigureRequest,
  InputSessionSnapshot,
  RawInputSample,
  RecordedInputAction,
  TimeControlCommand,
  TimeControlReceipt,
  TimeControlState,
  SceneDocumentCodecResult,
  SceneDocumentAuthoringRequest,
  SceneDocumentAuthoringResult,
  SceneDocumentDecodeRequest,
  SceneDocumentEncodeRequest,
} from '@asha/contracts';
import {
  RuntimeBridgeError,
  frameCursor,
  type CompositionStatus,
  type FpsBoundsCapability,
  type EngineHandle,
  type FpsPrimaryFireRequest,
  type FpsPrimaryFireResult,
  type FpsRuntimeSessionLoadRequest,
  type FpsRuntimeSessionSnapshot,
  type FpsStoredEntityDefinition,
  type FpsTransformCapability,
  type EnemyDirectNavMovementResult,
  type GameRuleRuntimeReadout,
  type RuntimeBridge,
} from './bridge.js';
import type { CombatRuntimeReadout } from '@asha/runtime-session';
import type { CombatFeedbackProjection } from '@asha/runtime-session';
import {
  createGeneratedTunnelEnemyPolicyFixture,
  validateEnemyPolicySource,
  type EnemyPolicyProposal,
  type EnemyPolicyVec3,
} from '@asha/runtime-session';
import type {
  GeneratedTunnelOperationRequest,
  GeneratedTunnelReadout,
  GeneratedTunnelReadoutRequest,
} from '@asha/runtime-session';
import type {
  EncounterDirectorReadout,
  EncounterDirectorReadoutRequest,
  EncounterTransitionRequest,
  RuntimeSessionEncounterTransitionReceipt,
} from '@asha/runtime-session';
import {
  buildEncounterTransitionReceipt,
  validateEncounterDirectorReadoutRequest,
  validateEncounterTransitionRequest,
} from '@asha/runtime-session';
import type {
  NavPathQueryRequest,
  NavPathReadout,
  NavPolicyViewReadout,
  NavProjectionReadout,
} from '@asha/runtime-session';
import {
  GENERATED_TUNNEL_NAV_POLICY_VIEW,
  GENERATED_TUNNEL_NO_PATH,
  GENERATED_TUNNEL_REACHABLE_PATH,
} from '@asha/runtime-session';
import type { RuntimeActionIntentEnvelope } from '@asha/runtime-session';
import {
  buildRuntimeSessionEnemyNavPath,
  ecrpActorPosition,
  ecrpEntityTransform,
} from './runtime-session-enemy-authority.js';
import {
  buildRuntimeSessionAnimationIntentReadout,
  type RuntimeSessionAnimationIntentReadout,
} from './runtime-session-animation.js';
import {
  buildEcrpProjectState,
  buildEcrpRuntimeReadout,
  defaultRuntimeSessionEcrpProjectLoadInput,
  validateEcrpProjectLoadInput,
} from './runtime-session-ecrp.js';
import {
  acceptedAutonomousMovementReceipt,
  lifecycleStatusReadout,
  lifecycleStatusToEncounterLifecycle,
  rejectedAutonomousPolicyProposalReceipt,
  runtimeActionReceiptToAutonomousReceipt,
  validateAutonomousPolicyProposal,
  validateAutonomousPolicyTickInput,
  validateGeneratedTunnelOperationRequest,
  validateInitializeInput,
  validateLifecycleStatusRequest,
  validateRestartIntent,
  validateRuntimeActionIntentEnvelope,
} from './runtime-session-lifecycle.js';
import {
  compositionHashRecord,
  identityHashRecord,
  renderFrameHashRecord,
  runtimeProjectionFrameHashRecord,
  stableHash,
} from './runtime-session-hash.js';
import { RuntimeSessionProgress } from './runtime-session-rust-progress.js';
import {
  encounterReadoutFromFpsSnapshot,
  encounterTransitionResultForReceipt,
  fpsEncounterLifecycleInput,
  fpsEncounterStateToReadoutState,
} from './runtime-session-rust-encounter.js';
import type {
  RuntimeSessionActionIntentReceipt,
  RuntimeSessionAutonomousPolicyProposalReceipt,
  RuntimeSessionAutonomousPolicyProposalRejection,
  RuntimeSessionAutonomousPolicyTickInput,
  RuntimeSessionAutonomousPolicyTickReadout,
  RuntimeSessionCameraCollisionInputReceipt,
  RuntimeSessionCameraCreateReceipt,
  RuntimeSessionCameraInputReceipt,
  RuntimeSessionCameraProjectionReadout,
  RuntimeSessionCommandReceipt,
  RuntimeSessionCombatFeedbackProjectionRequest,
  RuntimeSessionCombatReadoutRequest,
  RuntimeSessionEcrpEntityState,
  RuntimeSessionEcrpProjectCapabilityDefinition,
  RuntimeSessionEcrpProjectLoadInput,
  RuntimeSessionEcrpProjectLoadReceipt,
  RuntimeSessionEcrpProjectState,
  RuntimeSessionEcrpReadout,
  RuntimeSessionEcrpTransformState,
  RuntimeSessionFacade,
  RuntimeSessionGeneratedTunnelOperationReceipt,
  RuntimeSessionGameRuleCatalogValidationReceipt,
  RuntimeSessionGameRuleEffectIntentReceipt,
  RuntimeSessionGameExtensionWeaponEffectReceipt,
  RuntimeSessionIdentity,
  RuntimeSessionInitializeInput,
  RuntimeSessionLifecycleHealthReadout,
  RuntimeSessionLifecycleRestartReceipt,
  RuntimeSessionLifecycleState,
  RuntimeSessionLifecycleStatusReadout,
  RuntimeSessionLifecycleStatusRequest,
  RuntimeSessionNonClaim,
  RuntimeSessionProjectionSummary,
  RuntimeSessionReplayRecord,
  RuntimeSessionRestartIntent,
  RuntimeSessionRestartIntentRejection,
  RuntimeSessionRestartResult,
  RuntimeSessionStateSummary,
  RuntimeSessionTelemetrySummary,
  RuntimeSessionTickInput,
  RuntimeSessionTickResult,
  RuntimeSessionProjectCloseReceipt,
  RuntimeSessionProjectLoadInput,
  RuntimeSessionProjectLoadReceipt,
} from '@asha/runtime-session';
import { loadRuntimeSessionProject } from './runtime-project-loader.js';

export class RustBackedRuntimeSessionFacade implements RuntimeSessionFacade {
  readonly #bridge: RuntimeBridge;
  #identity: RuntimeSessionIdentity | null = null;
  #engine: EngineHandle | null = null;
  readonly #progress = new RuntimeSessionProgress();
  #snapshot: FpsRuntimeSessionSnapshot | null = null;
  #ecrpProjectState: RuntimeSessionEcrpProjectState | null = null;
  #runtimeTransforms = new Map<number, RuntimeSessionEcrpTransformState>();
  #replayRecords: RuntimeSessionReplayRecord[] = [];
  #runtimeProjectLifecycle = { generation: 0, revision: 0 };
  constructor(bridge: RuntimeBridge) {
    this.#bridge = bridge;
  }

  initialize(input: RuntimeSessionInitializeInput): RuntimeSessionStateSummary {
    validateInitializeInput(input);
    const engine = this.#bridge.initializeEngine({ seed: input.seed });
    const composition = input.projectBundle === undefined
      ? this.#bridge.getProjectBundleCompositionStatus()
      : this.#bridge.loadProjectBundle(input.projectBundle); // vocab-allow: compatibility initialization only.
    const defaultProject = input.projectBundle === undefined
      ? null
      : defaultRuntimeSessionEcrpProjectLoadInput(input as RuntimeSessionInitializeInput & { readonly projectBundle: NonNullable<RuntimeSessionInitializeInput['projectBundle']> });
    const snapshot = defaultProject === null
      ? null
      : this.#bridge.loadFpsRuntimeSession(fpsLoadRequestFromEcrpProject(defaultProject));
    this.#engine = engine;
    this.#identity = {
      sessionId: input.sessionId,
      mode: 'rust',
      seed: input.seed,
      project: input.project,
      projectBundle: input.projectBundle ?? null,
      nonClaims: rustRuntimeSessionNonClaims(),
    };
    this.#progress.initialize();
    this.#snapshot = snapshot;
    this.#ecrpProjectState = defaultProject === null ? null : buildEcrpProjectState(defaultProject);
    this.#runtimeTransforms = new Map();
    this.#replayRecords = [];
    this.#runtimeProjectLifecycle = { generation: 0, revision: 0 };
    this.#record('initialize', snapshot?.replayHash);
    return this.#stateSummary(composition);
  }

  async loadProject(input: RuntimeSessionProjectLoadInput): Promise<RuntimeSessionProjectLoadReceipt> {
    this.#requireInitialized('loadProject');
    const receipt = await loadRuntimeSessionProject(
      this.#bridge,
      input,
      this.#runtimeProjectLifecycle,
    );
    this.#runtimeProjectLifecycle = receipt.lifecycle;
    this.#progress.advanceSequence();
    this.#record('loadProject', receipt.activeProject?.admissionHash);
    return receipt;
  }

  closeProject(): RuntimeSessionProjectCloseReceipt {
    this.#requireInitialized('closeProject');
    const receipt = this.#bridge.closeRuntimeProject({
      expectedLifecycle: this.#runtimeProjectLifecycle,
    });
    this.#runtimeProjectLifecycle = receipt.lifecycle;
    if (receipt.accepted) {
      this.#snapshot = null;
      this.#ecrpProjectState = null;
      this.#runtimeTransforms = new Map();
    }
    this.#progress.advanceSequence();
    this.#record('closeProject', receipt.closedManifestHash ?? undefined);
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

  loadEcrpProject(input: RuntimeSessionEcrpProjectLoadInput): RuntimeSessionEcrpProjectLoadReceipt {
    const identity = this.#requireInitialized('loadEcrpProject');
    const before = this.#sessionHash();
    const diagnostics = validateEcrpProjectLoadInput(input);

    if (diagnostics.length > 0) {
      this.#progress.advanceSequence();
      this.#record('loadEcrpProject');
      return {
        kind: 'runtime_session.ecrp_project_load_receipt.v0',
        sequenceId: this.#progress.sequenceId,
        accepted: false,
        diagnostics,
        entityCount: 0,
        bootstrapHash: null,
        sessionHashBefore: before,
        sessionHashAfter: this.#sessionHash(),
      };
    }

    this.#bridge.loadProjectBundle(input.projectBundle.runtimeRequest); // vocab-allow: RuntimeSession ECRP load adapts the legacy bridge operation.
    const snapshot = this.#bridge.loadFpsRuntimeSession(fpsLoadRequestFromEcrpProject(input));
    this.#progress.advanceSequence();
    this.#identity = {
      ...identity,
      project: input.projectBundle.project,
      projectBundle: input.projectBundle.runtimeRequest,
    };
    this.#snapshot = snapshot;
    this.#ecrpProjectState = buildEcrpProjectState(input);
    this.#runtimeTransforms = new Map();
    this.#record('loadEcrpProject', snapshot.replayHash);
    return {
      kind: 'runtime_session.ecrp_project_load_receipt.v0',
      sequenceId: this.#progress.sequenceId,
      accepted: true,
      diagnostics: [],
      entityCount: snapshot.health.length,
      bootstrapHash: snapshot.entityHash,
      sessionHashBefore: before,
      sessionHashAfter: this.#sessionHash(),
    };
  }

  submitCommands(batch: CommandBatch): RuntimeSessionCommandReceipt {
    this.#requireInitialized('submitCommands');
    const before = this.#sessionHash();
    const result = this.#bridge.submitCommands(batch);
    this.#progress.recordCommandBatch(result.accepted, result.rejected);
    this.#record('submitCommands');
    return {
      sequenceId: this.#progress.sequenceId,
      batch,
      result,
      acceptedCommandCount: this.#progress.acceptedCommandCount,
      rejectedCommandCount: this.#progress.rejectedCommandCount,
      sessionHashBefore: before,
      sessionHashAfter: this.#sessionHash(),
    };
  }

  tick(input: RuntimeSessionTickInput = {}): RuntimeSessionTickResult {
    this.#requireInitialized('tick');
    const nextTick = this.#progress.nextSimulationTick(input.tick);
    const step = this.#bridge.stepSimulation({ tick: nextTick });
    this.#progress.recordSimulationTick(step.tick);
    this.#record('tick');
    return {
      sequenceId: this.#progress.sequenceId,
      tick: this.#progress.sessionTick,
      step,
      composition: this.#bridge.getProjectBundleCompositionStatus(),
      sessionHash: this.#sessionHash(),
    };
  }

  createCamera(request: CameraCreateRequest): RuntimeSessionCameraCreateReceipt {
    this.#requireInitialized('createCamera');
    const snapshot = this.#bridge.createCamera(request);
    this.#progress.advanceSequence();
    this.#record('createCamera');
    return {
      sequenceId: this.#progress.sequenceId,
      request,
      snapshot,
      sessionHash: this.#sessionHash(),
    };
  }

  applyCameraModeCommand(command: CameraModeCommand): CameraModeChangeReceipt {
    this.#requireInitialized('applyCameraModeCommand');
    const receipt = this.#bridge.applyCameraModeCommand(command);
    this.#progress.advanceSequence();
    this.#record('applyCameraModeCommand');
    return receipt;
  }
  applyCameraNavigationInput(input: CameraNavigationInputEnvelope): CameraNavigationReceipt {
    this.#requireInitialized('applyCameraNavigationInput');
    const receipt = this.#bridge.applyCameraNavigationInput(input);
    this.#progress.advanceSequence();
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
    this.#progress.advanceSequence();
    this.#record('applyFirstPersonCameraInput');
    return {
      sequenceId: this.#progress.sequenceId,
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
    this.#progress.advanceSequence();
    this.#record('applyCollisionConstrainedCameraInput');
    return {
      sequenceId: this.#progress.sequenceId,
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

    if (envelope.action !== 'primary_fire' || envelope.phase !== 'pressed') {
      this.#progress.advanceSequence();
      this.#record('submitRuntimeActionIntent', undefined, envelope.source);
      return {
        sequenceId: this.#progress.sequenceId,
        envelope,
        accepted: envelope.action === 'primary_fire' && envelope.phase === 'released',
        status: envelope.action === 'primary_fire' && envelope.phase === 'released' ? 'accepted' : 'unsupported',
        rejection: envelope.action === 'primary_fire' && envelope.phase === 'released'
          ? null
          : {
              reason: 'combat_runtime_not_wired',
              detail: 'Rust-backed RuntimeSession only accepts pressed primary_fire intents in this authority slice.',
            },
        combatReadout: null,
        sessionHashBefore: before,
        sessionHashAfter: this.#sessionHash(),
      };
    }

    const camera = this.#bridge.readCameraControllerState({ camera: envelope.camera }).snapshot;
    this.#progress.advanceSequence();
    const fire = this.#bridge.applyFpsPrimaryFire({
      tick: envelope.tick,
      origin: camera.pose.position,
      direction: camera.basis.forward,
    });
    this.#progress.recordProjectedAuthorityTick(envelope.tick);
    this.#snapshot = this.#bridge.readFpsRuntimeSession();
    this.#record('submitRuntimeActionIntent', fire.replayHash, envelope.source);
    return {
      sequenceId: this.#progress.sequenceId,
      envelope,
      accepted: true,
      status: 'accepted',
      rejection: null,
      combatReadout: combatReadoutFromFpsPrimaryFire(fire, envelope.tick),
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
    if (result.primaryFire !== null) {
      this.#progress.recordProjectionTick(primaryFire.tick);
    }
    this.#progress.observeAuthorityTick(primaryFire.tick);
    this.#snapshot = this.#bridge.readFpsRuntimeSession();
    this.#progress.advanceSequence();
    this.#record('submitGameExtensionWeaponEffect', result.replayEvidence.replayHash);
    return {
      sequenceId: this.#progress.sequenceId,
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
    this.#progress.advanceSequence();
    this.#record('validateGameRuleCatalog', receipt.evidence.at(-1)?.contentHash);
    return {
      ...receipt,
      sequenceId: this.#progress.sequenceId,
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
    this.#progress.advanceSequence();
    this.#record('submitGameRuleEffectIntent', receipt.replayHash);
    return {
      ...receipt,
      sequenceId: this.#progress.sequenceId,
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

    const sequenceIdBefore = this.#progress.sequenceId;
    const sessionHashBefore = this.#sessionHash();
    const step = this.tick(input.tick === undefined ? {} : { tick: input.tick });
    const sourceDiagnostics =
      input.policySource === undefined ? [] : validateEnemyPolicySource(input.policySource);
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
      ...(enemyPolicyPosition === undefined ? {} : { enemyPosition: enemyPolicyPosition }),
      ...(targetPolicyPosition === undefined ? {} : { targetPosition: targetPolicyPosition }),
      queryFixturePath: (scenario) => scenario === 'generated_tunnel_no_path'
        ? GENERATED_TUNNEL_NO_PATH
        : GENERATED_TUNNEL_REACHABLE_PATH,
    });
    const navPolicyView: NavPolicyViewReadout = {
      ...GENERATED_TUNNEL_NAV_POLICY_VIEW,
      latestPath: navPath,
    };
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
        const movement = this.#applyRustAutonomousMovementProposal(proposal, targetPolicyPosition);
        proposalReceipts.push(acceptedAutonomousMovementReceipt(proposal, movement));
        continue;
      }

      const actionReceipt = this.#submitRustEnemyPolicyPrimaryFire(
        proposal,
        fixture.view.enemy.position,
        fixture.view.target.position,
      );
      proposalReceipts.push(runtimeActionReceiptToAutonomousReceipt(proposal, actionReceipt));
    }

    this.#progress.advanceSequence();
    const recordHashesBeforePolicyRecord = this.#replayRecords.map((record) => record.recordHash);
    const movementSummary = proposalReceipts.find((receipt) => receipt.movement !== null)?.movement ?? null;
    const combatSummary = proposalReceipts.find((receipt) => receipt.combat !== null)?.combat ?? null;
    const authorityNavPathHash = movementSummary?.pathHash ?? navPath.pathHash;
    const tickHash = stableHash({
      loopId: 'generated_tunnel_enemy_policy_loop.v0',
      authority: 'rust_bridge',
      tick: step.tick,
      proposalFrameHash: fixture.frame.proposalHash,
      receiptStatuses: proposalReceipts.map((receipt) => receipt.status),
      receiptRejections: proposalReceipts.map((receipt) => receipt.rejection?.reason ?? null),
      navPathHash: authorityNavPathHash,
      replayRecordHashes: recordHashesBeforePolicyRecord,
      sequenceIdAfter: this.#progress.sequenceId,
      runtimeSnapshotReplayHash: this.#snapshot?.replayHash ?? null,
    });
    this.#record('runAutonomousPolicyTick', tickHash);

    const telemetry = this.readTelemetry();
    const acceptedRuntimeActionCount = proposalReceipts.filter(
      (receipt) => receipt.actionReceipt?.accepted === true,
    ).length;
    const rejectedRuntimeActionCount = proposalReceipts.filter(
      (receipt) => receipt.actionReceipt !== null && receipt.actionReceipt.accepted === false,
    ).length;

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
        pathHash: authorityNavPathHash,
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
        recordHashes: telemetry.replayRecords.map((record) => record.recordHash),
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
    if (request.scenario !== undefined && request.scenario !== 'current_session') {
      throw new RuntimeBridgeError('invalid_input', 'Rust-backed RuntimeSession only exposes current_session lifecycle status');
    }
    return lifecycleStatusReadout({
      scenario: 'current_session',
      state: lifecycleStateFromFpsSnapshot(this.#requireSnapshot()),
      identity,
      sequenceId: this.#progress.sequenceId,
      tick: this.#progress.sessionTick,
      restartCount: this.#progress.restartCount,
      sessionHash: this.#sessionHash(),
      restartReason: 'rust_epoch_restart',
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
      resetHash: statusAfter.hashes.replayHash,
    };
  }

  readEncounterDirector(request: EncounterDirectorReadoutRequest = {}): EncounterDirectorReadout {
    const identity = this.#requireInitialized('readEncounterDirector');
    validateEncounterDirectorReadoutRequest(request);
    const lifecycle = this.#encounterLifecycleFromScenario(request.lifecycleScenario);
    const snapshot = this.#bridge.readFpsEncounterDirector(fpsEncounterLifecycleInput(lifecycle));
    return encounterReadoutFromFpsSnapshot({
      snapshot,
      sequenceId: this.#progress.sequenceId,
      tick: this.#progress.sessionTick,
      sessionSeed: identity.seed,
      sessionHash: this.#sessionHash(),
    });
  }

  requestEncounterTransition(request: EncounterTransitionRequest): RuntimeSessionEncounterTransitionReceipt {
    const identity = this.#requireInitialized('requestEncounterTransition');
    const sessionHashBefore = this.#sessionHash();
    const validationRejection = validateEncounterTransitionRequest(request);
    const lifecycle = validationRejection === undefined
      ? this.#encounterLifecycleFromScenario(request.lifecycleScenario)
      : this.#encounterLifecycleFromScenario();
    const beforeSnapshot = this.#bridge.readFpsEncounterDirector(fpsEncounterLifecycleInput(lifecycle));
    const before = encounterReadoutFromFpsSnapshot({
      snapshot: beforeSnapshot,
      sequenceId: this.#progress.sequenceId,
      tick: this.#progress.sessionTick,
      sessionSeed: identity.seed,
      sessionHash: sessionHashBefore,
    });
    const result = validationRejection === undefined
      ? this.#bridge.applyFpsEncounterTransition({
          presetId: request.presetId,
          action: request.action,
          lifecycle: fpsEncounterLifecycleInput(lifecycle),
        })
      : null;

    this.#progress.advanceSequence();
    if (result?.accepted) {
      this.#record('requestEncounterTransition', result.replayHash);
    } else {
      this.#record('requestEncounterTransition');
    }

    const afterSnapshot = result === null
      ? beforeSnapshot
      : {
          ...beforeSnapshot,
          backend: result.backend,
          authoritySurface: result.authoritySurface,
          mutationOwner: result.mutationOwner,
          workspaceTrace: result.workspaceTrace,
          state: result.state,
          lifecycle: result.lifecycle,
          encounterHash: result.encounterHash,
          replayHash: result.replayHash,
        };
    const after = encounterReadoutFromFpsSnapshot({
      snapshot: afterSnapshot,
      sequenceId: this.#progress.sequenceId,
      tick: this.#progress.sessionTick,
      sessionSeed: identity.seed,
      sessionHash: this.#sessionHash(),
    });
    return buildEncounterTransitionReceipt({
      request,
      sequenceId: this.#progress.sequenceId,
      before,
      after,
      result: result === null
        ? {
            accepted: false,
            state: fpsEncounterStateToReadoutState(beforeSnapshot.state),
            rejectionReason: validationRejection ?? 'invalid_encounter_transition',
          }
        : encounterTransitionResultForReceipt(result),
      sessionHashBefore,
      sessionHashAfter: this.#sessionHash(),
    });
  }

  readCombatReadout(_request: RuntimeSessionCombatReadoutRequest = {}): CombatRuntimeReadout {
    void _request;
    this.#requireInitialized('readCombatReadout');
    throw new RuntimeBridgeError('operation_unimplemented', 'Rust-backed combat readout requires an action receipt in this slice');
  }

  readCombatFeedbackProjection(
    _request: RuntimeSessionCombatFeedbackProjectionRequest = {},
  ): CombatFeedbackProjection {
    void _request;
    this.#requireInitialized('readCombatFeedbackProjection');
    throw new RuntimeBridgeError('operation_unimplemented', 'Rust-backed combat feedback projection is not wired yet');
  }

  readGeneratedTunnelReadout(_request: GeneratedTunnelReadoutRequest = {}): GeneratedTunnelReadout {
    void _request;
    this.#requireInitialized('readGeneratedTunnelReadout');
    throw new RuntimeBridgeError('operation_unimplemented', 'Rust-backed generated tunnel readout is not wired yet');
  }

  readNavProjection(): NavProjectionReadout {
    this.#requireInitialized('readNavProjection');
    throw new RuntimeBridgeError('operation_unimplemented', 'Rust-backed nav projection is not wired yet');
  }

  queryNavPath(_request: NavPathQueryRequest = {}): NavPathReadout {
    void _request;
    this.#requireInitialized('queryNavPath');
    throw new RuntimeBridgeError('operation_unimplemented', 'Rust-backed nav path query is not wired yet');
  }

  readNavPolicyView(): NavPolicyViewReadout {
    this.#requireInitialized('readNavPolicyView');
    throw new RuntimeBridgeError('operation_unimplemented', 'Rust-backed nav policy view is not wired yet');
  }

  requestGeneratedTunnelOperation(
    request: GeneratedTunnelOperationRequest,
  ): RuntimeSessionGeneratedTunnelOperationReceipt {
    this.#requireInitialized('requestGeneratedTunnelOperation');
    validateGeneratedTunnelOperationRequest(request);
    const before = this.#sessionHash();
    this.#progress.advanceSequence();
    if (request.operation === 'regenerate') {
      this.#record('requestGeneratedTunnelOperation');
      return {
        sequenceId: this.#progress.sequenceId,
        request,
        operation: request.operation,
        status: 'unsupported',
        reason: 'generated_tunnel_operation_not_wired',
        detail: 'Generated tunnel regeneration remains an authoring operation outside RuntimeSession.',
        sessionHashBefore: before,
        sessionHashAfter: this.#sessionHash(),
      };
    }
    const applied = this.#bridge.applyGeneratedTunnelToRuntimeWorld({
      preset: request.presetId ?? 'tiny-enclosed',
      seed: request.seed ?? 17,
    });
    this.#record('requestGeneratedTunnelOperation', applied.collisionProjectionHash);
    return {
      sequenceId: this.#progress.sequenceId,
      request,
      operation: request.operation,
      status: 'applied',
      presetId: applied.preset,
      seed: applied.seed,
      grid: applied.grid,
      configHash: applied.configHash,
      outputHash: applied.outputHash,
      collisionSourceHash: applied.collisionSourceHash,
      collisionProjectionHash: applied.collisionProjectionHash,
      runtimeFrame: applied.runtimeFrame,
      sessionHashBefore: before,
      sessionHashAfter: this.#sessionHash(),
    };
  }

  planVoxelConversion(request: VoxelConversionPlanRequest): VoxelConversionPlan {
    this.#requireInitialized('planVoxelConversion');
    return this.#bridge.planVoxelConversion(request);
  }

  registerVoxelConversionSource(
    request: VoxelConversionSourceRegistrationRequest,
  ): VoxelConversionSourceRegistration {
    this.#requireInitialized('registerVoxelConversionSource');
    return this.#bridge.registerVoxelConversionSource(request);
  }

  registerVoxelConversionMeshAsset(
    request: VoxelConversionMeshAssetRegistrationRequest,
  ): VoxelConversionSourceRegistration {
    this.#requireInitialized('registerVoxelConversionMeshAsset');
    return this.#bridge.registerVoxelConversionMeshAsset(request);
  }

  importVoxelConversionMeshSource(
    request: VoxelConversionMeshSourceImportRequest,
  ): VoxelConversionMeshSourceImportReceipt {
    this.#requireInitialized('importVoxelConversionMeshSource');
    return this.#bridge.importVoxelConversionMeshSource(request);
  }

  readVoxelConversionSourceMetadata(
    request: VoxelConversionSourceMetadataRequest,
  ): VoxelConversionSourceMetadataReadout {
    this.#requireInitialized('readVoxelConversionSourceMetadata');
    return this.#bridge.readVoxelConversionSourceMetadata(request);
  }

  previewVoxelConversion(request: VoxelConversionPreviewRequest): VoxelConversionPreview {
    this.#requireInitialized('previewVoxelConversion');
    return this.#bridge.previewVoxelConversion(request);
  }

  applyVoxelConversion(request: VoxelConversionApplyRequest): VoxelConversionReceipt {
    this.#requireInitialized('applyVoxelConversion');
    return this.#bridge.applyVoxelConversion(request);
  }

  exportVoxelConversionEvidence(
    evidence: readonly VoxelConversionEvidenceRef[],
  ): readonly VoxelConversionEvidenceRef[] {
    this.#requireInitialized('exportVoxelConversionEvidence');
    return this.#bridge.exportVoxelConversionEvidence(evidence);
  }

  readVoxelModelInfo(request: VoxelModelInfoRequest): VoxelModelInfoReadout {
    this.#requireInitialized('readVoxelModelInfo');
    return this.#bridge.readVoxelModelInfo(request);
  }

  readVoxelModelWindow(request: VoxelModelWindowRequest): VoxelModelWindowReadout {
    this.#requireInitialized('readVoxelModelWindow');
    return this.#bridge.readVoxelModelWindow(request);
  }

  exportVoxelVolumeAsset(request: VoxelVolumeAssetExportRequest): VoxelVolumeAssetExportReceipt {
    this.#requireInitialized('exportVoxelVolumeAsset');
    return this.#bridge.exportVoxelVolumeAsset(request);
  }

  saveVoxelVolumeAsset(request: VoxelVolumeAssetSaveRequest): VoxelVolumeAssetSaveReceipt {
    this.#requireInitialized('saveVoxelVolumeAsset');
    return this.#bridge.saveVoxelVolumeAsset(request);
  }

  updateVoxelVolumeAssetPalette(
    request: VoxelVolumeAssetPaletteUpdateRequest,
  ): VoxelVolumeAssetPaletteUpdateReceipt {
    this.#requireInitialized('updateVoxelVolumeAssetPalette');
    return this.#bridge.updateVoxelVolumeAssetPalette(request);
  }

  initializeVoxelVolumeAuthoring(
    request: VoxelVolumeAuthoringInitializeRequest,
  ): VoxelVolumeAuthoringInitializeReceipt {
    this.#requireInitialized('initializeVoxelVolumeAuthoring');
    return this.#bridge.initializeVoxelVolumeAuthoring(request);
  }

  loadVoxelVolumeAsset(request: VoxelVolumeAssetLoadRequest): VoxelVolumeAssetLoadReceipt {
    this.#requireInitialized('loadVoxelVolumeAsset');
    return this.#bridge.loadVoxelVolumeAsset(request);
  }

  unloadVoxelVolumeAsset(request: VoxelVolumeAssetUnloadRequest): VoxelVolumeAssetUnloadReceipt {
    this.#requireInitialized('unloadVoxelVolumeAsset');
    return this.#bridge.unloadVoxelVolumeAsset(request);
  }

  validateVoxelAnnotationLayer(
    request: VoxelAnnotationLayerValidationRequest,
  ): VoxelAnnotationLayerValidationReport {
    this.#requireInitialized('validateVoxelAnnotationLayer');
    return this.#bridge.validateVoxelAnnotationLayer(request);
  }

  loadVoxelAnnotationLayer(request: VoxelAnnotationLayerLoadRequest): VoxelAnnotationLayerLoadReceipt {
    this.#requireInitialized('loadVoxelAnnotationLayer');
    return this.#bridge.loadVoxelAnnotationLayer(request);
  }

  readVoxelAnnotationQuery(request: VoxelAnnotationQueryRequest): VoxelAnnotationQueryReadout {
    this.#requireInitialized('readVoxelAnnotationQuery');
    return this.#bridge.readVoxelAnnotationQuery(request);
  }

  applyVoxelAnnotationEdit(request: VoxelAnnotationEditRequest): VoxelAnnotationEditReceipt {
    this.#requireInitialized('applyVoxelAnnotationEdit');
    return this.#bridge.applyVoxelAnnotationEdit(request);
  }

  exportVoxelAnnotationLayer(request: VoxelAnnotationLayerExportRequest): VoxelAnnotationLayerExportReceipt {
    this.#requireInitialized('exportVoxelAnnotationLayer');
    return this.#bridge.exportVoxelAnnotationLayer(request);
  }

  readVoxelEditHistory(request: VoxelEditHistoryReadRequest): VoxelEditHistorySummary {
    this.#requireInitialized('readVoxelEditHistory');
    return this.#bridge.readVoxelEditHistory(request);
  }

  previewVoxelEditRevert(request: VoxelEditHistoryRevertRequest): VoxelEditHistoryRevertReceipt {
    this.#requireInitialized('previewVoxelEditRevert');
    return this.#bridge.previewVoxelEditRevert(request);
  }

  applyVoxelEditRevert(request: VoxelEditHistoryRevertRequest): VoxelEditHistoryRevertReceipt {
    this.#requireInitialized('applyVoxelEditRevert');
    return this.#bridge.applyVoxelEditRevert(request);
  }

  undoVoxelEdit(request: VoxelEditHistoryUndoRequest): VoxelEditHistoryUndoReceipt {
    this.#requireInitialized('undoVoxelEdit');
    return this.#bridge.undoVoxelEdit(request);
  }

  redoVoxelEdit(request: VoxelEditHistoryRedoRequest): VoxelEditHistoryRedoReceipt {
    this.#requireInitialized('redoVoxelEdit');
    return this.#bridge.redoVoxelEdit(request);
  }

  readEcrpRuntimeReadout(): RuntimeSessionEcrpReadout {
    const identity = this.#requireInitialized('readEcrpRuntimeReadout');
    const snapshot = this.#requireSnapshot();
    return buildEcrpRuntimeReadout({
      identity,
      projectState: this.#ecrpProjectState,
      lifecycleState: lifecycleStateFromFpsSnapshot(snapshot),
      runtimeTransforms: this.#runtimeTransforms,
      sequenceId: this.#progress.sequenceId,
      tick: this.#progress.sessionTick,
      sessionHash: this.#sessionHash(),
      authority: {
        mode: 'rust',
        source: 'rust_bridge',
        surface: snapshot.authoritySurface,
        readSets: snapshot.readSets,
      },
    });
  }

  readCameraProjection(request: CameraProjectionRequest): RuntimeSessionCameraProjectionReadout {
    this.#requireInitialized('readCameraProjection');
    const snapshot = this.#bridge.readCameraProjection(request);
    return {
      sequenceId: this.#progress.sequenceId,
      request,
      snapshot,
      projectionHash: snapshot.projectionHash,
    };
  }

  readAnimationIntent(): RuntimeSessionAnimationIntentReadout {
    this.#requireInitialized('readAnimationIntent');
    const snapshot = this.#requireSnapshot();
    return buildRuntimeSessionAnimationIntentReadout({
      sequenceId: this.#progress.sequenceId,
      tick: this.#progress.sessionTick,
      lifecycleState: lifecycleStateFromFpsSnapshot(snapshot),
    });
  }

  readProjection(): RuntimeSessionProjectionSummary {
    this.#requireInitialized('readProjection');
    const cursor = frameCursor(this.#progress.latestProjectionTick);
    const projectedRuntimeFrame = this.#bridge.readProjectionFrame(cursor);
    const retainedScene = this.#bridge.readRenderDiffs(cursor);
    const frame = {
      ops: [...projectedRuntimeFrame.scene.ops, ...retainedScene.ops],
    };
    const runtimeFrame = {
      ...projectedRuntimeFrame,
      scene: frame,
    };
    const composition = this.#bridge.getProjectBundleCompositionStatus();
    return {
      sequenceId: this.#progress.sequenceId,
      cursor,
      frame,
      runtimeFrame,
      composition,
      renderDiffCount: frame.ops.length,
      presentationOpCount: runtimeFrame.presentation.ops.length,
      projectionHash: stableHash({
        sequenceId: this.#progress.sequenceId,
        composition: compositionHashRecord(composition),
        frame: renderFrameHashRecord(frame),
        runtimeFrame: runtimeProjectionFrameHashRecord(runtimeFrame),
      }),
    };
  }

  readDeveloperConsole(): DeveloperConsoleSnapshot {
    this.#requireInitialized('readDeveloperConsole');
    return this.#bridge.readDeveloperConsole();
  }

  readTelemetry(): RuntimeSessionTelemetrySummary {
    this.#requireInitialized('readTelemetry');
    return {
      sequenceId: this.#progress.sequenceId,
      tick: this.#progress.sessionTick,
      composition: this.#bridge.getProjectBundleCompositionStatus(),
      acceptedCommandCount: this.#progress.acceptedCommandCount,
      rejectedCommandCount: this.#progress.rejectedCommandCount,
      restartCount: this.#progress.restartCount,
      sessionHash: this.#sessionHash(),
      replayRecords: [...this.#replayRecords],
    };
  }

  restart(): RuntimeSessionRestartResult {
    this.#requireInitialized('restart');
    const before = this.#requireSnapshot();
    const snapshot = this.#bridge.restartFpsRuntimeSession({ expectedEpoch: before.sessionEpoch });
    this.#snapshot = snapshot;
    this.#progress.restart();
    this.#runtimeTransforms = new Map();
    this.#record('restart', snapshot.replayHash);
    return {
      sequenceId: this.#progress.sequenceId,
      tick: this.#progress.sessionTick,
      composition: this.#bridge.getProjectBundleCompositionStatus(),
      restartCount: this.#progress.restartCount,
      sessionHash: this.#sessionHash(),
    };
  }

  #rejectSessionRestart(
    intent: RuntimeSessionRestartIntent,
    statusBefore: RuntimeSessionLifecycleStatusReadout,
    sessionHashBefore: string,
    rejection: RuntimeSessionRestartIntentRejection,
  ): RuntimeSessionLifecycleRestartReceipt {
    this.#progress.advanceSequence();
    this.#record('requestSessionRestart');
    const statusAfter = this.readLifecycleStatus();
    return {
      kind: 'runtime_session.restart_receipt.v0',
      sequenceId: this.#progress.sequenceId,
      intent,
      accepted: false,
      status: 'rejected',
      rejection,
      statusBefore,
      statusAfter,
      restart: null,
      sessionHashBefore,
      sessionHashAfter: this.#sessionHash(),
      resetHash: statusAfter.hashes.replayHash,
    };
  }

  #applyRustAutonomousMovementProposal(
    proposal: Extract<EnemyPolicyProposal, { readonly kind: 'enemy_policy.move_toward_target.v0' }>,
    targetPosition: EnemyPolicyVec3 | undefined,
  ): EnemyDirectNavMovementResult {
    const snapshot = this.#requireSnapshot();
    if (proposal.nextWaypoint === null) {
      throw new RuntimeBridgeError('invalid_input', 'enemy movement proposal cannot be applied without a next waypoint');
    }
    const movement = this.#bridge.applyEnemyDirectNavMovement({
      entity: snapshot.enemyEntity,
      seedPosition: proposal.from,
      target: targetPosition ?? proposal.nextWaypoint,
      maxStepUnits: 0.35,
    });
    const enemy = this.#ecrpProjectState?.entities.find((entity) => entity.entity === snapshot.enemyEntity);
    const current = enemy === undefined
      ? null
      : ecrpEntityTransform({
          entity: enemy,
          runtimeTransforms: this.#runtimeTransforms,
        });
    this.#runtimeTransforms.set(snapshot.enemyEntity, {
      position: movement.nextWaypoint,
      yawDegrees: current?.yawDegrees ?? 0,
      pitchDegrees: current?.pitchDegrees ?? 0,
    });
    return movement;
  }

  #submitRustEnemyPolicyPrimaryFire(
    proposal: Extract<EnemyPolicyProposal, { readonly kind: 'enemy_policy.primary_fire_intent.v0' }>,
    enemyPosition: EnemyPolicyVec3,
    targetPosition: EnemyPolicyVec3,
  ): RuntimeSessionActionIntentReceipt {
    const envelope = proposal.intent;
    const before = this.#sessionHash();
    this.#progress.advanceSequence();
    const fire = this.#bridge.applyFpsPrimaryFire({
      tick: envelope.tick,
      origin: enemyPosition,
      direction: directionBetween(enemyPosition, targetPosition),
      shooterRole: 'enemy',
      targetRole: 'player',
    });
    this.#progress.recordProjectionTick(envelope.tick);
    this.#snapshot = this.#bridge.readFpsRuntimeSession();
    this.#record('submitRuntimeActionIntent', fire.replayHash, envelope.source);
    return {
      sequenceId: this.#progress.sequenceId,
      envelope,
      accepted: true,
      status: 'accepted',
      rejection: null,
      combatReadout: combatReadoutFromFpsPrimaryFire(fire, envelope.tick),
      sessionHashBefore: before,
      sessionHashAfter: this.#sessionHash(),
    };
  }

  #encounterLifecycleFromScenario(
    scenario?: EncounterDirectorReadoutRequest['lifecycleScenario'],
  ): ReturnType<typeof lifecycleStatusToEncounterLifecycle> {
    const lifecycleScenario = scenario === undefined || scenario === 'active' ? 'current_session' : scenario;
    return lifecycleStatusToEncounterLifecycle(this.readLifecycleStatus({ scenario: lifecycleScenario }));
  }

  #requireInitialized(operation: string): RuntimeSessionIdentity {
    if (this.#identity === null || this.#engine === null) {
      throw new RuntimeBridgeError('not_initialized', `${operation} before RuntimeSession initialize`);
    }
    return this.#identity;
  }

  #requireSnapshot(): FpsRuntimeSessionSnapshot {
    if (this.#snapshot === null) {
      throw new RuntimeBridgeError('not_initialized', 'FPS RuntimeSession snapshot is unavailable before initialize');
    }
    return this.#snapshot;
  }

  #stateSummary(composition: CompositionStatus): RuntimeSessionStateSummary {
    const identity = this.#requireInitialized('stateSummary');
    return {
      identity,
      engine: this.#engine as EngineHandle,
      composition,
      sequenceId: this.#progress.sequenceId,
      tick: this.#progress.sessionTick,
      sessionHash: this.#sessionHash(),
    };
  }

  #record(
    kind: RuntimeSessionReplayRecord['kind'],
    authorityHash?: string,
    actionSource?: RuntimeActionIntentEnvelope['source'],
  ): void {
    this.#replayRecords.push({
      sequenceId: this.#progress.sequenceId,
      kind,
      ...(actionSource === undefined ? {} : { actionSource }),
      recordHash: authorityHash ?? stableHash({
        kind,
        ...(actionSource === undefined ? {} : { actionSource }),
        sequenceId: this.#progress.sequenceId,
        tick: this.#progress.sessionTick,
        composition: compositionHashRecord(this.#bridge.getProjectBundleCompositionStatus()),
        fps: this.#snapshot === null
          ? null
          : {
              entityHash: this.#snapshot.entityHash,
              healthHash: this.#snapshot.healthHash,
              replayHash: this.#snapshot.replayHash,
              epoch: this.#snapshot.sessionEpoch,
            },
      }),
    });
  }

  #sessionHash(): string {
    const snapshot = this.#snapshot;
    return stableHash({
      identity: this.#identity === null ? null : identityHashRecord(this.#identity),
      sequenceId: this.#progress.sequenceId,
      tick: this.#progress.sessionTick,
      acceptedCommandCount: this.#progress.acceptedCommandCount,
      rejectedCommandCount: this.#progress.rejectedCommandCount,
      restartCount: this.#progress.restartCount,
      fps: snapshot === null
        ? null
        : {
            backend: snapshot.backend,
            authoritySurface: snapshot.authoritySurface,
            entityHash: snapshot.entityHash,
            healthHash: snapshot.healthHash,
            replayHash: snapshot.replayHash,
            epoch: snapshot.sessionEpoch,
          },
      composition: this.#identity === null ? null : compositionHashRecord(this.#bridge.getProjectBundleCompositionStatus()),
    });
  }
}
function rustRuntimeSessionNonClaims(): readonly RuntimeSessionNonClaim[] {
  return ['not_raw_state_store', 'not_arbitrary_json_bridge', 'not_renderer'];
}

function fpsLoadRequestFromEcrpProject(input: RuntimeSessionEcrpProjectLoadInput): FpsRuntimeSessionLoadRequest {
  const projectState = buildEcrpProjectState(input);
  const definitions: FpsStoredEntityDefinition[] = projectState.entities.map((entity) =>
    fpsStoredEntityDefinition(entity),
  );
  return {
    projectBundle: `${input.projectBundle.project.gameId}:${input.sceneDocument.id}`,
    bootstrapResolutionRegistry: input.bootstrapResolutionRegistry,
    sceneDocument: input.sceneDocument,
    definitions,
    gameRuleModules: input.gameRuleModules ?? [],
  };
}

function fpsStoredEntityDefinition(entity: RuntimeSessionEcrpEntityState): FpsStoredEntityDefinition {
  const definition = entity.definition;
  const transform = definition.capabilities.find((capability) => capability.kind === 'transform');
  const collisionBody = definition.capabilities.find((capability) => capability.kind === 'collisionBody');
  const health = definition.capabilities.find((capability) => capability.kind === 'health');
  const weapon = definition.capabilities.find((capability) => capability.kind === 'weaponMount');
  const policyBinding = definition.capabilities.find((capability) => capability.kind === 'policyBinding');
  const renderProjection = definition.capabilities.find((capability) => capability.kind === 'renderProjection');
  const faction = definition.capabilities.find((capability) => capability.kind === 'faction');
  const spawnMarker = definition.capabilities.find((capability) => capability.kind === 'spawnMarker');
  return {
    entity: entity.entity,
    stableId: definition.stableId,
    displayName: definition.displayName,
    sourcePath: definition.source.relativePath,
    tags: [
      ...(faction?.kind === 'faction' ? [`faction:${faction.factionId}`] : []),
      ...(spawnMarker?.kind === 'spawnMarker' ? [`spawn:${spawnMarker.markerId}`] : []),
    ],
    role: entity.role,
    transform: transform?.kind === 'transform' ? fpsTransform(transform) : null,
    bounds: collisionBody?.kind === 'collisionBody' ? fpsWorldBounds(transform, collisionBody) : null,
    renderVisible: renderProjection?.kind === 'renderProjection' ? renderProjection.visible ?? true : null,
    staticCollider: collisionBody?.kind === 'collisionBody' ? collisionBody.staticCollider ?? false : null,
    health: health?.kind === 'health' ? { current: health.current, max: health.max } : null,
    weapon: weapon?.kind === 'weaponMount'
      ? {
          weaponId: weapon.weaponId,
          damage: 40,
          rangeUnits: 16,
          ammo: 2,
          cooldownTicksAfterFire: 4,
        }
      : null,
    policyBinding: policyBinding?.kind === 'policyBinding'
      ? {
          bindingId: `${definition.stableId}:policy`,
          policyId: policyBinding.policyId,
          viewKind: 'runtime_session.fps.policy_view.v0',
          viewVersion: 'v0',
          allowedIntents: ['runtime.intent.move_direct_nav.v0', 'runtime.intent.primary_fire.v0'],
          runtimeMoment: 'autonomous_policy_tick',
        }
      : null,
  };
}

function fpsTransform(
  capability: Extract<RuntimeSessionEcrpProjectCapabilityDefinition, { readonly kind: 'transform' }>,
): FpsTransformCapability {
  return {
    translation: capability.initial.position,
    rotation: [0, 0, 0, 1],
    scale: [1, 1, 1],
  };
}

function fpsWorldBounds(
  transform: RuntimeSessionEcrpProjectCapabilityDefinition | undefined,
  collisionBody: Extract<RuntimeSessionEcrpProjectCapabilityDefinition, { readonly kind: 'collisionBody' }>,
): FpsBoundsCapability {
  const position = transform?.kind === 'transform' ? transform.initial.position : [0, 0, 0] as const;
  return {
    min: [
      position[0] - collisionBody.halfExtents[0],
      position[1] - collisionBody.halfExtents[1],
      position[2] - collisionBody.halfExtents[2],
    ],
    max: [
      position[0] + collisionBody.halfExtents[0],
      position[1] + collisionBody.halfExtents[1],
      position[2] + collisionBody.halfExtents[2],
    ],
  };
}

function lifecycleStateFromFpsSnapshot(snapshot: FpsRuntimeSessionSnapshot): RuntimeSessionLifecycleState {
  const player = fpsLifecycleHealth(snapshot, snapshot.playerEntity);
  const enemy = fpsLifecycleHealth(snapshot, snapshot.enemyEntity);
  const terminalEvent =
    snapshot.lifecycleStatus.state === 'enemy_defeated'
      ? {
          kind: 'runtime_lifecycle.enemy_defeated.v0' as const,
          entity: snapshot.lifecycleStatus.entity,
          tick: snapshot.lifecycleStatus.tick,
          reason: 'combat_health_zero' as const,
          eventHash: stableHash({
            kind: 'runtime_lifecycle.enemy_defeated.v0',
            entity: snapshot.lifecycleStatus.entity,
            tick: snapshot.lifecycleStatus.tick,
            reason: 'combat_health_zero',
            replayHash: snapshot.replayHash,
          }),
        }
      : null;
  return {
    player,
    enemy,
    terminalEvent,
    revision: snapshot.replayRecords.length,
  };
}

function fpsLifecycleHealth(
  snapshot: FpsRuntimeSessionSnapshot,
  entity: number,
): RuntimeSessionLifecycleHealthReadout {
  const health = snapshot.health.find((entry) => entry.entity === entity);
  const current = health?.current ?? 0;
  const max = health?.max ?? 0;
  return {
    entity,
    current,
    max,
    dead: current <= 0,
    healthHash: snapshot.healthHash,
  };
}

function directionBetween(
  origin: EnemyPolicyVec3,
  target: EnemyPolicyVec3,
): [number, number, number] {
  const dx = target[0] - origin[0];
  const dy = target[1] - origin[1];
  const dz = target[2] - origin[2];
  const length = Math.hypot(dx, dy, dz);
  if (length === 0) return [0, 0, 1];
  return [dx / length, dy / length, dz / length];
}

function combatReadoutFromFpsPrimaryFire(
  result: FpsPrimaryFireResult,
  tick: number,
): CombatRuntimeReadout {
  if (result.target === null || result.targetHealthBefore === null || result.targetHealthAfter === null) {
    return {
      scenario: 'runtime_session_loaded_project_fire_hit',
      outcome: {
        kind: 'miss',
        reason: 'noTarget',
      },
      events: [{ kind: 'fire_missed', shooter: result.shooter, reason: 'noTarget', tick }],
      health: [],
      nextFireControl: {
        ammo: 2,
        cooldownTicksRemaining: 4,
        cooldownTicksAfterFire: 4,
      },
      healthHash: result.healthHash,
      replayHash: result.replayHash,
      authority: combatAuthorityFromFpsPrimaryFire(result),
      fixture: null,
    };
  }

  const defeated = result.targetHealthAfter.current <= 0;
  return {
    scenario: 'runtime_session_loaded_project_fire_hit',
    outcome: {
      kind: 'hit',
      target: result.target,
      distance: 0,
      hitPosition: null,
      defeated,
    },
    events: [
      { kind: 'fire_hit', shooter: result.shooter, target: result.target, distance: 0, tick },
      {
        kind: 'damage_applied',
        target: result.target,
        amount: result.targetHealthBefore.current - result.targetHealthAfter.current,
        before: result.targetHealthBefore.current,
        after: result.targetHealthAfter.current,
      },
      ...(defeated ? [{ kind: 'entity_defeated' as const, target: result.target }] : []),
    ],
    health: [{
      entity: result.target,
      current: result.targetHealthAfter.current,
      max: result.targetHealthAfter.max,
      dead: defeated,
    }],
    nextFireControl: {
      ammo: 2,
      cooldownTicksRemaining: 4,
      cooldownTicksAfterFire: 4,
    },
    healthHash: result.healthHash,
    replayHash: result.replayHash,
    authority: combatAuthorityFromFpsPrimaryFire(result),
    fixture: null,
  };
}

function combatAuthorityFromFpsPrimaryFire(result: FpsPrimaryFireResult): CombatRuntimeReadout['authority'] {
  return {
    source: result.backend === 'native_rust' ? 'rust_bridge' : 'reference_bridge',
    backend: result.backend,
    surface: result.authoritySurface,
    mutationOwner: result.mutationOwner,
    workspaceTrace: result.workspaceTrace,
  };
}
