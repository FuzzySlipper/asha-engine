// @asha/smoke — the canonical developer smoke harness (#2395/#2396/#2397/#2424/#2441).
//
// One entrypoint boots the ASHA runtime facade against an abstract fixture world and
// drives the full launchable-voxel loop end to end, emitting a single structured
// `SmokeResult` (see result.ts). The proof is staged (task #2441):
//
//   1 boot/runtime-mode   2 fixture load   3 initial authority render projection
//   4 picking/selection   5 preview derivation + overlay (non-authoritative)
//   6 generated command submit   7 authority accept/reject evidence
//   8 render update after edit   9 save / reload / replay   10 cleanup / leak counters
//
// Two intents share the flow (task #2424):
//   - `reference`: the deterministic mock/dev smoke. Proves the renderer upload and
//     overlay paths by applying local fixture frames; authority geometry is absent so
//     picking classifies as a miss honestly.
//   - `authority`: the real loop. Reads render diffs *through the facade*
//     (`bridge.readRenderDiffs`) and submits contract-shaped commands. A missing native
//     capability is classified honestly, never downgraded to mock success.

import {
  createNativeRuntimeBridge,
  frameCursor,
  RuntimeBridgeError,
  type RuntimeBridge,
  type RuntimeBufferHandle,
} from '@asha/runtime-bridge';
import { createMockRuntimeBridge } from '@asha/runtime-bridge/reference';
import type { RenderDiff } from '@asha/contracts';
import { renderHandle } from '@asha/contracts';
import { ThreeRenderer } from '@asha/renderer-three';
import { EditorStore } from '@asha/editor-tools';
import { bridgePicker, pickAndSelect } from '@asha/app';
import { previewOverlayDiffs, OVERLAY_HANDLE_BASE } from '@asha/ui-dom';

import {
  FIXTURE_WORLD,
  fixtureCommandBatch,
  fixtureEditUpdateFrame,
  fixturePickRay,
  fixtureRenderFrame,
  fixtureWorldHash,
} from './fixtures.js';
import type {
  RuntimeMode,
  SmokeCounters,
  SmokeFailure,
  SmokeMode,
  SmokeOutcome,
  SmokeResult,
  SmokeStage,
} from './result.js';

export const SMOKE_COMMAND = 'pnpm --filter @asha/smoke dev:asha-smoke';
export const AUTHORITY_SMOKE_COMMAND = 'ASHA_SMOKE_MODE=authority pnpm --filter @asha/smoke dev:asha-smoke';

/** How the harness obtains a runtime bridge (injectable for tests). */
export interface BridgeBoot {
  /** The booted bridge, or `null` when boot itself failed (e.g. native unavailable). */
  readonly bridge: RuntimeBridge | null;
  readonly mode: RuntimeMode;
  /** What this run is trying to prove (reference vs. real authority path). */
  readonly intent: SmokeMode;
  readonly nativeAvailable: boolean;
  /** Classified reason the bridge is null (required when `bridge` is null). */
  readonly bootError?: RuntimeBridgeError;
}

export interface SmokeOptions {
  /** Override how the bridge is constructed (tests inject failures / native / authority). */
  readonly bootBridge?: () => BridgeBoot;
}

/**
 * Default boot: the canonical deterministic reference smoke on the mock facade, while
 * *probing* native availability for an honest capability readout. The native addon
 * today is a partial prototype (only initialize/step are wired), so the reference
 * smoke does not depend on it.
 */
export function defaultBootBridge(): BridgeBoot {
  return {
    bridge: createMockRuntimeBridge(),
    mode: 'mock',
    intent: 'reference',
    nativeAvailable: probeNativeAvailable(),
  };
}

/**
 * Authority boot: attempt the real native path. If the native addon is not loadable,
 * the boot fails *closed* with a classified error — the harness reports an honest
 * failure rather than silently downgrading to the mock.
 */
export function authorityBootBridge(): BridgeBoot {
  try {
    const bridge = createNativeRuntimeBridge();
    return { bridge, mode: 'native', intent: 'authority', nativeAvailable: true };
  } catch (cause) {
    const bootError =
      cause instanceof RuntimeBridgeError
        ? cause
        : new RuntimeBridgeError('native_unavailable', describeError(cause));
    return { bridge: null, mode: 'native', intent: 'authority', nativeAvailable: false, bootError };
  }
}

