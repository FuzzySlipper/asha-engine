import { createMockRuntimeBridge } from './mock.js';
import {
  createRuntimeSessionFacade,
  type RuntimeSessionFacade,
} from './runtime-session.js';
import type { RuntimeBridge } from './bridge.js';

export * from './mock.js';
export {
  ReferenceGameRuntimeLauncher,
  createReferenceGameRuntimeLauncher,
  referenceBackendProfile,
} from './launcher.js';
export type {
  GameRuntimeLauncher,
  GameRuntimeConfig,
  GameRuntimeSession,
} from './launcher.js';

export interface MockRuntimeSessionOptions {
  readonly bridge?: RuntimeBridge;
}

export function createMockRuntimeSession(options: MockRuntimeSessionOptions = {}): RuntimeSessionFacade {
  return createRuntimeSessionFacade({ bridge: options.bridge ?? createMockRuntimeBridge() });
}
