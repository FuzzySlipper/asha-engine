import type {
  CameraCollisionShape,
  CameraCollisionSnapshot,
  CameraControllerReadRequest,
  CameraControllerState,
  CameraCreateRequest,
  CameraModeChangeReceipt,
  CameraModeCommand,
  CameraNavigationInputEnvelope,
  CameraNavigationReceipt,
  CameraProjectionRequest,
  CameraProjectionSnapshot,
  CameraSnapshot,
  CollisionAxis,
  CollisionConstrainedCameraInputEnvelope,
  CommandBatch,
  CommandResult,
  DeveloperConsoleSnapshot,
  Face,
  FirstPersonCameraInputEnvelope,
  ModelMaterialPreviewRequest,
  ModelMaterialPreviewSnapshot,
  PickRay,
  PickResult,
  VoxelInstancePickRequest,
  VoxelInstancePickResult,
  VoxelProjectionBindingRequest,
  VoxelProjectionBindingReceipt,
  RenderFrameDiff,
  RuntimeProjectionFrame,
  TimeControlCommand,
  TimeControlReceipt,
  TimeControlState,
  SceneObjectCommandRequest,
  SceneObjectCommandResult,
  SceneObjectSnapshot,
  SceneDocumentCodecResult,
  SceneDocumentAuthoringRequest,
  SceneDocumentAuthoringResult,
  SceneDocumentDecodeRequest,
  SceneDocumentEncodeRequest,
  ProjectContentAuthoringRequest,
  ProjectContentAuthoringResult,
  ProjectContentCodecResult,
  ProjectContentDecodeRequest,
  ProjectContentEncodeRequest,
  ProjectResourceBeginRequest,
  ProjectResourceTransactionReceipt,
  ProjectSourceBatchValidationReceipt,
  RuntimeProjectCloseReceipt,
  RuntimeProjectCloseRequest,
  RuntimeProjectLoadReceipt,
  RuntimeProjectLoadRequest,
  RuntimeProjectSourceBatch,
  StagedProjectResourceRef,
  ProceduralEnvironmentApplyRequest,
  ProceduralEnvironmentApplyResult,
  ProceduralEnvironmentPreviewRequest,
  ProceduralEnvironmentPreviewResult,
  ScreenPointToPickRayRequest,
  VoxelCoord,
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
  VoxelSelectionSnapshot,
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
  VoxelEditHistoryReadRequest, VoxelEditHistoryRedoReceipt, VoxelEditHistoryRedoRequest,
  VoxelEditHistoryRevertReceipt, VoxelEditHistoryRevertRequest, VoxelEditHistorySummary,
  VoxelEditHistoryUndoReceipt, VoxelEditHistoryUndoRequest,
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
  GameExtensionHookReceipt,
  GameExtensionReplayEvidence,
  GameRuleCatalog,
  GameRuleResolutionReceipt,
  InputActionReplayReceipt,
  InputContextChangeReceipt,
  InputContextCommand,
  InputContextStackState,
  InputResolutionReceipt,
  InputSessionConfigureRequest,
  InputSessionSnapshot,
  RawInputSample,
  RecordedInputAction,
} from '@asha/contracts';
import {
  RuntimeBridgeError,
  nonNegativeSafeInteger,
  u32,
  type CompositionStatus,
  type ComposedRuntimeSessionReadout,
  type EnemyDirectNavMovementRequest,
  type EnemyDirectNavMovementResult,
  type EngineConfig,
  type EngineHandle,
  type FrameCursor,
  type FpsEncounterDirectorSnapshot,
  type FpsEncounterLifecycleInput,
  type FpsEncounterStateReadout,
  type FpsEncounterTransitionRequest,
  type FpsEncounterTransitionResult,
  type FpsPrimaryFireRequest,
  type FpsPrimaryFireResult,
  type GameExtensionWeaponEffectInvocationRequest,
  type GameExtensionWeaponEffectInvocationResult,
  type GameRuleCatalogValidationReceipt,
  type GameRuleEffectIntentRequest,
  type GameRuleRuntimeReadout,
  type GameplayModuleViewSnapshot,
  type GameplayPrefabPartInteractionReceipt,
  type GeneratedTunnelRuntimeApplyReceipt,
  type GeneratedTunnelRuntimeApplyRequest,
  type FpsRuntimeSessionLoadRequest,
  type FpsRuntimeSessionRestartRequest,
  type FpsRuntimeSessionSnapshot,
  type ReplayFixture,
  type ReplaySessionHandle,
  type ReplayStepReport,
  type RuntimeBridge,
  type RuntimeBufferHandle,
  type RuntimeBufferView,
  type StepInputEnvelope,
  type StepResult,
  type VoxelMeshEvidenceRequest,
  type VoxelMeshEvidenceSnapshot,
  type ProjectBundleLoadRequest,
  type ProjectResourceStageInput,
  type ProjectBundleSaveSummary,
  type WorkspaceAuthoringCloseInput,
  type WorkspaceAuthoringCloseReceipt,
  type WorkspaceAuthoringOpenInput,
  type WorkspaceAuthoringProjectionRequest,
  type WorkspaceAuthoringProjectionSummary,
  type WorkspaceAuthoringStateSummary,
  type WorkspaceAuthoringStoredConfirmationInput,
  type WorkspaceAuthoringStoredConfirmationReceipt,
} from './bridge.js';
import { collisionCameraAttemptedPose } from './camera-collision-movement.js';
import { mockCameraProjectionSnapshot } from './mock-camera-projection.js';
import { MockGameRuleRuntime } from './mock-game-rules.js';
import { MockCameraControllers } from './mock-camera-controller.js';
import { MockInputSession } from './mock-input-session.js';
import { MockTimeController } from './mock-time-control.js';
import { MockRuntimeProjectLifecycle } from './mock-runtime-project.js';
import {
  applyMockSceneObjectCommand,
  initialMockSceneDocument,
  mockModelMaterialPreview,
  sceneObjectSnapshotFromDocument,
} from './mock-scene.js';
import { fnv1a64, validateVec3 } from './mock-primitives.js';

// ── Mock implementation ───────────────────────────────────────────────────────
// Targets the facade so most TS tests need no addon load. Behaviour mirrors the
// Rust `ReferenceBridge` so native/mock parity is meaningful.

type MutableCameraSnapshot = CameraSnapshot;
type Vec3 = readonly [number, number, number];

interface StaticRoomCollider {
  readonly id: string;
  readonly min: Vec3;
  readonly max: Vec3;
}

function finite(value: number, field: string): number {
  if (!Number.isFinite(value)) {
    throw new RuntimeBridgeError('invalid_input', `${field} must be finite`);
  }
  return value;
}

function validateViewport(viewport: { readonly width: number; readonly height: number }): void {
  if (!Number.isInteger(viewport.width) || viewport.width <= 0) {
    throw new RuntimeBridgeError('invalid_input', 'viewport width must be a positive integer');
  }
  if (!Number.isInteger(viewport.height) || viewport.height <= 0) {
    throw new RuntimeBridgeError('invalid_input', 'viewport height must be a positive integer');
  }
}

function validateProjection(projection: CameraCreateRequest['projection']): void {
  finite(projection.fovYDegrees, 'fovYDegrees');
  finite(projection.near, 'near');
  finite(projection.far, 'far');
  if (projection.fovYDegrees <= 0 || projection.fovYDegrees >= 180) {
    throw new RuntimeBridgeError('invalid_input', 'fovYDegrees must be in (0, 180)');
  }
  if (projection.near <= 0 || projection.far <= projection.near) {
    throw new RuntimeBridgeError('invalid_input', 'projection near/far must satisfy 0 < near < far');
  }
}

function f32(value: number): number {
  return Math.fround(value);
}

function motionDelta(value: number): number {
  const rounded = f32(value);
  return Math.abs(rounded) < 0.000001 ? 0 : rounded;
}

function basisFromPose(pose: CameraSnapshot['pose']): CameraSnapshot['basis'] {
  const yaw = f32((pose.yawDegrees * Math.PI) / 180);
  const pitch = f32((pose.pitchDegrees * Math.PI) / 180);
  const cp = f32(Math.cos(pitch));
  const sp = f32(Math.sin(pitch));
  const sy = f32(Math.sin(yaw));
  const cy = f32(Math.cos(yaw));
  return {
    forward: [f32(sy * cp), sp, f32(-cy * cp)],
    right: [cy, 0, sy],
    up: [f32(-sy * sp), cp, f32(cy * sp)],
  };
}

function vec3Distance(from: Vec3, to: Vec3): number {
  const dx = to[0] - from[0];
  const dy = to[1] - from[1];
  const dz = to[2] - from[2];
  return Math.sqrt(dx * dx + dy * dy + dz * dz);
}

function directNavNextWaypoint(from: Vec3, target: Vec3, maxStepUnits: number): Vec3 {
  const distance = vec3Distance(from, target);
  if (distance <= maxStepUnits) {
    return [
      Number(target[0].toFixed(3)),
      Number(target[1].toFixed(3)),
      Number(target[2].toFixed(3)),
    ];
  }
  const ratio = maxStepUnits / distance;
  return [
    Number((from[0] + (target[0] - from[0]) * ratio).toFixed(3)),
    Number((from[1] + (target[1] - from[1]) * ratio).toFixed(3)),
    Number((from[2] + (target[2] - from[2]) * ratio).toFixed(3)),
  ];
}