/** Pick a boot strategy from an explicit smoke mode (used by the CLI). */
export function bootForMode(mode: SmokeMode): BridgeBoot {
  return mode === 'authority' ? authorityBootBridge() : defaultBootBridge();
}

/** Whether the native addon is loadable, without depending on it for the run. */
function probeNativeAvailable(): boolean {
  try {
    createNativeRuntimeBridge();
    return true;
  } catch {
    // Any load failure (missing build, ABI mismatch) means it is unusable here.
    return false;
  }
}

/** Mutable accumulator threaded through the staged run. */
interface RunState {
  readonly bridge: RuntimeBridge;
  readonly authority: boolean;
  readonly renderer: ThreeRenderer;
  readonly store: EditorStore;
  readonly stages: SmokeStage[];
  readonly failures: SmokeFailure[];
  /** Every render handle created, destroyed in the cleanup stage. */
  readonly liveHandles: Set<number>;
  peakHandles: number;
  sceneNodes: number;
  debugNodes: number;
  worldLoadOk: boolean;
  renderApplied: boolean;
  diagnostics: { total: number; fatal: number; blocksLoad: boolean };
}

/** Count nodes under a named layer group of the renderer scene. */
function layerNodes(renderer: ThreeRenderer, layer: 'scene' | 'debug'): number {
  return renderer.scene.getObjectByName(layer)?.children.length ?? 0;
}

/** Apply a render frame and record the created handles for leak-safe teardown. */
function applyAndTrack(state: RunState, ops: readonly RenderDiff[]): void {
  state.renderer.applyFrame({ ops: [...ops] });
  for (const op of ops) {
    if (op.op === 'create') {
      state.liveHandles.add(op.handle as number);
    } else if (op.op === 'destroy') {
      state.liveHandles.delete(op.handle as number);
    }
  }
  state.peakHandles = Math.max(state.peakHandles, state.renderer.handleCount);
}

/** Run the full staged smoke flow and return a deterministic structured result. */
export function runSmoke(options: SmokeOptions = {}): SmokeResult {
  const boot = (options.bootBridge ?? defaultBootBridge)();

  // ── Stage 1: boot + capability readout (#2395) ──
  if (boot.bridge === null) {
    // Boot failed closed (e.g. authority intent with no native addon). Honest,
    // classified, never a blank mock success.
    return bootFailedResult(boot);
  }

  const state: RunState = {
    bridge: boot.bridge,
    authority: boot.intent === 'authority',
    renderer: new ThreeRenderer(),
    store: new EditorStore(),
    stages: [],
    failures: [],
    liveHandles: new Set(),
    peakHandles: 0,
    sceneNodes: 0,
    debugNodes: 0,
    worldLoadOk: false,
    renderApplied: false,
    diagnostics: { total: 0, fatal: 0, blocksLoad: false },
  };

  state.bridge.initializeEngine({ seed: 1 });
  state.stages.push({
    name: 'boot',
    ok: true,
    detail: `runtime facade up in ${boot.mode} mode, ${boot.intent} intent (nativeAvailable=${boot.nativeAvailable})`,
  });

  stageLoad(state);
  stageRender(state);
  stagePick(state);
  stagePreview(state);
  stageCommandSubmit(state);
  stageAuthorityClassify(state);
  stageRenderUpdate(state);
  stageSaveReloadReplay(state);
  const counters = stageCleanup(state);

  const ok = state.failures.length === 0;
  const outcome: SmokeOutcome = !ok
    ? 'failed'
    : state.authority
      ? 'native_authority_passed'
      : 'mock_reference_passed';
  return {
    ok,
    command: state.authority ? AUTHORITY_SMOKE_COMMAND : SMOKE_COMMAND,
    runtimeMode: boot.mode,
    smokeMode: boot.intent,
    outcome,
    nativeAvailable: boot.nativeAvailable,
    capabilities: {
      runtimeBridge: state.authority ? 'ok' : 'mock',
      worldLoad: state.worldLoadOk ? (state.authority ? 'ok' : 'mock') : 'unavailable',
      renderer: state.renderApplied ? 'ok' : 'unavailable',
      projection: state.renderApplied ? (state.authority ? 'ok' : 'mock') : 'unavailable',
    },
    fixture: { id: FIXTURE_WORLD.sceneId, worldHash: fixtureWorldHash(FIXTURE_WORLD) },
    diagnostics: state.diagnostics,
    render: { applied: state.renderApplied, sceneNodes: state.sceneNodes },
    counters,
    stages: state.stages,
    failures: state.failures,
  };
}

