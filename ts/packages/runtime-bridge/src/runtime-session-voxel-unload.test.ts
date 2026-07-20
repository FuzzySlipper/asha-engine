import { test } from 'node:test';
import assert from 'node:assert/strict';

import type {
  VoxelVolumeAssetUnloadReceipt,
  VoxelVolumeAssetUnloadRequest,
} from '@asha/contracts';

import {
  RuntimeBridgeError,
  createNativeRuntimeBridge,
  createRuntimeSessionFacade,
} from './index.js';
import { MockRuntimeBridge } from './mock.js';
import type { RuntimeSessionFacade } from '@asha/runtime-session';

const UNLOAD_REQUEST = {
  grid: 7,
  volumeAssetId: 'voxel/converted-room',
  expectedSessionHash: 'fnv1a64:session-before-unload',
} satisfies VoxelVolumeAssetUnloadRequest;

function sessionInput() {
  return {
    sessionId: 'runtime-session.voxel-unload.test',
    seed: 17,
    project: {
      gameId: 'asha-test',
      workspaceId: 'workspace.local',
    },
  };
}

class CapturingVoxelUnloadBridge extends MockRuntimeBridge {
  request: VoxelVolumeAssetUnloadRequest | null = null;

  override unloadVoxelVolumeAsset(
    request: VoxelVolumeAssetUnloadRequest,
  ): VoxelVolumeAssetUnloadReceipt {
    this.request = request;
    return {
      request,
      unloaded: true,
      modelId: 'voxel-model:grid:7:volume:voxel/converted-room',
      volumeAssetId: request.volumeAssetId,
      grid: request.grid,
      removedVoxelCount: 128,
      sessionHash: 'fnv1a64:session-after-unload',
      replayHash: 'fnv1a64:replay-after-unload',
      diagnostics: [],
    };
  }
}

void test('Rust-backed RuntimeSession forwards voxel volume unload and returns its authority receipt', () => {
  const bridge = new CapturingVoxelUnloadBridge();
  const session = createRuntimeSessionFacade({ bridge, mode: 'rust' });
  session.initialize(sessionInput());

  const receipt = session.unloadVoxelVolumeAsset(UNLOAD_REQUEST);

  assert.deepEqual(bridge.request, UNLOAD_REQUEST);
  assert.equal(receipt.unloaded, true);
  assert.equal(receipt.removedVoxelCount, 128);
  assert.equal(receipt.request.expectedSessionHash, UNLOAD_REQUEST.expectedSessionHash);
  assert.equal(receipt.sessionHash, 'fnv1a64:session-after-unload');
});

void test('reference RuntimeSession fails closed for voxel volume unload', () => {
  const session = createRuntimeSessionFacade({
    bridge: new MockRuntimeBridge(),
    mode: 'reference',
  });
  session.initialize(sessionInput());

  assert.throws(
    () => session.unloadVoxelVolumeAsset(UNLOAD_REQUEST),
    (error: unknown) =>
      error instanceof RuntimeBridgeError && error.kind === 'operation_unimplemented',
  );
});

