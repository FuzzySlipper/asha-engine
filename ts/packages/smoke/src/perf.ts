// Launchable-voxel performance baseline harness (#2460).
//
// A deterministic, logged perf scenario over the canonical launch fixture, on ONE
// stable host, for *trend / regression tracking* — NOT product performance targets.
// It reuses the smoke building blocks (the same facade, ThreeRenderer, EditorStore,
// and fixtures) but adds phase timings + structural counters and emits machine-
// readable JSON(L).
//
// Discipline (per the task):
//   • Timings are LOGGED/TRENDED, never CI-failing thresholds — `ok` reflects only
//     the structural invariants (leaks, preview remesh, bounded render ops, replay
//     divergence, command acceptance), which MAY fail hard.
//   • Absolute timing values are same-machine baselines; do not generalize them to
//     final product performance (see docs/perf-baseline.md).
//   • The harness adds no flaky timing gate to the normal CI gate — it is operator-/
//     CI-artifact-runnable via `dev:asha-perf`, separate from `check-all.sh`.

import type { RenderDiff, VoxelCommand } from '@asha/contracts';
import { renderHandle } from '@asha/contracts';
import { EditorStore } from '@asha/editor-tools';
import { ThreeRenderer } from '@asha/renderer-three/backend';
import {
  frameCursor,
  RuntimeBridgeError,
  type CommandBatch,
  type RuntimeBridge,
  type RuntimeBufferHandle,
} from '@asha/runtime-bridge';
import { OVERLAY_HANDLE_BASE, previewOverlayDiffs } from '@asha/ui-dom';

import { bootForMode, type BridgeBoot } from './harness.js';
import type { SmokeMode } from './result.js';
import {
  FIXTURE_WORLD,
  fixtureEditUpdateFrame,
  fixtureRenderFrame,
  fixtureVoxelCommand,
  fixtureWorldHash,
} from './fixtures.js';

/** The documented perf command (referenced by docs + Den). */
export const PERF_COMMAND = 'pnpm --filter @asha/smoke dev:asha-perf';

/** How many edit→render cycles the aggregate loop runs (overridable for tuning). */
export const DEFAULT_EDIT_CYCLES = 32;

/** A monotonic clock in milliseconds (injected so tests are deterministic). */
export type PerfClock = () => number;

/** Run-identifying metadata — enough to compare runs over time on the same host. */
export interface PerfMetadata {
  /** Output schema version, bumped on a breaking field change. */
  readonly schema: number;
  readonly command: string;
  /** Source revision; `unknown` if not supplied (the harness never shells out). */
  readonly commit: string;
  readonly branch: string;
  /** Stable host label — the anchor for same-host trend comparison. */
  readonly hostLabel: string;
  readonly runtimeMode: BridgeBoot['mode'];
  readonly smokeMode: SmokeMode;
  readonly fixtureId: number;
  readonly fixtureWorldHash: string;
  readonly node: string;
  readonly platform: string;
  readonly arch: string;
  readonly cpus: number;
  readonly cpuModel: string;
  readonly totalMemMb: number;
  /** Wall-clock of the run — NON-deterministic; excluded from trend comparison. */
  readonly timestamp: string;
}

/** A single timed phase. Compare `ms` across runs on the same host. */
export interface PerfTiming {
  readonly phase: string;
  readonly ms: number;
  /** Repetitions folded into `ms` (mean per op = `ms / iterations`). */
  readonly iterations: number;
}

/** Structural counters — these are the *stable*, comparable trend fields. */
export interface PerfCounters {
  readonly peakHandles: number;
  readonly leakedHandles: number;
  readonly sceneNodes: number;
  readonly overlayCells: number;
  readonly fallbackMaterials: number;
  readonly spriteFallbacks: number;
  readonly commandsAccepted: number;
  readonly commandsRejected: number;
  readonly renderOpsApplied: number;
  readonly editCycles: number;
  readonly replaySteps: number;
  readonly replayDiverged: boolean;
  readonly outstandingBuffers: number;
}

/** A structural invariant — these MAY fail the run hard (unlike timings). */
export interface PerfInvariant {
  readonly name: string;
  readonly held: boolean;
  readonly detail: string;
}

/** The full perf run record (one JSONL line). */
export interface PerfResult {
  /** True iff every structural invariant held. Timings never affect this. */
  readonly ok: boolean;
  readonly meta: PerfMetadata;
  readonly timings: readonly PerfTiming[];
  readonly counters: PerfCounters;
  readonly invariants: readonly PerfInvariant[];
}

