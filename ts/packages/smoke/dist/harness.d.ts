import { RuntimeBridgeError, type RuntimeBridge } from '@asha/runtime-bridge';
import type { RuntimeMode, SmokeMode, SmokeResult } from './result.js';
export declare const SMOKE_COMMAND = "pnpm --filter @asha/smoke dev:asha-smoke";
export declare const AUTHORITY_SMOKE_COMMAND = "ASHA_SMOKE_MODE=authority pnpm --filter @asha/smoke dev:asha-smoke";
/** How the harness obtains a runtime bridge (injectable for tests). */
export interface BridgeBoot {
    /** The booted bridge, or `null` when boot itself failed (e.g. native unavailable). */
    readonly bridge: RuntimeBridge | null;
    readonly mode: RuntimeMode;
    /** What this run is trying to prove (reference vs. real authority path). */
    readonly intent: SmokeMode;
    readonly nativeAvailable: boolean;
    /** Classified reason the bridge is null (required when `bridge` is null). */
    readonly bootError?: RuntimeBridgeError;
}
export interface SmokeOptions {
    /** Override how the bridge is constructed (tests inject failures / native / authority). */
    readonly bootBridge?: () => BridgeBoot;
}
/**
 * Default boot: the canonical deterministic reference smoke on the mock facade, while
 * *probing* native availability for an honest capability readout. The native addon
 * today is a partial prototype (only initialize/step are wired), so the reference
 * smoke does not depend on it.
 */
export declare function defaultBootBridge(): BridgeBoot;
/**
 * Authority boot: attempt the real native path. If the native addon is not loadable,
 * the boot fails *closed* with a classified error — the harness reports an honest
 * failure rather than silently downgrading to the mock.
 */
export declare function authorityBootBridge(): BridgeBoot;
/** Pick a boot strategy from an explicit smoke mode (used by the CLI). */
export declare function bootForMode(mode: SmokeMode): BridgeBoot;
/** Run the full staged smoke flow and return a deterministic structured result. */
export declare function runSmoke(options?: SmokeOptions): SmokeResult;
//# sourceMappingURL=harness.d.ts.map