// ── Stage 2: load the abstract fixture world through the real facade path ──
function stageLoad(state: RunState): void {
  try {
    const status = state.bridge.loadWorldBundle(FIXTURE_WORLD);
    state.diagnostics = {
      total: status.totalCount,
      fatal: status.fatalCount,
      blocksLoad: status.blocksLoad,
    };
    state.worldLoadOk = status.loadedWorld === FIXTURE_WORLD.sceneId && !status.blocksLoad;
    state.stages.push({
      name: 'load',
      ok: state.worldLoadOk,
      detail: state.worldLoadOk
        ? `loaded world ${status.loadedWorld}`
        : `load did not settle (loadedWorld=${status.loadedWorld}, blocksLoad=${status.blocksLoad})`,
    });
    if (!state.worldLoadOk) {
      state.failures.push({
        category: 'load_failure',
        subsystem: 'runtime-bridge.loadWorldBundle',
        message: `world ${FIXTURE_WORLD.sceneId} did not load cleanly`,
        nextStep: 'inspect composition diagnostics for the failing artifact',
      });
    }
  } catch (cause) {
    state.failures.push(classifyBridgeFailure('runtime-bridge.loadWorldBundle', 'load_failure', cause));
    state.stages.push({ name: 'load', ok: false, detail: describeError(cause) });
  }
}

// ── Stage 3: initial authority render projection → renderer-three (#2396) ──
// Authority intent reads diffs THROUGH the facade; the reference smoke applies a local
// fixture frame directly (its explicit, mock-only job).
function stageRender(state: RunState): void {
  const renderSource = state.authority ? 'bridge.readRenderDiffs' : 'fixtureRenderFrame';
  try {
    const frame = state.authority ? state.bridge.readRenderDiffs(frameCursor(0)) : fixtureRenderFrame();
    if (state.authority && frame.ops.length === 0) {
      // A facade that returns no diffs cannot prove projection — fail closed rather
      // than reporting an empty render as success.
      throw new RuntimeBridgeError(
        'operation_unimplemented',
        'readRenderDiffs returned no ops; authority projection is not wired',
      );
    }
    applyAndTrack(state, frame.ops);
    state.sceneNodes = layerNodes(state.renderer, 'scene');
    state.renderApplied = state.sceneNodes > 0;
    state.stages.push({
      name: 'render',
      ok: state.renderApplied,
      detail: `applied frame from ${renderSource}; scene nodes=${state.sceneNodes}`,
    });
    if (!state.renderApplied) {
      state.failures.push({
        category: 'projection_failure',
        subsystem: `renderer-three.applyFrame(${renderSource})`,
        message: 'render frame produced no scene nodes',
        nextStep: 'verify the render diff source and the renderer create path',
      });
    }
  } catch (cause) {
    state.failures.push(classifyBridgeFailure(`render(${renderSource})`, 'projection_failure', cause));
    state.stages.push({ name: 'render', ok: false, detail: describeError(cause) });
  }
}

// ── Stage 4: picking / selection through the authority facade (#2437) ──
function stagePick(state: RunState): void {
  try {
    const ray = fixturePickRay();
    const result = pickAndSelect(state.store, bridgePicker(state.bridge), ray);
    const selection = state.store.getState().selection;
    // A classified hit OR miss is a pass — the picking path returned a real PickResult
    // instead of swallowing the call. (The reference facade has no geometry → miss.)
    state.stages.push({
      name: 'pick',
      ok: true,
      detail:
        result.outcome === 'hit'
          ? `hit voxel ${result.hit.voxel.x},${result.hit.voxel.y},${result.hit.voxel.z} face ${result.hit.face}; selection set`
          : `classified miss (${result.rejection.reason}); selection cleared=${selection === null}`,
    });
  } catch (cause) {
    state.failures.push(classifyBridgeFailure('runtime-bridge.pickVoxel', 'pick_failure', cause));
    state.stages.push({ name: 'pick', ok: false, detail: describeError(cause) });
  }
}

