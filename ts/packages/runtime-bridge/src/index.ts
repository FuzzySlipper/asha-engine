// @asha/runtime-bridge — the public, transport-agnostic runtime facade (ADR 0006).
//
// App / UI / renderer / devtools couple ONLY to this package for runtime. They do
// not know whether the implementation is napi-rs (`@asha/native-bridge`), a mock,
// or the WASM replay path. The facade exports generated-compatible contract types
// and explicit buffer-handle APIs — never raw addon exports, WASM memory, or JSON
// escape hatches.
//
// The public facade is hand-written for readability but MUST satisfy the
// manifest-derived conformance test (see conformance.test.ts).

import type {
  CameraCreateRequest,
  CameraProjectionRequest,
  CameraProjectionSnapshot,
  CameraSnapshot,
  CameraCollisionSnapshot,
  CollisionConstrainedCameraInputEnvelope,
  ScreenPointToPickRayRequest,
  VoxelSelectionSnapshot,
  CommandBatch,
  CommandResult,
  FirstPersonCameraInputEnvelope,
  Face,
  PickRay,
  PickResult,
  CatalogEntry,
  MaterialProjection,
  RenderFrameDiff,
  StaticMeshAsset,
  ModelMaterialPreviewRequest,
  ModelMaterialPreviewSnapshot,
  FlatSceneDocument,
  SceneNodeId,
  SceneNodeRecord,
  SceneObjectCommandRejection,
  SceneObjectCommandRequest,
  SceneObjectCommandResult,
  SceneObjectSnapshot,
  VoxelCoord,
} from '@asha/contracts';
import { loadNativeAddon, NativeAddonUnavailable, type NativeAddon } from '@asha/native-bridge';
import { MANIFEST_OPERATIONS } from './generated/operations.js';

export { MANIFEST_OPERATIONS } from './generated/operations.js';
export type { BridgeOperation, BridgeSurface } from './generated/operations.js';

// `submit_commands` / `pick_voxel` carry the generated voxel border (manifest
// `protocol_voxel::{CommandBatch, CommandResult, PickRay, PickResult}`). Re-exported
// so consumers still couple only to this facade package for the runtime surface
// (ADR 0006).
export type {
  CameraCreateRequest,
  CameraProjectionRequest,
  CameraProjectionSnapshot,
  CameraSnapshot,
  CameraCollisionSnapshot,
  CollisionConstrainedCameraInputEnvelope,
  ScreenPointToPickRayRequest,
  PickRaySnapshot,
  VoxelSelectionSnapshot,
  CommandBatch,
  CommandResult,
  FirstPersonCameraInputEnvelope,
  PickRay,
  PickResult,
  CatalogEntry,
  MaterialProjection,
  StaticMeshAsset,
  ModelMaterialPreviewRequest,
  ModelMaterialPreviewSnapshot,
  FlatSceneDocument,
  SceneNodeId,
  SceneNodeRecord,
  SceneObjectCommandRejection,
  SceneObjectCommandRequest,
  SceneObjectCommandResult,
  SceneObjectSnapshot,
} from '@asha/contracts';

// Render-diff decode (moved from the former @asha/wasm-bridge). Transport-neutral
// payload → contract types; backs `readRenderDiffs`. See render-decode.ts.
export {
  decodeRenderDiff,
  decodeRenderFrameDiff,
  RenderDecodeError,
  RenderDiffStream,
  FrameMemory,
} from './render-decode.js';

// ── Opaque handle types ───────────────────────────────────────────────────────
// Branded numbers so a buffer handle can't be passed where an engine handle is
// expected. They carry no transport detail and never expose a StateStore.

export type EngineHandle = number & { readonly __brand: 'EngineHandle' };
export type RuntimeBufferHandle = number & { readonly __brand: 'RuntimeBufferHandle' };
export type FrameCursor = number & { readonly __brand: 'FrameCursor' };
export type ReplaySessionHandle = number & { readonly __brand: 'ReplaySessionHandle' };

