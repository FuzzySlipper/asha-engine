import type {
  CameraCollisionShape,
  CameraCollisionSnapshot,
  CameraCreateRequest,
  CameraProjectionRequest,
  CameraProjectionSnapshot,
  CameraSnapshot,
  CollisionAxis,
  CollisionConstrainedCameraInputEnvelope,
  CommandBatch,
  CommandResult,
  Face,
  FirstPersonCameraInputEnvelope,
  FlatSceneDocument,
  MaterialProjection,
  ModelMaterialPreviewRequest,
  ModelMaterialPreviewSnapshot,
  PickRay,
  PickResult,
  RenderFrameDiff,
  SceneNodeId,
  SceneNodeRecord,
  SceneObjectCommandRejection,
  SceneObjectCommandRequest,
  SceneObjectCommandResult,
  SceneObjectSnapshot,
  ScreenPointToPickRayRequest,
  VoxelCoord,
  VoxelSelectionSnapshot,
} from '@asha/contracts';
import {
  RuntimeBridgeError,
  nonNegativeSafeInteger,
  u32,
  type CompositionStatus,
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
  type WorldLoadRequest,
  type WorldSaveSummary,
} from './bridge.js';

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

function horizontalMovementBasisFromPose(pose: CameraSnapshot['pose']): Pick<CameraSnapshot['basis'], 'forward' | 'right'> {
  const yaw = f32((pose.yawDegrees * Math.PI) / 180);
  const sy = f32(Math.sin(yaw));
  const cy = f32(Math.cos(yaw));
  return {
    forward: [sy, 0, f32(-cy)],
    right: [cy, 0, sy],
  };
}

function matrixKey(values: readonly number[]): string {
  return values.map((value) => value.toFixed(3)).join(',');
}

function fnv1a64(text: string): string {
  let hash = 0xcbf29ce484222325n;
  const prime = 0x100000001b3n;
  const mask = 0xffffffffffffffffn;
  for (let i = 0; i < text.length; i += 1) {
    hash ^= BigInt(text.charCodeAt(i));
    hash = (hash * prime) & mask;
  }
  return hash.toString(16).padStart(16, '0');
}