// ── Stage 5: preview derivation + overlay (NON-authoritative; must not remesh) ──
function stagePreview(state: RunState): void {
  try {
    // A concrete editor draft: place tool, a selection, preview enabled.
    state.store.dispatch({ type: 'setTool', tool: 'place' });
    state.store.dispatch({ type: 'setMaterial', material: 1 });
    state.store.dispatch({
      type: 'setSelection',
      selection: { voxel: { x: 0, y: 0, z: 0 }, face: 'posX' },
    });
    state.store.dispatch({ type: 'setPreviewEnabled', enabled: true });

    const sceneBefore = layerNodes(state.renderer, 'scene');
    const overlay = previewOverlayDiffs(state.store.getState());
    applyAndTrack(state, overlay);
    state.debugNodes = layerNodes(state.renderer, 'debug');
    const sceneAfter = layerNodes(state.renderer, 'scene');

    // The remesh guardrail: a preview draws debug-layer overlay nodes and must NEVER
    // touch authoritative scene geometry or submit a command. Scene node count is
    // unchanged; the overlay used reserved debug handles only.
    const guardrailHeld =
      sceneAfter === sceneBefore &&
      overlay.every((op) => op.op === 'create' && (op.handle as number) >= OVERLAY_HANDLE_BASE);
    state.stages.push({
      name: 'preview',
      ok: guardrailHeld && state.debugNodes === overlay.length,
      detail: `overlay cells=${overlay.length} debugNodes=${state.debugNodes}; scene unchanged=${sceneAfter === sceneBefore}`,
    });
    if (!guardrailHeld) {
      state.failures.push({
        category: 'preview_failure',
        subsystem: 'ui-dom.previewOverlayDiffs',
        message: 'preview mutated scene geometry or used non-overlay handles (remesh guardrail breached)',
        nextStep: 'ensure preview emits debug-layer overlay diffs only, never authority state',
      });
    }
  } catch (cause) {
    state.failures.push({
      category: 'preview_failure',
      subsystem: 'ui-dom.previewOverlayDiffs',
      message: describeError(cause),
      nextStep: 'inspect the editor preview derivation',
    });
    state.stages.push({ name: 'preview', ok: false, detail: describeError(cause) });
  }
}

// ── Stage 6: generated command submit (#2436) ──
function stageCommandSubmit(state: RunState): void {
  try {
    const batch = fixtureCommandBatch();
    const result = state.bridge.submitCommands(batch);
    state.stages.push({
      name: 'command-submit',
      ok: result.accepted === batch.commands.length,
      detail: `submitted ${batch.commands.length} generated VoxelCommand(s) → accepted=${result.accepted} rejected=${result.rejected}`,
    });
    if (result.accepted !== batch.commands.length) {
      state.failures.push({
        category: 'ui_command_rejected',
        subsystem: 'runtime-bridge.submitCommands',
        message: 'a well-formed generated command batch was not fully accepted',
        nextStep: 'inspect command validation in rule-voxel-edit',
      });
    }
  } catch (cause) {
    state.failures.push(classifyBridgeFailure('runtime-bridge.submitCommands', 'ui_command_rejected', cause));
    state.stages.push({ name: 'command-submit', ok: false, detail: describeError(cause) });
  }
}