const STATIC_ROOM_COLLIDERS: readonly StaticRoomCollider[] = [
  { id: 'static-room.wall.north', min: [-3, -1, -3], max: [3, 2, -2] },
  { id: 'static-room.wall.south', min: [-3, -1, 2], max: [3, 2, 3] },
  { id: 'static-room.wall.west', min: [-3, -1, -3], max: [-2, 2, 3] },
  { id: 'static-room.wall.east', min: [2, -1, -3], max: [3, 2, 3] },
  { id: 'static-room.target.01', min: [-0.31, 0, -1.66], max: [0.31, 2.2, -1.04] },
  { id: 'static-room.target.02', min: [1.01, 0, -0.89], max: [1.49, 0.85, -0.41] },
  { id: 'static-room.target.03', min: [-1.41, 0, -1.16], max: [-0.89, 1.05, -0.64] },
  { id: 'static-room.target.04', min: [0.63, 0, 0.88], max: [1.07, 0.75, 1.32] },
];

const STATIC_ROOM_COLLISION_SOURCE_HASH = `fnv1a64:${fnv1a64(
  STATIC_ROOM_COLLIDERS.map((collider) => `${collider.id}:${collider.min.join(',')}:${collider.max.join(',')}`).join('|'),
)}`;
const STATIC_ROOM_COLLISION_PROJECTION_HASH = `fnv1a64:${fnv1a64(
  `${STATIC_ROOM_COLLISION_SOURCE_HASH}|axis-separable-static-room|${STATIC_ROOM_COLLIDERS.length}`,
)}`;

interface AabbEvidence {
  readonly min: Vec3;
  readonly max: Vec3;
}

function aabbForPose(pose: CameraSnapshot['pose'], shape: CameraCollisionShape): AabbEvidence {
  return {
    min: [
      f32(pose.position[0] - shape.halfExtents[0]),
      f32(pose.position[1] - shape.halfExtents[1]),
      f32(pose.position[2] - shape.halfExtents[2]),
    ],
    max: [
      f32(pose.position[0] + shape.halfExtents[0]),
      f32(pose.position[1] + shape.halfExtents[1]),
      f32(pose.position[2] + shape.halfExtents[2]),
    ],
  };
}

function aabbOverlaps(a: AabbEvidence, b: StaticRoomCollider): boolean {
  return (
    a.min[0] < b.max[0] &&
    a.max[0] > b.min[0] &&
    a.min[1] < b.max[1] &&
    a.max[1] > b.min[1] &&
    a.min[2] < b.max[2] &&
    a.max[2] > b.min[2]
  );
}

function sweptAabb(start: AabbEvidence, end: AabbEvidence): AabbEvidence {
  return {
    min: [
      Math.min(start.min[0], end.min[0]),
      Math.min(start.min[1], end.min[1]),
      Math.min(start.min[2], end.min[2]),
    ],
    max: [
      Math.max(start.max[0], end.max[0]),
      Math.max(start.max[1], end.max[1]),
      Math.max(start.max[2], end.max[2]),
    ],
  };
}

function staticRoomMoveBlocked(
  fromPose: CameraSnapshot['pose'],
  toPose: CameraSnapshot['pose'],
  shape: CameraCollisionShape,
): boolean {
  const from = aabbForPose(fromPose, shape);
  const to = aabbForPose(toPose, shape);
  const swept = sweptAabb(from, to);
  return STATIC_ROOM_COLLIDERS.some((collider) => aabbOverlaps(to, collider) || aabbOverlaps(swept, collider));
}

function poseWithAxis(pose: CameraSnapshot['pose'], axis: number, value: number): CameraSnapshot['pose'] {
  const position = [pose.position[0], pose.position[1], pose.position[2]] as [number, number, number];
  position[axis] = f32(value);
  return {
    position,
    yawDegrees: pose.yawDegrees,
    pitchDegrees: pose.pitchDegrees,
  };
}

export class MockRuntimeBridge implements RuntimeBridge {
  #engine: EngineHandle | null = null;
  #buffer: Uint8Array = new Uint8Array();
  #replaySteps = 0;
  #loadedProjectBundle: number | null = null;
  #sceneDocument = initialMockSceneDocument();
  #nextCamera = 1;
  #cameras = new Map<number, MutableCameraSnapshot>();
  #cameraControllers = new MockCameraControllers();
  #enemyTransforms = new Map<number, Vec3>();
  #fpsSeed: FpsRuntimeSessionLoadRequest | null = null;
  #fpsSnapshot: FpsRuntimeSessionSnapshot | null = null;
  #fpsEncounter: FpsEncounterStateReadout = initialFpsEncounterState();
  #fpsEpoch = 0;
  #gameRules = new MockGameRuleRuntime();
  #inputSession = new MockInputSession();
  #timeController = new MockTimeController();
  #workspaceAuthoringGeneration = 0;
  #workspaceAuthoringState: WorkspaceAuthoringStateSummary | null = null;
  #workspaceAuthoringCursor = 0;
  readonly #runtimeProjects = new MockRuntimeProjectLifecycle();

