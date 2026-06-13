import { type RuntimeBridge } from '@asha/runtime-bridge';
import type { RuntimeMode, SmokeResult } from './result.js';
export declare const SMOKE_COMMAND = "pnpm --filter @asha/smoke dev:asha-smoke";
/** How the harness obtains a runtime bridge (injectable for tests). */
export interface BridgeBoot {
    readonly bridge: RuntimeBridge;
    readonly mode: RuntimeMode;
    readonly nativeAvailable: boolean;
}
export interface SmokeOptions {
    /** Override how the bridge is constructed (tests inject failures / native). */
    readonly bootBridge?: () => BridgeBoot;
}
/**
 * Default boot: run the canonical smoke on the fully-wired mock facade (the
 * deterministic reference), while *probing* native availability for the capability
 * readout. The native addon today is a partial prototype (only initialize/step are
 * wired), so the canonical dev smoke does not depend on it; native mode is opt-in
 * by injecting a `bootBridge`. Reporting `nativeAvailable` keeps the readout honest.
 */
export declare function defaultBootBridge(): BridgeBoot;
/** Run the full smoke flow and return a deterministic structured result. */
export declare function runSmoke(options?: SmokeOptions): SmokeResult;
//# sourceMappingURL=harness.d.ts.map