export const frameCursor = (frame: number): FrameCursor => frame as FrameCursor;

// ── Error taxonomy ────────────────────────────────────────────────────────────

export type RuntimeBridgeErrorKind =
  | 'not_initialized'
  | 'invalid_input'
  | 'unknown_handle'
  | 'buffer_expired'
  | 'native_unavailable'
  // A stable operation exists on the facade but has no native implementation
  // wired yet. The native bridge throws this instead of silently falling back to
  // mock/reference behaviour — the seam is explicit and fail-closed.
  | 'operation_unimplemented'
  | 'internal';

/** Typed, classified error for every facade operation. No JSON error blobs. */
export class RuntimeBridgeError extends Error {
  constructor(readonly kind: RuntimeBridgeErrorKind, message: string) {
    super(`runtime bridge error [${kind}]: ${message}`);
    this.name = 'RuntimeBridgeError';
  }
}

function nonNegativeSafeInteger(value: number, field: string): number {
  if (!Number.isSafeInteger(value) || value < 0) {
    throw new RuntimeBridgeError('invalid_input', `${field} must be a non-negative safe integer`);
  }
  return value;
}

function u32(value: number, field: string): number {
  nonNegativeSafeInteger(value, field);
  if (value > 0xffffffff) {
    throw new RuntimeBridgeError('invalid_input', `${field} must fit in u32`);
  }
  return value;
}

// ── Prototype operation payloads ──────────────────────────────────────────────
// PROTOTYPE: replaced by generated protocol_runtime / protocol_script contracts
// once the codegen emitter lands. The facade *shape* is the stable part.
//
// The simplified world DTOs below are deliberate subsets of the generated
// protocol contracts (@asha/contracts: WorldBundleManifest / SaveSummary /
// DiagnosticReportSet). `world-dto-conformance.test.ts` is a compile-time guard
// that fails when a shared field's type drifts in the generated contract, keeping
// this prototype debt visible until the DTOs are replaced outright.

export interface EngineConfig {
  readonly seed: number;
}
export interface StepInputEnvelope {
  readonly tick: number;
}
export interface StepResult {
  readonly tick: number;
  readonly diffCount: number;
}
// `CommandBatch` / `CommandResult` are NOT prototype DTOs: they are the generated
// voxel command border (imported from `@asha/contracts`). `submitCommands` carries
// the real `VoxelCommand` union — there is no `{ kind: 'smoke-edit' }` placeholder
// command tunnel; an ad-hoc command shape fails to type-check at the call site.
/** Borrowed, read-only view over bridge-owned bytes (large payloads, e.g. mesh). */
export interface RuntimeBufferView {
  readonly handle: RuntimeBufferHandle;
  readonly bytes: Uint8Array;
}
// Quarantined replay payloads.
export interface ReplayFixture {
  readonly name: string;
  readonly steps: number;
}
export interface ReplayStepReport {
  readonly step: number;
  readonly hash: string;
  readonly diverged: boolean;
}
// World load/save composition payloads (#2363). PROTOTYPE: replaced by generated
// protocol_world_bundle / protocol_diagnostics contracts once the emitter wires them.
export interface WorldLoadRequest {
  readonly bundleSchemaVersion: number;
  readonly protocolVersion: number;
  readonly sceneId: number;
}
export interface CompositionStatus {
  readonly loadedWorld: number | null;
  readonly fatalCount: number;
  readonly totalCount: number;
  readonly blocksLoad: boolean;
}
export interface WorldSaveSummary {
  readonly artifactsWritten: number;
  readonly compactedEdits: number;
  readonly retainedEdits: number;
}
// Compact voxel mesh/remesh evidence (#2646). Prototype DTOs until generated
// protocol_render contracts grow the same shapes.
export interface VoxelMeshEvidenceRequest {
  readonly grid: number;
  readonly chunks: readonly { readonly x: number; readonly y: number; readonly z: number }[];
}
export interface VoxelMeshStatsEvidence {
  readonly vertices: number;
  readonly indices: number;
  readonly quads: number;
  readonly facesEmitted: number;
  readonly facesCulled: number;
}
export interface VoxelMeshBoundsEvidence {
  readonly min: readonly [number, number, number];
  readonly max: readonly [number, number, number];
}
export interface VoxelMeshChunkEvidence {
  readonly coord: { readonly x: number; readonly y: number; readonly z: number };
  readonly resident: boolean;
  readonly visible: boolean;
  readonly contentHash: string | null;
  readonly meshHash: string | null;
  readonly stats: VoxelMeshStatsEvidence | null;
  readonly bounds: VoxelMeshBoundsEvidence | null;
  readonly materialSlots: readonly number[];
}
export interface VoxelMeshEvidenceSnapshot {
  readonly grid: number;
  readonly fixtureId: string;
  readonly worldHash: string;
  readonly meshingStrategy: string;
  readonly chunks: readonly VoxelMeshChunkEvidence[];
  readonly diagnostics: readonly string[];
}

