// @asha/native-bridge — thin, typed loader for the napi-rs runtime addon.
//
// Scope (ADR 0006): this package wraps the compiled `native-bridge.<platform>.node`
// addon (built from engine-rs/crates/bridge/native-bridge) and exposes its exports
// with explicit TypeScript signatures. It contains NO semantic logic and NO schema
// definitions — it is transport glue. It is imported ONLY by `@asha/runtime-bridge`
// (enforced by governance/ownership.toml); app/UI/renderer never import it.
import { createRequire } from 'node:module';
/** Raised when the native addon cannot be loaded (missing build / ABI mismatch). */
export class NativeAddonUnavailable extends Error {
    constructor(message) {
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
export function loadNativeAddon(modulePath = './native-bridge.node') {
    const require = createRequire(import.meta.url);
    try {
        const mod = require(modulePath);
        if (typeof mod.initializeEngine !== 'function' || typeof mod.stepSimulation !== 'function') {
            throw new NativeAddonUnavailable(`addon at ${modulePath} is missing expected exports (initializeEngine/stepSimulation)`);
        }
        return mod;
    }
    catch (cause) {
        if (cause instanceof NativeAddonUnavailable)
            throw cause;
        const reason = cause instanceof Error ? cause.message : String(cause);
        throw new NativeAddonUnavailable(`failed to load native addon at ${modulePath}: ${reason}`);
    }
}
//# sourceMappingURL=index.js.map