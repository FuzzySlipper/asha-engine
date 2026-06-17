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
  CommandBatch,
  CommandResult,
  FirstPersonCameraInputEnvelope,
  PickRay,
  PickResult,
  RenderFrameDiff,
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
  CommandBatch,
  CommandResult,
  FirstPersonCameraInputEnvelope,
  PickRay,
  PickResult,
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

// ── The facade surface ────────────────────────────────────────────────────────
// Bounded verbs only — mirrors bridge-manifest.toml. No generic call(method, json).

export interface RuntimeBridge {
  initializeEngine(config: EngineConfig): EngineHandle;
  stepSimulation(input: StepInputEnvelope): StepResult;
  submitCommands(batch: CommandBatch): CommandResult;
  pickVoxel(ray: PickRay): PickResult;
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

function basisFromPose(pose: CameraSnapshot['pose']): CameraSnapshot['basis'] {
  const yaw = (pose.yawDegrees * Math.PI) / 180;
  const pitch = (pose.pitchDegrees * Math.PI) / 180;
  const cp = Math.cos(pitch);
  const sp = Math.sin(pitch);
  const sy = Math.sin(yaw);
  const cy = Math.cos(yaw);
  return {
    forward: [sy * cp, sp, -cy * cp],
    right: [cy, 0, sy],
    up: [-sy * sp, cp, cy * sp],
  };
}

function projectionSnapshot(snapshot: CameraSnapshot, viewport = snapshot.viewport): CameraProjectionSnapshot {
  return {
    ...snapshot,
    viewport,
    // Placeholder deterministic matrices for #2564 facade coverage. #2565 owns
    // the Rust/reference golden math and will replace this with strict evidence.
    viewMatrix: [1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, -snapshot.pose.position[0], -snapshot.pose.position[1], -snapshot.pose.position[2], 1],
    projectionMatrix: [1, 0, 0, 0, 0, 1, 0, 0, 0, 0, -1, -1, 0, 0, -snapshot.projection.near * 2, 0],
    viewProjectionMatrix: [1, 0, 0, 0, 0, 1, 0, 0, 0, -snapshot.pose.position[1], -1, -1, -snapshot.pose.position[0], 0, -snapshot.projection.near * 2, 0],
    projectionHash: `sha256:mock-camera-${snapshot.camera as number}-${snapshot.tick}`,
  };
}

export class MockRuntimeBridge implements RuntimeBridge {
  #engine: EngineHandle | null = null;
  #buffer: Uint8Array = new Uint8Array();
  #replaySteps = 0;
  #loadedWorld: number | null = null;
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
    return { tick: input.tick, diffCount: input.tick % 4 };
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
    const distance = i.dtSeconds * i.moveSpeedUnitsPerSecond;
    const position = [
      prior.pose.position[0] + (basis.forward[0] * i.moveForward + basis.right[0] * i.moveRight + basis.up[0] * i.moveUp) * distance,
      prior.pose.position[1] + (basis.forward[1] * i.moveForward + basis.right[1] * i.moveRight + basis.up[1] * i.moveUp) * distance,
      prior.pose.position[2] + (basis.forward[2] * i.moveForward + basis.right[2] * i.moveRight + basis.up[2] * i.moveUp) * distance,
    ] as const;
    const pitchDegrees = Math.max(-89, Math.min(89, prior.pose.pitchDegrees + i.pitchDeltaDegrees));
    const pose = {
      position,
      yawDegrees: prior.pose.yawDegrees + i.yawDeltaDegrees,
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
    // Fail closed on a newer bundle; the prior loaded world is left untouched
    // (we only set #loadedWorld on success — the staged commit/swap).
    if (request.bundleSchemaVersion > 1 || request.protocolVersion > 1) {
      throw new RuntimeBridgeError(
        'invalid_input',
        `unsupported bundle schema ${request.bundleSchemaVersion} / protocol ${request.protocolVersion}`,
      );
    }
    this.#loadedWorld = request.sceneId;
    return { loadedWorld: request.sceneId, fatalCount: 0, totalCount: 0, blocksLoad: false };
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
  'step_simulation',
]);

function nativeUnimplemented(manifestName: string): RuntimeBridgeError {
  return new RuntimeBridgeError(
    'operation_unimplemented',
    `native bridge operation '${manifestName}' is not wired; the native facade is ` +
      `fail-closed (no mock fallback). Wire its #[napi] export and add it to ` +
      `NATIVE_WIRED_OPERATIONS.`,
  );
}

export class NativeRuntimeBridge implements RuntimeBridge {
  readonly #addon: NativeAddon;
  #seed = 0;
  #initialized = false;

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
    this.#initialized = true;
    return handle;
  }

  stepSimulation(input: StepInputEnvelope): StepResult {
    if (!this.#initialized) {
      throw new RuntimeBridgeError('not_initialized', 'step before initializeEngine');
    }
    const diffCount = this.#addon.stepSimulation(this.#seed, input.tick);
    return { tick: input.tick, diffCount };
  }

  // ── Unwired operations: fail-closed, never mock-backed ─────────────────────
  // Replace each body with its real native call (and add the manifest name to
  // NATIVE_WIRED_OPERATIONS) when the codegen emitter wires the `#[napi]` export.
  submitCommands(): CommandResult {
    throw nativeUnimplemented('submit_commands');
  }

  pickVoxel(): PickResult {
    throw nativeUnimplemented('pick_voxel');
  }

  readRenderDiffs(): RenderFrameDiff {
    throw nativeUnimplemented('read_render_diffs');
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

  loadWorldBundle(): CompositionStatus {
    throw nativeUnimplemented('load_world_bundle');
  }

  saveCurrentWorld(): WorldSaveSummary {
    throw nativeUnimplemented('save_current_world');
  }

  getCompositionStatus(): CompositionStatus {
    throw nativeUnimplemented('get_composition_status');
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
