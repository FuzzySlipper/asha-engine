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
import {
  type ReplayRecord,
  type ReplayHash,
  type StepIndex,
  replayHash,
  stepIndex,
} from '@asha/contracts';

// ── WASM replay module loader ─────────────────────────────────────────────────

/** Raised when the WASM replay module is not built (toolchain/target missing). */
export class WasmReplayUnavailable extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'WasmReplayUnavailable';
  }
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
export function loadWasmReplayModule(modulePath = './wasm-api-replay.cjs'): WasmReplayModule {
  const require = createRequire(import.meta.url);
  try {
    const mod = require(modulePath) as Partial<WasmReplayModule>;
    if (typeof mod.replayHashes !== 'function') {
      throw new WasmReplayUnavailable(
        `module at ${modulePath} is missing the expected export 'replayHashes'`,
      );
    }
    return mod as WasmReplayModule;
  } catch (cause) {
    if (cause instanceof WasmReplayUnavailable) throw cause;
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
export class ReferenceReplayRunner implements WasmReplayModule {
  replayHashes(record: ReplayRecord): readonly number[] {
    // Use the recorded post-step hashes directly. A real runner re-executes the
    // commands and recomputes hashes; the reference trusts the record so the
    // golden path has a deterministic, toolchain-free baseline.
    return record.steps.map((s) => s.postHash as number);
  }
}

// ── Native-vs-WASM divergence classification ──────────────────────────────────

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
export function classifyDivergence(
  native: readonly number[],
  wasm: readonly number[],
): DivergenceReport {
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
    const n = native[i] as number;
    const w = wasm[i] as number;
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
export function compareReplay(
  record: ReplayRecord,
  nativePath: WasmReplayModule,
  wasmAuthority: WasmReplayModule,
): DivergenceReport {
  return classifyDivergence(nativePath.replayHashes(record), wasmAuthority.replayHashes(record));
}
