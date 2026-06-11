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
  stepSimulation(seed: number, tick: number): number;
}

/** Raised when the native addon cannot be loaded (missing build / ABI mismatch). */
export class NativeAddonUnavailable extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'NativeAddonUnavailable';
  }
}

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
    const mod = require(modulePath) as Partial<NativeAddon>;
    if (typeof mod.initializeEngine !== 'function' || typeof mod.stepSimulation !== 'function') {
      throw new NativeAddonUnavailable(
        `addon at ${modulePath} is missing expected exports (initializeEngine/stepSimulation)`,
      );
    }
    return mod as NativeAddon;
  } catch (cause) {
    if (cause instanceof NativeAddonUnavailable) throw cause;
    const reason = cause instanceof Error ? cause.message : String(cause);
    throw new NativeAddonUnavailable(`failed to load native addon at ${modulePath}: ${reason}`);
  }
}
