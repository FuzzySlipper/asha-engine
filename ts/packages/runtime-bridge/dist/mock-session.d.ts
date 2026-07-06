import { type RuntimeSessionFacade, type RuntimeSessionNonClaim } from './runtime-session.js';
import type { RuntimeBridge } from './bridge.js';
export interface MockRuntimeSessionOptions {
    readonly bridge?: RuntimeBridge;
}
export interface ReferenceRuntimeBackendProfile {
    readonly entrypoint: '@asha/runtime-bridge/reference';
    readonly backendKind: 'reference_fixture';
    readonly transport: 'reference_bridge';
    readonly productAuthority: false;
    readonly allowedUse: readonly ['tests', 'compatibility-fixtures', 'offline-smoke'];
    readonly disallowedUse: readonly ['product-authority', 'live-demo-default', 'studio-live-attach'];
    readonly nonClaims: readonly RuntimeSessionNonClaim[];
}
export declare const REFERENCE_RUNTIME_BACKEND_PROFILE: ReferenceRuntimeBackendProfile;
export declare function createMockRuntimeSession(options?: MockRuntimeSessionOptions): RuntimeSessionFacade;
//# sourceMappingURL=mock-session.d.ts.map