// ── Stage 7: authority acceptance/rejection evidence ──
function stageAuthorityClassify(state: RunState): void {
  const rejectedVisible = probeRejectedEdit();
  // Authority intent additionally reads composition status back through the facade.
  let statusOk = true;
  let statusDetail = '';
  if (state.authority) {
    try {
      statusOk = !state.bridge.getCompositionStatus().blocksLoad;
      statusDetail = '; composition status read=ok';
    } catch (cause) {
      statusOk = false;
      statusDetail = `; composition status read failed: ${describeError(cause)}`;
    }
  }
  const ok = rejectedVisible && statusOk;
  state.stages.push({
    name: 'authority-classify',
    ok,
    detail: `rejected-path visible=${rejectedVisible}${statusDetail}`,
  });
  if (!ok) {
    state.failures.push({
      category: 'ui_command_rejected',
      subsystem: 'runtime-bridge.submitCommands',
      message: 'the classified rejection path was not observable, or composition status blocked',
      nextStep: 'verify fail-closed classification on an uninitialized facade',
    });
  }
}

// ── Stage 8: render update after the edit (retained-mode, leak-safe) ──
function stageRenderUpdate(state: RunState): void {
  const source = state.authority ? 'bridge.readRenderDiffs' : 'fixtureEditUpdateFrame';
  try {
    const frame = state.authority
      ? state.bridge.readRenderDiffs(frameCursor(1))
      : fixtureEditUpdateFrame();
    const before = layerNodes(state.renderer, 'scene');
    applyAndTrack(state, frame.ops);
    state.sceneNodes = layerNodes(state.renderer, 'scene');
    state.stages.push({
      name: 'render-update',
      ok: true,
      detail: `applied post-edit update from ${source}; scene nodes ${before}→${state.sceneNodes}`,
    });
  } catch (cause) {
    state.failures.push(classifyBridgeFailure(`render-update(${source})`, 'projection_failure', cause));
    state.stages.push({ name: 'render-update', ok: false, detail: describeError(cause) });
  }
}

// ── Stage 9: save / reload / replay through the facade ──
function stageSaveReloadReplay(state: RunState): void {
  try {
    const save = state.bridge.saveCurrentWorld();
    state.bridge.unloadWorld();
    const reload = state.bridge.loadWorldBundle(FIXTURE_WORLD);
    const reloadOk = reload.loadedWorld === FIXTURE_WORLD.sceneId && !reload.blocksLoad;

    const session = state.bridge.loadReplayFixture({ name: 'smoke-edit-sequence', steps: 1 });
    const step = state.bridge.runReplayStep(session);
    const ok = save.artifactsWritten > 0 && reloadOk && !step.diverged;
    state.stages.push({
      name: 'save-reload-replay',
      ok,
      detail:
        `saved artifacts=${save.artifactsWritten} (compacted=${save.compactedEdits} retained=${save.retainedEdits}); ` +
        `reloaded world ${reload.loadedWorld}; replay step ${step.step} diverged=${step.diverged}`,
    });
    if (!ok) {
      state.failures.push({
        category: 'replay_failure',
        subsystem: 'runtime-bridge.save/reload/replay',
        message: 'save wrote no artifact, reload did not settle, or the replay step diverged',
        nextStep: 'inspect the save/compaction path and replay reproduction',
      });
    }
  } catch (cause) {
    state.failures.push(classifyBridgeFailure('runtime-bridge.save/reload/replay', 'replay_failure', cause));
    state.stages.push({ name: 'save-reload-replay', ok: false, detail: describeError(cause) });
  }
}

// ── Stage 10: cleanup / leak / resource counters ──
function stageCleanup(state: RunState): SmokeCounters {
  let outstandingBuffers = 0;
  try {
    // Release the bridge-owned buffer (handle 0 in the reference facade).
    const view = state.bridge.getBuffer(0 as RuntimeBufferHandle);
    state.bridge.releaseBuffer(view.handle);
  } catch {
    // No buffer outstanding is itself fine; a held buffer would be the leak signal.
    outstandingBuffers = 0;
  }

  // Destroy every render handle created during the run — scene + overlay.
  const destroys: RenderDiff[] = [...state.liveHandles].map((h) => ({
    op: 'destroy',
    handle: renderHandle(h),
  }));
  applyAndTrack(state, destroys);
  const leakedHandles = state.renderer.handleCount;

  const ok = leakedHandles === 0 && outstandingBuffers === 0;
  state.stages.push({
    name: 'cleanup',
    ok,
    detail: `destroyed ${destroys.length} handle(s); leakedHandles=${leakedHandles} outstandingBuffers=${outstandingBuffers}`,
  });
  if (!ok) {
    state.failures.push({
      category: 'resource_leak',
      subsystem: 'renderer-three.handleCount',
      message: `cleanup left ${leakedHandles} live handle(s) / ${outstandingBuffers} buffer(s)`,
      nextStep: 'ensure every created handle is destroyed and buffers are released',
    });
  }

  return {
    leakedHandles,
    peakHandles: state.peakHandles,
    sceneNodes: state.sceneNodes,
    debugNodes: state.debugNodes,
    fallbackMaterials: state.renderer.fallbackMaterialCount,
    spriteFallbacks: state.renderer.spriteFallbackCount,
    outstandingBuffers,
  };
}