// ── The facade surface ────────────────────────────────────────────────────────
// Bounded verbs only — mirrors bridge-manifest.toml. No generic call(method, json).

export interface RuntimeBridge {
  initializeEngine(config: EngineConfig): EngineHandle;
  stepSimulation(input: StepInputEnvelope): StepResult;
  submitCommands(batch: CommandBatch): CommandResult;
  pickVoxel(ray: PickRay): PickResult;
  applyCollisionConstrainedCameraInput(input: CollisionConstrainedCameraInputEnvelope): CameraCollisionSnapshot;
  selectVoxel(request: ScreenPointToPickRayRequest): VoxelSelectionSnapshot;
  readVoxelMeshEvidence(request: VoxelMeshEvidenceRequest): VoxelMeshEvidenceSnapshot;
  readModelMaterialPreview(request: ModelMaterialPreviewRequest): ModelMaterialPreviewSnapshot;
  readSceneObjectSnapshot(): SceneObjectSnapshot;
  applySceneObjectCommand(request: SceneObjectCommandRequest): SceneObjectCommandResult;
  readRenderDiffs(cursor: FrameCursor): RenderFrameDiff;
  createCamera(request: CameraCreateRequest): CameraSnapshot;
  applyFirstPersonCameraInput(input: FirstPersonCameraInputEnvelope): CameraSnapshot;
  readCameraProjection(request: CameraProjectionRequest): CameraProjectionSnapshot;
  getBuffer(handle: RuntimeBufferHandle): RuntimeBufferView;
  releaseBuffer(handle: RuntimeBufferHandle): void;
  // World load/save composition (operational; not a replay-verification replacement).
  loadWorldBundle(request: WorldLoadRequest): CompositionStatus;
  saveCurrentWorld(): WorldSaveSummary;
  getCompositionStatus(): CompositionStatus;
  unloadWorld(): void;
  // Quarantined: replay/golden harness, not the production renderer path.
  loadReplayFixture(fixture: ReplayFixture): ReplaySessionHandle;
  runReplayStep(session: ReplaySessionHandle): ReplayStepReport;
}

// ── Mock implementation ───────────────────────────────────────────────────────
// Targets the facade so most TS tests need no addon load. Behaviour mirrors the
// Rust `ReferenceBridge` so native/mock parity is meaningful.

