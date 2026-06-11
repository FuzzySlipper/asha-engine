import { type ReplayRecord, type ReplayHash, type StepIndex } from '@asha/contracts';
/** Raised when the WASM replay module is not built (toolchain/target missing). */
export declare class WasmReplayUnavailable extends Error {
    constructor(message: string);
}
/**
 * Extracts the per-step post hashes from a recorded run — the deterministic
 * fingerprints compared between native and WASM. Used by {@link classifyDivergence}
 * and as a toolchain-free baseline alongside the real WASM authority below.
 */
export interface ReplayHasher {
    replayHashes(record: ReplayRecord): readonly number[];
}
/**
 * A deterministic, dependency-free hasher used as the native reference and as the
 * CI baseline. It re-derives each step's post hash from the recorded outcome so a
 * replay fixture can be exercised even without a wasm32 toolchain.
 *
 * This is NOT the authority: the compiled `wasm-api` module ({@link
 * loadWasmReplayAuthority}) runs the real `sim-replay` logic under WASM.
 */
export declare class ReferenceReplayRunner implements ReplayHasher {
    replayHashes(record: ReplayRecord): readonly number[];
}
/**
 * The narrow surface the wasm-bindgen `wasm-api` module exposes: the authoritative
 * `sim-replay` divergence classifier, compiled to wasm32. Operates on replay
 * artifacts in `sim-replay`'s text format (the `harness/goldens/replays/*.replay`
 * format), returning a terse `"<class>\t<step>"` pair.
 */
export interface WasmApiModule {
    classify_divergence(expected: string, actual: string): string;
    divergence_class_labels(): string;
}
/** Typed result of the WASM replay authority over two replay artifacts. */
export interface ReplayDivergence {
    /** `sim-replay` DivergenceClass label, or `'match'` when the records reproduce. */
    readonly class: string;
    readonly matched: boolean;
    /** Diverging step index, or null for whole-record / no divergence. */
    readonly step: number | null;
}
/** A loaded WASM replay authority. */
export interface WasmReplayAuthority {
    /** Classify two replay artifacts (text format) via the real Rust logic under WASM. */
    classifyRecords(expected: string, actual: string): ReplayDivergence;
    /** The class labels the module can emit (for label↔kind sync assertions). */
    classLabels(): readonly string[];
}
/**
 * Load the compiled wasm-api replay authority (wasm-bindgen `--target nodejs`
 * output, `.cjs`). Throws a classified {@link WasmReplayUnavailable} when unbuilt.
 * Build with `harness/ci/check-wasm-replay.sh`.
 */
export declare function loadWasmReplayAuthority(modulePath?: string): WasmReplayAuthority;
/** Stable classification of a native-vs-WASM replay comparison. */
export type DivergenceKind = 'match' | 'length_divergence' | 'hash_divergence';
export interface DivergenceReport {
    readonly kind: DivergenceKind;
    /** Zero-based index of the first diverging step, or null for `match`. */
    readonly firstDivergingStep: StepIndex | null;
    /** Hash from the native run at the divergence (or null). */
    readonly nativeHash: ReplayHash | null;
    /** Hash from the WASM run at the divergence (or null). */
    readonly wasmHash: ReplayHash | null;
    readonly detail: string;
}
/**
 * Compare per-step hashes from the native path against the WASM replay authority.
 * Pure and total — the concrete determinism check determinism.md mandates. WASM is
 * authoritative: a `hash_divergence` means the native path must be fixed (or the
 * divergence intentionally classified and tested).
 */
export declare function classifyDivergence(native: readonly number[], wasm: readonly number[]): DivergenceReport;
/**
 * Run a replay record through both paths and classify. When the WASM module is
 * unavailable, pass {@link ReferenceReplayRunner} for both to get a self-consistent
 * baseline (and record the blocker out-of-band).
 */
export declare function compareReplay(record: ReplayRecord, nativePath: ReplayHasher, wasmAuthority: ReplayHasher): DivergenceReport;
//# sourceMappingURL=index.d.ts.map