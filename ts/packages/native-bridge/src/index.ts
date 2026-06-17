// @asha/native-bridge — thin, typed loader for the napi-rs runtime addon.
//
// Scope (ADR 0006): this package wraps the compiled `native-bridge.<platform>.node`
// addon (built from engine-rs/crates/bridge/native-bridge) and exposes its exports
// with explicit TypeScript signatures. It contains NO semantic logic and NO schema
// definitions — it is transport glue. It is imported ONLY by `@asha/runtime-bridge`
// (enforced by governance/ownership.toml); app/UI/renderer never import it.

import { createRequire } from 'node:module';

/**
 * The typed surface the compiled addon exports. Mirrors the `#[napi]` functions in
 * `native-bridge/src/lib.rs`. Kept in lockstep with the bridge manifest's stable
 * operations; the generated `#[napi]` wrappers (one-in/one-out) replace the
 * hand-written stubs once the codegen emitter lands.
 */
export interface NativeAddon {
  initializeEngine(seed: number): number;
  loadWorldBundle(
    handle: number,
    bundleSchemaVersion: number,
    protocolVersion: number,
    sceneId: number,
  ): {
    loadedWorld: number | null;
    fatalCount: number;
    totalCount: number;
    blocksLoad: boolean;
  };
  submitCommands(handle: number, commandsJson: string): {
    accepted: number;
    rejected: number;
    rejections: unknown[];
  };
  stepSimulation(handle: number, tick: number): number;
  readRenderDiffs(handle: number, cursor: number): { ops: unknown[] };
  saveCurrentWorld(handle: number): {
    artifactsWritten: number;
    compactedEdits: number;
    retainedEdits: number;
  };
  getCompositionStatus(handle: number): {
    loadedWorld: number | null;
    fatalCount: number;
    totalCount: number;
    blocksLoad: boolean;
  };
}

/** Raised when the native addon cannot be loaded (missing build / ABI mismatch). */
export class NativeAddonUnavailable extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'NativeAddonUnavailable';
  }
}

const REQUIRED_EXPORTS = [
  'initializeEngine',
  'loadWorldBundle',
  'submitCommands',
  'stepSimulation',
  'readRenderDiffs',
  'saveCurrentWorld',
  'getCompositionStatus',
] as const;

/**
 * Attempt to load the compiled addon. Returns a typed handle or throws a
 * classified {@link NativeAddonUnavailable} — never a raw module-resolution error,
 * so `@asha/runtime-bridge` can re-map it to a `native_unavailable` bridge error.
 *
 * Build the addon with `napi build --platform --release` in the native-bridge crate.
 */
export function loadNativeAddon(modulePath = './native-bridge.node'): NativeAddon {
  const require = createRequire(import.meta.url);
  try {
    const mod = require(modulePath) as Partial<Record<(typeof REQUIRED_EXPORTS)[number], unknown>>;
    const missing = REQUIRED_EXPORTS.filter((name) => typeof mod[name] !== 'function');
    if (missing.length > 0) {
      throw new NativeAddonUnavailable(
        `addon at ${modulePath} is missing expected exports (${missing.join(', ')})`,
      );
    }
    return mod as NativeAddon;
  } catch (cause) {
    if (cause instanceof NativeAddonUnavailable) throw cause;
    const reason = cause instanceof Error ? cause.message : String(cause);
    throw new NativeAddonUnavailable(`failed to load native addon at ${modulePath}: ${reason}`);
  }
}
