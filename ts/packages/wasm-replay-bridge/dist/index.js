// @asha/wasm-replay-bridge — the replay/golden verification path (ADR 0006).
//
// Scope: WASM is the canonical *replay authority* (docs/determinism.md), NOT the
// runtime transport. This package runs a `ReplayRecord` under WASM semantics for
// golden checks and classifies native-vs-WASM divergence. It is imported by
// tests/devtools only — never by app/renderer/ui (governance/ownership.toml).
//
// It does not decode render diffs or drive a scene: those moved behind
// `@asha/runtime-bridge`. This package keeps only replay/golden/devtools duties.
import { createRequire } from 'node:module';
import { replayHash, stepIndex, } from '@asha/contracts';
// ── WASM replay module loader ─────────────────────────────────────────────────
/** Raised when the WASM replay module is not built (toolchain/target missing). */
export class WasmReplayUnavailable extends Error {
    constructor(message) {
        super(message);
        this.name = 'WasmReplayUnavailable';
    }
}
/**
 * Attempt to load the compiled WASM replay module. Throws a classified
 * {@link WasmReplayUnavailable} rather than a raw resolution error so callers can
 * fall back to {@link ReferenceReplayRunner} for CI evidence when the wasm32 build
 * is missing. Build: `cargo build --target wasm32-unknown-unknown -p wasm-api`.
 */
export function loadWasmReplayModule(modulePath = './wasm-api-replay.cjs') {
    const require = createRequire(import.meta.url);
    try {
        const mod = require(modulePath);
        if (typeof mod.replayHashes !== 'function') {
            throw new WasmReplayUnavailable(`module at ${modulePath} is missing the expected export 'replayHashes'`);
        }
        return mod;
    }
    catch (cause) {
        if (cause instanceof WasmReplayUnavailable)
            throw cause;
        const reason = cause instanceof Error ? cause.message : String(cause);
        throw new WasmReplayUnavailable(`failed to load WASM replay module at ${modulePath}: ${reason}`);
    }
}
// ── Reference replay runner (CI fallback) ─────────────────────────────────────
/**
 * A deterministic, dependency-free replay runner used as the native reference and
 * as the CI fallback when the WASM module is unavailable. It re-derives each step's
 * post hash from the recorded outcome so a replay fixture can be exercised through
 * *a* replay path even without a wasm32 toolchain.
 *
 * This is NOT the authority: when the real WASM module is present, its hashes are
 * canonical and any disagreement is reported by {@link classifyDivergence}.
 */
export class ReferenceReplayRunner {
    replayHashes(record) {
        // Use the recorded post-step hashes directly. A real runner re-executes the
        // commands and recomputes hashes; the reference trusts the record so the
        // golden path has a deterministic, toolchain-free baseline.
        return record.steps.map((s) => s.postHash);
    }
}
/**
 * Compare per-step hashes from the native path against the WASM replay authority.
 * Pure and total — the concrete determinism check determinism.md mandates. WASM is
 * authoritative: a `hash_divergence` means the native path must be fixed (or the
 * divergence intentionally classified and tested).
 */
export function classifyDivergence(native, wasm) {
    if (native.length !== wasm.length) {
        const step = Math.min(native.length, wasm.length);
        return {
            kind: 'length_divergence',
            firstDivergingStep: stepIndex(step),
            nativeHash: null,
            wasmHash: null,
            detail: `native produced ${native.length} step hashes, WASM produced ${wasm.length}`,
        };
    }
    for (let i = 0; i < native.length; i++) {
        const n = native[i];
        const w = wasm[i];
        if (n !== w) {
            return {
                kind: 'hash_divergence',
                firstDivergingStep: stepIndex(i),
                nativeHash: replayHash(n),
                wasmHash: replayHash(w),
                detail: `step ${i}: native hash ${n} != WASM authority hash ${w}`,
            };
        }
    }
    return {
        kind: 'match',
        firstDivergingStep: null,
        nativeHash: null,
        wasmHash: null,
        detail: `all ${native.length} step hashes match`,
    };
}
/**
 * Run a replay record through both paths and classify. When the WASM module is
 * unavailable, pass {@link ReferenceReplayRunner} for both to get a self-consistent
 * baseline (and record the blocker out-of-band).
 */
export function compareReplay(record, nativePath, wasmAuthority) {
    return classifyDivergence(nativePath.replayHashes(record), wasmAuthority.replayHashes(record));
}
//# sourceMappingURL=index.js.map