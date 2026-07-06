import type {
  CameraCreateRequest,
  CameraProjectionRequest,
  CollisionConstrainedCameraInputEnvelope,
  CommandBatch,
  FirstPersonCameraInputEnvelope,
} from '@asha/contracts';
import {
  RuntimeBridgeError,
  frameCursor,
  type CompositionStatus,
  type FpsEncounterDirectorSnapshot,
  type FpsEncounterLifecycleInput,
  type FpsEncounterStateReadout,
  type FpsEncounterTransitionResult,
  type EngineHandle,
  type FpsPrimaryFireResult,
  type FpsRuntimeSessionLoadRequest,
  type FpsRuntimeSessionSnapshot,
  type FpsStoredEntityDefinition,
  type FpsTransformCapability,
  type RuntimeBridge,
} from './bridge.js';
import type { CombatRuntimeReadout } from './combat-readout.js';
import type { CombatFeedbackProjection } from './combat-feedback.js';
import type {
  GeneratedTunnelOperationRequest,
  GeneratedTunnelReadout,
  GeneratedTunnelReadoutRequest,
} from './generated-tunnel.js';
import type {
  EncounterDirectorReadout,
  EncounterDirectorReadoutRequest,
  EncounterDirectorState,
  EncounterTransitionRequest,
  RuntimeSessionEncounterTransitionReceipt,
} from './encounter-director.js';
import {
  buildEncounterDirectorReadout,
  buildEncounterTransitionReceipt,
  validateEncounterDirectorReadoutRequest,
  validateEncounterTransitionRequest,
} from './encounter-director.js';
import type {
  NavPathQueryRequest,
  NavPathReadout,
  NavPolicyViewReadout,
  NavProjectionReadout,
} from './nav-readout.js';
import type { RuntimeActionIntentEnvelope } from './runtime-action.js';
import {
  buildEcrpProjectState,
  buildEcrpRuntimeReadout,
  defaultRuntimeSessionEcrpProjectLoadInput,
  validateEcrpProjectLoadInput,
} from './runtime-session-ecrp.js';
import {
  lifecycleStatusReadout,
  lifecycleStatusToEncounterLifecycle,
  validateInitializeInput,
  validateLifecycleStatusRequest,
  validateRestartIntent,
  validateRuntimeActionIntentEnvelope,
} from './runtime-session-lifecycle.js';
import {
  compositionHashRecord,
  identityHashRecord,
  renderFrameHashRecord,
  stableHash,
} from './runtime-session-hash.js';
import type {
  RuntimeSessionActionIntentReceipt,
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
} from './runtime-session.js';

export class RustBackedRuntimeSessionFacade implements RuntimeSessionFacade {
  readonly #bridge: RuntimeBridge;
  #identity: RuntimeSessionIdentity | null = null;
  #engine: EngineHandle | null = null;
  #sequenceId = 0;
  #tick = 0;
  #acceptedCommandCount = 0;
  #rejectedCommandCount = 0;
  #restartCount = 0;
  #snapshot: FpsRuntimeSessionSnapshot | null = null;
  #ecrpProjectState: RuntimeSessionEcrpProjectState | null = null;
  #runtimeTransforms = new Map<number, RuntimeSessionEcrpTransformState>();
  #replayRecords: RuntimeSessionReplayRecord[] = [];

  constructor(bridge: RuntimeBridge) {
    this.#bridge = bridge;
  }

  initialize(input: RuntimeSessionInitializeInput): RuntimeSessionStateSummary {
    validateInitializeInput(input);
    const engine = this.#bridge.initializeEngine({ seed: input.seed });
    const composition = this.#bridge.loadWorldBundle(input.projectBundle);
    const defaultProject = defaultRuntimeSessionEcrpProjectLoadInput(input);
    const snapshot = this.#bridge.loadFpsRuntimeSession(fpsLoadRequestFromEcrpProject(defaultProject));
    this.#engine = engine;
    this.#identity = {
      sessionId: input.sessionId,
      mode: 'rust',
      seed: input.seed,
      project: input.project,
      projectBundle: input.projectBundle,
      nonClaims: rustRuntimeSessionNonClaims(),
    };
    this.#sequenceId = 0;
    this.#tick = 0;
    this.#acceptedCommandCount = 0;
    this.#rejectedCommandCount = 0;
    this.#restartCount = 0;
    this.#snapshot = snapshot;
    this.#ecrpProjectState = buildEcrpProjectState(defaultProject);
    this.#runtimeTransforms = new Map();
    this.#replayRecords = [];
    this.#record('initialize', snapshot.replayHash);
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