void test('native conversion save unload and reload preserves a prior resident model', (t) => {
  let session: RuntimeSessionFacade;
  try {
    session = createRuntimeSessionFacade({ bridge: createNativeRuntimeBridge(), mode: 'rust' });
  } catch (error) {
    if (error instanceof RuntimeBridgeError && error.kind === 'native_unavailable') {
      t.skip('native addon not built (run harness/ci/check-native.sh)');
      return;
    }
    throw error;
  }
  session.initialize(sessionInput());

  const registration = session.registerVoxelConversionMeshAsset({
    source: {
      assetId: 'mesh/native-unload-quad',
      assetKind: 'mesh',
      assetVersion: 1,
      sourceHash: 'sha256:native-unload-quad',
      meshPrimitive: 'default',
    },
    meshAsset: {
      assetId: 'mesh/native-unload-quad',
      sourcePath: 'assets/meshes/native-unload-quad.mesh.json',
      positions: [[0, 0, 0], [1, 0, 0], [1, 1, 0], [0, 1, 0]],
      normals: [],
      indices: [0, 1, 2, 0, 2, 3],
      groups: [{ materialSlot: 0, start: 0, count: 6 }],
      materialSlots: [{ sourceMaterialSlot: 0, sourceMaterialId: 'material/surface-a' }],
    },
  });
  assert.equal(registration.registered, true);

  const applyConversion = () => {
    const plan = session.planVoxelConversion({
      source: registration.source,
      target: {
        grid: 2,
        volumeAssetId: 'voxel/generated',
        origin: { x: 0, y: 0, z: 0 },
      },
      settings: {
        mode: 'surface',
        fitPolicy: 'contain',
        originPolicy: 'target_min',
        resolution: [4, 4, 1],
        voxelSize: 1,
        maxOutputVoxels: 16,
        transform: [1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1],
        materialMap: {
          entries: [{
            sourceMaterialSlot: 0,
            sourceMaterialId: 'material/surface-a',
            voxelMaterial: 1,
          }],
          textureAssets: [],
          textureBindings: [],
          defaultVoxelMaterial: null,
        },
      },
    });
    assert.equal(plan.diagnostics.length, 0);
    const preview = session.previewVoxelConversion({
      planId: plan.planId,
      expectedPlanHash: plan.planHash,
    });
    assert.equal(preview.diagnostics.length, 0);
    const receipt = session.applyVoxelConversion({
      planId: plan.planId,
      expectedPlanHash: plan.planHash,
      expectedPreviewHash: preview.outputHash,
    });
    assert.equal(receipt.applied, true, JSON.stringify(receipt.diagnostics));
  };

  applyConversion();
  const firstInfo = session.readVoxelModelInfo({
    grid: 2,
    volumeAssetId: 'voxel/generated',
    includeMaterialCounts: true,
  });
  const firstExport = session.exportVoxelVolumeAsset({
    grid: 2,
    volumeAssetId: 'voxel/generated',
    targetAssetId: 'voxel-volume/native-unload-predecessor',
    label: 'Native unload predecessor',
    createdBy: '@asha/runtime-bridge',
    sourceTool: '@asha/runtime-bridge',
    maxSparseRuns: 16,
    expectedSessionHash: firstInfo.sessionHash,
  });
  assert.equal(firstExport.exported, true, JSON.stringify(firstExport.diagnostics));
  const firstUnload = session.unloadVoxelVolumeAsset({
    grid: 2,
    volumeAssetId: 'voxel/generated',
    expectedSessionHash: firstInfo.sessionHash,
  });
  assert.equal(firstUnload.unloaded, true, JSON.stringify(firstUnload.diagnostics));
  const predecessor = session.loadVoxelVolumeAsset({
    asset: firstExport.asset!,
    targetGrid: 2,
    targetVolumeAssetId: 'voxel/predecessor',
    replaceExisting: false,
    includeMaterialCounts: true,
  });
  assert.equal(predecessor.loaded, true, JSON.stringify(predecessor.diagnostics));

  applyConversion();
  const currentInfo = session.readVoxelModelInfo({
    grid: 2,
    volumeAssetId: 'voxel/generated',
    includeMaterialCounts: true,
  });
  const saved = session.saveVoxelVolumeAsset({
    exportRequest: {
      grid: 2,
      volumeAssetId: 'voxel/generated',
      targetAssetId: 'voxel-volume/native-unload-current',
      label: 'Native unload current',
      createdBy: '@asha/runtime-bridge',
      sourceTool: '@asha/runtime-bridge',
      maxSparseRuns: 16,
      expectedSessionHash: currentInfo.sessionHash,
    },
    targetProjectBundle: 'asha-testing',
    targetAssetPath: 'assets/voxels/native-unload-current.avxl.json',
    representationKind: 'sparse_runs',
    expectedExistingCanonicalJsonHash: null,
    expectedCanonicalJsonHash: null,
    expectedVoxelDataHash: null,
  });
  assert.equal(saved.saved, true, JSON.stringify(saved.diagnostics));

  const lowerUnload = session.unloadVoxelVolumeAsset({
    grid: 2,
    volumeAssetId: 'voxel/predecessor',
    expectedSessionHash: predecessor.sessionHash,
  });
  assert.equal(lowerUnload.unloaded, false);
  assert.equal(lowerUnload.diagnostics[0]?.code, 'stale_runtime_snapshot');

  const unloaded = session.unloadVoxelVolumeAsset({
    grid: 2,
    volumeAssetId: 'voxel/generated',
    expectedSessionHash: currentInfo.sessionHash,
  });
  assert.equal(unloaded.unloaded, true, JSON.stringify(unloaded.diagnostics));
  assert.equal(unloaded.removedVoxelCount, currentInfo.voxelCount);
  const absent = session.readVoxelModelInfo({
    grid: 2,
    volumeAssetId: 'voxel/generated',
    includeMaterialCounts: true,
  });
  assert.equal(absent.resident, false);

  const reloaded = session.loadVoxelVolumeAsset({
    asset: saved.asset!,
    targetGrid: 2,
    targetVolumeAssetId: 'voxel/generated',
    replaceExisting: false,
    includeMaterialCounts: true,
  });
  assert.equal(reloaded.loaded, true, JSON.stringify(reloaded.diagnostics));
  assert.equal(reloaded.voxelCount, currentInfo.voxelCount);
  assert.deepEqual(reloaded.bounds, saved.asset?.bounds);
  assert.equal(reloaded.voxelDataHash, saved.voxelDataHash);
});
