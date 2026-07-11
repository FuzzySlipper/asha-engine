import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { VoxelCommand } from '@asha/contracts';
import type { RuntimeSessionFacade } from '@asha/runtime-session';

import {
  RuntimeBridgeError,
  createNativeRuntimeBridge,
  createRuntimeSessionFacade,
} from './index.js';

function createNativeSession(): RuntimeSessionFacade {
  const session = createRuntimeSessionFacade({ bridge: createNativeRuntimeBridge(), mode: 'rust' });
  session.initialize({
    sessionId: 'runtime-session.multi-chunk-authoring.test',
    seed: 17,
    project: { gameId: 'asha-test', workspaceId: 'workspace.local' },
    projectBundle: { bundleSchemaVersion: 1, protocolVersion: 1, sceneId: 42 },
  });
  return session;
}

function applyQuadConversion(session: RuntimeSessionFacade): void {
  const registration = session.registerVoxelConversionMeshAsset({
    source: {
      assetId: 'mesh/native-multi-chunk-quad',
      assetKind: 'mesh',
      assetVersion: 1,
      sourceHash: 'sha256:native-multi-chunk-quad',
      meshPrimitive: 'default',
    },
    meshAsset: {
      assetId: 'mesh/native-multi-chunk-quad',
      sourcePath: 'assets/meshes/native-multi-chunk-quad.mesh.json',
      positions: [[0, 0, 0], [1, 0, 0], [1, 1, 0], [0, 1, 0]],
      normals: [],
      indices: [0, 1, 2, 0, 2, 3],
      groups: [{ materialSlot: 0, start: 0, count: 6 }],
      materialSlots: [{ sourceMaterialSlot: 0, sourceMaterialId: 'material/surface-a' }],
    },
  });
  assert.equal(registration.registered, true);
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
}

function complexShapeCommands(): VoxelCommand[] {
  const commands: VoxelCommand[] = [];
  for (let y = 0; y < 4; y += 1) {
    for (let z = 0; z < 4; z += 1) {
      for (let x = 4; x < 8; x += 1) {
        const shell = x === 4 || x === 7 || y === 0 || y === 3 || z === 0 || z === 3;
        if (shell) {
          commands.push({
            op: 'setVoxel',
            grid: 2,
            coord: { x, y, z },
            value: { kind: 'solid', material: 1 },
          });
        }
      }
    }
  }
  for (let y = 4; y < 8; y += 1) {
    commands.push({
      op: 'setVoxel',
      grid: 2,
      coord: { x: 4, y, z: 0 },
      value: { kind: 'solid', material: 1 },
    });
  }
  commands.push({
    op: 'setVoxel',
    grid: 2,
    coord: { x: 5, y: 7, z: 0 },
    value: { kind: 'solid', material: 1 },
  });
  assert.equal(commands.length, 61);
  return commands;
}