type MutableCameraSnapshot = CameraSnapshot;

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
    return handle;
  }

  stepSimulation(input: StepInputEnvelope): StepResult {
    if (this.#engine === null) {
      throw new RuntimeBridgeError('not_initialized', 'step before initializeEngine');
    }
    const tick = nonNegativeSafeInteger(input.tick, 'tick');
    return { tick, diffCount: tick % 4 };
  }

  submitCommands(batch: CommandBatch): CommandResult {
    if (this.#engine === null) {
      throw new RuntimeBridgeError('not_initialized', 'submitCommands before initializeEngine');
    }
    // The mock is a transport stand-in, NOT authority: it does not re-validate the
    // voxel edit (Rust `rule-voxel-edit` owns that, exercised on the native path).
    // It fail-closes on transport preconditions (init) and accepts well-typed
    // commands, returning the classified result shape with no rejections.
    return { accepted: batch.commands.length, rejected: 0, rejections: [] };
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
    for (const [idx, halfExtent] of input.shape.halfExtents.entries()) {
      finite(halfExtent, `shape.halfExtents[${idx}]`);
      if (halfExtent <= 0) {
        throw new RuntimeBridgeError('invalid_input', 'collision shape halfExtents must be positive');
      }
    }
    if (input.policy.mode !== 'axis_separable_slide' || input.policy.maxIterations < 1 || input.policy.maxIterations > 3) {
      throw new RuntimeBridgeError('invalid_input', 'only axis_separable_slide with maxIterations in 1..=3 is supported');
    }
    const distance = input.input.dtSeconds * input.input.moveSpeedUnitsPerSecond;
    const attemptedPose = {
      position: [
        f32(before.pose.position[0] + before.basis.forward[0] * input.input.moveForward * distance + before.basis.right[0] * input.input.moveRight * distance + before.basis.up[0] * input.input.moveUp * distance),
        f32(before.pose.position[1] + before.basis.forward[1] * input.input.moveForward * distance + before.basis.right[1] * input.input.moveRight * distance + before.basis.up[1] * input.input.moveUp * distance),
        f32(before.pose.position[2] + before.basis.forward[2] * input.input.moveForward * distance + before.basis.right[2] * input.input.moveRight * distance + before.basis.up[2] * input.input.moveUp * distance),
      ] as readonly [number, number, number],
      yawDegrees: before.pose.yawDegrees + input.input.yawDeltaDegrees,
      pitchDegrees: Math.max(-89, Math.min(89, before.pose.pitchDegrees + input.input.pitchDeltaDegrees)),
    };
    const attempted: CameraSnapshot = { ...before, tick: input.tick, pose: attemptedPose, basis: basisFromPose(attemptedPose) };
    const after: CameraSnapshot = attempted;
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
        collided: false,
        blockedAxes: [],
        correction: [0, 0, 0],
        queriedAabb: {
          min: [
            after.pose.position[0] - input.shape.halfExtents[0],
            after.pose.position[1] - input.shape.halfExtents[1],
            after.pose.position[2] - input.shape.halfExtents[2],
          ],
          max: [
            after.pose.position[0] + input.shape.halfExtents[0],
            after.pose.position[1] + input.shape.halfExtents[1],
            after.pose.position[2] + input.shape.halfExtents[2],
          ],
        },
        worldHash: 'mock-voxel-world',
        collisionProjectionHash: 'fnv1a64:mock-collision-projection',
      },
      movementHash: `fnv1a64:${fnv1a64(`${input.camera}|${input.tick}|${JSON.stringify(before.pose)}|${JSON.stringify(after.pose)}`)}`,
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

/** Construct the default mock bridge. */
export function createMockRuntimeBridge(): RuntimeBridge {
  return new MockRuntimeBridge();
}

// ── Native implementation factory ─────────────────────────────────────────────
// The ONLY place that touches `@asha/native-bridge`. Wraps the addon's wired
// exports and re-classifies load failures into the bridge error taxonomy.
//
// Fail-closed by construction: `NativeRuntimeBridge` implements `RuntimeBridge`
// directly — it does NOT extend `MockRuntimeBridge`, so an unwired operation can
// never silently inherit mock/reference behaviour. Every stable + quarantined
// operation is either routed to a real `#[napi]` export (and listed in
// NATIVE_WIRED_OPERATIONS) or throws a classified `operation_unimplemented`.
// `native-fail-closed.test.ts` enforces that this stays true for every manifest op.