/** Options for {@link runPerf} (all injectable for deterministic tests). */
export interface PerfOptions {
  readonly mode?: SmokeMode;
  readonly bootBridge?: () => BridgeBoot;
  readonly editCycles?: number;
  /** Override the timing clock (default `performance.now`). */
  readonly clock?: PerfClock;
  /** Metadata the host supplies (commit/branch/hostLabel); the rest is derived. */
  readonly meta?: Partial<
    Pick<PerfMetadata, 'commit' | 'branch' | 'hostLabel' | 'timestamp'>
  >;
}

interface OsBasics {
  readonly platform: string;
  readonly arch: string;
  readonly cpus: number;
  readonly cpuModel: string;
  readonly totalMemMb: number;
}

/** Read-only host basics. Imported lazily so the module stays environment-agnostic. */
async function osBasics(): Promise<OsBasics> {
  const os = await import('node:os');
  const cpus = os.cpus();
  return {
    platform: os.platform(),
    arch: os.arch(),
    cpus: cpus.length,
    cpuModel: cpus[0]?.model.trim() ?? 'unknown',
    totalMemMb: Math.round(os.totalmem() / (1024 * 1024)),
  };
}

/** Mutable run accumulator. */
interface PerfState {
  readonly bridge: RuntimeBridge;
  readonly authority: boolean;
  readonly renderer: ThreeRenderer;
  readonly store: EditorStore;
  readonly clock: PerfClock;
  readonly timings: PerfTiming[];
  readonly liveHandles: Set<number>;
  peakHandles: number;
  renderOpsApplied: number;
  commandsAccepted: number;
  commandsRejected: number;
}

/** Time `fn`, push a `PerfTiming`, and return its value. */
function phase<T>(state: PerfState, name: string, iterations: number, fn: () => T): T {
  const t0 = state.clock();
  const value = fn();
  state.timings.push({ phase: name, ms: state.clock() - t0, iterations });
  return value;
}

/** Apply a render frame, tracking handles + total ops for leak/throughput counters. */
function applyAndTrack(state: PerfState, ops: readonly RenderDiff[]): void {
  state.renderer.applyFrame({ ops: [...ops] });
  state.renderOpsApplied += ops.length;
  for (const op of ops) {
    if (op.op === 'create') state.liveHandles.add(op.handle as number);
    else if (op.op === 'destroy') state.liveHandles.delete(op.handle as number);
  }
  state.peakHandles = Math.max(state.peakHandles, state.renderer.handleCount);
}

function sceneNodeCount(renderer: ThreeRenderer): number {
  return renderer.scene.getObjectByName('scene')?.children.length ?? 0;
}

/** A small region fill within the resident 2×2×2 origin chunk (half-open [min,max)). */
function fillRegionCommand(): VoxelCommand {
  return {
    op: 'fillRegion',
    grid: 1,
    min: { x: 0, y: 0, z: 0 },
    max: { x: 2, y: 2, z: 2 },
    value: { kind: 'solid', material: 1 },
  };
}

/** The inverse of the one-cell edit: clear the origin voxel. */
function clearOriginCommand(): VoxelCommand {
  return { op: 'setVoxel', grid: 1, coord: { x: 0, y: 0, z: 0 }, value: { kind: 'empty' } };
}

/** Submit a batch, accumulating accept/reject counters. */
function submit(state: PerfState, batch: CommandBatch): void {
  const result = state.bridge.submitCommands(batch);
  state.commandsAccepted += result.accepted;
  state.commandsRejected += result.rejected;
}

/**
 * Run the launchable-voxel perf scenario and return a structured record. Reference
 * (mock) mode is the deterministic baseline; authority mode exercises the native
 * path (and fails closed honestly if the addon is unavailable — surfaced as a boot
 * invariant, not a silent skip).
 */
