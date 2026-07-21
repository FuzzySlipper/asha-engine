// Facade conformance + mock smoke (task #2250).
//
// (1) Conformance: the hand-written facade exposes EXACTLY the manifest operations.
// (2) Mock smoke: the default mock implements the facade with typed, classified
//     errors and deterministic behaviour matching the Rust ReferenceBridge.
// (3) Native unavailable: the native factory throws a classified bridge error when
//     the addon is not built (the expected state in offline CI).

import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { test } from 'node:test';
import assert from 'node:assert/strict';

import type {
  CameraCreateRequest,
  CameraProjectionRequest,
  FirstPersonCameraInputEnvelope,
  CommandBatch,
  VoxelCommand,
} from '@asha/contracts';

const MODEL_MATERIAL_PREVIEW_REQUEST: ModelMaterialPreviewRequest = {
  catalogEntry: {
    id: 'material.copper',
    kind: 'material',
    version: 1,
    hash: 'sha256-material-copper',
    sourcePath: null,
    label: 'Copper',
    dependencies: [],
    material: {
      render: { color: { r: 0.8, g: 0.4, b: 0.2, a: 1 }, texture: null, roughness: 0.6, textureTint: { r: 1, g: 1, b: 1, a: 1 }, emissionColor: { r: 0.8, g: 0.4, b: 0.2, a: 1 }, emissive: 0, uvStrategy: 'flat' },
      collision: { solid: true, collidable: true, occludes: true, structuralClass: 'solid' },
    },
  },
  meshAsset: {
    asset: 'mesh.preview-triangle',
    payload: {
      layout: { vertexCount: 3, indexCount: 3, indexWidth: 'u32', attributes: [{ name: 'position', components: 3, kind: 'f32' }] },
      groups: [{ materialSlot: 0, start: 0, count: 3 }],
      bounds: { min: [0, 0, 0], max: [1, 1, 0] },
      source: {
        kind: 'inline',
        positions: [0, 0, 0, 1, 0, 0, 0, 1, 0],
        normals: [0, 0, 1, 0, 0, 1, 0, 0, 1],
        indices: [0, 1, 2],
      },
      provenance: 'staticAsset',
    },
    materialSlots: [{ slot: 0, material: 'material.copper' }],
    collision: { kind: 'aabbFallback' },
  },
  instanceHandle: 7001 as import('@asha/contracts').RenderHandle,
};

import {
  MANIFEST_OPERATIONS,
  RuntimeBridgeError,
  assertNativeRustRuntimeBridgeAuthority,
  createNativeRustRuntimeBridgeProvider,
  createNativeRuntimeBridge,
  frameCursor,
  installNativeRustRuntimeBridgeProvider,
  resolveNativeRustRuntimeBridgeProvider,
  type ModelMaterialPreviewRequest,
  type NativeRustRuntimeBridgeProvider,
  type NativeRustRuntimeBridgeProviderGlobalName,
  type RuntimeBufferHandle,
} from './index.js';
import { fnv1a64 } from './mock-primitives.js';
import {
  MockRuntimeBridge,
  createMockRuntimeBridge,
} from './reference.js';

function writeStaleNativeAddonModule(): string {
  const dir = mkdtempSync(join(tmpdir(), 'asha-runtime-bridge-'));
  const modulePath = join(dir, 'stale-native-addon.cjs');
  writeFileSync(
    modulePath,
    `module.exports = {
      initializeEngine() {},
      submitCommands() {},
      stepSimulation() {},
      applyEnemyDirectNavMovement() {},
      readFpsRuntimeSession() {},
      applyFpsPrimaryFire() {},
      restartFpsRuntimeSession() {},
      readRenderDiffs() {}
    };`,
  );
  return modulePath;
}

