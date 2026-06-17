// Native facade parity / fail-closed conformance (task #2423).
//
// Proves the seam closed in this task: a *loaded* native facade either executes a
// real native implementation or throws a classified `operation_unimplemented`
// error for every manifest operation. It must NEVER silently inherit mock /
// reference behaviour for an unwired op (the prior `extends MockRuntimeBridge`
// hazard). We inject a fake addon so the test runs without a built `.node` binary.

import { test } from 'node:test';
import assert from 'node:assert/strict';

import type { NativeAddon } from '@asha/native-bridge';
import {
  MANIFEST_OPERATIONS,
  NATIVE_WIRED_OPERATIONS,
  NativeRuntimeBridge,
  RuntimeBridgeError,
  frameCursor,
  type RuntimeBridge,
  type RuntimeBufferHandle,
  type ReplaySessionHandle,
} from './index.js';

const CAMERA_CREATE_REQUEST = {
  initialPose: { position: [0, 1.6, 0] as const, yawDegrees: 0, pitchDegrees: 0 },
  projection: { fovYDegrees: 60, near: 0.1, far: 1000 },
  viewport: { width: 1280, height: 720 },
} as const;

const CAMERA_INPUT = {
  camera: 1 as import('@asha/contracts').CameraHandle,
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
} as const;

const REQUIRED_NATIVE_CONFORMANCE_OPS = [
  'initialize_engine',
  'load_world_bundle',
  'submit_commands',
  'step_simulation',
  'read_render_diffs',
  'save_current_world',
  'get_composition_status',
] as const;

// A fake addon with sentinel return values distinct from MockRuntimeBridge, so a
// silent mock fallback would be observable in the wired-op assertions below.
function fakeAddon(calls: string[] = []): NativeAddon {
  return {
    initializeEngine: (seed: number) => {
      calls.push(`initialize:${seed}`);
      return seed + 100;
    },
    loadWorldBundle: (_handle: number, bundleSchemaVersion: number, protocolVersion: number, sceneId: number) => {
      calls.push(`load:${bundleSchemaVersion}:${protocolVersion}:${sceneId}`);
      return { loadedWorld: sceneId + 1000, fatalCount: 0, totalCount: 0, blocksLoad: false };
    },
    submitCommands: (_handle: number, commandsJson: string) => {
      calls.push(`submit:${commandsJson}`);
      return { accepted: JSON.parse(commandsJson).length, rejected: 0, rejections: [] };
    },
    stepSimulation: (_handle: number, tick: number) => {
      calls.push(`step:${tick}`);
      return 9;
    },
    readRenderDiffs: (_handle: number, cursor: number) => {
      calls.push(`render:${cursor}`);
      return { ops: [{ op: 'sentinel' }] } as never;
    },
    saveCurrentWorld: (_handle: number) => {
      calls.push('save');
      return { artifactsWritten: 5, compactedEdits: 2, retainedEdits: 3 };
    },
    getCompositionStatus: (_handle: number) => {
      calls.push('status');
      return { loadedWorld: 2001, fatalCount: 0, totalCount: 0, blocksLoad: false };
    },
  } as unknown as NativeAddon;
}

// One invocation per facade method. The native bridge is fully initialized first
// so that wired ops exercise their happy path rather than `not_initialized`.
// Typed against the `RuntimeBridge` interface (which carries the operation
// payloads); a `NativeRuntimeBridge` instance is assignable to it.
const INVOKE = new Map<string, (b: RuntimeBridge) => unknown>([
  ['initializeEngine', (b) => b.initializeEngine({ seed: 7 })],
  ['stepSimulation', (b) => b.stepSimulation({ tick: 6 })],
  ['submitCommands', (b) => b.submitCommands({ commands: [] })],
  [
    'pickVoxel',
    (b) => b.pickVoxel({ grid: 1, origin: [0, 0, 0], direction: [1, 0, 0], maxDistance: 10 }),
  ],
  ['readRenderDiffs', (b) => b.readRenderDiffs(frameCursor(0))],
  ['createCamera', (b) => b.createCamera(CAMERA_CREATE_REQUEST)],
  ['applyFirstPersonCameraInput', (b) => b.applyFirstPersonCameraInput(CAMERA_INPUT)],
  ['readCameraProjection', (b) => b.readCameraProjection({ camera: CAMERA_INPUT.camera, viewport: null })],
  ['getBuffer', (b) => b.getBuffer(0 as RuntimeBufferHandle)],
  ['releaseBuffer', (b) => b.releaseBuffer(0 as RuntimeBufferHandle)],
  [
    'loadWorldBundle',
    (b) => b.loadWorldBundle({ bundleSchemaVersion: 1, protocolVersion: 1, sceneId: 1 }),
  ],
  ['saveCurrentWorld', (b) => b.saveCurrentWorld()],
  ['getCompositionStatus', (b) => b.getCompositionStatus()],
  ['unloadWorld', (b) => b.unloadWorld()],
  ['loadReplayFixture', (b) => b.loadReplayFixture({ name: 'x', steps: 1 })],
  ['runReplayStep', (b) => b.runReplayStep(0 as ReplaySessionHandle)],
]);