/**
 * Manifest names of operations whose native (`#[napi]`) implementation is actually
 * wired. Everything else on {@link NativeRuntimeBridge} fail-closes with
 * `operation_unimplemented`. Adding a name here is the explicit signal that a
 * native implementation landed; the native conformance test keeps this set and the
 * routed methods in lockstep with the bridge manifest.
 */
export const NATIVE_WIRED_OPERATIONS: ReadonlySet<string> = new Set<string>([
  'initialize_engine',
  'load_world_bundle',
  'submit_commands',
  'step_simulation',
  'read_render_diffs',
  'save_current_world',
  'get_composition_status',
]);

function nativeUnimplemented(manifestName: string): RuntimeBridgeError {
  return new RuntimeBridgeError(
    'operation_unimplemented',
    `native bridge operation '${manifestName}' is not wired; the native facade is ` +
      `fail-closed (no mock fallback). Wire its #[napi] export and add it to ` +
      `NATIVE_WIRED_OPERATIONS.`,
  );
}

const RUST_ERROR_KIND: Readonly<Record<string, RuntimeBridgeErrorKind>> = {
  NotInitialized: 'not_initialized',
  InvalidInput: 'invalid_input',
  UnknownHandle: 'unknown_handle',
  BufferExpired: 'buffer_expired',
  Internal: 'internal',
};

function classifyNativeAddonError(cause: RuntimeBridgeError | Error | string | object): RuntimeBridgeError {
  if (cause instanceof RuntimeBridgeError) return cause;
  const message = cause instanceof Error ? cause.message : String(cause);
  const match = /^(\w+):\s*(.*)$/u.exec(message);
  if (match?.[1]) {
    const kind = RUST_ERROR_KIND[match[1]];
    if (kind) return new RuntimeBridgeError(kind, match[2] || message);
  }
  return new RuntimeBridgeError('internal', message);
}

function callNative<T>(body: () => T): T {
  try {
    return body();
  } catch (cause) {
    throw classifyNativeAddonError(cause as RuntimeBridgeError | Error | string | object);
  }
}

export class NativeRuntimeBridge implements RuntimeBridge {
  readonly #addon: NativeAddon;
  #seed = 0;
  #initialized = false;

  #engineHandle: EngineHandle | null = null;

  constructor(addon: NativeAddon) {
    this.#addon = addon;
  }

  // ── Wired native operations ───────────────────────────────────────────────
  initializeEngine(config: EngineConfig): EngineHandle {
    if (!Number.isInteger(config.seed) || config.seed < 0) {
      throw new RuntimeBridgeError('invalid_input', `seed must be a non-negative integer`);
    }
    this.#seed = config.seed;
    const handle = this.#addon.initializeEngine(config.seed) as EngineHandle;
    this.#engineHandle = handle;
    this.#initialized = true;
    return handle;
  }

