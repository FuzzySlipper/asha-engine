import type { PerfCounters, PerfInvariant, PerfResult, PerfTiming } from './perf.js';
/** The documented GPU-lane command (manual/opt-in, non-gating). */
export declare const GPU_PERF_COMMAND = "pnpm --filter @asha/smoke dev:asha-gpu-perf";
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
    readonly fixtureProjectBundleHash: string | null;
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
/**
 * Run the optional GPU/WebGL perf lane. Without explicit opt-in + render context it
 * returns a classified skip so normal CI/developer machines are never gated on a GPU.
 */
export declare function runGpuPerf(options?: GpuPerfOptions): Promise<GpuPerfResult>;
/** Human-readable one-screen summary for the manual GPU lane. */
export declare function formatGpuPerf(result: GpuPerfResult): string;
//# sourceMappingURL=gpu-perf.d.ts.map