  #unsupportedAfterInit(method: string, message: string): never {
    if (this.#engine === null) {
      throw new RuntimeBridgeError('not_initialized', `${method} before initializeEngine`);
    }
    throw new RuntimeBridgeError('operation_unimplemented', message);
  }

  initializeEngine(config: EngineConfig): EngineHandle {
    if (!Number.isInteger(config.seed) || config.seed < 0) {
      throw new RuntimeBridgeError('invalid_input', `seed must be a non-negative integer`);
    }
    const handle = config.seed as EngineHandle;
    this.#engine = handle;
    // Deterministic: little-endian seed bytes, mirroring ReferenceBridge.
    const bytes = new Uint8Array(8);
    new DataView(bytes.buffer).setBigUint64(0, BigInt(config.seed), true);
    this.#buffer = bytes;
    this.#fpsSeed = null;
    this.#fpsSnapshot = null;
    this.#fpsEncounter = initialFpsEncounterState();
    this.#fpsEpoch = 0;
    this.#gameRules.reset();
    this.#cameraControllers.clear();
    this.#inputSession.initialize();
    this.#timeController.initialize();
    this.#runtimeProjects.reset();
    return handle;
  }

  openWorkspaceAuthoring(input: WorkspaceAuthoringOpenInput): WorkspaceAuthoringStateSummary {
    if (this.#workspaceAuthoringState?.status === 'open') {
      throw new RuntimeBridgeError('invalid_input', 'workspace authoring is already open');
    }
    this.#workspaceAuthoringGeneration += 1;
    this.#workspaceAuthoringCursor = 0;
    this.#workspaceAuthoringState = {
      kind: 'workspace_authoring.state.v0',
      status: 'open',
      identity: {
        kind: 'workspace_authoring.identity.v0',
        authoringId: input.authoringId,
        mode: 'rust',
        generation: this.#workspaceAuthoringGeneration,
        seed: input.seed,
        project: input.project,
        projectBundle: input.projectBundle,
        nonClaims: [
          'not_gameplay_runtime_session',
          'not_simulation_loop',
          'not_stored_truth',
          'not_renderer_authority',
        ],
      },
      composition: {
        loadedProjectBundle: input.projectBundle.sceneId,
        fatalCount: 0,
        totalCount: 0,
        blocksLoad: false,
      },
      workingRevision: 0,
      storedRevision: 0,
      dirty: false,
      lastStoredCanonicalJsonHash: null,
      authoritySnapshotHash: 'fnv1a64:mock-authoring-0',
      lifecycleHash: 'fnv1a64:mock-authoring-lifecycle-0',
    };
    return this.#workspaceAuthoringState;
  }

  readWorkspaceAuthoringState(): WorkspaceAuthoringStateSummary {
    if (this.#workspaceAuthoringState === null) {
      throw new RuntimeBridgeError('not_initialized', 'workspace authoring state before open');
    }
    return this.#workspaceAuthoringState;
  }

  readWorkspaceAuthoringProjection(
    request: WorkspaceAuthoringProjectionRequest,
  ): WorkspaceAuthoringProjectionSummary {
    const state = this.#requireMockWorkspaceAuthoring('readWorkspaceAuthoringProjection');
    this.#validateMockWorkspaceBinding(request.expectedWorkspaceId, request.expectedGeneration);
    if (request.expectedWorkingRevision !== state.workingRevision || request.cursor !== this.#workspaceAuthoringCursor) {
      throw new RuntimeBridgeError('stale_authority_snapshot', 'stale mock projection request');
    }
    const cursor = request.cursor;
    this.#workspaceAuthoringCursor += 1;
    return {
      kind: 'workspace_authoring.projection.v0',
      workspaceId: state.identity.project.workspaceId,
      generation: state.identity.generation,
      workingRevision: state.workingRevision,
      cursor,
      nextCursor: this.#workspaceAuthoringCursor as FrameCursor,
      delivery: cursor === 0 ? 'replace' : 'apply',
      frame: { ops: [] },
      renderDiffCount: 0,
      projectionHash: `fnv1a64:mock-projection-${cursor}`,
    };
  }

  confirmWorkspaceAuthoringStored(
    input: WorkspaceAuthoringStoredConfirmationInput,
  ): WorkspaceAuthoringStoredConfirmationReceipt {
    this.#validateMockWorkspaceBinding(input.expectedWorkspaceId, input.expectedGeneration);
    throw new RuntimeBridgeError('invalid_input', 'mock bridge has no current Rust save candidate');
  }

  closeWorkspaceAuthoring(input: WorkspaceAuthoringCloseInput): WorkspaceAuthoringCloseReceipt {
    const state = this.#requireMockWorkspaceAuthoring('closeWorkspaceAuthoring');
    this.#validateMockWorkspaceBinding(input.expectedWorkspaceId, input.expectedGeneration);
    if (state.dirty && input.discardUnsavedWorkingState !== true) {
      throw new RuntimeBridgeError('invalid_input', 'workspace authoring has unsaved work');
    }
    this.#workspaceAuthoringState = { ...state, status: 'closed' };
    return {
      kind: 'workspace_authoring.close_receipt.v0',
      closed: true,
      workspaceId: state.identity.project.workspaceId,
      generation: state.identity.generation,
      discardedUnsavedWorkingState: state.dirty,
      lifecycleHash: `fnv1a64:mock-close-${state.identity.generation}`,
    };
  }

  #requireMockWorkspaceAuthoring(operation: string): WorkspaceAuthoringStateSummary {
    const state = this.#workspaceAuthoringState;
    if (state === null || state.status !== 'open') {
      throw new RuntimeBridgeError('not_initialized', `${operation} requires workspace authoring`);
    }
    return state;
  }

  #validateMockWorkspaceBinding(workspaceId: string, generation: number): void {
    const state = this.#requireMockWorkspaceAuthoring('workspace binding');
    if (state.identity.project.workspaceId !== workspaceId || state.identity.generation !== generation) {
      throw new RuntimeBridgeError('stale_authority_snapshot', 'foreign mock workspace binding');
    }
  }

  #recordMockWorkspaceMutation(): void {
    const state = this.#workspaceAuthoringState;
    if (state?.status !== 'open') return;
    const workingRevision = state.workingRevision + 1;
    this.#workspaceAuthoringState = {
      ...state,
      workingRevision,
      dirty: workingRevision !== state.storedRevision,
      authoritySnapshotHash: `fnv1a64:mock-authoring-${workingRevision}`,
      lifecycleHash: `fnv1a64:mock-authoring-lifecycle-${workingRevision}`,
    };
  }

  configureInputSession(request: InputSessionConfigureRequest): InputSessionSnapshot { return this.#inputSession.configure(request); }
  applyInputContextCommand(command: InputContextCommand): InputContextChangeReceipt { return this.#inputSession.applyContextCommand(command); }
  submitRawInput(sample: RawInputSample): InputResolutionReceipt { return this.#inputSession.resolve(sample); }
  replayResolvedInputAction(record: RecordedInputAction): InputActionReplayReceipt { return this.#inputSession.replay(record); }
  readInputContextState(): InputContextStackState { return this.#inputSession.readContextState(); }

  applyTimeControlCommand(command: TimeControlCommand): TimeControlReceipt { return this.#timeController.apply(command); }

  readTimeControlState(): TimeControlState { return this.#timeController.read(); }

  stepSimulation(input: StepInputEnvelope): StepResult {
    if (this.#engine === null) {
      throw new RuntimeBridgeError('not_initialized', 'step before initializeEngine');
    }
    const tick = nonNegativeSafeInteger(input.tick, 'tick');
    return this.#timeController.step(tick);
  }

  applyEnemyDirectNavMovement(request: EnemyDirectNavMovementRequest): EnemyDirectNavMovementResult {
    if (this.#engine === null) {
      throw new RuntimeBridgeError('not_initialized', 'applyEnemyDirectNavMovement before initializeEngine');
    }
    const entity = nonNegativeSafeInteger(request.entity, 'entity');
    if (entity === 0) {
      throw new RuntimeBridgeError('invalid_input', 'entity must be positive');
    }
    validateVec3(request.seedPosition, 'seedPosition');
    validateVec3(request.target, 'target');
    if (!Number.isFinite(request.maxStepUnits) || request.maxStepUnits <= 0) {
      throw new RuntimeBridgeError('invalid_input', 'maxStepUnits must be finite and positive');
    }
    const existing = this.#enemyTransforms.get(entity);
    const from = existing ?? request.seedPosition;
    const nextWaypoint = directNavNextWaypoint(from, request.target, request.maxStepUnits);
    this.#enemyTransforms.set(entity, nextWaypoint);
    return {
      entity,
      authoritySource: existing === undefined ? 'seeded_from_request' : 'rust_entity_store',
      authorityTransport: 'reference_bridge',
      from,
      target: request.target,
      nextWaypoint,
      distanceUnits: Number(vec3Distance(from, request.target).toFixed(3)),
      reached: vec3Distance(from, request.target) <= request.maxStepUnits,
      pathHash: `fnv1a64:${fnv1a64(JSON.stringify({ entity, from, target: request.target, nextWaypoint }))}`,
      transformHash: `fnv1a64:${fnv1a64(JSON.stringify({ entity, position: nextWaypoint }))}`,
      projectionChanged: false,
    };
  }

  loadFpsRuntimeSession(request: FpsRuntimeSessionLoadRequest): FpsRuntimeSessionSnapshot {
    if (this.#engine === null) {
      throw new RuntimeBridgeError('not_initialized', 'loadFpsRuntimeSession before initializeEngine');
    }
    if (request.projectBundle.trim() === '' || request.definitions.length === 0) {
      throw new RuntimeBridgeError('invalid_input', 'FPS RuntimeSession ProjectBundle is invalid');
    }
    const player = request.definitions.find((definition) => definition.role === 'player');
    const enemy = request.definitions.find((definition) => definition.role === 'enemy');
    if (player === undefined || enemy === undefined) {
      throw new RuntimeBridgeError('invalid_input', 'FPS RuntimeSession requires player and enemy definitions');
    }
    this.#fpsEpoch += 1;
    this.#fpsSeed = request;
    this.#fpsEncounter = initialFpsEncounterState();
    const health = request.definitions.flatMap((definition) =>
      definition.health === null
        ? []
        : [{ entity: definition.entity, current: definition.health.current, max: definition.health.max }],
    );
    const policyBindings = request.definitions.flatMap((definition) =>
      definition.policyBinding === null ? [] : [{ entity: definition.entity, ...definition.policyBinding }],
    );
    const entityHash = `fnv1a64:${fnv1a64(JSON.stringify({ projectBundle: request.projectBundle, definitions: request.definitions.map((d) => d.entity) }))}`;
    const healthHash = `fnv1a64:${fnv1a64(JSON.stringify(health))}`;
    const replayHash = `fnv1a64:${fnv1a64(`${entityHash}|${healthHash}|runtime_session.fps.bootstrap.v0`)}`;
    this.#fpsSnapshot = {
      backend: 'reference_bridge',
      authoritySurface: 'runtime_session.fps.reference.v0',
      projectBundle: request.projectBundle,
      sessionEpoch: this.#fpsEpoch,
      lifecycleStatus: { state: 'active' },
      playerEntity: player.entity,
      enemyEntity: enemy.entity,
      health,
      policyBindings,
      replayRecords: [{ replayUnit: 'runtime_session.fps.bootstrap.reference.v0', entityHash, healthHash, recordHash: replayHash }],
      readSets: [
        { viewKind: 'runtime_session.lifecycle.v0', owner: 'reference-bridge', readSet: ['mock.lifecycle'] },
        { viewKind: 'runtime_session.health.v0', owner: 'reference-bridge', readSet: ['mock.health'] },
      ],
      entityHash,
      healthHash,
      replayHash,
    };
    return this.#fpsSnapshot;
  }

  readFpsRuntimeSession(): FpsRuntimeSessionSnapshot {
    if (this.#fpsSnapshot === null) {
      throw new RuntimeBridgeError('not_initialized', 'readFpsRuntimeSession before loadFpsRuntimeSession');
    }
    return this.#fpsSnapshot;
  }

  applyFpsPrimaryFire(request: FpsPrimaryFireRequest): FpsPrimaryFireResult {
    if (this.#fpsSnapshot === null || this.#fpsSeed === null) {
      throw new RuntimeBridgeError('not_initialized', 'applyFpsPrimaryFire before loadFpsRuntimeSession');
    }
    const tick = nonNegativeSafeInteger(request.tick, 'tick');
    validateVec3(request.origin, 'origin');
    validateVec3(request.direction, 'direction');
    const shooterRole = request.shooterRole ?? 'player';
    const targetRole = request.targetRole ?? 'enemy';
    const shooter = this.#fpsSeed.definitions.find((definition) => definition.role === shooterRole);
    const target = this.#fpsSeed.definitions.find((definition) => definition.role === targetRole);
    if (shooter === undefined || target === undefined) {
      throw new RuntimeBridgeError('invalid_input', 'FPS RuntimeSession is missing shooter or target role');
    }
    const enemyPolicyWeapon = {
      weaponId: 'weapon.enemy_policy.primary',
      damage: 10,
      rangeUnits: 16,
      ammo: 2,
      cooldownTicksAfterFire: 4,
    };
    const weapon = shooter.weapon ?? (
      shooterRole === 'enemy' && targetRole === 'player' ? enemyPolicyWeapon : null
    );
    if (weapon === null) {
      throw new RuntimeBridgeError('invalid_input', 'FPS RuntimeSession shooter is missing weapon');
    }
    const before = this.#fpsSnapshot.health.find((health) => health.entity === target.entity) ?? null;
    const after = before === null
      ? null
      : { ...before, current: Math.max(0, before.current - weapon.damage) };
    const health = this.#fpsSnapshot.health.map((entry) => (entry.entity === target.entity && after !== null ? after : entry));
    const lifecycleStatus = targetRole === 'enemy' && after !== null && after.current === 0
      ? { state: 'enemy_defeated' as const, entity: target.entity, tick }
      : this.#fpsSnapshot.lifecycleStatus;
    const healthHash = `fnv1a64:${fnv1a64(JSON.stringify(health))}`;
    const replayHash = `fnv1a64:${fnv1a64(`${this.#fpsSnapshot.entityHash}|${healthHash}|${tick}|runtime_session.fps.primary_fire.reference.v0`)}`;
    const record = {
      replayUnit: 'runtime_session.fps.primary_fire.reference.v0',
      entityHash: this.#fpsSnapshot.entityHash,
      healthHash,
      recordHash: replayHash,
    };
    this.#fpsSnapshot = {
      ...this.#fpsSnapshot,
      lifecycleStatus,
      health,
      healthHash,
      replayHash,
      replayRecords: [...this.#fpsSnapshot.replayRecords, record],
    };
    return {
      backend: 'reference_bridge',
      authoritySurface: 'runtime_session.fps.reference_primary_fire.v0',
      mutationOwner: 'reference-bridge',
      workspaceTrace: ['reference fixture primary-fire receipt'],
      shooter: shooter.entity,
      target: target.entity,
      targetHealthBefore: before,
      targetHealthAfter: after,
      lifecycleStatus,
      targetRenderVisible: targetRole === 'enemy' && lifecycleStatus.state === 'enemy_defeated' ? false : true,
      entityHash: this.#fpsSnapshot.entityHash,
      healthHash,
      replayHash,
    };
  }

  readComposedRuntimeSession(): ComposedRuntimeSessionReadout {
    throw new RuntimeBridgeError(
      'operation_unimplemented',
      'reference bridge does not claim composed gameplay authority',
    );
  }

  readGameplayModuleView(): GameplayModuleViewSnapshot {
    throw new RuntimeBridgeError(
      'operation_unimplemented',
      'reference bridge does not claim composed gameplay module views',
    );
  }

  applyGameplayPrefabPartInteraction(): GameplayPrefabPartInteractionReceipt {
    throw new RuntimeBridgeError(
      'operation_unimplemented',
      'reference bridge does not claim composed prefab interaction authority',
    );
  }

  invokeGameExtensionWeaponEffect(
    request: GameExtensionWeaponEffectInvocationRequest,
  ): GameExtensionWeaponEffectInvocationResult {
    if (this.#fpsSnapshot === null || this.#fpsSeed === null) {
      throw new RuntimeBridgeError('not_initialized', 'invokeGameExtensionWeaponEffect before loadFpsRuntimeSession');
    }
    const declared = this.#fpsSeed.gameRuleModules.find(
      (manifest) => manifest.moduleRef.moduleId === request.hook.moduleRef.moduleId,
    );
    if (declared === undefined || JSON.stringify(declared.moduleRef) !== JSON.stringify(request.hook.moduleRef)) {
      throw new RuntimeBridgeError('invalid_input', 'game rule module is not declared by the loaded RuntimeSession');
    }
    const hookReceipt: GameExtensionHookReceipt = {
      moduleRef: request.hook.moduleRef,
      hookId: request.hook.hookId,
      requestId: request.hook.requestId,
      status: 'proposed',
      inputHash: request.hook.inputHash,
      proposal: request.hook.target === null
        ? { kind: 'noop', proposalId: `${request.hook.requestId}.noop`, proposalHash: 'fnv1a64:mock-noop' }
        : {
            kind: 'damageModifier',
            proposalId: `${request.hook.requestId}.damage_bonus`,
            target: request.hook.target,
            channelId: 'combat.primary_fire.damage',
            amountDelta: 5,
            tags: ['reference-mock-module'],
            proposalHash: `fnv1a64:${fnv1a64(JSON.stringify(request.hook))}`,
          },
      diagnostics: [],
      trace: [{
        step: 1,
        code: 'mock.module.proposed_damage_modifier',
        message: 'mock bridge returned a typed extension proposal',
        refs: [request.hook.moduleRef.moduleId],
      }],
      proposalHash: `fnv1a64:${fnv1a64(`${request.hook.inputHash}|proposal`)}`,
    };
    const primaryFire = this.applyFpsPrimaryFire(request.primaryFire);
    const replayEvidence: GameExtensionReplayEvidence = {
      moduleRef: request.hook.moduleRef,
      hookId: request.hook.hookId,
      requestId: request.hook.requestId,
      inputHash: request.hook.inputHash,
      proposalHash: hookReceipt.proposalHash,
      validationStatus: 'accepted',
      eventHashes: [primaryFire.replayHash],
      rejectionHashes: [],
      replayHash: `fnv1a64:${fnv1a64(`${hookReceipt.proposalHash}|${primaryFire.replayHash}`)}`,
    };
    return { hookReceipt, replayEvidence, primaryFire };
  }

  validateGameRuleCatalog(catalog: GameRuleCatalog): GameRuleCatalogValidationReceipt {
    if (this.#engine === null) {
      throw new RuntimeBridgeError('not_initialized', 'validateGameRuleCatalog before initializeEngine');
    }
    return this.#gameRules.validateCatalog(catalog);
  }

  submitGameRuleEffectIntent(input: GameRuleEffectIntentRequest): GameRuleResolutionReceipt {
    if (this.#engine === null) {
      throw new RuntimeBridgeError('not_initialized', 'submitGameRuleEffectIntent before initializeEngine');
    }
    return this.#gameRules.submitEffectIntent(input);
  }

  readGameRuleRuntimeReadout(): GameRuleRuntimeReadout {
    if (this.#engine === null) {
      throw new RuntimeBridgeError('not_initialized', 'readGameRuleRuntimeReadout before initializeEngine');
    }
    return this.#gameRules.readRuntimeReadout();
  }

  restartFpsRuntimeSession(request: FpsRuntimeSessionRestartRequest): FpsRuntimeSessionSnapshot {
    if (this.#fpsSeed === null) {
      throw new RuntimeBridgeError('not_initialized', 'restartFpsRuntimeSession before loadFpsRuntimeSession');
    }
    const expectedEpoch = nonNegativeSafeInteger(request.expectedEpoch, 'expectedEpoch');
    if (expectedEpoch !== this.#fpsEpoch) {
      throw new RuntimeBridgeError('invalid_input', `restart expected epoch ${expectedEpoch} but current epoch is ${this.#fpsEpoch}`);
    }
    return this.loadFpsRuntimeSession(this.#fpsSeed);
  }

  readFpsEncounterDirector(lifecycle: FpsEncounterLifecycleInput): FpsEncounterDirectorSnapshot {
    if (this.#fpsSnapshot === null) {
      throw new RuntimeBridgeError('not_initialized', 'readFpsEncounterDirector before loadFpsRuntimeSession');
    }
    return this.#fpsEncounterSnapshot(lifecycle);
  }

  applyFpsEncounterTransition(request: FpsEncounterTransitionRequest): FpsEncounterTransitionResult {
    if (this.#fpsSnapshot === null) {
      throw new RuntimeBridgeError('not_initialized', 'applyFpsEncounterTransition before loadFpsRuntimeSession');
    }
    let accepted = true;
    let rejectionReason: FpsEncounterTransitionResult['rejectionReason'] = null;
    let eventKind: FpsEncounterTransitionResult['eventKind'] = null;
    if (request.presetId !== 'generated-tunnel-small-encounter') {
      accepted = false;
      rejectionReason = 'unknown_encounter_preset';
    } else if (request.action === 'reset') {
      eventKind = 'runtime_encounter.reset.v0';
      this.#fpsEncounter = { ...initialFpsEncounterState(), revision: this.#fpsEncounter.revision + 1, lastTransition: 'reset' };
    } else if (request.action === 'activate') {
      if (this.#fpsEncounter.status !== 'pending') {
        accepted = false;
        rejectionReason = 'encounter_not_pending';
      } else {
        eventKind = 'runtime_encounter.activated.v0';
        this.#fpsEncounter = {
          ...this.#fpsEncounter,
          status: 'active',
          spawnedEnemyIds: ['encounter.generated_tunnel_small.wave_1.enemy_001'],
          revision: this.#fpsEncounter.revision + 1,
          lastTransition: 'activated',
        };
      }
    } else if (request.action === 'sync_lifecycle') {
      eventKind = 'runtime_encounter.lifecycle_synced.v0';
      if (request.lifecycle.playerDead || request.lifecycle.outcomeKind === 'lost') {
        this.#fpsEncounter = {
          ...this.#fpsEncounter,
          status: 'failed',
          revision: this.#fpsEncounter.revision + 1,
          lastTransition: 'failed',
        };
      } else if (request.lifecycle.enemyDead || request.lifecycle.outcomeKind === 'won') {
        this.#fpsEncounter = {
          ...this.#fpsEncounter,
          status: 'cleared',
          spawnedEnemyIds: ['encounter.generated_tunnel_small.wave_1.enemy_001'],
          defeatedEnemyIds: ['encounter.generated_tunnel_small.wave_1.enemy_001'],
          revision: this.#fpsEncounter.revision + 1,
          lastTransition: 'cleared',
        };
      } else {
        this.#fpsEncounter = {
          ...this.#fpsEncounter,
          revision: this.#fpsEncounter.revision + 1,
        };
      }
    } else {
      accepted = false;
      rejectionReason = 'invalid_encounter_transition';
    }
    const encounterHash = fpsEncounterHash(this.#fpsEncounter, request.lifecycle);
    const replayHash = `fnv1a64:${fnv1a64(JSON.stringify({
      presetId: request.presetId,
      action: request.action,
      accepted,
      rejectionReason,
      eventKind,
      encounterHash,
    }))}`;
    if (accepted) {
      this.#fpsSnapshot = {
        ...this.#fpsSnapshot,
        replayHash,
        replayRecords: [
          ...this.#fpsSnapshot.replayRecords,
          {
            replayUnit: eventKind ?? 'runtime_session.fps.encounter_transition.reference.v0',
            entityHash: this.#fpsSnapshot.entityHash,
            healthHash: this.#fpsSnapshot.healthHash,
            recordHash: replayHash,
          },
        ],
      };
    }
    return {
      backend: 'reference_bridge',
      authoritySurface: 'runtime_session.fps.reference_encounter_transition.v0',
      mutationOwner: 'reference-bridge',
      workspaceTrace: ['reference fixture encounter transition'],
      accepted,
      rejectionReason,
      eventKind,
      state: this.#fpsEncounter,
      lifecycle: request.lifecycle,
      encounterHash,
      replayHash,
    };
  }

  #fpsEncounterSnapshot(lifecycle: FpsEncounterLifecycleInput): FpsEncounterDirectorSnapshot {
    if (this.#fpsSnapshot === null) {
      throw new RuntimeBridgeError('not_initialized', 'readFpsEncounterDirector before loadFpsRuntimeSession');
    }
    const encounterHash = fpsEncounterHash(this.#fpsEncounter, lifecycle);
    return {
      backend: 'reference_bridge',
      authoritySurface: 'runtime_session.fps.reference_encounter_director.v0',
      mutationOwner: 'reference-bridge',
      workspaceTrace: ['reference fixture encounter readout'],
      state: this.#fpsEncounter,
      lifecycle,
      readSets: [
        { viewKind: 'runtime_session.encounter_director.v0', owner: 'reference-bridge', readSet: ['mock.encounter'] },
      ],
      encounterHash,
      replayHash: this.#fpsSnapshot.replayHash,
    };
  }

  submitCommands(batch: CommandBatch): CommandResult {
    if (this.#engine === null && this.#workspaceAuthoringState?.status !== 'open') {
      throw new RuntimeBridgeError('not_initialized', 'submitCommands before initializeEngine');
    }
    const rejections: Array<CommandResult['rejections'][number]> = [];
    for (const command of batch.commands) {
      const value = command.op === 'setVoxel' || command.op === 'fillRegion' ? command.value : null;
      if (value?.kind === 'solid' && (value.material < 1 || value.material > 16)) {
        rejections.push({ reason: 'unknownMaterial', material: value.material });
      }
    }
    const result = {
      accepted: batch.commands.length - rejections.length,
      rejected: rejections.length,
      rejections,
    };
    if (result.accepted > 0) this.#recordMockWorkspaceMutation();
    return result;
  }

  pickVoxel(ray: PickRay): PickResult {
    if (this.#engine === null) {
      throw new RuntimeBridgeError('not_initialized', 'pickVoxel before initializeEngine');
    }
    // The mock hosts no authority voxel geometry (Rust `svc-collision` owns the
    // raycast on the native path), so a pick always classifies as a miss. It still
    // fail-closes on the transport precondition (init) and validates the ray shape.
    if (ray.direction.every((c) => c === 0)) {
      throw new RuntimeBridgeError('invalid_input', 'pick ray direction must be non-zero');
    }
    return { outcome: 'miss', rejection: { reason: 'noHit' } };
  }

  configureVoxelProjectionInstances(
    _request: VoxelProjectionBindingRequest,
  ): VoxelProjectionBindingReceipt {
    void _request;
    return this.#unsupportedAfterInit(
      'configureVoxelProjectionInstances',
      'mock bridge does not own retained voxel instance authority',
    );
  }

  pickVoxelInstance(_request: VoxelInstancePickRequest): VoxelInstancePickResult {
    void _request;
    return this.#unsupportedAfterInit(
      'pickVoxelInstance',
      'mock bridge does not own transformed voxel picking authority',
    );
  }

  applyCollisionConstrainedCameraInput(input: CollisionConstrainedCameraInputEnvelope): CameraCollisionSnapshot {
    if (this.#engine === null) {
      throw new RuntimeBridgeError('not_initialized', 'applyCollisionConstrainedCameraInput before initializeEngine');
    }
    if (input.grid !== 1) {
      throw new RuntimeBridgeError('invalid_input', 'collision camera input targets an unknown grid');
    }
    const before = this.#cameras.get(input.camera);
    if (before === undefined) {
      throw new RuntimeBridgeError('unknown_handle', 'unknown camera handle');
    }
    if (!this.#cameraControllers.isFirstPerson(input.camera)) {
      throw new RuntimeBridgeError('invalid_input', 'collision camera input requires firstPerson camera mode');
    }
    const cameraInput = input.input;
    finite(cameraInput.moveForward, 'moveForward');
    finite(cameraInput.moveRight, 'moveRight');
    finite(cameraInput.moveUp, 'moveUp');
    finite(cameraInput.yawDeltaDegrees, 'yawDeltaDegrees');
    finite(cameraInput.pitchDeltaDegrees, 'pitchDeltaDegrees');
    finite(cameraInput.dtSeconds, 'dtSeconds');
    finite(cameraInput.moveSpeedUnitsPerSecond, 'moveSpeedUnitsPerSecond');
    if (cameraInput.dtSeconds < 0 || cameraInput.moveSpeedUnitsPerSecond < 0) {
      throw new RuntimeBridgeError('invalid_input', 'dtSeconds and moveSpeedUnitsPerSecond must be non-negative');
    }
    if (input.movementMode === 'grounded' && cameraInput.moveUp !== 0) {
      throw new RuntimeBridgeError(
        'invalid_input',
        'grounded camera input requires moveUp to be zero; select freeFlight for vertical locomotion',
      );
    }
    for (const [idx, halfExtent] of input.shape.halfExtents.entries()) {
      finite(halfExtent, `shape.halfExtents[${idx}]`);
      if (halfExtent <= 0) {
        throw new RuntimeBridgeError('invalid_input', 'collision shape halfExtents must be positive');
      }
    }
    if (input.policy.mode !== 'axis_separable_slide' || input.policy.maxIterations < 1 || input.policy.maxIterations > 3) {
      throw new RuntimeBridgeError('invalid_input', 'only axis_separable_slide with maxIterations in 1..=3 is supported');
    }
    const attemptedPose = collisionCameraAttemptedPose(before, input);
    const attempted: CameraSnapshot = { ...before, tick: input.tick, pose: attemptedPose, basis: basisFromPose(attemptedPose) };
    const delta = [
      motionDelta(attempted.pose.position[0] - before.pose.position[0]),
      motionDelta(attempted.pose.position[1] - before.pose.position[1]),
      motionDelta(attempted.pose.position[2] - before.pose.position[2]),
    ] as const;
    let afterPose: CameraSnapshot['pose'] = {
      position: before.pose.position,
      yawDegrees: attempted.pose.yawDegrees,
      pitchDegrees: attempted.pose.pitchDegrees,
    };
    const blockedAxes: CollisionAxis[] = [];
    for (const [axisIndex, axis] of [
      [0, 'x'],
      [1, 'y'],
      [2, 'z'],
    ] as const) {
      if (delta[axisIndex] === 0) {
        continue;
      }
      const candidatePose = poseWithAxis(afterPose, axisIndex, afterPose.position[axisIndex] + delta[axisIndex]);
      if (staticRoomMoveBlocked(afterPose, candidatePose, input.shape)) {
        blockedAxes.push(axis);
      } else {
        afterPose = candidatePose;
      }
    }
    const after: CameraSnapshot = { ...before, tick: input.tick, pose: afterPose, basis: basisFromPose(afterPose) };
    const queriedAabb = aabbForPose(after.pose, input.shape);
    const correction = [
      f32(after.pose.position[0] - attempted.pose.position[0]),
      f32(after.pose.position[1] - attempted.pose.position[1]),
      f32(after.pose.position[2] - attempted.pose.position[2]),
    ] as const;
    this.#cameras.set(input.camera, after);
    this.#cameraControllers.syncFirstPerson(after);
    return {
      camera: input.camera,
      tick: input.tick,
      before,
      attempted,
      after,
      collision: {
        grid: input.grid,
        movementMode: input.movementMode,
        shape: input.shape,
        policy: input.policy,
        collided: blockedAxes.length > 0,
        blockedAxes,
        correction,
        queriedAabb,
        collisionSourceHash: STATIC_ROOM_COLLISION_SOURCE_HASH,
        collisionProjectionHash: STATIC_ROOM_COLLISION_PROJECTION_HASH,
      },
      movementHash: `fnv1a64:${fnv1a64(
        `${input.camera}|${input.tick}|${input.movementMode}|${JSON.stringify(before.pose)}|${JSON.stringify(attempted.pose)}|${JSON.stringify(after.pose)}|${STATIC_ROOM_COLLISION_SOURCE_HASH}|${STATIC_ROOM_COLLISION_PROJECTION_HASH}`,
      )}`,
    };
  }

  applyGeneratedTunnelToRuntimeWorld(_request: GeneratedTunnelRuntimeApplyRequest): GeneratedTunnelRuntimeApplyReceipt {
    void _request; throw new RuntimeBridgeError('operation_unimplemented', 'generated tunnel apply requires Rust authority');
  }

  selectVoxel(request: ScreenPointToPickRayRequest): VoxelSelectionSnapshot {
    if (this.#engine === null) {
      throw new RuntimeBridgeError('not_initialized', 'selectVoxel before initializeEngine');
    }
    const camera = this.#cameras.get(request.camera);
    if (camera === undefined) {
      throw new RuntimeBridgeError('unknown_handle', 'unknown camera handle');
    }
    if (request.grid !== 1) {
      throw new RuntimeBridgeError('invalid_input', 'selectVoxel request targets an unknown grid');
    }
    finite(request.maxDistance, 'maxDistance');
    if (request.maxDistance <= 0) {
      throw new RuntimeBridgeError('invalid_input', 'maxDistance must be positive');
    }
    const viewport = request.viewport ?? camera.viewport;
    validateViewport(viewport);
    const sx = request.screenPoint.space === 'pixel' ? request.screenPoint.x / viewport.width : request.screenPoint.x;
    const sy = request.screenPoint.space === 'pixel' ? request.screenPoint.y / viewport.height : request.screenPoint.y;
    if (!Number.isFinite(sx) || !Number.isFinite(sy) || sx < 0 || sx > 1 || sy < 0 || sy > 1) {
      throw new RuntimeBridgeError('invalid_input', 'screen point must be inside the viewport');
    }
    const ndcX = sx * 2 - 1;
    const ndcY = 1 - sy * 2;
    const tanY = Math.tan((camera.projection.fovYDegrees * Math.PI) / 360);
    const tanX = tanY * (viewport.width / viewport.height);
    const raw = [
      camera.basis.forward[0] + camera.basis.right[0] * ndcX * tanX + camera.basis.up[0] * ndcY * tanY,
      camera.basis.forward[1] + camera.basis.right[1] * ndcX * tanX + camera.basis.up[1] * ndcY * tanY,
      camera.basis.forward[2] + camera.basis.right[2] * ndcX * tanX + camera.basis.up[2] * ndcY * tanY,
    ] as const;
    const len = Math.hypot(raw[0], raw[1], raw[2]);
    if (!Number.isFinite(len) || len <= 0) {
      throw new RuntimeBridgeError('invalid_input', 'derived pick ray direction is invalid');
    }
    const origin = [camera.pose.position[0], camera.pose.position[1], camera.pose.position[2]] as const;
    const direction = [raw[0] / len, raw[1] / len, raw[2] / len] as const;
    const pickRay = {
      camera: request.camera,
      tick: camera.tick,
      grid: request.grid,
      screenPoint: request.screenPoint,
      origin,
      direction,
      maxDistance: request.maxDistance,
      cameraProjectionHash: mockCameraProjectionSnapshot(camera, viewport).projectionHash,
      rayHash: `fnv1a64:${fnv1a64(`${request.camera}|${request.grid}|${origin.join(',')}|${direction.join(',')}|${request.maxDistance}`)}`,
    };

    // Mock fixture mirrors the canonical launch world enough for downstream
    // conformance: a flat solid terrain slab covering x/y [-16,16) at z=[0,1).
    let selectedVoxel: VoxelCoord | null = null;
    let selectedFace: Face | null = null;
    let editAnchor: VoxelCoord | null = null;
    if (direction[2] < 0) {
      const t = (1 - origin[2]) / direction[2];
      const x = origin[0] + direction[0] * t;
      const y = origin[1] + direction[1] * t;
      if (t >= 0 && t <= request.maxDistance && x >= -16 && x < 16 && y >= 0 && y < 16) {
        selectedVoxel = { x: Math.floor(x), y: Math.floor(y), z: 0 };
        selectedFace = 'posZ';
        editAnchor = { x: selectedVoxel.x, y: selectedVoxel.y, z: 1 };
      }
    }
    const outcome = selectedVoxel === null ? 'miss' : 'hit';
    return {
      pickRay,
      outcome,
      selectedVoxel,
      selectedFace,
      editAnchor,
      selectionHash: `fnv1a64:${fnv1a64(`${pickRay.rayHash}|${outcome}|${JSON.stringify(selectedVoxel)}|${JSON.stringify(editAnchor)}`)}`,
    };
  }

  readVoxelMeshEvidence(request: VoxelMeshEvidenceRequest): VoxelMeshEvidenceSnapshot {
    if (this.#engine === null) {
      throw new RuntimeBridgeError('not_initialized', 'readVoxelMeshEvidence before initializeEngine');
    }
    if (request.grid !== 1) {
      throw new RuntimeBridgeError('invalid_input', 'readVoxelMeshEvidence request targets an unknown grid');
    }
    const chunks = request.chunks.length === 0 ? [{ x: 0, y: 0, z: 0 }] : request.chunks;
    return {
      grid: request.grid,
      fixtureId: 'basic-voxel-landscape-interaction',
      voxelStateHash: 'mock-voxel-state',
      meshingStrategy: 'visible-face',
      chunks: chunks.map((coord) => ({
        coord,
        resident: coord.x === 0 && coord.y === 0 && coord.z === 0,
        visible: coord.x === 0 && coord.y === 0 && coord.z === 0,
        contentHash: coord.x === 0 && coord.y === 0 && coord.z === 0 ? 'mock-content' : null,
        meshHash: coord.x === 0 && coord.y === 0 && coord.z === 0 ? 'fnv1a64:mock-mesh' : null,
        stats:
          coord.x === 0 && coord.y === 0 && coord.z === 0
            ? { vertices: 48, indices: 72, quads: 12, facesEmitted: 12, facesCulled: 12 }
            : null,
        bounds: coord.x === 0 && coord.y === 0 && coord.z === 0 ? { min: [0, 0, 0], max: [2, 2, 1] } : null,
        materialSlots: coord.x === 0 && coord.y === 0 && coord.z === 0 ? [1] : [],
      })),
      diagnostics: [],
    };
  }

  planVoxelConversion(_request: VoxelConversionPlanRequest): VoxelConversionPlan {
    void _request;
    if (this.#engine === null) {
      throw new RuntimeBridgeError('not_initialized', 'planVoxelConversion before initializeEngine');
    }
    throw new RuntimeBridgeError('operation_unimplemented', 'mock bridge does not own voxel conversion authority');
  }

  registerVoxelConversionSource(
    _request: VoxelConversionSourceRegistrationRequest,
  ): VoxelConversionSourceRegistration {
    void _request;
    if (this.#engine === null) {
      throw new RuntimeBridgeError('not_initialized', 'registerVoxelConversionSource before initializeEngine');
    }
    throw new RuntimeBridgeError('operation_unimplemented', 'mock bridge does not own voxel conversion authority');
  }

  registerVoxelConversionMeshAsset(
    _request: VoxelConversionMeshAssetRegistrationRequest,
  ): VoxelConversionSourceRegistration {
    void _request;
    if (this.#engine === null) {
      throw new RuntimeBridgeError('not_initialized', 'registerVoxelConversionMeshAsset before initializeEngine');
    }
    throw new RuntimeBridgeError('operation_unimplemented', 'mock bridge does not own voxel conversion authority');
  }

  importVoxelConversionMeshSource(
    _request: VoxelConversionMeshSourceImportRequest,
  ): VoxelConversionMeshSourceImportReceipt {
    void _request;
    if (this.#engine === null) {
      throw new RuntimeBridgeError('not_initialized', 'importVoxelConversionMeshSource before initializeEngine');
    }
    throw new RuntimeBridgeError('operation_unimplemented', 'mock bridge does not own voxel conversion authority');
  }

  readVoxelConversionSourceMetadata(
    _request: VoxelConversionSourceMetadataRequest,
  ): VoxelConversionSourceMetadataReadout {
    void _request;
    if (this.#engine === null) {
      throw new RuntimeBridgeError('not_initialized', 'readVoxelConversionSourceMetadata before initializeEngine');
    }
    throw new RuntimeBridgeError('operation_unimplemented', 'mock bridge does not own voxel conversion authority');
  }

  previewVoxelConversion(_request: VoxelConversionPreviewRequest): VoxelConversionPreview {
    void _request;
    if (this.#engine === null) {
      throw new RuntimeBridgeError('not_initialized', 'previewVoxelConversion before initializeEngine');
    }
    throw new RuntimeBridgeError('operation_unimplemented', 'mock bridge does not own voxel conversion authority');
  }

  applyVoxelConversion(_request: VoxelConversionApplyRequest): VoxelConversionReceipt {
    void _request;
    if (this.#engine === null) {
      throw new RuntimeBridgeError('not_initialized', 'applyVoxelConversion before initializeEngine');
    }
    throw new RuntimeBridgeError('operation_unimplemented', 'mock bridge does not own voxel conversion authority');
  }

  exportVoxelConversionEvidence(
    _evidence: readonly VoxelConversionEvidenceRef[],
  ): readonly VoxelConversionEvidenceRef[] {
    void _evidence;
    if (this.#engine === null) {
      throw new RuntimeBridgeError('not_initialized', 'exportVoxelConversionEvidence before initializeEngine');
    }
    throw new RuntimeBridgeError('operation_unimplemented', 'mock bridge does not own voxel conversion authority');
  }

  readVoxelModelInfo(_request: VoxelModelInfoRequest): VoxelModelInfoReadout { void _request; return this.#unsupportedAfterInit('readVoxelModelInfo', 'mock bridge does not own voxel model authority'); }

  readVoxelModelWindow(_request: VoxelModelWindowRequest): VoxelModelWindowReadout { void _request; return this.#unsupportedAfterInit('readVoxelModelWindow', 'mock bridge does not own voxel model authority'); }

  exportVoxelVolumeAsset(_request: VoxelVolumeAssetExportRequest): VoxelVolumeAssetExportReceipt { void _request; return this.#unsupportedAfterInit('exportVoxelVolumeAsset', 'mock bridge does not own voxel volume asset export authority'); }

  saveVoxelVolumeAsset(_request: VoxelVolumeAssetSaveRequest): VoxelVolumeAssetSaveReceipt { void _request; return this.#unsupportedAfterInit('saveVoxelVolumeAsset', 'mock bridge does not own voxel volume asset save authority'); }

  updateVoxelVolumeAssetPalette(_request: VoxelVolumeAssetPaletteUpdateRequest): VoxelVolumeAssetPaletteUpdateReceipt { void _request; return this.#unsupportedAfterInit('updateVoxelVolumeAssetPalette', 'mock bridge does not own durable voxel palette authority'); }

  initializeVoxelVolumeAuthoring(_request: VoxelVolumeAuthoringInitializeRequest): VoxelVolumeAuthoringInitializeReceipt { void _request; return this.#unsupportedAfterInit('initializeVoxelVolumeAuthoring', 'mock bridge does not own voxel volume authoring initialization authority'); }

  loadVoxelVolumeAsset(_request: VoxelVolumeAssetLoadRequest): VoxelVolumeAssetLoadReceipt { void _request; return this.#unsupportedAfterInit('loadVoxelVolumeAsset', 'mock bridge does not own voxel volume asset load authority'); }

  unloadVoxelVolumeAsset(_request: VoxelVolumeAssetUnloadRequest): VoxelVolumeAssetUnloadReceipt { void _request; return this.#unsupportedAfterInit('unloadVoxelVolumeAsset', 'mock bridge does not own voxel volume asset unload authority'); }

  validateVoxelAnnotationLayer(_request: VoxelAnnotationLayerValidationRequest): VoxelAnnotationLayerValidationReport { void _request; return this.#unsupportedAfterInit('validateVoxelAnnotationLayer', 'mock bridge does not own voxel annotation validation authority'); }

  loadVoxelAnnotationLayer(_request: VoxelAnnotationLayerLoadRequest): VoxelAnnotationLayerLoadReceipt { void _request; return this.#unsupportedAfterInit('loadVoxelAnnotationLayer', 'mock bridge does not own voxel annotation load authority'); }

  readVoxelAnnotationQuery(_request: VoxelAnnotationQueryRequest): VoxelAnnotationQueryReadout { void _request; return this.#unsupportedAfterInit('readVoxelAnnotationQuery', 'mock bridge does not own voxel annotation query authority'); }

  applyVoxelAnnotationEdit(_request: VoxelAnnotationEditRequest): VoxelAnnotationEditReceipt { void _request; return this.#unsupportedAfterInit('applyVoxelAnnotationEdit', 'mock bridge does not own voxel annotation edit authority'); }

  exportVoxelAnnotationLayer(_request: VoxelAnnotationLayerExportRequest): VoxelAnnotationLayerExportReceipt { void _request; return this.#unsupportedAfterInit('exportVoxelAnnotationLayer', 'mock bridge does not own voxel annotation export authority'); }

  readVoxelEditHistory(_request: VoxelEditHistoryReadRequest): VoxelEditHistorySummary { void _request; return this.#unsupportedAfterInit('readVoxelEditHistory', 'mock bridge does not own voxel edit history authority'); }

  previewVoxelEditRevert(_request: VoxelEditHistoryRevertRequest): VoxelEditHistoryRevertReceipt { void _request; return this.#unsupportedAfterInit('previewVoxelEditRevert', 'mock bridge does not own voxel edit history authority'); }

  applyVoxelEditRevert(_request: VoxelEditHistoryRevertRequest): VoxelEditHistoryRevertReceipt { void _request; return this.#unsupportedAfterInit('applyVoxelEditRevert', 'mock bridge does not own voxel edit history authority'); }

  undoVoxelEdit(_request: VoxelEditHistoryUndoRequest): VoxelEditHistoryUndoReceipt { void _request; return this.#unsupportedAfterInit('undoVoxelEdit', 'mock bridge does not own voxel edit history authority'); }

  redoVoxelEdit(_request: VoxelEditHistoryRedoRequest): VoxelEditHistoryRedoReceipt { void _request; return this.#unsupportedAfterInit('redoVoxelEdit', 'mock bridge does not own voxel edit history authority'); }

  readModelMaterialPreview(request: ModelMaterialPreviewRequest): ModelMaterialPreviewSnapshot {
    if (this.#engine === null) {
      throw new RuntimeBridgeError('not_initialized', 'readModelMaterialPreview before initializeEngine');
    }
    return mockModelMaterialPreview(request);
  }

  readSceneObjectSnapshot(): SceneObjectSnapshot {
    if (this.#engine === null) {
      throw new RuntimeBridgeError('not_initialized', 'readSceneObjectSnapshot before initializeEngine');
    }
    return sceneObjectSnapshotFromDocument(this.#sceneDocument);
  }

  decodeSceneDocument(_request: SceneDocumentDecodeRequest): SceneDocumentCodecResult {
    void _request;
    return this.#unsupportedAfterInit(
      'decodeSceneDocument',
      'Canonical stored scene decoding requires Rust authority',
    );
  }

  encodeSceneDocument(_request: SceneDocumentEncodeRequest): SceneDocumentCodecResult {
    void _request;
    return this.#unsupportedAfterInit(
      'encodeSceneDocument',
      'Canonical stored scene encoding requires Rust authority',
    );
  }

  applySceneDocumentAuthoring(_request: SceneDocumentAuthoringRequest): SceneDocumentAuthoringResult {
    void _request;
    return this.#unsupportedAfterInit(
      'applySceneDocumentAuthoring',
      'Stored scene authoring transactions require Rust authority',
    );
  }

  decodeProjectContent(_request: ProjectContentDecodeRequest): ProjectContentCodecResult {
    void _request;
    return this.#unsupportedAfterInit(
      'decodeProjectContent',
      'Canonical project-content decoding requires Rust authority',
    );
  }

  encodeProjectContent(_request: ProjectContentEncodeRequest): ProjectContentCodecResult {
    void _request;
    return this.#unsupportedAfterInit(
      'encodeProjectContent',
      'Canonical project-content encoding requires Rust authority',
    );
  }

  applyProjectContentAuthoring(
    _request: ProjectContentAuthoringRequest,
  ): ProjectContentAuthoringResult {
    void _request;
    return this.#unsupportedAfterInit(
      'applyProjectContentAuthoring',
      'Project-content authoring transactions require Rust authority',
    );
  }

  previewProceduralEnvironment(
    _request: ProceduralEnvironmentPreviewRequest,
  ): ProceduralEnvironmentPreviewResult {
    void _request;
    return this.#unsupportedAfterInit(
      'previewProceduralEnvironment',
      'Procedural environment materialization requires Rust authority',
    );
  }

  applyProceduralEnvironment(
    _request: ProceduralEnvironmentApplyRequest,
  ): ProceduralEnvironmentApplyResult {
    void _request;
    return this.#unsupportedAfterInit(
      'applyProceduralEnvironment',
      'Procedural environment candidate apply requires Rust authority',
    );
  }

  applySceneObjectCommand(request: SceneObjectCommandRequest): SceneObjectCommandResult {
    if (this.#engine === null) {
      throw new RuntimeBridgeError('not_initialized', 'applySceneObjectCommand before initializeEngine');
    }
    const result = applyMockSceneObjectCommand(this.#sceneDocument, request);
    if (result.outcome !== null) {
      this.#sceneDocument = result.outcome.document;
    }
    return result;
  }

  readRenderDiffs(cursor: FrameCursor): RenderFrameDiff {
    if (this.#engine === null) {
      throw new RuntimeBridgeError('not_initialized', 'readRenderDiffs before initializeEngine');
    }
    if (!Number.isInteger(cursor as number) || (cursor as number) < 0) {
      throw new RuntimeBridgeError('invalid_input', `frame cursor must be a non-negative integer`);
    }
    return { ops: [] };
  }

  readProjectionFrame(cursor: FrameCursor): RuntimeProjectionFrame {
    const scene = this.readRenderDiffs(cursor);
    return {
      schemaVersion: 1,
      authorityTick: cursor as number,
      scene,
      presentation: {
        replayScope: 'excludedFromReplayTruth',
        ops: [],
      },
    };
  }

  readDeveloperConsole(): DeveloperConsoleSnapshot {
    if (this.#engine === null) {
      throw new RuntimeBridgeError('not_initialized', 'readDeveloperConsole before initializeEngine');
    }
    return {
      schemaVersion: 1,
      records: [{
        sequence: 0,
        severity: 'info',
        category: 'capability',
        source: 'authority',
        message: 'mock runtime capability set attached',
        correlation: `engine:${this.#engine}`,
        authorityTick: 0,
        session: `engine:${this.#engine}`,
        detail: {
          code: 'capability_attached',
          operation: 'initialize_engine',
          resourceKind: null,
          resourceId: null,
          reason: null,
        },
      }],
      droppedRecordCount: 0,
      firstSequence: 0,
      nextSequence: 1,
      snapshotHash: `mock-console:${this.#engine}`,
    };
  }

  createCamera(request: CameraCreateRequest): CameraSnapshot {
    if (this.#engine === null) {
      throw new RuntimeBridgeError('not_initialized', 'createCamera before initializeEngine');
    }
    validateProjection(request.projection);
    validateViewport(request.viewport);
    for (const [index, value] of request.initialPose.position.entries()) {
      finite(value, `initialPose.position[${index}]`);
    }
    finite(request.initialPose.yawDegrees, 'initialPose.yawDegrees');
    finite(request.initialPose.pitchDegrees, 'initialPose.pitchDegrees');
    const camera = this.#nextCamera++ as CameraSnapshot['camera'];
    const snapshot: MutableCameraSnapshot = {
      camera,
      tick: 0,
      pose: request.initialPose,
      basis: basisFromPose(request.initialPose),
      projection: request.projection,
      viewport: request.viewport,
    };
    this.#cameras.set(camera as number, snapshot);
    this.#cameraControllers.create(snapshot);
    return snapshot;
  }

  applyCameraModeCommand(command: CameraModeCommand): CameraModeChangeReceipt {
    if (this.#engine === null) {
      throw new RuntimeBridgeError('not_initialized', 'applyCameraModeCommand before initializeEngine');
    }
    const receipt = this.#cameraControllers.applyMode(command);
    if (receipt === undefined) {
      throw new RuntimeBridgeError('unknown_handle', `no camera for handle ${command.camera}`);
    }
    if (receipt.accepted) this.#cameras.set(command.camera, receipt.after.snapshot);
    return receipt;
  }

  applyCameraNavigationInput(input: CameraNavigationInputEnvelope): CameraNavigationReceipt {
    if (this.#engine === null) {
      throw new RuntimeBridgeError('not_initialized', 'applyCameraNavigationInput before initializeEngine');
    }
    const receipt = this.#cameraControllers.applyNavigation(input);
    if (receipt === undefined) {
      throw new RuntimeBridgeError('unknown_handle', `no camera for handle ${input.camera}`);
    }
    if (receipt.accepted) this.#cameras.set(input.camera, receipt.after.snapshot);
    return receipt;
  }

  readCameraControllerState(request: CameraControllerReadRequest): CameraControllerState {
    if (this.#engine === null) {
      throw new RuntimeBridgeError('not_initialized', 'readCameraControllerState before initializeEngine');
    }
    const controller = this.#cameraControllers.read(request.camera);
    if (controller === undefined) {
      throw new RuntimeBridgeError('unknown_handle', `no camera for handle ${request.camera}`);
    }
    return controller;
  }

  applyFirstPersonCameraInput(envelope: FirstPersonCameraInputEnvelope): CameraSnapshot {
    if (this.#engine === null) {
      throw new RuntimeBridgeError('not_initialized', 'applyFirstPersonCameraInput before initializeEngine');
    }
    const prior = this.#cameras.get(envelope.camera as number);
    if (!prior) {
      throw new RuntimeBridgeError('unknown_handle', `no camera for handle ${envelope.camera}`);
    }
    if (!this.#cameraControllers.isFirstPerson(envelope.camera)) {
      throw new RuntimeBridgeError('invalid_input', 'first-person camera input requires firstPerson camera mode');
    }
    const i = envelope.input;
    finite(i.moveForward, 'moveForward');
    finite(i.moveRight, 'moveRight');
    finite(i.moveUp, 'moveUp');
    finite(i.yawDeltaDegrees, 'yawDeltaDegrees');
    finite(i.pitchDeltaDegrees, 'pitchDeltaDegrees');
    finite(i.dtSeconds, 'dtSeconds');
    finite(i.moveSpeedUnitsPerSecond, 'moveSpeedUnitsPerSecond');
    if (i.dtSeconds < 0 || i.moveSpeedUnitsPerSecond < 0) {
      throw new RuntimeBridgeError('invalid_input', 'dtSeconds and moveSpeedUnitsPerSecond must be non-negative');
    }
    const basis = prior.basis;
    const distance = f32(i.dtSeconds * i.moveSpeedUnitsPerSecond);
    const position = [
      f32(
        prior.pose.position[0] +
          f32(
            f32(basis.forward[0] * i.moveForward) +
              f32(basis.right[0] * i.moveRight) +
              f32(basis.up[0] * i.moveUp),
          ) *
            distance,
      ),
      f32(
        prior.pose.position[1] +
          f32(
            f32(basis.forward[1] * i.moveForward) +
              f32(basis.right[1] * i.moveRight) +
              f32(basis.up[1] * i.moveUp),
          ) *
            distance,
      ),
      f32(
        prior.pose.position[2] +
          f32(
            f32(basis.forward[2] * i.moveForward) +
              f32(basis.right[2] * i.moveRight) +
              f32(basis.up[2] * i.moveUp),
          ) *
            distance,
      ),
    ] as const;
    const pitchDegrees = Math.max(-89, Math.min(89, f32(prior.pose.pitchDegrees + i.pitchDeltaDegrees)));
    const pose = {
      position,
      yawDegrees: f32(prior.pose.yawDegrees + i.yawDeltaDegrees),
      pitchDegrees,
    };
    const snapshot: MutableCameraSnapshot = {
      ...prior,
      tick: envelope.tick,
      pose,
      basis: basisFromPose(pose),
    };
    this.#cameras.set(envelope.camera as number, snapshot);
    this.#cameraControllers.syncFirstPerson(snapshot);
    return snapshot;
  }

  readCameraProjection(request: CameraProjectionRequest): CameraProjectionSnapshot {
    if (this.#engine === null) {
      throw new RuntimeBridgeError('not_initialized', 'readCameraProjection before initializeEngine');
    }
    const snapshot = this.#cameras.get(request.camera as number);
    if (!snapshot) {
      throw new RuntimeBridgeError('unknown_handle', `no camera for handle ${request.camera}`);
    }
    if (request.viewport !== null) validateViewport(request.viewport);
    return mockCameraProjectionSnapshot(snapshot, request.viewport ?? snapshot.viewport);
  }

  getBuffer(handle: RuntimeBufferHandle): RuntimeBufferView {
    if ((handle as number) !== 0) {
      throw new RuntimeBridgeError('unknown_handle', `no buffer for handle ${handle}`);
    }
    return { handle, bytes: this.#buffer };
  }

  releaseBuffer(handle: RuntimeBufferHandle): void {
    if ((handle as number) !== 0) {
      throw new RuntimeBridgeError('unknown_handle', `no buffer for handle ${handle}`);
    }
    this.#buffer = new Uint8Array();
  }

  loadProjectBundle(request: ProjectBundleLoadRequest): CompositionStatus {
    const bundleSchemaVersion = u32(request.bundleSchemaVersion, 'bundleSchemaVersion');
    const protocolVersion = u32(request.protocolVersion, 'protocolVersion');
    const sceneId = nonNegativeSafeInteger(request.sceneId, 'sceneId');
    // Fail closed on a newer bundle; the prior loaded world is left untouched
    // (we only set #loadedProjectBundle on success — the staged commit/swap).
    if (bundleSchemaVersion > 2 || protocolVersion > 1) {
      throw new RuntimeBridgeError(
        'invalid_input',
        `unsupported bundle schema ${bundleSchemaVersion} / protocol ${protocolVersion}`,
      );
    }
    this.#loadedProjectBundle = sceneId;
    return { loadedProjectBundle: sceneId, fatalCount: 0, totalCount: 0, blocksLoad: false };
  }

  beginRuntimeProjectSourceResources(
    request: ProjectResourceBeginRequest,
  ): ProjectResourceTransactionReceipt {
    if (this.#engine === null) {
      throw new RuntimeBridgeError('not_initialized', 'beginRuntimeProjectSourceResources before initializeEngine');
    }
    return this.#runtimeProjects.begin(request);
  }

  stageRuntimeProjectSourceResource(
    request: ProjectResourceStageInput,
  ): StagedProjectResourceRef {
    if (this.#engine === null) {
      throw new RuntimeBridgeError('not_initialized', 'stageRuntimeProjectSourceResource before initializeEngine');
    }
    return this.#runtimeProjects.stage(request);
  }

  admitRuntimeProjectSourceBatch(
    request: RuntimeProjectSourceBatch,
  ): ProjectSourceBatchValidationReceipt {
    if (this.#engine === null) {
      throw new RuntimeBridgeError('not_initialized', 'admitRuntimeProjectSourceBatch before initializeEngine');
    }
    return this.#runtimeProjects.admit(request);
  }

  loadRuntimeProject(request: RuntimeProjectLoadRequest): RuntimeProjectLoadReceipt {
    if (this.#engine === null) {
      throw new RuntimeBridgeError('not_initialized', 'loadRuntimeProject before initializeEngine');
    }
    return this.#runtimeProjects.load(request);
  }

  closeRuntimeProject(request: RuntimeProjectCloseRequest): RuntimeProjectCloseReceipt {
    if (this.#engine === null) {
      throw new RuntimeBridgeError('not_initialized', 'closeRuntimeProject before initializeEngine');
    }
    return this.#runtimeProjects.close(request);
  }

  saveProjectBundle(): ProjectBundleSaveSummary {
    if (this.#loadedProjectBundle === null) {
      throw new RuntimeBridgeError('not_initialized', 'saveProjectBundle with no project bundle loaded');
    }
    return { artifactsWritten: 3, compactedEdits: 0, retainedEdits: 0 };
  }

  getProjectBundleCompositionStatus(): CompositionStatus {
    return { loadedProjectBundle: this.#loadedProjectBundle, fatalCount: 0, totalCount: 0, blocksLoad: false };
  }

  unloadProjectBundle(): void {
    this.#loadedProjectBundle = null;
    this.#runtimeProjects.clearResources();
  }

  loadReplayFixture(fixture: ReplayFixture): ReplaySessionHandle {
    this.#replaySteps = fixture.steps;
    return 0 as ReplaySessionHandle;
  }

  runReplayStep(session: ReplaySessionHandle): ReplayStepReport {
    const step = this.#replaySteps;
    this.#replaySteps = Math.max(0, this.#replaySteps - 1);
    return { step, hash: `mock-${session}-${step}`, diverged: false };
  }
}

function initialFpsEncounterState(): FpsEncounterStateReadout {
  return {
    presetId: 'generated-tunnel-small-encounter',
    status: 'pending',
    spawnedEnemyIds: [],
    defeatedEnemyIds: [],
    revision: 0,
    lastTransition: 'initialized',
  };
}

function fpsEncounterHash(
  state: FpsEncounterStateReadout,
  lifecycle: FpsEncounterLifecycleInput,
): string {
  return `fnv1a64:${fnv1a64(JSON.stringify({ state, lifecycle }))}`;
}

/** Construct the default mock bridge. */
export function createMockRuntimeBridge(): RuntimeBridge {
  return new MockRuntimeBridge();
}
