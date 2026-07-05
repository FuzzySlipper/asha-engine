import { createMockRuntimeBridge } from './mock.js';
import { createRuntimeSessionFacade, } from './runtime-session.js';
export * from './mock.js';
export { ReferenceGameRuntimeLauncher, createReferenceGameRuntimeLauncher, referenceBackendProfile, } from './launcher.js';
export function createMockRuntimeSession(options = {}) {
    return createRuntimeSessionFacade({ bridge: options.bridge ?? createMockRuntimeBridge() });
}
//# sourceMappingURL=reference.js.map