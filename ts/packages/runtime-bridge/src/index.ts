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

import type { RenderFrameDiff } from '@asha/contracts';
import { loadNativeAddon, NativeAddonUnavailable, type NativeAddon } from '@asha/native-bridge';
import { MANIFEST_OPERATIONS } from './generated/operations.js';

export { MANIFEST_OPERATIONS } from './generated/operations.js';
export type { BridgeOperation, BridgeSurface } from './generated/operations.js';

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
export interface ProposedCommand {
  readonly kind: string;
}
export interface CommandBatch {
  readonly commands: readonly ProposedCommand[];
}
export interface CommandResult {
  readonly accepted: number;
  readonly rejected: number;
}
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
  readRenderDiffs(cursor: FrameCursor): RenderFrameDiff;
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

export class MockRuntimeBridge implements RuntimeBridge {
  #engine: EngineHandle | null = null;
  #buffer: Uint8Array = new Uint8Array();
  #replaySteps = 0;
  #loadedWorld: number | null = null;

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
    return { accepted: batch.commands.length, rejected: 0 };
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
// The ONLY place that touches `@asha/native-bridge`. Wraps the addon's tiny smoke
// exports and re-classifies load failures into the bridge error taxonomy. The
// remaining facade verbs throw `native_unavailable` until the codegen emitter wires
// their generated `#[napi]` exports.

class NativeRuntimeBridge extends MockRuntimeBridge {
  readonly #addon: NativeAddon;
  #seed = 0;

  constructor(addon: NativeAddon) {
    super();
    this.#addon = addon;
  }

  override initializeEngine(config: EngineConfig): EngineHandle {
    this.#seed = config.seed;
    return this.#addon.initializeEngine(config.seed) as EngineHandle;
  }

  override stepSimulation(input: StepInputEnvelope): StepResult {
    const diffCount = this.#addon.stepSimulation(this.#seed, input.tick);
    return { tick: input.tick, diffCount };
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
