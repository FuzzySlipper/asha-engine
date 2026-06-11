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

/** Raised when the WASM replay module is not built (toolchain/target missing). */
export class WasmReplayUnavailable extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'WasmReplayUnavailable';
  }
}

// ── Per-step hash extraction (pure) ───────────────────────────────────────────

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
export class ReferenceReplayRunner implements ReplayHasher {
  replayHashes(record: ReplayRecord): readonly number[] {
    // Use the recorded post-step hashes directly. A real runner re-executes the
    // commands and recomputes hashes; the reference trusts the record so the
    // golden path has a deterministic, toolchain-free baseline.
    return record.steps.map((s) => s.postHash as number);
  }
}

// ── WASM-backed divergence authority (the real sim-replay logic under WASM) ───

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

function parseDivergence(raw: string): ReplayDivergence {
  const tab = raw.indexOf('\t');
  const cls = tab >= 0 ? raw.slice(0, tab) : raw;
  const stepRaw = tab >= 0 ? raw.slice(tab + 1) : '-';
  const step = stepRaw === '-' ? null : Number.parseInt(stepRaw, 10);
  return { class: cls, matched: cls === 'match', step: Number.isNaN(step as number) ? null : step };
}

/**
 * Load the compiled wasm-api replay authority (wasm-bindgen `--target nodejs`
 * output, `.cjs`). Throws a classified {@link WasmReplayUnavailable} when unbuilt.
 * Build with `harness/ci/check-wasm-replay.sh`.
 */
export function loadWasmReplayAuthority(
  modulePath = './wasm/wasm_api.cjs',
): WasmReplayAuthority {
  const require = createRequire(import.meta.url);
  let mod: WasmApiModule;
  try {
    mod = require(modulePath) as WasmApiModule;
    if (typeof mod.classify_divergence !== 'function') {
      throw new WasmReplayUnavailable(`module at ${modulePath} is missing 'classify_divergence'`);
    }
  } catch (cause) {
    if (cause instanceof WasmReplayUnavailable) throw cause;
    const reason = cause instanceof Error ? cause.message : String(cause);
    throw new WasmReplayUnavailable(`failed to load wasm-api module at ${modulePath}: ${reason}`);
  }
  return {
    classifyRecords: (expected, actual) => parseDivergence(mod.classify_divergence(expected, actual)),
    classLabels: () => mod.divergence_class_labels().split('\n'),
  };
}

// ── Native-vs-WASM divergence classification (pure, hash-array level) ──────────

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
  nativePath: ReplayHasher,
  wasmAuthority: ReplayHasher,
): DivergenceReport {
  return classifyDivergence(nativePath.replayHashes(record), wasmAuthority.replayHashes(record));
}