export async function runPerf(options: PerfOptions = {}): Promise<PerfResult> {
  const mode: SmokeMode = options.mode ?? 'reference';
  const editCycles = options.editCycles ?? DEFAULT_EDIT_CYCLES;
  const clock: PerfClock = options.clock ?? defaultClock();
  const boot = (options.bootBridge ?? (() => bootForMode(mode)))();
  const os = await osBasics();

  const meta: PerfMetadata = {
    schema: 1,
    command: PERF_COMMAND,
    commit: options.meta?.commit ?? 'unknown',
    branch: options.meta?.branch ?? 'unknown',
    hostLabel: options.meta?.hostLabel ?? 'unlabeled-host',
    runtimeMode: boot.mode,
    smokeMode: mode,
    fixtureId: FIXTURE_WORLD.sceneId,
    fixtureWorldHash: fixtureWorldHash(FIXTURE_WORLD),
    node: process.version,
    platform: os.platform,
    arch: os.arch,
    cpus: os.cpus,
    cpuModel: os.cpuModel,
    totalMemMb: os.totalMemMb,
    timestamp: options.meta?.timestamp ?? new Date().toISOString(),
  };

  // A boot that failed closed (e.g. authority intent, no native addon) is an honest
  // structural failure — recorded, never downgraded to a fake mock pass.
  if (boot.bridge === null) {
    const error =
      boot.bootError ?? new RuntimeBridgeError('native_unavailable', 'bridge boot failed');
    return {
      ok: false,
      meta,
      timings: [],
      counters: emptyCounters(editCycles),
      invariants: [
        { name: 'bridge-boot', held: false, detail: `boot failed closed: ${error.message}` },
      ],
    };
  }

  const state: PerfState = {
    bridge: boot.bridge,
    authority: boot.intent === 'authority',
    renderer: new ThreeRenderer(),
    store: new EditorStore(),
    clock,
    timings: [],
    liveHandles: new Set(),
    peakHandles: 0,
    renderOpsApplied: 0,
    commandsAccepted: 0,
    commandsRejected: 0,
  };

  // ── Timed phases ──
  phase(state, 'initialize', 1, () => state.bridge.initializeEngine({ seed: 1 }));
  phase(state, 'world-load', 1, () => state.bridge.loadWorldBundle(FIXTURE_WORLD));

  const initialFrame = phase(state, 'render-projection-initial', 1, () =>
    state.authority ? state.bridge.readRenderDiffs(frameCursor(0)) : fixtureRenderFrame(),
  );
  phase(state, 'renderer-apply-initial', 1, () => applyAndTrack(state, initialFrame.ops));

  phase(state, 'edit-one-cell', 1, () => submit(state, { commands: [fixtureVoxelCommand()] }));
  phase(state, 'edit-region', 1, () => submit(state, { commands: [fillRegionCommand()] }));
  phase(state, 'edit-inverse', 1, () => submit(state, { commands: [clearOriginCommand()] }));

  const updateFrame = phase(state, 'render-update', 1, () =>
    state.authority ? state.bridge.readRenderDiffs(frameCursor(1)) : fixtureEditUpdateFrame(),
  );
  const sceneBeforePreview = sceneNodeCount(state.renderer);
  applyAndTrack(state, updateFrame.ops);

  // Preview overlay (non-authoritative): must draw debug-layer overlay only.
  state.store.dispatch({ type: 'setTool', tool: 'place' });
  state.store.dispatch({ type: 'setMaterial', material: 1 });
  state.store.dispatch({
    type: 'setSelection',
    selection: { voxel: { x: 0, y: 0, z: 0 }, face: 'posX' },
  });
  state.store.dispatch({ type: 'setPreviewEnabled', enabled: true });
  const sceneBeforeOverlay = sceneNodeCount(state.renderer);
  const overlay = phase(state, 'preview-overlay', 1, () =>
    previewOverlayDiffs(state.store.getState()),
  );
  applyAndTrack(state, overlay);
  const sceneAfterOverlay = sceneNodeCount(state.renderer);

  phase(state, 'save', 1, () => state.bridge.saveCurrentWorld());
  phase(state, 'reload', 1, () => state.bridge.loadWorldBundle(FIXTURE_WORLD));

  // Save→reload→replay evidence (the quarantined replay harness path).
  const replaySteps = 4;
  const replaySession = state.bridge.loadReplayFixture({ name: 'launch-perf', steps: replaySteps });
  let replayDiverged = false;
  phase(state, 'replay', replaySteps, () => {
    for (let i = 0; i < replaySteps; i++) {
      const report = state.bridge.runReplayStep(replaySession);
      replayDiverged = replayDiverged || report.diverged;
    }
  });

  // ── Repeated edit→render cycles: bounded-throughput trend + leak detection ──
  let perCycleOps = -1;
  let boundedRenderOps = true;
  phase(state, 'edit-render-cycles', editCycles, () => {
    for (let i = 0; i < editCycles; i++) {
      submit(state, { commands: [fixtureVoxelCommand()] });
      const before = state.renderOpsApplied;
      const frame = state.authority ? state.bridge.readRenderDiffs(frameCursor(1)) : fixtureEditUpdateFrame();
      applyAndTrack(state, frame.ops);
      const applied = state.renderOpsApplied - before;
      // Each cycle must apply a bounded, constant number of render ops — a regression
      // to unbounded per-edit diffs (full remesh) would break this.
      if (perCycleOps === -1) perCycleOps = applied;
      else if (applied !== perCycleOps) boundedRenderOps = false;
    }
  });

  // ── Teardown + leak accounting ──
  let outstandingBuffers = 0;
  try {
    const view = state.bridge.getBuffer(0 as RuntimeBufferHandle);
    state.bridge.releaseBuffer(view.handle);
  } catch {
    outstandingBuffers = 0;
  }
  const destroys: RenderDiff[] = [...state.liveHandles].map((h) => ({
    op: 'destroy',
    handle: renderHandle(h),
  }));
  applyAndTrack(state, destroys);
  const leakedHandles = state.renderer.handleCount;

  const counters: PerfCounters = {
    peakHandles: state.peakHandles,
    leakedHandles,
    sceneNodes: sceneBeforeOverlay, // authoritative scene-layer count (excludes overlay)
    overlayCells: overlay.length,
    fallbackMaterials: state.renderer.fallbackMaterialCount,
    spriteFallbacks: state.renderer.spriteFallbackCount,
    commandsAccepted: state.commandsAccepted,
    commandsRejected: state.commandsRejected,
    renderOpsApplied: state.renderOpsApplied,
    editCycles,
    replaySteps,
    replayDiverged,
    outstandingBuffers,
  };

  const submittedCommands = 3 + editCycles; // one-cell + region + inverse + cycles
  const invariants: PerfInvariant[] = [
    {
      name: 'no-handle-leak',
      held: leakedHandles === 0 && outstandingBuffers === 0,
      detail: `leakedHandles=${leakedHandles} outstandingBuffers=${outstandingBuffers}`,
    },
    {
      name: 'no-preview-remesh',
      held:
        sceneAfterOverlay === sceneBeforeOverlay &&
        sceneBeforePreview <= sceneBeforeOverlay &&
        overlay.every((op) => op.op === 'create' && (op.handle as number) >= OVERLAY_HANDLE_BASE),
      detail: `scene ${sceneBeforeOverlay}→${sceneAfterOverlay}; overlayCells=${overlay.length}`,
    },
    {
      name: 'bounded-render-ops-per-cycle',
      held: boundedRenderOps,
      detail: `perCycleOps=${perCycleOps} over ${editCycles} cycles`,
    },
    {
      name: 'commands-accepted',
      held: state.commandsAccepted === submittedCommands && state.commandsRejected === 0,
      detail: `accepted=${state.commandsAccepted}/${submittedCommands} rejected=${state.commandsRejected}`,
    },
    {
      name: 'replay-not-diverged',
      held: !replayDiverged,
      detail: `steps=${replaySteps} diverged=${replayDiverged}`,
    },
  ];

  return {
    ok: invariants.every((i) => i.held),
    meta,
    timings: state.timings,
    counters,
    invariants,
  };
}

