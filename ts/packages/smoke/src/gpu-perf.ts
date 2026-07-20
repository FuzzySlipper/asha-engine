// Launchable-voxel discrete-GPU/WebGL performance lane (#2461).
//
// This is a manual, opt-in, NON-GATING lane for real GL/Electron/WebGL hosts. It
// records host/GPU/runtime metadata and can carry contextual external WebGL
// calibration results beside ASHA metrics without turning them into acceptance
// criteria. The same-machine perf baseline in perf.ts remains independent.

import type { PerfCounters, PerfInvariant, PerfResult, PerfTiming } from './perf.js';
import { PERF_COMMAND, runPerf } from './perf.js';

/** The documented GPU-lane command (manual/opt-in, non-gating). */
export const GPU_PERF_COMMAND = 'pnpm --filter @asha/smoke dev:asha-gpu-perf';

export type GpuPerfStatus = 'completed' | 'skipped';
export type GpuPerfGating = 'non-gating';
export type GpuPerfRenderContext = 'electron-webgl' | 'browser-webgl' | 'external-gl';
export type GpuPerfSkipReason = 'gpu_context_not_enabled' | 'invalid_external_calibration';

export interface GpuDescriptor {
  readonly name: string;
  readonly driver: string;
  readonly vendor: string;
  readonly device: string;
}

export interface ExternalCalibration {
  readonly name: string;
  readonly score: number | null;
  readonly unit: string;
  readonly source: string;
  readonly notes: string;
  /** Always contextual; never a CI/review gate. */
  readonly gating: GpuPerfGating;
}

export interface GpuPerfMetadata {
  readonly schema: number;
  readonly command: string;
  readonly baseCommand: string;
  readonly lane: 'discrete-gpu-gl-render';
  readonly gating: GpuPerfGating;
  readonly commit: string;
  readonly branch: string;
  readonly hostLabel: string;
  readonly platform: string;
  readonly arch: string;
  readonly node: string;
  readonly fixtureId: number | null;
  readonly fixtureManifestHash: string | null;
  readonly renderContext: GpuPerfRenderContext | 'unavailable';
  readonly gpu: GpuDescriptor;
  readonly browser: string;
  readonly runtime: string;
  /** Wall-clock of the wrapper run; use for ordering only. */
  readonly timestamp: string;
}

export interface GpuPerfSkip {
  readonly reason: GpuPerfSkipReason;
  readonly detail: string;
}

export interface GpuPerfResult {
  /** True unless an ASHA structural invariant fails or calibration JSON is malformed. */
  readonly ok: boolean;
  readonly status: GpuPerfStatus;
  readonly meta: GpuPerfMetadata;
  readonly skip: GpuPerfSkip | null;
  /** ASHA launchable-voxel metrics, present only for completed GPU-lane runs. */
  readonly asha: {
    readonly timings: readonly PerfTiming[];
    readonly counters: PerfCounters;
    readonly invariants: readonly PerfInvariant[];
  } | null;
  /** Optional contextual WebGL/browser/GPU scores. Never gates this result. */
  readonly externalCalibrations: readonly ExternalCalibration[];
}

export interface GpuPerfOptions {
  readonly env?: Record<string, string | undefined>;
  readonly runBasePerf?: () => Promise<PerfResult>;
  readonly timestamp?: string;
}

function read(env: Record<string, string | undefined>, key: string, fallback = 'unknown'): string {
  const value = env[key];
  return value === undefined || value.trim() === '' ? fallback : value;
}

function selectedRenderContext(
  env: Record<string, string | undefined>,
): GpuPerfRenderContext | null {
  const value = env['ASHA_GPU_PERF_CONTEXT'];
  if (value === 'electron-webgl' || value === 'browser-webgl' || value === 'external-gl') {
    return value;
  }
  return null;
}

function descriptor(env: Record<string, string | undefined>): GpuDescriptor {
  return {
    name: read(env, 'ASHA_GPU_NAME'),
    driver: read(env, 'ASHA_GPU_DRIVER'),
    vendor: read(env, 'ASHA_GPU_VENDOR'),
    device: read(env, 'ASHA_GPU_DEVICE'),
  };
}

function metadata(
  env: Record<string, string | undefined>,
  base: PerfResult | null,
  renderContext: GpuPerfRenderContext | null,
  timestamp: string,
): GpuPerfMetadata {
  return {
    schema: 1,
    command: GPU_PERF_COMMAND,
    baseCommand: PERF_COMMAND,
    lane: 'discrete-gpu-gl-render',
    gating: 'non-gating',
    commit: base?.meta.commit ?? read(env, 'ASHA_PERF_COMMIT'),
    branch: base?.meta.branch ?? read(env, 'ASHA_PERF_BRANCH'),
    hostLabel: base?.meta.hostLabel ?? read(env, 'ASHA_PERF_HOST', 'unlabeled-gpu-host'),
    platform: base?.meta.platform ?? process.platform,
    arch: base?.meta.arch ?? process.arch,
    node: base?.meta.node ?? process.version,
    fixtureId: base?.meta.fixtureId ?? null,
    fixtureManifestHash: base?.meta.fixtureManifestHash ?? null,
    renderContext: renderContext ?? 'unavailable',
    gpu: descriptor(env),
    browser: read(env, 'ASHA_GPU_BROWSER'),
    runtime: read(env, 'ASHA_GPU_RUNTIME'),
    timestamp,
  };
}