test('every manifest op has a native invocation in this test', () => {
  for (const op of MANIFEST_OPERATIONS) {
    assert.ok(INVOKE.has(op.facadeMethod), `missing invocation for ${op.facadeMethod}`);
  }
});

test('unwired native ops fail closed with operation_unimplemented (no mock fallback)', () => {
  for (const op of MANIFEST_OPERATIONS) {
    if (NATIVE_WIRED_OPERATIONS.has(op.manifestName)) continue;
    const invoke = INVOKE.get(op.facadeMethod);
    assert.ok(invoke, `missing invocation for ${op.facadeMethod}`);
    const bridge = new NativeRuntimeBridge(fakeAddon());
    // A fresh, initialized bridge: proves the throw is fail-closed classification,
    // not an incidental `not_initialized`.
    bridge.initializeEngine({ seed: 1 });
    assert.throws(
      () => invoke(bridge),
      (e: unknown) => e instanceof RuntimeBridgeError && e.kind === 'operation_unimplemented',
      `${op.manifestName} must fail closed, not inherit mock behaviour`,
    );
  }
});

test('required native conformance operations are declared wired', () => {
  for (const manifestName of REQUIRED_NATIVE_CONFORMANCE_OPS) {
    assert.ok(
      NATIVE_WIRED_OPERATIONS.has(manifestName),
      `${manifestName} must be wired for native authority conformance`,
    );
  }
});

test('native conformance sequence routes through the addon without mock fallback', () => {
  const calls: string[] = [];
  const bridge: RuntimeBridge = new NativeRuntimeBridge(fakeAddon(calls));

  assert.equal(bridge.initializeEngine({ seed: 7 }) as number, 107);
  assert.deepEqual(bridge.loadWorldBundle({ bundleSchemaVersion: 1, protocolVersion: 1, sceneId: 1001 }), {
    loadedWorld: 2001,
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
  assert.deepEqual(bridge.stepSimulation({ tick: 6 }), { tick: 6, diffCount: 9 });
  assert.deepEqual(bridge.readRenderDiffs(frameCursor(0)), { ops: [{ op: 'sentinel' }] });
  assert.deepEqual(bridge.saveCurrentWorld(), { artifactsWritten: 5, compactedEdits: 2, retainedEdits: 3 });
  assert.deepEqual(bridge.getCompositionStatus(), {
    loadedWorld: 2001,
    fatalCount: 0,
    totalCount: 0,
    blocksLoad: false,
  });

  assert.deepEqual(calls, [
    'initialize:7',
    'load:1:1:1001',
    'submit:[{"op":"setVoxel","grid":1,"coord":{"x":0,"y":0,"z":0},"value":{"kind":"solid","material":1}}]',
    'step:6',
    'render:0',
    'save',
    'status',
  ]);
});

test('wired native ops route through the addon, not the mock', () => {
  const calls: string[] = [];
  const bridge = new NativeRuntimeBridge(fakeAddon(calls));
  // Mock would return the seed (7) and diffCount 2; the addon returns 107 / 9.
  assert.equal(bridge.initializeEngine({ seed: 7 }) as number, 107);
  assert.deepEqual(bridge.stepSimulation({ tick: 6 }), { tick: 6, diffCount: 9 });
  assert.deepEqual(calls, ['initialize:7', 'step:6']);
});

test('native bridge does not extend MockRuntimeBridge (no inherited mock methods)', () => {
  // Guards against re-introducing the `extends MockRuntimeBridge` seam: every
  // own/inherited facade method must be declared on NativeRuntimeBridge itself.
  const proto = NativeRuntimeBridge.prototype as unknown as Record<string, unknown>;
  for (const op of MANIFEST_OPERATIONS) {
    assert.ok(
      Object.prototype.hasOwnProperty.call(
        Object.getPrototypeOf(new NativeRuntimeBridge(fakeAddon())),
        op.facadeMethod,
      ),
      `${op.facadeMethod} must be declared on NativeRuntimeBridge, not inherited`,
    );
    assert.equal(typeof proto[op.facadeMethod], 'function');
  }
});

test('native bridge step before init fails closed (not_initialized)', () => {
  const bridge = new NativeRuntimeBridge(fakeAddon());
  assert.throws(
    () => bridge.stepSimulation({ tick: 1 }),
    (e: unknown) => e instanceof RuntimeBridgeError && e.kind === 'not_initialized',
  );
});

test('wired set names are real manifest operations', () => {
  const manifestNames = new Set(MANIFEST_OPERATIONS.map((o) => o.manifestName));
  for (const name of NATIVE_WIRED_OPERATIONS) {
    assert.ok(manifestNames.has(name), `${name} in NATIVE_WIRED_OPERATIONS is not a manifest op`);
  }
});
