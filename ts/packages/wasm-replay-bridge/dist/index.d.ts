import { type ReplayRecord, type ReplayHash, type StepIndex } from '@asha/contracts';
/** Raised when the WASM replay module is not built (toolchain/target missing). */
export declare class WasmReplayUnavailable extends Error {
    constructor(message: string);
}
/**
 * The narrow surface the compiled `wasm-api` module exposes for replay: given a
 * recorded run, return the post-step state hash for each step (the deterministic
 * fingerprint compared against the golden record). Mirrors design §8.8 ("render
 * diff retrieval / replay hooks") narrowed to replay duties.
 */
export interface WasmReplayModule {
    replayHashes(record: ReplayRecord): readonly number[];
}
/**
 * Attempt to load the compiled WASM replay module. Throws a classified
 * {@link WasmReplayUnavailable} rather than a raw resolution error so callers can
 * fall back to {@link ReferenceReplayRunner} for CI evidence when the wasm32 build
 * is missing. Build: `cargo build --target wasm32-unknown-unknown -p wasm-api`.
 */
export declare function loadWasmReplayModule(modulePath?: string): WasmReplayModule;
/**
 * A deterministic, dependency-free replay runner used as the native reference and
 * as the CI fallback when the WASM module is unavailable. It re-derives each step's
 * post hash from the recorded outcome so a replay fixture can be exercised through
 * *a* replay path even without a wasm32 toolchain.
 *
 * This is NOT the authority: when the real WASM module is present, its hashes are
 * canonical and any disagreement is reported by {@link classifyDivergence}.
 */
export declare class ReferenceReplayRunner implements WasmReplayModule {
    replayHashes(record: ReplayRecord): readonly number[];
}
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
export declare function compareReplay(record: ReplayRecord, nativePath: WasmReplayModule, wasmAuthority: WasmReplayModule): DivergenceReport;
//# sourceMappingURL=index.d.ts.map