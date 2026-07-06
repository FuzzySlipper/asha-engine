import { createMockRuntimeBridge } from './mock.js';
import { createRuntimeSessionFacade, } from './runtime-session.js';
import { referenceRuntimeSessionNonClaims } from './runtime-session-hash.js';
export const REFERENCE_RUNTIME_BACKEND_PROFILE = {
    entrypoint: '@asha/runtime-bridge/reference',
    backendKind: 'reference_fixture',
    transport: 'reference_bridge',
    productAuthority: false,
    allowedUse: ['tests', 'compatibility-fixtures', 'offline-smoke'],
    disallowedUse: ['product-authority', 'live-demo-default', 'studio-live-attach'],
    nonClaims: referenceRuntimeSessionNonClaims(),
};
export function createMockRuntimeSession(options = {}) {
    return createRuntimeSessionFacade({ bridge: options.bridge ?? createMockRuntimeBridge(), mode: 'reference' });
}
//# sourceMappingURL=mock-session.js.map