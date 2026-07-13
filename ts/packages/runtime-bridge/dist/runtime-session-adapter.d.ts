import { type RuntimeBridge } from './bridge.js';
import type { RuntimeSessionFacade, RuntimeSessionMode } from '@asha/runtime-session';
export interface RuntimeSessionFacadeOptions {
    readonly bridge: RuntimeBridge;
    readonly mode?: RuntimeSessionMode;
}
export declare function createRuntimeSessionFacade(options: RuntimeSessionFacadeOptions): RuntimeSessionFacade;
//# sourceMappingURL=runtime-session-adapter.d.ts.map