function validateVec3(value: Vec3, field: string): void {
  if (value.length !== 3 || value.some((component) => !Number.isFinite(component))) {
    throw new RuntimeBridgeError('invalid_input', `${field} must be a finite vec3`);
  }
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

const STATIC_ROOM_WORLD_HASH = `fnv1a64:${fnv1a64(
  STATIC_ROOM_COLLIDERS.map((collider) => `${collider.id}:${collider.min.join(',')}:${collider.max.join(',')}`).join('|'),
)}`;
const STATIC_ROOM_COLLISION_PROJECTION_HASH = `fnv1a64:${fnv1a64(
  `${STATIC_ROOM_WORLD_HASH}|axis-separable-static-room|${STATIC_ROOM_COLLIDERS.length}`,
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

type Matrix4 = CameraProjectionSnapshot['viewMatrix'];

function multiplyMatrix4(a: Matrix4, b: Matrix4): Matrix4 {
  const out = new Array<number>(16).fill(0);
  for (let col = 0; col < 4; col += 1) {
    for (let row = 0; row < 4; row += 1) {
      let sum = 0;
      for (let k = 0; k < 4; k += 1) {
        sum = f32(sum + f32((a[k * 4 + row] ?? 0) * (b[col * 4 + k] ?? 0)));
      }
      out[col * 4 + row] = sum;
    }
  }
  return out as unknown as Matrix4;
}

function viewMatrixFromSnapshot(snapshot: CameraSnapshot): Matrix4 {
  const { right, up, forward } = snapshot.basis;
  const position = snapshot.pose.position;
  const dotRight = f32(f32(right[0] * position[0]) + f32(right[1] * position[1]) + f32(right[2] * position[2]));
  const dotUp = f32(f32(up[0] * position[0]) + f32(up[1] * position[1]) + f32(up[2] * position[2]));
  const dotForward = f32(
    f32(forward[0] * position[0]) + f32(forward[1] * position[1]) + f32(forward[2] * position[2]),
  );
  return [
    right[0],
    up[0],
    -forward[0],
    0,
    right[1],
    up[1],
    -forward[1],
    0,
    right[2],
    up[2],
    -forward[2],
    0,
    -dotRight,
    -dotUp,
    dotForward,
    1,
  ];
}

function projectionMatrixFromSnapshot(
  snapshot: CameraSnapshot,
  viewport: CameraProjectionSnapshot['viewport'],
): CameraProjectionSnapshot['projectionMatrix'] {
  const aspect = f32(viewport.width / viewport.height);
  const f = f32(1 / Math.tan(f32((snapshot.projection.fovYDegrees * Math.PI) / 360)));
  const near = snapshot.projection.near;
  const far = snapshot.projection.far;
  return [
    f32(f / aspect),
    0,
    0,
    0,
    0,
    f,
    0,
    0,
    0,
    0,
    f32((far + near) / (near - far)),
    -1,
    0,
    0,
    f32((2 * far * near) / (near - far)),
    0,
  ];
}

function materialDescriptor(id: string, material: MaterialProjection): Extract<RenderFrameDiff['ops'][number], { readonly op: 'defineMaterial' }>['material'] {
  return {
    id,
    color: [material.render.color.r, material.render.color.g, material.render.color.b, material.render.color.a],
    texture: material.render.texture?.id ?? null,
    roughness: material.render.roughness,
    emissive: material.render.emissive,
    uvStrategy: material.render.uvStrategy,
  };
}

function projectionSnapshot(snapshot: CameraSnapshot, viewport = snapshot.viewport): CameraProjectionSnapshot {
  const viewMatrix = viewMatrixFromSnapshot(snapshot);
  const projectionMatrix = projectionMatrixFromSnapshot(snapshot, viewport);
  const viewProjectionMatrix = multiplyMatrix4(projectionMatrix, viewMatrix);
  const projectionHash = `fnv1a64:${fnv1a64(matrixKey([
    ...viewMatrix,
    ...projectionMatrix,
    ...viewProjectionMatrix,
  ]))}`;
  return {
    ...snapshot,
    viewport,
    viewMatrix,
    projectionMatrix,
    viewProjectionMatrix,
    projectionHash,
  };
}

function cloneFlatSceneDocument(document: FlatSceneDocument): FlatSceneDocument {
  return JSON.parse(JSON.stringify(document)) as FlatSceneDocument;
}

function initialMockSceneDocument(): FlatSceneDocument {
  return {
    schemaVersion: 1,
    id: 1 as FlatSceneDocument['id'],
    metadata: { name: 'Mock scene', authoringFormatVersion: 1 },
    dependencies: [],
    nodes: [
      {
        id: 1 as SceneNodeId,
        parent: null,
        childOrder: 0,
        label: 'Root',
        tags: [],
        transform: { translation: [0, 0, 0], rotation: [0, 0, 0, 1], scale: [1, 1, 1] },
        kind: { kind: 'emptyGroup' },
      },
      {
        id: 2 as SceneNodeId,
        parent: 1 as SceneNodeId,
        childOrder: 0,
        label: 'Preview cube',
        tags: [],
        transform: { translation: [0, 0, 0], rotation: [0, 0, 0, 1], scale: [1, 1, 1] },
        kind: {
          kind: 'staticMesh',
          asset: { id: 'static-mesh:preview/cube', version: { req: 'any' }, hash: null },
        },
      },
    ],
  };
}

function sceneDocumentHash(document: FlatSceneDocument): number {
  const hex = fnv1a64(JSON.stringify({
    ...document,
    nodes: [...document.nodes].sort((a, b) => a.id - b.id),
  }));
  return Number.parseInt(hex.slice(0, 13), 16);
}

function sceneObjectSnapshotFromDocument(document: FlatSceneDocument): SceneObjectSnapshot {
  return {
    documentHash: sceneDocumentHash(document),
    objects: [...document.nodes]
      .sort((a, b) => a.id - b.id)
      .map((node) => ({
        id: node.id,
        parent: node.parent,
        childOrder: node.childOrder,
        label: node.label,
        kind: node.kind.kind,
        hasRenderableAsset: node.kind.kind !== 'emptyGroup',
      })),
  };
}

function nodeIndex(document: FlatSceneDocument, id: SceneNodeId): number {
  return document.nodes.findIndex((node) => node.id === id);
}

function commandRejection(
  code: SceneObjectCommandRejection['code'],
  id: SceneNodeId | null,
  parent: SceneNodeId | null = null,
  expectedHash: number | null = null,
  actualHash: number | null = null,
): SceneObjectCommandResult {
  return {
    accepted: false,
    outcome: null,
    rejection: { code, id, parent, expectedHash, actualHash, validationErrors: [] },
  };
}

function descendantIds(document: FlatSceneDocument, root: SceneNodeId): Set<SceneNodeId> {
  const doomed = new Set<SceneNodeId>([root]);
  let changed = true;
  while (changed) {
    changed = false;
    for (const node of document.nodes) {
      if (node.parent !== null && doomed.has(node.parent) && !doomed.has(node.id)) {
        doomed.add(node.id);
        changed = true;
      }
    }
  }
  return doomed;
}

function createsCycle(document: FlatSceneDocument, id: SceneNodeId, parent: SceneNodeId | null): boolean {
  let current = parent;
  while (current !== null) {
    if (current === id) return true;
    const parentNode = document.nodes.find((node) => node.id === current);
    current = parentNode?.parent ?? null;
  }
  return false;
}

function applyMockSceneObjectCommand(document: FlatSceneDocument, request: SceneObjectCommandRequest): SceneObjectCommandResult {
  const actualHash = sceneDocumentHash(document);
  if (request.expectedDocumentHash !== actualHash) {
    return commandRejection('stale-scene-object-snapshot', null, null, request.expectedDocumentHash, actualHash);
  }
  let next = cloneFlatSceneDocument(document);
  let selected: SceneNodeId | null = null;

  switch (request.command.kind) {
    case 'create': {
      if (nodeIndex(next, request.command.record.id) !== -1) {
        return commandRejection('duplicate-scene-object', request.command.record.id);
      }
      if (request.command.record.parent !== null && nodeIndex(next, request.command.record.parent) === -1) {
        return commandRejection('missing-scene-object-parent', request.command.record.id, request.command.record.parent);
      }
      next = { ...next, nodes: [...next.nodes, request.command.record] };
      break;
    }
    case 'delete': {
      if (nodeIndex(next, request.command.id) === -1) {
        return commandRejection('missing-scene-object', request.command.id);
      }
      const doomed = descendantIds(next, request.command.id);
      next = { ...next, nodes: next.nodes.filter((node) => !doomed.has(node.id)) };
      break;
    }
    case 'rename': {
      if (request.command.label !== null && request.command.label.trim() === '') {
        return commandRejection('blank-scene-object-label', request.command.id);
      }
      const id = request.command.id;
      const label = request.command.label;
      const index = nodeIndex(next, id);
      if (index === -1) return commandRejection('missing-scene-object', id);
      const node = next.nodes[index] as SceneNodeRecord;
      next = { ...next, nodes: next.nodes.map((existing) => existing.id === id ? { ...node, label } : existing) };
      selected = id;
      break;
    }
    case 'reparent': {
      const id = request.command.id;
      const parent = request.command.parent;
      const childOrder = request.command.childOrder;
      const index = nodeIndex(next, id);
      if (index === -1) return commandRejection('missing-scene-object', id);
      if (parent === id) return commandRejection('scene-object-self-parent', id);
      if (parent !== null && nodeIndex(next, parent) === -1) {
        return commandRejection('missing-scene-object-parent', id, parent);
      }
      if (createsCycle(next, id, parent)) {
        return commandRejection('invalid-scene-after-command', id, parent);
      }
      const node = next.nodes[index] as SceneNodeRecord;
      next = { ...next, nodes: next.nodes.map((existing) => existing.id === id ? { ...node, parent, childOrder } : existing) };
      selected = id;
      break;
    }
    case 'select': {
      if (request.command.id !== null && nodeIndex(next, request.command.id) === -1) {
        return commandRejection('missing-scene-object', request.command.id);
      }
      selected = request.command.id;
      break;
    }
  }

  const snapshot = sceneObjectSnapshotFromDocument(next);
  return {
    accepted: true,
    outcome: { document: next, snapshot, selected },
    rejection: null,
  };
}

export class MockRuntimeBridge implements RuntimeBridge {
  #engine: EngineHandle | null = null;
  #buffer: Uint8Array = new Uint8Array();
  #replaySteps = 0;
  #loadedWorld: number | null = null;
  #sceneDocument = initialMockSceneDocument();
  #nextCamera = 1;
  #cameras = new Map<number, MutableCameraSnapshot>();
  #enemyTransforms = new Map<number, Vec3>();
  #fpsSeed: FpsRuntimeSessionLoadRequest | null = null;
  #fpsSnapshot: FpsRuntimeSessionSnapshot | null = null;
  #fpsEncounter: FpsEncounterStateReadout = initialFpsEncounterState();
  #fpsEpoch = 0;

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
    return handle;
  }

  stepSimulation(input: StepInputEnvelope): StepResult {
    if (this.#engine === null) {
      throw new RuntimeBridgeError('not_initialized', 'step before initializeEngine');
    }
    const tick = nonNegativeSafeInteger(input.tick, 'tick');
    return { tick, diffCount: tick % 4 };
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
    const player = this.#fpsSeed.definitions.find((definition) => definition.role === 'player');
    const enemy = this.#fpsSeed.definitions.find((definition) => definition.role === 'enemy');
    if (player?.weapon === null || player?.weapon === undefined || enemy === undefined) {
      throw new RuntimeBridgeError('invalid_input', 'FPS RuntimeSession is missing player weapon or enemy');
    }
    const before = this.#fpsSnapshot.health.find((health) => health.entity === enemy.entity) ?? null;
    const after = before === null
      ? null
      : { ...before, current: Math.max(0, before.current - player.weapon.damage) };
    const health = this.#fpsSnapshot.health.map((entry) => (entry.entity === enemy.entity && after !== null ? after : entry));
    const lifecycleStatus = after !== null && after.current === 0
      ? { state: 'enemy_defeated' as const, entity: enemy.entity, tick }
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
      shooter: player.entity,
      target: enemy.entity,
      targetHealthBefore: before,
      targetHealthAfter: after,
      lifecycleStatus,
      targetRenderVisible: lifecycleStatus.state === 'enemy_defeated' ? false : true,
      entityHash: this.#fpsSnapshot.entityHash,
      healthHash,
      replayHash,
    };
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
    if (this.#engine === null) {
      throw new RuntimeBridgeError('not_initialized', 'submitCommands before initializeEngine');
    }
    const rejections: Array<CommandResult['rejections'][number]> = [];
    for (const command of batch.commands) {
      const value = command.op === 'setVoxel' || command.op === 'fillRegion' ? command.value : null;
      if (value?.kind === 'solid' && (value.material < 1 || value.material > 16)) {
        rejections.push({ reason: 'unknownMaterial', material: value.material });
      }
    }
    return {
      accepted: batch.commands.length - rejections.length,
      rejected: rejections.length,
      rejections,
    };
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
    for (const [idx, halfExtent] of input.shape.halfExtents.entries()) {
      finite(halfExtent, `shape.halfExtents[${idx}]`);
      if (halfExtent <= 0) {
        throw new RuntimeBridgeError('invalid_input', 'collision shape halfExtents must be positive');
      }
    }
    if (input.policy.mode !== 'axis_separable_slide' || input.policy.maxIterations < 1 || input.policy.maxIterations > 3) {
      throw new RuntimeBridgeError('invalid_input', 'only axis_separable_slide with maxIterations in 1..=3 is supported');
    }
    const lookPose: CameraSnapshot['pose'] = {
      position: before.pose.position,
      yawDegrees: f32(before.pose.yawDegrees + input.input.yawDeltaDegrees),
      pitchDegrees: Math.max(-89, Math.min(89, f32(before.pose.pitchDegrees + input.input.pitchDeltaDegrees))),
    };
    const lookBasis = basisFromPose(lookPose);
    const movementBasis = horizontalMovementBasisFromPose(lookPose);
    const distance = f32(input.input.dtSeconds * input.input.moveSpeedUnitsPerSecond);
    const attemptedPose = {
      position: [
        f32(
          before.pose.position[0] +
            f32(
                f32(movementBasis.forward[0] * input.input.moveForward) +
                f32(movementBasis.right[0] * input.input.moveRight) +
                f32(lookBasis.up[0] * input.input.moveUp),
            ) *
              distance,
        ),
        f32(
          before.pose.position[1] +
            f32(
                f32(movementBasis.forward[1] * input.input.moveForward) +
                f32(movementBasis.right[1] * input.input.moveRight) +
                f32(lookBasis.up[1] * input.input.moveUp),
            ) *
              distance,
        ),
        f32(
          before.pose.position[2] +
            f32(
                f32(movementBasis.forward[2] * input.input.moveForward) +
                f32(movementBasis.right[2] * input.input.moveRight) +
                f32(lookBasis.up[2] * input.input.moveUp),
            ) *
              distance,
        ),
      ] as readonly [number, number, number],
      yawDegrees: lookPose.yawDegrees,
      pitchDegrees: lookPose.pitchDegrees,
    };
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
    return {
      camera: input.camera,
      tick: input.tick,
      before,
      attempted,
      after,
      collision: {
        grid: input.grid,
        shape: input.shape,
        policy: input.policy,
        collided: blockedAxes.length > 0,
        blockedAxes,
        correction,
        queriedAabb,
        worldHash: STATIC_ROOM_WORLD_HASH,
        collisionProjectionHash: STATIC_ROOM_COLLISION_PROJECTION_HASH,
      },
      movementHash: `fnv1a64:${fnv1a64(
        `${input.camera}|${input.tick}|${JSON.stringify(before.pose)}|${JSON.stringify(attempted.pose)}|${JSON.stringify(after.pose)}|${STATIC_ROOM_WORLD_HASH}|${STATIC_ROOM_COLLISION_PROJECTION_HASH}`,
      )}`,
    };
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
      cameraProjectionHash: projectionSnapshot(camera, viewport).projectionHash,
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
      worldHash: 'mock-voxel-world',
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


  readModelMaterialPreview(request: ModelMaterialPreviewRequest): ModelMaterialPreviewSnapshot {
    if (this.#engine === null) {
      throw new RuntimeBridgeError('not_initialized', 'readModelMaterialPreview before initializeEngine');
    }
    const entry = request.catalogEntry;
    if (entry.kind !== 'material' || entry.material === null) {
      throw new RuntimeBridgeError('invalid_input', `catalog entry '${entry.id}' is not a material`);
    }
    if (!request.meshAsset.materialSlots.some((slot) => slot.material === entry.id)) {
      throw new RuntimeBridgeError('invalid_input', `mesh asset '${request.meshAsset.asset}' does not reference material '${entry.id}'`);
    }
    return {
      catalogEntry: entry,
      material: entry.material,
      meshAsset: request.meshAsset,
      previewDiff: {
        ops: [
          { op: 'defineMaterial', material: materialDescriptor(entry.id, entry.material) },
          { op: 'defineStaticMesh', asset: request.meshAsset },
          {
            op: 'createStaticMeshInstance',
            handle: request.instanceHandle,
            parent: null,
            instance: {
              asset: request.meshAsset.asset,
              transform: { translation: [0, 0, 0], rotation: [0, 0, 0, 1], scale: [1, 1, 1] },
              materialOverrides: [],
              metadata: { source: null, tags: [], label: `Preview ${request.meshAsset.asset}` },
            },
          },
        ],
      },
      rendererClassification: 'reference_preview',
      diagnostics: ['native runtime readback for model/material preview may fail closed until wired'],
    };
  }

  readSceneObjectSnapshot(): SceneObjectSnapshot {
    if (this.#engine === null) {
      throw new RuntimeBridgeError('not_initialized', 'readSceneObjectSnapshot before initializeEngine');
    }
    return sceneObjectSnapshotFromDocument(this.#sceneDocument);
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
    return snapshot;
  }

  applyFirstPersonCameraInput(envelope: FirstPersonCameraInputEnvelope): CameraSnapshot {
    if (this.#engine === null) {
      throw new RuntimeBridgeError('not_initialized', 'applyFirstPersonCameraInput before initializeEngine');
    }
    const prior = this.#cameras.get(envelope.camera as number);
    if (!prior) {
      throw new RuntimeBridgeError('unknown_handle', `no camera for handle ${envelope.camera}`);
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
    return projectionSnapshot(snapshot, request.viewport ?? snapshot.viewport);
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

  loadWorldBundle(request: WorldLoadRequest): CompositionStatus {
    const bundleSchemaVersion = u32(request.bundleSchemaVersion, 'bundleSchemaVersion');
    const protocolVersion = u32(request.protocolVersion, 'protocolVersion');
    const sceneId = nonNegativeSafeInteger(request.sceneId, 'sceneId');
    // Fail closed on a newer bundle; the prior loaded world is left untouched
    // (we only set #loadedWorld on success — the staged commit/swap).
    if (bundleSchemaVersion > 1 || protocolVersion > 1) {
      throw new RuntimeBridgeError(
        'invalid_input',
        `unsupported bundle schema ${bundleSchemaVersion} / protocol ${protocolVersion}`,
      );
    }
    this.#loadedWorld = sceneId;
    return { loadedWorld: sceneId, fatalCount: 0, totalCount: 0, blocksLoad: false };
  }

  saveCurrentWorld(): WorldSaveSummary {
    if (this.#loadedWorld === null) {
      throw new RuntimeBridgeError('not_initialized', 'saveCurrentWorld with no world loaded');
    }
    return { artifactsWritten: 3, compactedEdits: 0, retainedEdits: 0 };
  }

  getCompositionStatus(): CompositionStatus {
    return { loadedWorld: this.#loadedWorld, fatalCount: 0, totalCount: 0, blocksLoad: false };
  }

  unloadWorld(): void {
    this.#loadedWorld = null;
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
