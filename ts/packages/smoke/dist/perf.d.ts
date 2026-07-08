import { type BridgeBoot } from './harness.js';
import type { SmokeMode } from './result.js';
/** The documented perf command (referenced by docs + Den). */
export declare const PERF_COMMAND = "pnpm --filter @asha/smoke dev:asha-perf";
/** How many edit→render cycles the aggregate loop runs (overridable for tuning). */
export declare const DEFAULT_EDIT_CYCLES = 32;
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
    readonly fixtureProjectBundleHash: string;
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
    readonly meta?: Partial<Pick<PerfMetadata, 'commit' | 'branch' | 'hostLabel' | 'timestamp'>>;
}
/**
 * Run the launchable-voxel perf scenario and return a structured record. Reference
 * (mock) mode is the deterministic baseline; authority mode exercises the native
 * path (and fails closed honestly if the addon is unavailable — surfaced as a boot
 * invariant, not a silent skip).
 */
export declare function runPerf(options?: PerfOptions): Promise<PerfResult>;
/** A human-readable one-screen summary (logged by the CLI alongside the JSON). */
export declare function formatPerf(result: PerfResult): string;
//# sourceMappingURL=perf.d.ts.map