    const snapshot = this.#bridge.loadFpsRuntimeSession(fpsLoadRequestFromEcrpProject(input));
    this.#bridge.loadWorldBundle(input.projectBundle.runtimeRequest);
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
      sequenceId: this.#sequenceId,
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

    if (envelope.action !== 'primary_fire' || envelope.phase !== 'pressed') {
      this.#record('submitRuntimeActionIntent');
      return {
        sequenceId: this.#sequenceId,
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

    const fire = this.#bridge.applyFpsPrimaryFire({
      tick: envelope.tick,
      origin: [0, 1.62, 0],
      direction: [0, 0, -1],
    });
    this.#snapshot = this.#bridge.readFpsRuntimeSession();
    this.#record('submitRuntimeActionIntent', fire.replayHash);
    return {
      sequenceId: this.#sequenceId,
      envelope,
      accepted: true,
      status: 'accepted',
      rejection: null,
      combatReadout: combatReadoutFromFpsPrimaryFire(fire, envelope.tick),
      sessionHashBefore: before,
      sessionHashAfter: this.#sessionHash(),
    };
  }

  runAutonomousPolicyTick(_input: RuntimeSessionAutonomousPolicyTickInput): RuntimeSessionAutonomousPolicyTickReadout {
    void _input;
    this.#requireInitialized('runAutonomousPolicyTick');
    throw new RuntimeBridgeError(
      'operation_unimplemented',
      'Rust-backed RuntimeSession policy tick authority is not wired on this facade slice.',
    );
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
      sequenceId: this.#sequenceId,
      tick: this.#tick,
      restartCount: this.#restartCount,
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
      sequenceId: this.#sequenceId,
      tick: this.#tick,
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
      sequenceId: this.#sequenceId,
      tick: this.#tick,
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

    this.#sequenceId += 1;
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
      sequenceId: this.#sequenceId,
      tick: this.#tick,
      sessionSeed: identity.seed,
      sessionHash: this.#sessionHash(),
    });
    return buildEncounterTransitionReceipt({
      request,
      sequenceId: this.#sequenceId,
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
    _request: GeneratedTunnelOperationRequest,
  ): RuntimeSessionGeneratedTunnelOperationReceipt {
    void _request;
    this.#requireInitialized('requestGeneratedTunnelOperation');
    throw new RuntimeBridgeError('operation_unimplemented', 'Rust-backed generated tunnel operation authority is not wired yet');
  }

  readEcrpRuntimeReadout(): RuntimeSessionEcrpReadout {
    const identity = this.#requireInitialized('readEcrpRuntimeReadout');
    const snapshot = this.#requireSnapshot();
    return buildEcrpRuntimeReadout({
      identity,
      projectState: this.#ecrpProjectState,
      lifecycleState: lifecycleStateFromFpsSnapshot(snapshot),
      runtimeTransforms: this.#runtimeTransforms,
      sequenceId: this.#sequenceId,
      tick: this.#tick,
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
    this.#requireInitialized('restart');
    const before = this.#requireSnapshot();
    const snapshot = this.#bridge.restartFpsRuntimeSession({ expectedEpoch: before.sessionEpoch });
    this.#snapshot = snapshot;
    this.#sequenceId += 1;
    this.#tick = 0;
    this.#acceptedCommandCount = 0;
    this.#rejectedCommandCount = 0;
    this.#runtimeTransforms = new Map();
    this.#restartCount += 1;
    this.#record('restart', snapshot.replayHash);
    return {
      sequenceId: this.#sequenceId,
      tick: this.#tick,
      composition: this.#bridge.getCompositionStatus(),
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
      resetHash: statusAfter.hashes.replayHash,
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
      sequenceId: this.#sequenceId,
      tick: this.#tick,
      sessionHash: this.#sessionHash(),
    };
  }

  #record(kind: RuntimeSessionReplayRecord['kind'], authorityHash?: string): void {
    this.#replayRecords.push({
      sequenceId: this.#sequenceId,
      kind,
      recordHash: authorityHash ?? stableHash({
        kind,
        sequenceId: this.#sequenceId,
        tick: this.#tick,
        composition: compositionHashRecord(this.#bridge.getCompositionStatus()),
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
      sequenceId: this.#sequenceId,
      tick: this.#tick,
      acceptedCommandCount: this.#acceptedCommandCount,
      rejectedCommandCount: this.#rejectedCommandCount,
      restartCount: this.#restartCount,
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
      composition: this.#identity === null ? null : compositionHashRecord(this.#bridge.getCompositionStatus()),
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
    projectBundle: `${input.projectBundle.project.gameId}:${input.sceneDocument.sceneId}`,
    definitions,
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
    bounds: collisionBody?.kind === 'collisionBody'
      ? {
          min: [-collisionBody.halfExtents[0], -collisionBody.halfExtents[1], -collisionBody.halfExtents[2]],
          max: [collisionBody.halfExtents[0], collisionBody.halfExtents[1], collisionBody.halfExtents[2]],
        }
      : null,
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
          allowedIntents: ['enemy_policy.move_toward_target.v0', 'enemy_policy.primary_fire_intent.v0'],
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

function fpsEncounterLifecycleInput(
  lifecycle: ReturnType<typeof lifecycleStatusToEncounterLifecycle>,
): FpsEncounterLifecycleInput {
  return {
    outcomeKind: lifecycle.outcomeKind,
    terminal: lifecycle.terminal,
    enemyDead: lifecycle.enemyDead,
    playerDead: lifecycle.playerDead,
    lifecycleHash: lifecycle.lifecycleHash,
  };
}

function encounterReadoutFromFpsSnapshot(input: {
  readonly snapshot: FpsEncounterDirectorSnapshot;
  readonly sequenceId: number;
  readonly tick: number;
  readonly sessionSeed: number;
  readonly sessionHash: string;
}): EncounterDirectorReadout {
  return buildEncounterDirectorReadout({
    state: fpsEncounterStateToReadoutState(input.snapshot.state),
    sequenceId: input.sequenceId,
    tick: input.tick,
    sessionSeed: input.sessionSeed,
    sessionHash: input.sessionHash,
    lifecycle: input.snapshot.lifecycle,
    authority: {
      source: input.snapshot.backend === 'native_rust' ? 'rust_bridge' : 'reference_bridge',
      backend: input.snapshot.backend,
      surface: input.snapshot.authoritySurface,
      mutationOwner: input.snapshot.mutationOwner,
      readSets: input.snapshot.readSets,
      workspaceTrace: input.snapshot.workspaceTrace,
    },
  });
}

function fpsEncounterStateToReadoutState(state: FpsEncounterStateReadout): EncounterDirectorState {
  return {
    presetId: requireGeneratedTunnelEncounterPreset(state.presetId),
    status: state.status,
    spawnedEnemyIds: generatedTunnelEncounterIds(state.spawnedEnemyIds),
    defeatedEnemyIds: generatedTunnelEncounterIds(state.defeatedEnemyIds),
    revision: state.revision,
    lastTransition: state.lastTransition,
  };
}

function encounterTransitionResultForReceipt(result: FpsEncounterTransitionResult) {
  return {
    accepted: result.accepted,
    state: fpsEncounterStateToReadoutState(result.state),
    ...(result.eventKind === null ? {} : { eventKind: result.eventKind }),
    ...(result.rejectionReason === null ? {} : { rejectionReason: result.rejectionReason }),
  };
}

function requireGeneratedTunnelEncounterPreset(value: string): EncounterDirectorState['presetId'] {
  if (value !== 'generated-tunnel-small-encounter') {
    throw new RuntimeBridgeError('internal', `unsupported Rust encounter preset '${value}'`);
  }
  return value;
}

function generatedTunnelEncounterIds(ids: readonly string[]): EncounterDirectorState['spawnedEnemyIds'] {
  return ids.map((id) => {
    if (id !== 'encounter.generated_tunnel_small.wave_1.enemy_001') {
      throw new RuntimeBridgeError('internal', `unsupported Rust encounter instance '${id}'`);
    }
    return id;
  });
}
