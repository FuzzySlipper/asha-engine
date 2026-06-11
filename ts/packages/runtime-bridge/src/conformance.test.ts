// Facade conformance + mock smoke (task #2250).
//
// (1) Conformance: the hand-written facade exposes EXACTLY the manifest operations.
// (2) Mock smoke: the default mock implements the facade with typed, classified
//     errors and deterministic behaviour matching the Rust ReferenceBridge.
// (3) Native unavailable: the native factory throws a classified bridge error when
//     the addon is not built (the expected state in offline CI).

import { test } from 'node:test';
import assert from 'node:assert/strict';

import {
  MANIFEST_OPERATIONS,
  MockRuntimeBridge,
  RuntimeBridgeError,
  createMockRuntimeBridge,
  createNativeRuntimeBridge,
  frameCursor,
  type RuntimeBufferHandle,
} from './index.js';

test('facade exposes exactly the manifest operations (conformance)', () => {
  const bridge = createMockRuntimeBridge();
  const expected = MANIFEST_OPERATIONS.map((o) => o.facadeMethod).sort();
  const actual = MANIFEST_OPERATIONS.map((o) => o.facadeMethod)
    .filter((m) => typeof (bridge as unknown as Record<string, unknown>)[m] === 'function')
    .sort();
  assert.deepEqual(actual, expected, 'every manifest op must be a facade method');

  // No extra public methods beyond the manifest on the mock prototype.
  const proto = Object.getOwnPropertyNames(MockRuntimeBridge.prototype).filter(
    (n) => n !== 'constructor',
  );
  const known = new Set(MANIFEST_OPERATIONS.map((o) => o.facadeMethod));
  assert.deepEqual(
    proto.filter((n) => !known.has(n)),
    [],
    'mock must not expose methods outside the manifest',
  );
});

test('mock: init then step is deterministic', () => {
  const bridge = createMockRuntimeBridge();
  const handle = bridge.initializeEngine({ seed: 7 });
  assert.equal(handle as number, 7);
  assert.deepEqual(bridge.stepSimulation({ tick: 6 }), { tick: 6, diffCount: 2 });
});

test('mock: step before init throws a classified error', () => {
  const bridge = createMockRuntimeBridge();
  assert.throws(
    () => bridge.stepSimulation({ tick: 1 }),
    (e: unknown) => e instanceof RuntimeBridgeError && e.kind === 'not_initialized',
  );
});

test('mock: buffer round-trip and unknown handle classification', () => {
  const bridge = createMockRuntimeBridge();
  bridge.initializeEngine({ seed: 0x01020304 });
  const view = bridge.getBuffer(0 as RuntimeBufferHandle);
  const expected = new Uint8Array(8);
  new DataView(expected.buffer).setBigUint64(0, BigInt(0x01020304), true);
  assert.deepEqual(view.bytes, expected);
  assert.throws(
    () => bridge.getBuffer(99 as RuntimeBufferHandle),
    (e: unknown) => e instanceof RuntimeBridgeError && e.kind === 'unknown_handle',
  );
});

test('mock: readRenderDiffs returns a contract-shaped frame', () => {
  const bridge = createMockRuntimeBridge();
  bridge.initializeEngine({ seed: 1 });
  const frame = bridge.readRenderDiffs(frameCursor(0));
  assert.deepEqual(frame, { ops: [] });
});

test('native factory throws native_unavailable when addon is not built', () => {
  assert.throws(
    () => createNativeRuntimeBridge(),
    (e: unknown) => e instanceof RuntimeBridgeError && e.kind === 'native_unavailable',
  );
});
