import { type RuntimeSessionFacade } from './runtime-session.js';
import type { RuntimeBridge } from './bridge.js';
export * from './mock.js';
export { ReferenceGameRuntimeLauncher, createReferenceGameRuntimeLauncher, referenceBackendProfile, } from './launcher.js';
export type { GameRuntimeLauncher, GameRuntimeConfig, GameRuntimeSession, } from './launcher.js';
export interface MockRuntimeSessionOptions {
    readonly bridge?: RuntimeBridge;
}
export declare function createMockRuntimeSession(options?: MockRuntimeSessionOptions): RuntimeSessionFacade;
//# sourceMappingURL=reference.d.ts.map