void test('facade exposes exactly the manifest operations (conformance)', () => {
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

// Launcher compatibility proofs were removed with the retired launcher surface.

void test('native Rust RuntimeBridge provider resolver fails closed without a provider', async () => {
  const resolution = await resolveNativeRustRuntimeBridgeProvider({ globalScope: {} });
  assert.equal(resolution.status, 'unavailable');
  assert.equal(resolution.bridge, null);
  assert.equal(resolution.diagnostics[0]?.code, 'missing_rust_runtime_backend');
  assert.equal(resolution.profile.productAuthority, true);
  assert.equal(resolution.profile.referenceFallback, false);
});

void test('native Rust RuntimeBridge provider resolver rejects spoofed reference metadata', async () => {
  const resolution = await resolveNativeRustRuntimeBridgeProvider({
    provider: {
      kind: 'asha.runtime_bridge.native_rust_provider.v1',
      backend: 'reference_bridge',
      productAuthority: true,
      referenceFallback: true,
      createRuntimeBridge: createMockRuntimeBridge,
    },
  });
  assert.equal(resolution.status, 'unavailable');
  assert.equal(resolution.diagnostics[0]?.code, 'invalid_rust_runtime_provider');
});

void test('native Rust RuntimeBridge provider resolver reports missing operations', async () => {
  const provider: NativeRustRuntimeBridgeProvider = {
    kind: 'asha.runtime_bridge.native_rust_provider.v1',
    backend: 'native_rust',
    productAuthority: true,
    referenceFallback: false,
    createRuntimeBridge: () => ({
      initializeEngine() {
        return 1;
      },
    } as unknown as ReturnType<typeof createMockRuntimeBridge>),
  };
  const resolution = await resolveNativeRustRuntimeBridgeProvider({ provider });
  assert.equal(resolution.status, 'unavailable');
  assert.equal(resolution.diagnostics[0]?.code, 'missing_runtime_bridge_operation');
  assert.match(resolution.diagnostics[0]?.message ?? '', /beginRuntimeProjectSourceResources/);
});

void test('native Rust RuntimeBridge provider resolver accepts public native provider shape', async () => {
  const bridge = createMockRuntimeBridge();
  const provider: NativeRustRuntimeBridgeProvider = {
    kind: 'asha.runtime_bridge.native_rust_provider.v1',
    backend: 'native_rust',
    productAuthority: true,
    referenceFallback: false,
    createRuntimeBridge: () => bridge,
  };
  const resolution = await resolveNativeRustRuntimeBridgeProvider({
    globalScope: { ashaRuntimeBridge: provider },
  });
  assert.equal(resolution.status, 'available');
  assert.equal(resolution.bridge, bridge);
  assert.equal(resolution.providerGlobal, 'globalThis.ashaRuntimeBridge');
  assert.equal(resolution.profile.providerContract, 'asha.runtime_bridge.native_rust_provider.v1');
});

void test('native Rust RuntimeBridge provider resolver rejects retired provider globals', async () => {
  const provider = createNativeRustRuntimeBridgeProvider({ bridge: createMockRuntimeBridge() });
  const retiredGlobalFromUntypedConsumer = 'ashaDemoRuntimeBridge' as unknown as
    NativeRustRuntimeBridgeProviderGlobalName;
  const resolution = await resolveNativeRustRuntimeBridgeProvider({
    globalScope: { ashaDemoRuntimeBridge: provider },
    providerGlobalNames: [retiredGlobalFromUntypedConsumer],
  });
  assert.equal(resolution.status, 'unavailable');
  assert.equal(resolution.diagnostics[0]?.code, 'missing_rust_runtime_backend');
});

void test('native Rust RuntimeBridge provider helper creates product-authority provider metadata', async () => {
  const bridge = createMockRuntimeBridge();
  const provider = createNativeRustRuntimeBridgeProvider({ bridge });
  assert.equal(provider.kind, 'asha.runtime_bridge.native_rust_provider.v1');
  assert.equal(provider.backend, 'native_rust');
  assert.equal(provider.productAuthority, true);
  assert.equal(provider.referenceFallback, false);

  const resolution = await resolveNativeRustRuntimeBridgeProvider({ provider });
  assert.equal(resolution.status, 'available');
  assert.equal(resolution.bridge, bridge);
});

void test('standalone host can install the native RuntimeBridge provider before app boot', async () => {
  const bridge = createMockRuntimeBridge();
  const globalScope: Record<string, NativeRustRuntimeBridgeProvider | null | undefined> = {};
  const installation = installNativeRustRuntimeBridgeProvider({
    globalScope,
    createRuntimeBridge: () => bridge,
  });
  assert.equal(installation.providerGlobal, 'globalThis.ashaRuntimeBridge');
  assert.equal(installation.profile.referenceFallback, false);
  assert.equal(globalScope['ashaRuntimeBridge'], installation.provider);

  const resolution = await resolveNativeRustRuntimeBridgeProvider({ globalScope });
  assert.equal(resolution.status, 'available');
  assert.equal(resolution.bridge, bridge);
});

void test('native RuntimeBridge provider helper rejects ambiguous host wiring', () => {
  assert.throws(
    () => createNativeRustRuntimeBridgeProvider({
      bridge: createMockRuntimeBridge(),
      createRuntimeBridge: createMockRuntimeBridge,
    }),
    (e: unknown) => e instanceof RuntimeBridgeError
      && e.kind === 'invalid_input'
      && e.message.includes('exactly one bridge or createRuntimeBridge'),
  );
});

void test('native Rust RuntimeBridge authority validator rejects reference-backed readouts', () => {
  assert.throws(
    () => assertNativeRustRuntimeBridgeAuthority({
      ecrpAuthority: { mode: 'rust', source: 'reference_bridge' },
      fpsSnapshot: { backend: 'reference_bridge' },
    }),
    (e: unknown) => e instanceof RuntimeBridgeError
      && e.kind === 'invalid_input'
      && e.message.includes('reference_bridge'),
  );
});

void test('manifest exposes public camera view operations', () => {
  const cameraOps = MANIFEST_OPERATIONS.filter((op) => op.facadeMethod.includes('Camera'));
  assert.deepEqual(
    cameraOps.map((op) => [op.manifestName, op.facadeMethod, op.surface]),
    [
      ['apply_collision_constrained_camera_input', 'applyCollisionConstrainedCameraInput', 'stable'],
      ['create_camera', 'createCamera', 'stable'],
      ['apply_camera_mode_command', 'applyCameraModeCommand', 'stable'],
      ['apply_camera_navigation_input', 'applyCameraNavigationInput', 'stable'],
      ['read_camera_controller_state', 'readCameraControllerState', 'stable'],
      ['apply_first_person_camera_input', 'applyFirstPersonCameraInput', 'stable'],
      ['read_camera_projection', 'readCameraProjection', 'stable'],
    ],
  );
});

void test('mock: init then step is deterministic', () => {
  const bridge = createMockRuntimeBridge();
  const handle = bridge.initializeEngine({ seed: 7 });
  assert.equal(handle as number, 7);
  assert.deepEqual(bridge.stepSimulation({ tick: 6 }), { tick: 6, diffCount: 0 });
});

void test('mock: camera view operations produce deterministic public evidence', () => {
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

void test('mock: selectVoxel derives camera ray and edit anchor from generated view contracts', () => {
  const bridge = createMockRuntimeBridge();
  bridge.initializeEngine({ seed: 1 });
  const camera = bridge.createCamera({
    initialPose: { position: [1.5, 1.5, 4], yawDegrees: 0, pitchDegrees: 0 },
    projection: { fovYDegrees: 60, near: 0.1, far: 1000 },
    viewport: { width: 1280, height: 720 },
  });

  const selection = bridge.selectVoxel({
    camera: camera.camera,
    grid: 1,
    viewport: null,
    screenPoint: { x: 0.5, y: 0.5, space: 'normalized_0_1' },
    maxDistance: 10,
  });

  assert.equal(selection.outcome, 'hit');
  assert.deepEqual(selection.pickRay.direction, [0, 0, -1]);
  assert.deepEqual(selection.selectedVoxel, { x: 1, y: 1, z: 0 });
  assert.equal(selection.selectedFace, 'posZ');
  assert.deepEqual(selection.editAnchor, { x: 1, y: 1, z: 1 });

  const miss = bridge.selectVoxel({
    camera: camera.camera,
    grid: 1,
    viewport: null,
    screenPoint: { x: 0.5, y: 0.5, space: 'normalized_0_1' },
    maxDistance: 1,
  });
  assert.equal(miss.outcome, 'miss');
  assert.equal(miss.selectedVoxel, null);
});

void test('mock: camera-first-person-basic matches committed golden fixture', () => {
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

void test('mock: step before init throws a classified error', () => {
  const bridge = createMockRuntimeBridge();
  assert.throws(
    () => bridge.stepSimulation({ tick: 1 }),
    (e: unknown) => e instanceof RuntimeBridgeError && e.kind === 'not_initialized',
  );
});

void test('mock: buffer round-trip and unknown handle classification', () => {
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


void test('mock: readModelMaterialPreview returns public render-diff evidence without renderer internals', () => {
  const bridge = createMockRuntimeBridge();
  bridge.initializeEngine({ seed: 1 });
  const snapshot = bridge.readModelMaterialPreview(MODEL_MATERIAL_PREVIEW_REQUEST);
  assert.equal(snapshot.catalogEntry.id, 'material.copper');
  assert.equal(snapshot.meshAsset.asset, 'mesh.preview-triangle');
  assert.equal(snapshot.rendererClassification, 'reference_preview');
  assert.deepEqual(snapshot.previewDiff.ops.map((op) => op.op), ['defineMaterial', 'defineStaticMesh', 'createStaticMeshInstance']);
  assert.ok(snapshot.diagnostics.some((diagnostic) => diagnostic.includes('fail closed')));
});

void test('mock: scene-object snapshot and apply command use typed public contracts', () => {
  const bridge = createMockRuntimeBridge();
  bridge.initializeEngine({ seed: 1 });

  const snapshot = bridge.readSceneObjectSnapshot();
  assert.equal(snapshot.objects[0]?.kind, 'emptyGroup');
  assert.ok(snapshot.objects.some((object) => object.hasRenderableAsset));

  const result = bridge.applySceneObjectCommand({
    expectedDocumentHash: snapshot.documentHash,
    command: { kind: 'rename', id: snapshot.objects[0].id, label: 'Renamed root' },
  });
  assert.equal(result.accepted, true);
  assert.equal(result.rejection, null);
  assert.equal(result.outcome?.selected, snapshot.objects[0].id);
  assert.equal(result.outcome?.snapshot.objects[0]?.label, 'Renamed root');

  const stale = bridge.applySceneObjectCommand({
    expectedDocumentHash: snapshot.documentHash,
    command: { kind: 'select', id: snapshot.objects[0].id },
  });
  assert.equal(stale.accepted, false);
  assert.equal(stale.rejection?.code, 'stale-scene-object-snapshot');
});

void test('mock: stored scene codec is explicitly Rust-authority-only', () => {
  const bridge = createMockRuntimeBridge();
  bridge.initializeEngine({ seed: 1 });
  assert.throws(
    () => bridge.decodeSceneDocument({ sourceText: '{}' }),
    (error: unknown) => error instanceof RuntimeBridgeError && error.kind === 'operation_unimplemented',
  );
  assert.throws(
    () => bridge.encodeSceneDocument({
      document: {
        schemaVersion: 1,
        id: 1 as import('@asha/contracts').SceneId,
        metadata: { name: null, authoringFormatVersion: 1 },
        dependencies: [],
        nodes: [],
      },
    }),
    (error: unknown) => error instanceof RuntimeBridgeError && error.kind === 'operation_unimplemented',
  );
});

void test('mock: readRenderDiffs returns a contract-shaped frame', () => {
  const bridge = createMockRuntimeBridge();
  bridge.initializeEngine({ seed: 1 });
  const frame = bridge.readRenderDiffs(frameCursor(0));
  assert.deepEqual(frame, { ops: [] });
});

void test('mock: readProjectionFrame preserves the G1 scene plus presentation envelope', () => {
  const bridge = createMockRuntimeBridge();
  bridge.initializeEngine({ seed: 1 });
  const frame = bridge.readProjectionFrame(frameCursor(4));
  assert.equal(frame.schemaVersion, 1);
  assert.equal(frame.authorityTick, 4);
  assert.deepEqual(frame.scene, { ops: [] });
  assert.deepEqual(frame.presentation, {
    replayScope: 'excludedFromReplayTruth',
    ops: [],
  });
});

// Legacy bundle lifecycle tests were deleted with the retired lifecycle.

void test('mock: submitCommands carries the generated VoxelCommand union (the launch path)', () => {
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

void test('mock: submitCommands before init fails closed', () => {
  const bridge = createMockRuntimeBridge();
  assert.throws(
    () => bridge.submitCommands({ commands: [] }),
    (e: unknown) => e instanceof RuntimeBridgeError && e.kind === 'not_initialized',
  );
});

void test('an ad-hoc `{ kind }` command is NOT the launch path (compile-time guard)', () => {
  const bridge = createMockRuntimeBridge();
  bridge.initializeEngine({ seed: 1 });
  // The placeholder command shape the launch path used to accept must no longer
  // type-check: `submitCommands` takes the generated VoxelCommand union only.
  // @ts-expect-error — `{ kind: 'smoke-edit' }` is not a VoxelCommand.
  const bad: CommandBatch = { commands: [{ kind: 'smoke-edit' }] };
  assert.equal(bad.commands.length, 1);
});

void test('mock: pickVoxel carries a PickRay and returns a classified PickResult', () => {
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

void test('mock: readVoxelMeshEvidence returns compact chunk evidence and fails closed', () => {
  const bridge = createMockRuntimeBridge();
  assert.throws(
    () => bridge.readVoxelMeshEvidence({ grid: 1, chunks: [] }),
    (e: unknown) => e instanceof RuntimeBridgeError && e.kind === 'not_initialized',
  );
  bridge.initializeEngine({ seed: 1 });
  const snapshot = bridge.readVoxelMeshEvidence({ grid: 1, chunks: [] });
  assert.equal(snapshot.fixtureId, 'basic-voxel-landscape-interaction');
  assert.equal(snapshot.meshingStrategy, 'visible-face');
  assert.equal(snapshot.chunks.length, 1);
  assert.equal(snapshot.chunks[0]?.meshHash, 'fnv1a64:mock-mesh');
});

void test('mock: voxel update telemetry is bounded to the latest projection cursor', () => {
  const bridge = createMockRuntimeBridge();
  bridge.initializeEngine({ seed: 1 });
  assert.throws(
    () => bridge.readVoxelUpdateTelemetry({ grid: 1, projectionCursor: 0 }),
    (error: unknown) => error instanceof RuntimeBridgeError && error.kind === 'not_initialized',
  );
  bridge.submitCommands({
    commands: [{
      op: 'setVoxel',
      grid: 1,
      coord: { x: 0, y: 0, z: 0 },
      value: { kind: 'solid', material: 1 },
    }],
  });
  bridge.readRenderDiffs(frameCursor(4));
  const readout = bridge.readVoxelUpdateTelemetry({ grid: 1, projectionCursor: 4 });
  assert.equal(readout.compatibilityVersion, 'voxel-update-telemetry.v0');
  assert.equal(readout.committedCommandBatchCount, 1);
  assert.equal(readout.acceptedCommandCount, 1);
  assert.equal(readout.touchedVoxelCount, 1);
  assert.equal(readout.pendingDirtyChunkCount, 0);
  assert.deepEqual(bridge.readRenderDiffs(frameCursor(4)), { ops: [] });
  assert.deepEqual(
    bridge.readVoxelUpdateTelemetry({ grid: 1, projectionCursor: 4 }),
    readout,
    'an exact duplicate drain with no pending work is idempotent',
  );
  bridge.submitCommands({
    commands: [{
      op: 'setVoxel',
      grid: 1,
      coord: { x: 1, y: 0, z: 0 },
      value: { kind: 'solid', material: 1 },
    }],
  });
  assert.throws(
    () => bridge.readRenderDiffs(frameCursor(4)),
    (error: unknown) => error instanceof RuntimeBridgeError && error.kind === 'invalid_input',
  );
  bridge.readRenderDiffs(frameCursor(5));
  const second = bridge.readVoxelUpdateTelemetry({ grid: 1, projectionCursor: 5 });
  assert.equal(second.committedCommandBatchCount, 1);
  assert.equal(second.acceptedCommandCount, 1);
  assert.throws(
    () => bridge.readVoxelUpdateTelemetry({ grid: 1, projectionCursor: 4 }),
    (error: unknown) => error instanceof RuntimeBridgeError && error.kind === 'invalid_input',
  );
  assert.throws(
    () => bridge.readVoxelUpdateTelemetry({ grid: 1, projectionCursor: 6 }),
    (error: unknown) => error instanceof RuntimeBridgeError && error.kind === 'invalid_input',
  );
  assert.throws(
    () => bridge.readVoxelUpdateTelemetry({ grid: 2, projectionCursor: 5 }),
    (error: unknown) => error instanceof RuntimeBridgeError && error.kind === 'invalid_input',
  );
});

void test('mock: pickVoxel before init fails closed', () => {
  const bridge = createMockRuntimeBridge();
  assert.throws(
    () => bridge.pickVoxel({ grid: 1, origin: [0, 0, 0], direction: [1, 0, 0], maxDistance: 10 }),
    (e: unknown) => e instanceof RuntimeBridgeError && e.kind === 'not_initialized',
  );
});

void test('native factory classifies a missing addon path', () => {
  assert.throws(
    () => createNativeRuntimeBridge('./definitely-not-built.node'),
    (e: unknown) => e instanceof RuntimeBridgeError && e.kind === 'native_unavailable',
  );
});

void test('native factory rejects stale addons missing encounter authority exports', () => {
  const modulePath = writeStaleNativeAddonModule();
  try {
    assert.throws(
      () => createNativeRuntimeBridge(modulePath),
      (e: unknown) =>
        e instanceof RuntimeBridgeError &&
        e.kind === 'native_unavailable' &&
        e.message.includes('readFpsEncounterDirector') &&
        e.message.includes('applyFpsEncounterTransition'),
    );
  } finally {
    rmSync(dirname(modulePath), { recursive: true, force: true });
  }
});

void test('native project source staging keeps binary bytes out of JSON and consumes handles once', (t) => {
  let bridge;
  try {
    bridge = createNativeRuntimeBridge();
  } catch (error) {
    if (error instanceof RuntimeBridgeError && error.kind === 'native_unavailable') {
      t.skip('native addon not built (run harness/ci/check-native.sh)');
      return;
    }
    throw error;
  }

  bridge.initializeEngine({ seed: 19 });
  const lockBytes = new TextEncoder().encode('asset-lock');
  const sceneBytes = new TextEncoder().encode('entry-scene');
  const voxelBytes = new TextEncoder().encode('voxel-house');
  const manifestJson = JSON.stringify({
    bundleSchemaVersion: 2,
    protocolVersion: 1,
    project: { id: 7, name: 'native-source-batch' },
    entryScene: 10,
    scenes: [{ id: 10, schemaVersion: 1, artifact: 'scene/entry.json' }],
    assetLock: { artifact: 'assets/lock.json', assetCount: 0 },
    generationProvenance: null,
    artifacts: [
      {
        path: 'assets/lock.json',
        class: 'durable',
        role: 'assetLock',
        contentHash: fnv1a64('asset-lock'),
      },
      {
        path: 'scene/entry.json',
        class: 'durable',
        role: 'sceneDocument',
        contentHash: fnv1a64('entry-scene'),
      },
      {
        path: 'voxel/house.avox',
        class: 'durable',
        role: 'voxelVolumeAsset',
        contentHash: fnv1a64('voxel-house'),
      },
    ],
  });
  const transaction = bridge.beginRuntimeProjectSourceResources({ manifestJson });
  const resource = bridge.stageRuntimeProjectSourceResource({
    generation: transaction.generation,
    path: 'voxel/house.avox',
    bytes: voxelBytes,
  });
  const batch = {
    manifestJson,
    resourceGeneration: transaction.generation,
    bodies: [
      { kind: 'inline' as const, path: 'assets/lock.json', bytes: [...lockBytes] },
      { kind: 'inline' as const, path: 'scene/entry.json', bytes: [...sceneBytes] },
      { kind: 'resource' as const, path: 'voxel/house.avox', resource },
    ],
  };

  const accepted = bridge.admitRuntimeProjectSourceBatch(batch);
  assert.equal(accepted.accepted, true);
  assert.deepEqual(accepted.paths, [
    'assets/lock.json',
    'scene/entry.json',
    'voxel/house.avox',
  ]);
  assert.equal(accepted.manifestHash, transaction.manifestHash);

  const replayed = bridge.admitRuntimeProjectSourceBatch(batch);
  assert.equal(replayed.accepted, false);
  assert.equal(replayed.diagnostics[0]?.code, 'unknownResourceHandle');
});