void test('native compact-equivalent edits grow adjacent chunks and survive save reload', (t) => {
  let session: RuntimeSessionFacade;
  try {
    session = createNativeSession();
  } catch (error) {
    if (error instanceof RuntimeBridgeError && error.kind === 'native_unavailable') {
      t.skip('native addon not built (run harness/ci/check-native.sh)');
      return;
    }
    throw error;
  }
  applyQuadConversion(session);

  const before = session.readVoxelModelInfo({
    grid: 2,
    volumeAssetId: 'voxel/generated',
    includeMaterialCounts: true,
  });
  const skippedChunk = session.submitCommands({
    commands: [{
      op: 'setVoxel',
      grid: 2,
      coord: { x: 100, y: 0, z: 0 },
      value: { kind: 'solid', material: 1 },
    }],
  });
  assert.equal(skippedChunk.result.accepted, 0);
  assert.equal(skippedChunk.result.rejected, 1);
  assert.equal(skippedChunk.result.rejections[0]?.reason, 'chunkNotResident');
  assert.equal(session.readVoxelModelInfo({
    grid: 2,
    volumeAssetId: 'voxel/generated',
    includeMaterialCounts: true,
  }).sessionHash, before.sessionHash);

  const edit = session.submitCommands({ commands: complexShapeCommands() });
  assert.equal(edit.result.accepted, 61);
  assert.equal(edit.result.rejected, 0, JSON.stringify(edit.result.rejections));
  const authored = session.readVoxelModelInfo({
    grid: 2,
    volumeAssetId: 'voxel/generated',
    includeMaterialCounts: true,
  });
  assert.equal(authored.voxelCount, before.voxelCount + 61);
  assert.deepEqual(authored.bounds?.max, { x: 7, y: 7, z: 3 });
  assert.deepEqual(authored.materialCounts, [{ material: 1, voxelCount: authored.voxelCount }]);

  const staleExport = session.exportVoxelVolumeAsset({
    grid: 2,
    volumeAssetId: 'voxel/generated',
    targetAssetId: 'voxel-volume/native-multi-chunk-stale',
    label: 'Stale native multi-chunk shape',
    createdBy: '@asha/runtime-bridge',
    sourceTool: '@asha/runtime-bridge',
    maxSparseRuns: 128,
    expectedSessionHash: before.sessionHash,
  });
  assert.equal(staleExport.exported, false);
  assert.equal(staleExport.diagnostics[0]?.code, 'stale_runtime_snapshot');
  const limitedExport = session.exportVoxelVolumeAsset({
    grid: 2,
    volumeAssetId: 'voxel/generated',
    targetAssetId: 'voxel-volume/native-multi-chunk-limited',
    label: 'Limited native multi-chunk shape',
    createdBy: '@asha/runtime-bridge',
    sourceTool: '@asha/runtime-bridge',
    maxSparseRuns: 1,
    expectedSessionHash: authored.sessionHash,
  });
  assert.equal(limitedExport.exported, false);
  assert.equal(limitedExport.diagnostics[0]?.code, 'export_limit_exceeded');
  const exported = session.exportVoxelVolumeAsset({
    grid: 2,
    volumeAssetId: 'voxel/generated',
    targetAssetId: 'voxel-volume/native-multi-chunk-export',
    label: 'Exported native multi-chunk shape',
    createdBy: '@asha/runtime-bridge',
    sourceTool: '@asha/runtime-bridge',
    maxSparseRuns: 128,
    expectedSessionHash: authored.sessionHash,
  });
  assert.equal(exported.exported, true, JSON.stringify(exported.diagnostics));
  assert.deepEqual(
    [...new Set(exported.asset?.provenance.map((entry) => entry.kind))],
    ['converted', 'authored', 'runtime_export'],
  );

  const saved = session.saveVoxelVolumeAsset({
    exportRequest: {
      grid: 2,
      volumeAssetId: 'voxel/generated',
      targetAssetId: 'voxel-volume/native-multi-chunk-shape',
      label: 'Native multi-chunk shape',
      createdBy: '@asha/runtime-bridge',
      sourceTool: '@asha/runtime-bridge',
      maxSparseRuns: 128,
      expectedSessionHash: authored.sessionHash,
    },
    targetProjectBundle: 'asha-testing',
    targetAssetPath: 'assets/voxels/native-multi-chunk-shape.avxl.json',
    representationKind: 'sparse_runs',
    expectedExistingCanonicalJsonHash: null,
    expectedCanonicalJsonHash: null,
    expectedVoxelDataHash: null,
  });
  assert.equal(saved.saved, true, JSON.stringify(saved.diagnostics));
  assert.deepEqual(
    [...new Set(saved.asset?.provenance.map((entry) => entry.kind))],
    ['converted', 'authored', 'runtime_export'],
  );
  assert.equal(saved.asset?.representation.sparseRuns.reduce(
    (count, run) => count + run.length,
    0,
  ), authored.voxelCount);

  const unloaded = session.unloadVoxelVolumeAsset({
    grid: 2,
    volumeAssetId: 'voxel/generated',
    expectedSessionHash: authored.sessionHash,
  });
  assert.equal(unloaded.unloaded, true, JSON.stringify(unloaded.diagnostics));
  const reloaded = session.loadVoxelVolumeAsset({
    asset: saved.asset!,
    targetGrid: 2,
    targetVolumeAssetId: 'voxel/generated',
    replaceExisting: false,
    includeMaterialCounts: true,
  });
  assert.equal(reloaded.loaded, true, JSON.stringify(reloaded.diagnostics));
  assert.equal(reloaded.voxelCount, authored.voxelCount);
  assert.deepEqual(reloaded.bounds, authored.bounds);
  assert.deepEqual(reloaded.materialCounts, authored.materialCounts);
  assert.equal(reloaded.voxelDataHash, saved.voxelDataHash);
});
