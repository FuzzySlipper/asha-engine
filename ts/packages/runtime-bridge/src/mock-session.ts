import { createMockRuntimeBridge } from './mock.js';
import {
  createRuntimeSessionFacade,
  type RuntimeSessionFacade,
  type RuntimeSessionNonClaim,
} from './runtime-session.js';
import type { RuntimeBridge } from './bridge.js';
import { referenceRuntimeSessionNonClaims } from './runtime-session-hash.js';

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

export const REFERENCE_RUNTIME_BACKEND_PROFILE: ReferenceRuntimeBackendProfile = {
  entrypoint: '@asha/runtime-bridge/reference',
  backendKind: 'reference_fixture',
  transport: 'reference_bridge',
  productAuthority: false,
  allowedUse: ['tests', 'compatibility-fixtures', 'offline-smoke'],
  disallowedUse: ['product-authority', 'live-demo-default', 'studio-live-attach'],
  nonClaims: referenceRuntimeSessionNonClaims(),
};

export function createMockRuntimeSession(options: MockRuntimeSessionOptions = {}): RuntimeSessionFacade {
  return createRuntimeSessionFacade({ bridge: options.bridge ?? createMockRuntimeBridge(), mode: 'reference' });
}
