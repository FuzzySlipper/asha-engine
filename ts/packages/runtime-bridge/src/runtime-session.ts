import type {
  CameraCreateRequest,
  CameraProjectionRequest,
  CameraProjectionSnapshot,
  CameraSnapshot,
  CommandBatch,
  CommandResult,
  FirstPersonCameraInputEnvelope,
  RenderFrameDiff,
} from '@asha/contracts';
import {
  RuntimeBridgeError,
  frameCursor,
  type CompositionStatus,
  type EngineHandle,
  type FrameCursor,
  type RuntimeBridge,
  type StepResult,
  type WorldLoadRequest,
} from './bridge.js';
import { createMockRuntimeBridge } from './mock.js';

export type RuntimeSessionMode = 'reference';

export interface RuntimeSessionProjectIdentity {
  readonly gameId: string;
  readonly workspaceId: string;
}

export interface RuntimeSessionInitializeInput {
  readonly sessionId: string;
  readonly seed: number;
  readonly project: RuntimeSessionProjectIdentity;
  readonly projectBundle: WorldLoadRequest;
}

export interface RuntimeSessionIdentity {
  readonly sessionId: string;
  readonly mode: RuntimeSessionMode;
  readonly seed: number;
  readonly project: RuntimeSessionProjectIdentity;
  readonly projectBundle: WorldLoadRequest;
  readonly nonClaims: readonly RuntimeSessionNonClaim[];
}

export type RuntimeSessionNonClaim =
  | 'not_native_runtime'
  | 'not_raw_state_store'
  | 'not_arbitrary_json_bridge'
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
    | 'restart';
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