  #requireHandle(operation: string): EngineHandle {
    if (!this.#initialized || this.#engineHandle === null) {
      throw new RuntimeBridgeError('not_initialized', `${operation} before initializeEngine`);
    }
    return this.#engineHandle;
  }

  loadWorldBundle(request: WorldLoadRequest): CompositionStatus {
    const handle = this.#requireHandle('loadWorldBundle');
    const bundleSchemaVersion = u32(request.bundleSchemaVersion, 'bundleSchemaVersion');
    const protocolVersion = u32(request.protocolVersion, 'protocolVersion');
    const sceneId = nonNegativeSafeInteger(request.sceneId, 'sceneId');
    return callNative(() =>
      this.#addon.loadWorldBundle(handle, bundleSchemaVersion, protocolVersion, sceneId) as CompositionStatus,
    );
  }

  submitCommands(batch: CommandBatch): CommandResult {
    const handle = this.#requireHandle('submitCommands');
    return callNative(() => this.#addon.submitCommands(handle, JSON.stringify(batch.commands)) as CommandResult);
  }

  stepSimulation(input: StepInputEnvelope): StepResult {
    const handle = this.#requireHandle('stepSimulation');
    const tick = nonNegativeSafeInteger(input.tick, 'tick');
    const diffCount = callNative(() => this.#addon.stepSimulation(handle, tick));
    return { tick, diffCount };
  }


  readModelMaterialPreview(_request: ModelMaterialPreviewRequest): ModelMaterialPreviewSnapshot {
    throw nativeUnimplemented('read_model_material_preview');
  }

  readSceneObjectSnapshot(): SceneObjectSnapshot {
    throw nativeUnimplemented('read_scene_object_snapshot');
  }

  applySceneObjectCommand(): SceneObjectCommandResult {
    throw nativeUnimplemented('apply_scene_object_command');
  }

  readRenderDiffs(cursor: FrameCursor): RenderFrameDiff {
    const handle = this.#requireHandle('readRenderDiffs');
    const frame = nonNegativeSafeInteger(cursor as number, 'frame cursor') as FrameCursor;
    return callNative(() => this.#addon.readRenderDiffs(handle, frame) as RenderFrameDiff);
  }

  saveCurrentWorld(): WorldSaveSummary {
    const handle = this.#requireHandle('saveCurrentWorld');
    return callNative(() => this.#addon.saveCurrentWorld(handle) as WorldSaveSummary);
  }

  getCompositionStatus(): CompositionStatus {
    const handle = this.#requireHandle('getCompositionStatus');
    return callNative(() => this.#addon.getCompositionStatus(handle) as CompositionStatus);
  }

  // ── Unwired operations: fail-closed, never mock-backed ─────────────────────
  // Replace each body with its real native call (and add the manifest name to
  // NATIVE_WIRED_OPERATIONS) when the codegen emitter wires the `#[napi]` export.
  pickVoxel(): PickResult {
    throw nativeUnimplemented('pick_voxel');
  }

  applyCollisionConstrainedCameraInput(): CameraCollisionSnapshot {
    throw nativeUnimplemented('apply_collision_constrained_camera_input');
  }

  selectVoxel(): VoxelSelectionSnapshot {
    throw nativeUnimplemented('select_voxel');
  }

  readVoxelMeshEvidence(): VoxelMeshEvidenceSnapshot {
    throw nativeUnimplemented('read_voxel_mesh_evidence');
  }

  createCamera(): CameraSnapshot {
    throw nativeUnimplemented('create_camera');
  }

  applyFirstPersonCameraInput(): CameraSnapshot {
    throw nativeUnimplemented('apply_first_person_camera_input');
  }

  readCameraProjection(): CameraProjectionSnapshot {
    throw nativeUnimplemented('read_camera_projection');
  }

  getBuffer(): RuntimeBufferView {
    throw nativeUnimplemented('get_buffer');
  }

  releaseBuffer(): void {
    throw nativeUnimplemented('release_buffer');
  }

  unloadWorld(): void {
    throw nativeUnimplemented('unload_world');
  }

  loadReplayFixture(): ReplaySessionHandle {
    throw nativeUnimplemented('load_replay_fixture');
  }

  runReplayStep(): ReplayStepReport {
    throw nativeUnimplemented('run_replay_step');
  }
}

/**
 * Construct the native (napi-rs) bridge. Throws a classified
 * {@link RuntimeBridgeError} of kind `native_unavailable` if the addon is not built
 * — callers can fall back to the mock for tests/dev.
 */
export function createNativeRuntimeBridge(modulePath?: string): RuntimeBridge {
  try {
    const addon = modulePath ? loadNativeAddon(modulePath) : loadNativeAddon();
    return new NativeRuntimeBridge(addon);
  } catch (cause) {
    if (cause instanceof NativeAddonUnavailable) {
      throw new RuntimeBridgeError('native_unavailable', cause.message);
    }
    throw cause;
  }
}

/** Operation count for quick sanity in consumers/tests. */
export const STABLE_OPERATION_COUNT = MANIFEST_OPERATIONS.filter(
  (o) => o.surface === 'stable',
).length;