function parseExternalCalibrations(raw: string | undefined): readonly ExternalCalibration[] | null {
  if (raw === undefined || raw.trim() === '') return [];
  try {
    const parsed: unknown = JSON.parse(raw);
    if (!Array.isArray(parsed)) return null;
    return parsed.map((entry): ExternalCalibration => {
      const record = entry as Record<string, unknown>;
      const rawScore = record['score'];
      const score = typeof rawScore === 'number' && Number.isFinite(rawScore) ? rawScore : null;
      const rawName = record['name'];
      const rawUnit = record['unit'];
      const rawSource = record['source'];
      const rawNotes = record['notes'];
      return {
        name: typeof rawName === 'string' && rawName.trim() !== '' ? rawName : 'unnamed',
        score,
        unit: typeof rawUnit === 'string' && rawUnit.trim() !== '' ? rawUnit : 'score',
        source:
          typeof rawSource === 'string' && rawSource.trim() !== '' ? rawSource : 'manual',
        notes: typeof rawNotes === 'string' ? rawNotes : '',
        gating: 'non-gating',
      };
    });
  } catch {
    return null;
  }
}

/**
 * Run the optional GPU/WebGL perf lane. Without explicit opt-in + render context it
 * returns a classified skip so normal CI/developer machines are never gated on a GPU.
 */
export async function runGpuPerf(options: GpuPerfOptions = {}): Promise<GpuPerfResult> {
  const env = options.env ?? process.env;
  const timestamp = options.timestamp ?? new Date().toISOString();
  const renderContext = selectedRenderContext(env);

  if (env['ASHA_GPU_PERF_ENABLE'] !== '1' || renderContext === null) {
    return {
      ok: true,
      status: 'skipped',
      meta: metadata(env, null, renderContext, timestamp),
      skip: {
        reason: 'gpu_context_not_enabled',
        detail:
          'Set ASHA_GPU_PERF_ENABLE=1 and ASHA_GPU_PERF_CONTEXT=electron-webgl|browser-webgl|external-gl on a repeatable GPU host.',
      },
      asha: null,
      externalCalibrations: [],
    };
  }

  const calibrations = parseExternalCalibrations(env['ASHA_GPU_EXTERNAL_CALIBRATION']);
  if (calibrations === null) {
    return {
      ok: false,
      status: 'skipped',
      meta: metadata(env, null, renderContext, timestamp),
      skip: {
        reason: 'invalid_external_calibration',
        detail: 'ASHA_GPU_EXTERNAL_CALIBRATION must be a JSON array of calibration records.',
      },
      asha: null,
      externalCalibrations: [],
    };
  }

  const base = await (options.runBasePerf ?? (() => runPerf()))();
  return {
    ok: base.ok,
    status: 'completed',
    meta: metadata(env, base, renderContext, timestamp),
    skip: null,
    asha: {
      timings: base.timings,
      counters: base.counters,
      invariants: base.invariants,
    },
    externalCalibrations: calibrations,
  };
}

/** Human-readable one-screen summary for the manual GPU lane. */
export function formatGpuPerf(result: GpuPerfResult): string {
  const lines: string[] = [];
  lines.push(`asha-gpu-perf ${result.status.toUpperCase()} (${result.meta.gating})`);
  lines.push(
    `context ${result.meta.renderContext} gpu=${result.meta.gpu.name} driver=${result.meta.gpu.driver}`,
  );
  lines.push(
    `host ${result.meta.hostLabel} ${result.meta.platform}/${result.meta.arch} node ${result.meta.node}`,
  );
  lines.push(`commit ${result.meta.commit} branch ${result.meta.branch} at ${result.meta.timestamp}`);
  if (result.skip !== null) {
    lines.push(`skip: ${result.skip.reason} — ${result.skip.detail}`);
  }
  if (result.asha !== null) {
    lines.push(
      `asha counters: peakHandles=${result.asha.counters.peakHandles} leaked=${result.asha.counters.leakedHandles} renderOps=${result.asha.counters.renderOpsApplied}`,
    );
    lines.push('asha invariants:');
    for (const inv of result.asha.invariants) {
      lines.push(`  [${inv.held ? 'OK' : 'XX'}] ${inv.name} — ${inv.detail}`);
    }
  }
  if (result.externalCalibrations.length > 0) {
    lines.push('external calibration (context only, non-gating):');
    for (const calibration of result.externalCalibrations) {
      lines.push(
        `  ${calibration.name}: ${calibration.score ?? 'n/a'} ${calibration.unit} (${calibration.source})`,
      );
    }
  } else {
    lines.push('external calibration: omitted (allowed; non-gating)');
  }
  return lines.join('\n') + '\n';
}