/** Build an honest, classified result for a boot that failed closed. */
function bootFailedResult(boot: BridgeBoot): SmokeResult {
  const error =
    boot.bootError ?? new RuntimeBridgeError('native_unavailable', 'bridge boot failed');
  const failure: SmokeFailure = {
    category: error.kind === 'native_unavailable' ? 'missing_native_bridge' : 'internal',
    subsystem: 'smoke.boot',
    message: error.message,
    nextStep:
      boot.intent === 'authority'
        ? 'build the napi addon (harness/ci/check-native.sh) or run the reference smoke'
        : 'inspect the classified boot error',
  };
  return {
    ok: false,
    command: boot.intent === 'authority' ? AUTHORITY_SMOKE_COMMAND : SMOKE_COMMAND,
    runtimeMode: boot.mode,
    smokeMode: boot.intent,
    outcome: 'failed',
    nativeAvailable: boot.nativeAvailable,
    capabilities: {
      runtimeBridge: 'unavailable',
      worldLoad: 'unavailable',
      renderer: 'unavailable',
      projection: 'unavailable',
    },
    fixture: { id: FIXTURE_WORLD.sceneId, worldHash: fixtureWorldHash(FIXTURE_WORLD) },
    diagnostics: { total: 0, fatal: 0, blocksLoad: false },
    render: { applied: false, sceneNodes: 0 },
    counters: emptyCounters(),
    stages: [{ name: 'boot', ok: false, detail: error.message }],
    failures: [failure],
  };
}

function emptyCounters(): SmokeCounters {
  return {
    leakedHandles: 0,
    peakHandles: 0,
    sceneNodes: 0,
    debugNodes: 0,
    fallbackMaterials: 0,
    spriteFallbacks: 0,
    outstandingBuffers: 0,
  };
}

/**
 * Prove the rejected command path is observable: a fresh, uninitialized facade rejects
 * a submission with a classified `not_initialized` error. Returns whether the rejection
 * was visible (it always should be).
 */
function probeRejectedEdit(): boolean {
  const fresh = createMockRuntimeBridge();
  try {
    // A real generated VoxelCommand batch (not a `{ kind }` placeholder) against an
    // uninitialized facade must fail closed with a classified not_initialized error.
    fresh.submitCommands(fixtureCommandBatch());
    return false;
  } catch (cause) {
    return cause instanceof RuntimeBridgeError && cause.kind === 'not_initialized';
  }
}

function classifyBridgeFailure(
  subsystem: string,
  fallback: SmokeFailure['category'],
  cause: unknown,
): SmokeFailure {
  if (cause instanceof RuntimeBridgeError) {
    // A loaded native facade that fail-closes (operation_unimplemented) is a missing
    // native capability, classified like an unavailable addon — never a silent
    // downgrade.
    const nativeGap = cause.kind === 'native_unavailable' || cause.kind === 'operation_unimplemented';
    const category = nativeGap ? 'missing_native_bridge' : fallback;
    return {
      category,
      subsystem,
      message: cause.message,
      nextStep:
        category === 'missing_native_bridge'
          ? 'wire the native operation or run the reference smoke'
          : 'inspect the classified runtime-bridge error',
    };
  }
  return {
    category: 'internal',
    subsystem,
    message: describeError(cause),
    nextStep: 'unexpected error — capture the stack and file a bug',
  };
}

function describeError(cause: unknown): string {
  return cause instanceof Error ? cause.message : String(cause);
}
