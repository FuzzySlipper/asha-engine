// Launchable-voxel discrete-GPU/WebGL performance lane (#2461).
//
// This is a manual, opt-in, NON-GATING lane for real GL/Electron/WebGL hosts. It
// records host/GPU/runtime metadata and can carry contextual external WebGL
// calibration results beside ASHA metrics without turning them into acceptance
// criteria. The same-machine perf baseline in perf.ts remains independent.
import { PERF_COMMAND, runPerf } from './perf.js';
/** The documented GPU-lane command (manual/opt-in, non-gating). */
export const GPU_PERF_COMMAND = 'pnpm --filter @asha/smoke dev:asha-gpu-perf';
function read(env, key, fallback = 'unknown') {
    const value = env[key];
    return value === undefined || value.trim() === '' ? fallback : value;
}
function selectedRenderContext(env) {
    const value = env['ASHA_GPU_PERF_CONTEXT'];
    if (value === 'electron-webgl' || value === 'browser-webgl' || value === 'external-gl') {
        return value;
    }
    return null;
}
function descriptor(env) {
    return {
        name: read(env, 'ASHA_GPU_NAME'),
        driver: read(env, 'ASHA_GPU_DRIVER'),
        vendor: read(env, 'ASHA_GPU_VENDOR'),
        device: read(env, 'ASHA_GPU_DEVICE'),
    };
}
function metadata(env, base, renderContext, timestamp) {
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
        fixtureProjectBundleHash: base?.meta.fixtureProjectBundleHash ?? null,
        renderContext: renderContext ?? 'unavailable',
        gpu: descriptor(env),
        browser: read(env, 'ASHA_GPU_BROWSER'),
        runtime: read(env, 'ASHA_GPU_RUNTIME'),
        timestamp,
    };
}
function parseExternalCalibrations(raw) {
    if (raw === undefined || raw.trim() === '')
        return [];
    try {
        const parsed = JSON.parse(raw);
        if (!Array.isArray(parsed))
            return null;
        return parsed.map((entry) => {
            const record = entry;
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
                source: typeof rawSource === 'string' && rawSource.trim() !== '' ? rawSource : 'manual',
                notes: typeof rawNotes === 'string' ? rawNotes : '',
                gating: 'non-gating',
            };
        });
    }
    catch {
        return null;
    }
}
/**
 * Run the optional GPU/WebGL perf lane. Without explicit opt-in + render context it
 * returns a classified skip so normal CI/developer machines are never gated on a GPU.
 */
export async function runGpuPerf(options = {}) {
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
                detail: 'Set ASHA_GPU_PERF_ENABLE=1 and ASHA_GPU_PERF_CONTEXT=electron-webgl|browser-webgl|external-gl on a repeatable GPU host.',
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
export function formatGpuPerf(result) {
    const lines = [];
    lines.push(`asha-gpu-perf ${result.status.toUpperCase()} (${result.meta.gating})`);
    lines.push(`context ${result.meta.renderContext} gpu=${result.meta.gpu.name} driver=${result.meta.gpu.driver}`);
    lines.push(`host ${result.meta.hostLabel} ${result.meta.platform}/${result.meta.arch} node ${result.meta.node}`);
    lines.push(`commit ${result.meta.commit} branch ${result.meta.branch} at ${result.meta.timestamp}`);
    if (result.skip !== null) {
        lines.push(`skip: ${result.skip.reason} — ${result.skip.detail}`);
    }
    if (result.asha !== null) {
        lines.push(`asha counters: peakHandles=${result.asha.counters.peakHandles} leaked=${result.asha.counters.leakedHandles} renderOps=${result.asha.counters.renderOpsApplied}`);
        lines.push('asha invariants:');
        for (const inv of result.asha.invariants) {
            lines.push(`  [${inv.held ? 'OK' : 'XX'}] ${inv.name} — ${inv.detail}`);
        }
    }
    if (result.externalCalibrations.length > 0) {
        lines.push('external calibration (context only, non-gating):');
        for (const calibration of result.externalCalibrations) {
            lines.push(`  ${calibration.name}: ${calibration.score ?? 'n/a'} ${calibration.unit} (${calibration.source})`);
        }
    }
    else {
        lines.push('external calibration: omitted (allowed; non-gating)');
    }
    return lines.join('\n') + '\n';
}
//# sourceMappingURL=gpu-perf.js.map