// Facade conformance + mock smoke (task #2250).
//
// (1) Conformance: the hand-written facade exposes EXACTLY the manifest operations.
// (2) Mock smoke: the default mock implements the facade with typed, classified
//     errors and deterministic behaviour matching the Rust ReferenceBridge.
// (3) Native unavailable: the native factory throws a classified bridge error when
//     the addon is not built (the expected state in offline CI).

import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import assert from 'node:assert/strict';

import type {
  CameraCreateRequest,
  CameraProjectionRequest,
  FirstPersonCameraInputEnvelope,
  CommandBatch,
  VoxelCommand,
} from '@asha/contracts';
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

test('manifest exposes public camera view operations', () => {
  const cameraOps = MANIFEST_OPERATIONS.filter((op) => op.facadeMethod.includes('Camera'));
  assert.deepEqual(
    cameraOps.map((op) => [op.manifestName, op.facadeMethod, op.surface]),
    [
      ['create_camera', 'createCamera', 'stable'],
      ['apply_first_person_camera_input', 'applyFirstPersonCameraInput', 'stable'],
      ['read_camera_projection', 'readCameraProjection', 'stable'],
    ],
  );
});

test('mock: init then step is deterministic', () => {
  const bridge = createMockRuntimeBridge();
  const handle = bridge.initializeEngine({ seed: 7 });
  assert.equal(handle as number, 7);
  assert.deepEqual(bridge.stepSimulation({ tick: 6 }), { tick: 6, diffCount: 2 });
});

test('mock: camera view operations produce deterministic public evidence', () => {
  const bridge = createMockRuntimeBridge();
  bridge.initializeEngine({ seed: 1 });

  const create: CameraCreateRequest = {
    initialPose: { position: [0, 1.6, 0], yawDegrees: 0, pitchDegrees: 0 },
    projection: { fovYDegrees: 60, near: 0.1, far: 1000 },
    viewport: { width: 1280, height: 720 },
  };
  const created = bridge.createCamera(create);
  assert.equal(created.camera as number, 1);
  assert.deepEqual(created.pose, create.initialPose);

  const input: FirstPersonCameraInputEnvelope = {
    camera: created.camera,
    tick: 1,
    input: {
      moveForward: 1,
      moveRight: 0,
      moveUp: 0,
      yawDeltaDegrees: 15,
      pitchDeltaDegrees: -5,
      dtSeconds: 1 / 60,
      moveSpeedUnitsPerSecond: 3,
    },
  };
  const moved = bridge.applyFirstPersonCameraInput(input);
  assert.equal(moved.tick, 1);
  assert.notDeepEqual(moved.pose, created.pose);

  const projection: CameraProjectionRequest = { camera: moved.camera, viewport: null };
  const snapshot = bridge.readCameraProjection(projection);
  assert.equal(snapshot.viewMatrix.length, 16);
  assert.equal(snapshot.projectionMatrix.length, 16);
  assert.equal(snapshot.viewProjectionMatrix.length, 16);
});

test('mock: camera-first-person-basic matches committed golden fixture', () => {
  const fixtureUrl = new URL(
    '../../../../harness/camera/goldens/camera-first-person-basic.json',
    import.meta.url,
  );
  const golden = JSON.parse(readFileSync(fixtureUrl, 'utf8')) as {
    expected: { readonly moved: unknown; readonly projection: unknown };
  };

  const bridge = createMockRuntimeBridge();
  bridge.initializeEngine({ seed: 1 });
  const created = bridge.createCamera({
    initialPose: { position: [0, 1.6, 0], yawDegrees: 0, pitchDegrees: 0 },
    projection: { fovYDegrees: 60, near: 0.1, far: 1000 },
    viewport: { width: 1280, height: 720 },
  });
  const moved = bridge.applyFirstPersonCameraInput({
    camera: created.camera,
    tick: 1,
    input: {
      moveForward: 1,
      moveRight: 0,
      moveUp: 0,
      yawDeltaDegrees: 15,
      pitchDeltaDegrees: -5,
      dtSeconds: 1 / 60,
      moveSpeedUnitsPerSecond: 3,
    },
  });
  const projection = bridge.readCameraProjection({ camera: moved.camera, viewport: null });

  assert.deepEqual(moved, golden.expected.moved);
  assert.deepEqual(projection, golden.expected.projection);
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

test('mock: pickVoxel carries a PickRay and returns a classified PickResult', () => {
  const bridge = createMockRuntimeBridge();
  bridge.initializeEngine({ seed: 1 });
  const result = bridge.pickVoxel({
    grid: 1,
    origin: [0, 0, 0],
    direction: [1, 0, 0],
    maxDistance: 10,
  });
  // The mock hosts no geometry, so it classifies as a miss (Rust authority owns hits).
  assert.deepEqual(result, { outcome: 'miss', rejection: { reason: 'noHit' } });
});

test('mock: pickVoxel before init fails closed', () => {
  const bridge = createMockRuntimeBridge();
  assert.throws(
    () => bridge.pickVoxel({ grid: 1, origin: [0, 0, 0], direction: [1, 0, 0], maxDistance: 10 }),
    (e: unknown) => e instanceof RuntimeBridgeError && e.kind === 'not_initialized',
  );
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
  // Parity with MockRuntimeBridge / Rust ReferenceBridge for the native authority sequence.
  const handle = bridge.initializeEngine({ seed: 7 }) as number;
  assert.equal(typeof handle, 'number');
  assert.deepEqual(bridge.loadWorldBundle({ bundleSchemaVersion: 1, protocolVersion: 1, sceneId: 1001 }), {
    loadedWorld: 1001,
    fatalCount: 0,
    totalCount: 0,
    blocksLoad: false,
  });
  assert.deepEqual(
    bridge.submitCommands({
      commands: [
        { op: 'setVoxel', grid: 1, coord: { x: 0, y: 0, z: 0 }, value: { kind: 'solid', material: 1 } },
      ],
    }),
    { accepted: 1, rejected: 0, rejections: [] },
  );
  assert.deepEqual(bridge.stepSimulation({ tick: 6 }), { tick: 6, diffCount: 2 });
  assert.deepEqual(bridge.readRenderDiffs(frameCursor(0)), { ops: [] });
  assert.deepEqual(bridge.saveCurrentWorld(), { artifactsWritten: 3, compactedEdits: 0, retainedEdits: 0 });
  assert.deepEqual(bridge.getCompositionStatus(), {
    loadedWorld: 1001,
    fatalCount: 0,
    totalCount: 0,
    blocksLoad: false,
  });
});