export interface RuntimeSessionRestartResult {
  readonly sequenceId: number;
  readonly tick: number;
  readonly composition: CompositionStatus;
  readonly restartCount: number;
  readonly sessionHash: string;
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

export interface RuntimeSessionCameraProjectionReadout {
  readonly sequenceId: number;
  readonly request: CameraProjectionRequest;
  readonly snapshot: CameraProjectionSnapshot;
  readonly projectionHash: string;
}

export interface RuntimeSessionFacade {
  initialize(input: RuntimeSessionInitializeInput): RuntimeSessionStateSummary;
  submitCommands(batch: CommandBatch): RuntimeSessionCommandReceipt;
  tick(input?: RuntimeSessionTickInput): RuntimeSessionTickResult;
  createCamera(request: CameraCreateRequest): RuntimeSessionCameraCreateReceipt;
  applyFirstPersonCameraInput(envelope: FirstPersonCameraInputEnvelope): RuntimeSessionCameraInputReceipt;
  readCameraProjection(request: CameraProjectionRequest): RuntimeSessionCameraProjectionReadout;
  readProjection(): RuntimeSessionProjectionSummary;
  readTelemetry(): RuntimeSessionTelemetrySummary;
  restart(): RuntimeSessionRestartResult;
}

export interface RuntimeSessionFacadeOptions {
  readonly bridge?: RuntimeBridge;
}

type RuntimeSessionHashPrimitive = string | number | boolean | null;
type RuntimeSessionHashValue =
  | RuntimeSessionHashPrimitive
  | readonly RuntimeSessionHashValue[]
  | RuntimeSessionHashRecord;
interface RuntimeSessionHashRecord {
  readonly [key: string]: RuntimeSessionHashValue | undefined;
}

export function createMockRuntimeSession(options: RuntimeSessionFacadeOptions = {}): RuntimeSessionFacade {
  return new ReferenceRuntimeSessionFacade(options.bridge ?? createMockRuntimeBridge());
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
  #replayRecords: RuntimeSessionReplayRecord[] = [];

  constructor(bridge: RuntimeBridge) {
    this.#bridge = bridge;
  }

  initialize(input: RuntimeSessionInitializeInput): RuntimeSessionStateSummary {
    validateInitializeInput(input);
    const engine = this.#bridge.initializeEngine({ seed: input.seed });
    const composition = this.#bridge.loadWorldBundle(input.projectBundle);
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
    this.#replayRecords = [];
    this.#record('initialize');
    return this.#stateSummary(composition);
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
    const identity = this.#requireInitialized('restart');
    this.#bridge.unloadWorld();
    this.#bridge.initializeEngine({ seed: identity.seed });
    const composition = this.#bridge.loadWorldBundle(identity.projectBundle);
    this.#sequenceId += 1;
    this.#tick = 0;
    this.#acceptedCommandCount = 0;
    this.#rejectedCommandCount = 0;
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

  #record(kind: RuntimeSessionReplayRecord['kind']): void {
    this.#replayRecords.push({
      sequenceId: this.#sequenceId,
      kind,
      recordHash: stableHash({
        kind,
        sequenceId: this.#sequenceId,
        tick: this.#tick,
        acceptedCommandCount: this.#acceptedCommandCount,
        rejectedCommandCount: this.#rejectedCommandCount,
        restartCount: this.#restartCount,
        composition: compositionHashRecord(this.#bridge.getCompositionStatus()),
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
      composition: this.#identity === null ? null : compositionHashRecord(this.#bridge.getCompositionStatus()),
    });
  }
}

function validateInitializeInput(input: RuntimeSessionInitializeInput): void {
  if (input.sessionId.trim().length === 0) {
    throw new RuntimeBridgeError('invalid_input', 'sessionId must be non-empty');
  }
  if (input.project.gameId.trim().length === 0) {
    throw new RuntimeBridgeError('invalid_input', 'project.gameId must be non-empty');
  }
  if (input.project.workspaceId.trim().length === 0) {
    throw new RuntimeBridgeError('invalid_input', 'project.workspaceId must be non-empty');
  }
  if (!Number.isSafeInteger(input.seed) || input.seed < 0) {
    throw new RuntimeBridgeError('invalid_input', 'seed must be a non-negative safe integer');
  }
}

function referenceRuntimeSessionNonClaims(): readonly RuntimeSessionNonClaim[] {
  return [
    'not_native_runtime',
    'not_raw_state_store',
    'not_arbitrary_json_bridge',
    'not_gameplay_loop',
    'not_renderer',
  ];
}

function identityHashRecord(identity: RuntimeSessionIdentity): RuntimeSessionHashRecord {
  return {
    sessionId: identity.sessionId,
    mode: identity.mode,
    seed: identity.seed,
    project: {
      gameId: identity.project.gameId,
      workspaceId: identity.project.workspaceId,
    },
    projectBundle: projectBundleHashRecord(identity.projectBundle),
    nonClaims: identity.nonClaims,
  };
}

function projectBundleHashRecord(projectBundle: WorldLoadRequest): RuntimeSessionHashRecord {
  return {
    bundleSchemaVersion: projectBundle.bundleSchemaVersion,
    protocolVersion: projectBundle.protocolVersion,
    sceneId: projectBundle.sceneId,
  };
}

function compositionHashRecord(composition: CompositionStatus): RuntimeSessionHashRecord {
  return {
    loadedWorld: composition.loadedWorld,
    fatalCount: composition.fatalCount,
    totalCount: composition.totalCount,
    blocksLoad: composition.blocksLoad,
  };
}

function renderFrameHashRecord(frame: RenderFrameDiff): RuntimeSessionHashRecord {
  return {
    opCount: frame.ops.length,
    opKinds: frame.ops.map((op) => op.op),
  };
}

function stableHash(value: RuntimeSessionHashValue | undefined): string {
  return `fnv1a64:${fnv1a64(stableStringify(value))}`;
}

function stableStringify(value: RuntimeSessionHashValue | undefined): string {
  if (value === undefined) {
    return 'undefined';
  }
  if (value === null || typeof value !== 'object') {
    return JSON.stringify(value);
  }
  if (Array.isArray(value)) {
    return `[${value.map((entry) => stableStringify(entry)).join(',')}]`;
  }
  const record = value as RuntimeSessionHashRecord;
  return `{${Object.keys(record)
    .sort()
    .map((key) => `${JSON.stringify(key)}:${stableStringify(record[key])}`)
    .join(',')}}`;
}

function fnv1a64(text: string): string {
  let hash = 0xcbf29ce484222325n;
  const prime = 0x100000001b3n;
  const mask = 0xffffffffffffffffn;
  for (let index = 0; index < text.length; index += 1) {
    hash ^= BigInt(text.charCodeAt(index));
    hash = (hash * prime) & mask;
  }
  return hash.toString(16).padStart(16, '0');
}
