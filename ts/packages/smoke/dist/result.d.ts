/** Which subsystem a smoke run failed in (stable codes for agent routing). */
export type SmokeFailureCategory = 'missing_native_bridge' | 'missing_wasm_target' | 'contract_drift' | 'missing_fixture' | 'load_failure' | 'projection_failure' | 'render_init_failure' | 'ui_command_rejected' | 'internal';
/** Status of one capability the harness probes. */
export type CapabilityStatus = 'ok' | 'mock' | 'unavailable';
/** Which transport backs the runtime facade for this run. */
export type RuntimeMode = 'native' | 'mock';
/** Outcome of a single named stage of the smoke run. */
export interface SmokeStage {
    readonly name: string;
    readonly ok: boolean;
    /** Stable, human/agent-legible evidence line for the stage. */
    readonly detail: string;
}
/** One failure, with the subsystem and an actionable next step. */
export interface SmokeFailure {
    readonly category: SmokeFailureCategory;
    readonly subsystem: string;
    readonly message: string;
    readonly nextStep: string;
}
/** The full, deterministic result of a smoke run. */
export interface SmokeResult {
    readonly ok: boolean;
    readonly command: string;
    readonly runtimeMode: RuntimeMode;
    /** Whether the native addon was loadable (vs. the mock fallback). */
    readonly nativeAvailable: boolean;
    /** Per-capability probe results. */
    readonly capabilities: {
        readonly runtimeBridge: CapabilityStatus;
        readonly worldLoad: CapabilityStatus;
        readonly renderer: CapabilityStatus;
        readonly projection: CapabilityStatus;
    };
    /** The abstract fixture world that was loaded (id + deterministic content hash). */
    readonly fixture: {
        readonly id: number;
        readonly worldHash: string;
    };
    /** Diagnostics summary from the load/composition path. */
    readonly diagnostics: {
        readonly total: number;
        readonly fatal: number;
        readonly blocksLoad: boolean;
    };
    /** Render/projection evidence (node count after applying the fixture frame). */
    readonly render: {
        readonly applied: boolean;
        readonly sceneNodes: number;
    };
    readonly stages: readonly SmokeStage[];
    readonly failures: readonly SmokeFailure[];
}
/** Render a result as a stable, multi-line text report (for the CLI + artifacts). */
export declare function formatResult(result: SmokeResult): string;
//# sourceMappingURL=result.d.ts.map