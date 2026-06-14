// Facade conformance + mock smoke (task #2250).
//
// (1) Conformance: the hand-written facade exposes EXACTLY the manifest operations.
// (2) Mock smoke: the default mock implements the facade with typed, classified
//     errors and deterministic behaviour matching the Rust ReferenceBridge.
// (3) Native unavailable: the native factory throws a classified bridge error when
//     the addon is not built (the expected state in offline CI).

import { test } from 'node:test';
import assert from 'node:assert/strict';

import type { CommandBatch, VoxelCommand } from '@asha/contracts';
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

test('mock: world load → save → status → unload, with fail-closed save', () => {
  const bridge = createMockRuntimeBridge();
  // Save before load fails closed.
  assert.throws(
    () => bridge.saveCurrentWorld(),
    (e: unknown) => e instanceof RuntimeBridgeError && e.kind === 'not_initialized',
  );
  const status = bridge.loadWorldBundle({
    bundleSchemaVersion: 1,
    protocolVersion: 1,
    sceneId: 100,
  });
  assert.equal(status.loadedWorld, 100);
  assert.equal(status.blocksLoad, false);
  assert.deepEqual(bridge.saveCurrentWorld(), {
    artifactsWritten: 3,
    compactedEdits: 0,
    retainedEdits: 0,
  });
  assert.equal(bridge.getCompositionStatus().loadedWorld, 100);
  bridge.unloadWorld();
  assert.equal(bridge.getCompositionStatus().loadedWorld, null);
});

test('mock: an unsupported bundle version fails closed without swapping the world', () => {
  const bridge = createMockRuntimeBridge();
  bridge.loadWorldBundle({ bundleSchemaVersion: 1, protocolVersion: 1, sceneId: 7 });
  assert.throws(
    () => bridge.loadWorldBundle({ bundleSchemaVersion: 99, protocolVersion: 1, sceneId: 8 }),
    (e: unknown) => e instanceof RuntimeBridgeError && e.kind === 'invalid_input',
  );
  // The prior world stays loaded (no partial swap).
  assert.equal(bridge.getCompositionStatus().loadedWorld, 7);
});

test('mock: submitCommands carries the generated VoxelCommand union (the launch path)', () => {
  const bridge = createMockRuntimeBridge();
  bridge.initializeEngine({ seed: 1 });
  // A real generated voxel command — the authority-owned union, not a `{ kind }` blob.
  const command: VoxelCommand = {
    op: 'setVoxel',
    grid: 1,
    coord: { x: 0, y: 0, z: 0 },
    value: { kind: 'solid', material: 1 },
  };
  const result = bridge.submitCommands({ commands: [command] });
  assert.deepEqual(result, { accepted: 1, rejected: 0, rejections: [] });
});

test('mock: submitCommands before init fails closed', () => {
  const bridge = createMockRuntimeBridge();
  assert.throws(
    () => bridge.submitCommands({ commands: [] }),
    (e: unknown) => e instanceof RuntimeBridgeError && e.kind === 'not_initialized',
  );
});

test('an ad-hoc `{ kind }` command is NOT the launch path (compile-time guard)', () => {
  const bridge = createMockRuntimeBridge();
  bridge.initializeEngine({ seed: 1 });
  // The placeholder command shape the launch path used to accept must no longer
  // type-check: `submitCommands` takes the generated VoxelCommand union only.
  // @ts-expect-error — `{ kind: 'smoke-edit' }` is not a VoxelCommand.
  const bad: CommandBatch = { commands: [{ kind: 'smoke-edit' }] };
  assert.equal(bad.commands.length, 1);
});

test('native factory classifies a missing addon path', () => {
  assert.throws(
    () => createNativeRuntimeBridge('./definitely-not-built.node'),
    (e: unknown) => e instanceof RuntimeBridgeError && e.kind === 'native_unavailable',
  );
});

test('native bridge matches the mock when the addon is built (else skip)', (t) => {
  let bridge;
  try {
    bridge = createNativeRuntimeBridge();
  } catch (e) {
    if (e instanceof RuntimeBridgeError && e.kind === 'native_unavailable') {
      t.skip('native addon not built (run harness/ci/check-native.sh)');
      return;
    }
    throw e;
  }
  // Parity with MockRuntimeBridge / Rust ReferenceBridge.
  assert.equal(bridge.initializeEngine({ seed: 7 }) as number, 7);
  assert.deepEqual(bridge.stepSimulation({ tick: 6 }), { tick: 6, diffCount: 2 });
});