function emptyCounters(editCycles: number): PerfCounters {
  return {
    peakHandles: 0,
    leakedHandles: 0,
    sceneNodes: 0,
    overlayCells: 0,
    fallbackMaterials: 0,
    spriteFallbacks: 0,
    commandsAccepted: 0,
    commandsRejected: 0,
    renderOpsApplied: 0,
    editCycles,
    replaySteps: 0,
    replayDiverged: false,
    outstandingBuffers: 0,
  };
}

/** The default millisecond clock (`performance.now`), resolved lazily. */
function defaultClock(): PerfClock {
  // `performance` is a Node global (perf_hooks) in supported runtimes.
  return () => performance.now();
}

/** A human-readable one-screen summary (logged by the CLI alongside the JSON). */
export function formatPerf(result: PerfResult): string {
  const lines: string[] = [];
  const m = result.meta;
  lines.push(`asha-perf ${result.ok ? 'OK' : 'FAILED'} (structural invariants)`);
  lines.push(`fixture ${m.fixtureId} hash ${m.fixtureWorldHash} / ${m.runtimeMode} ${m.smokeMode}`);
  lines.push(`host ${m.hostLabel} ${m.platform}/${m.arch} cpus=${m.cpus} mem=${m.totalMemMb}MB node ${m.node}`);
  lines.push(`commit ${m.commit} branch ${m.branch} at ${m.timestamp}`);
  lines.push('timings (ms):');
  for (const t of result.timings) {
    const mean = t.iterations > 1 ? ` (mean ${(t.ms / t.iterations).toFixed(4)})` : '';
    lines.push(`  ${t.phase.padEnd(26)} ${t.ms.toFixed(4)} ×${t.iterations}${mean}`);
  }
  const c = result.counters;
  lines.push(
    `counters: peakHandles=${c.peakHandles} leaked=${c.leakedHandles} renderOps=${c.renderOpsApplied} ` +
      `accepted=${c.commandsAccepted} rejected=${c.commandsRejected} fallbacks=${c.fallbackMaterials}/${c.spriteFallbacks} ` +
      `overlayCells=${c.overlayCells} replay=${c.replaySteps}(diverged=${c.replayDiverged})`,
  );
  lines.push('invariants:');
  for (const inv of result.invariants) {
    lines.push(`  [${inv.held ? 'OK' : 'XX'}] ${inv.name} — ${inv.detail}`);
  }
  return lines.join('\n') + '\n';
}
