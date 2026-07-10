import { execFileSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { test } from 'node:test';
import assert from 'node:assert/strict';

import type {
  VoxelVolumeAsset,
  VoxelVolumeAssetLoadRequest,
} from '@asha/contracts';
import {
  RuntimeBridgeError,
  createNativeRuntimeBridge,
  createRuntimeSessionFacade,
} from '@asha/runtime-bridge';
import type { RuntimeSessionFacade } from '@asha/runtime-session';

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), '../../../..');
const proofPath = resolve(repoRoot, 'harness/smoke-out/persisted-voxel-asset-consumer-proof.json');

function gitValue(args: readonly string[]): string {
  return execFileSync('git', [...args], { cwd: repoRoot, encoding: 'utf8' }).trim();
}

function cloneAsset(asset: VoxelVolumeAsset): VoxelVolumeAsset {
  return JSON.parse(JSON.stringify(asset)) as VoxelVolumeAsset;
}

function bootNativeBridge(t: { skip: (reason?: string) => void }): RuntimeSessionFacade | null {
  try {
    const bridge = createNativeRuntimeBridge();
    const session = createRuntimeSessionFacade({ bridge, mode: 'rust' });
    session.initialize({
      sessionId: 'runtime-session.persisted-voxel.consumer-proof',
      seed: 4911,
      project: { gameId: 'asha-persisted-voxel-proof', workspaceId: 'workspace.local' },
      projectBundle: { bundleSchemaVersion: 1, protocolVersion: 1, sceneId: 4911 },
    });
    return session;
  } catch (error) {
    if (error instanceof RuntimeBridgeError && error.kind === 'native_unavailable') {
      t.skip('native addon not built; run harness/ci/check-native.sh for the persisted voxel proof');
      return null;
    }
    throw error;
  }
}

function loadRequest(asset: VoxelVolumeAsset): VoxelVolumeAssetLoadRequest {
  return {
    asset,
    targetGrid: 7,
    targetVolumeAssetId: 'voxel/generated',
    replaceExisting: true,
    includeMaterialCounts: true,
  };
}

void test('persisted voxel asset public consumer proof saves, reloads, and records evidence', (t) => {
  const bridge = bootNativeBridge(t);
  if (bridge === null) return;

  const registration = bridge.registerVoxelConversionMeshAsset({
    source: {
      assetId: 'mesh/persisted-consumer-proof-quad',
      assetKind: 'mesh',
      assetVersion: 1,
      sourceHash: 'sha256:persisted-consumer-proof-quad',
      meshPrimitive: 'default',
    },
    meshAsset: {
      assetId: 'mesh/persisted-consumer-proof-quad',
      sourcePath: 'assets/meshes/persisted-consumer-proof-quad.mesh.json',
      positions: [[0, 0, 0], [1, 0, 0], [1, 1, 0], [0, 1, 0]],
      normals: [],
      indices: [0, 1, 2, 0, 2, 3],
      groups: [{ materialSlot: 0, start: 0, count: 6 }],
      materialSlots: [{ sourceMaterialSlot: 0, sourceMaterialId: 'material/surface-a' }],
    },
  });
  assert.equal(registration.registered, true);

  const plan = bridge.planVoxelConversion({
    source: registration.source,
    target: {
      grid: 7,
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
        entries: [{ sourceMaterialSlot: 0, sourceMaterialId: 'material/surface-a', voxelMaterial: 3 }],
        textureAssets: [],
        textureBindings: [],
        defaultVoxelMaterial: null,
      },
    },
  });
  assert.equal(plan.diagnostics.length, 0);

  const preview = bridge.previewVoxelConversion({
    planId: plan.planId,
    expectedPlanHash: plan.planHash,
  });
  assert.equal(preview.diagnostics.length, 0);

  const receipt = bridge.applyVoxelConversion({
    planId: plan.planId,
    expectedPlanHash: plan.planHash,
    expectedPreviewHash: preview.outputHash,
  });
  assert.equal(receipt.applied, true);

  const modelInfo = bridge.readVoxelModelInfo({
    grid: 7,
    volumeAssetId: 'voxel/generated',
    includeMaterialCounts: true,
  });
  assert.equal(modelInfo.resident, true);

  const exported = bridge.exportVoxelVolumeAsset({
    grid: 7,
    volumeAssetId: 'voxel/generated',
    targetAssetId: 'voxel-volume/persisted-consumer-proof',
    label: 'Persisted consumer proof',
    createdBy: '@asha/smoke',
    sourceTool: '@asha/runtime-bridge',
    maxSparseRuns: 16,
    expectedSessionHash: modelInfo.sessionHash,
  });
  assert.equal(exported.exported, true, JSON.stringify(exported.diagnostics));
  assert.ok(exported.asset !== null);
  assert.ok(exported.canonicalJson !== null);

  const saved = bridge.saveVoxelVolumeAsset({
    exportRequest: exported.request,
    targetProjectBundle: 'asha-testing-consumer-proof',
    targetAssetPath: 'assets/voxels/persisted-consumer-proof.avxl.json',
    representationKind: 'sparse_runs',
    expectedExistingCanonicalJsonHash: null,
    expectedCanonicalJsonHash: exported.canonicalJsonHash,
    expectedVoxelDataHash: exported.voxelDataHash,
  });
  assert.equal(saved.saved, true);
  assert.equal(saved.diff?.assetPath, 'assets/voxels/persisted-consumer-proof.avxl.json');
  assert.equal(saved.canonicalJsonHash, exported.canonicalJsonHash);
  assert.equal(saved.voxelDataHash, exported.voxelDataHash);

  const editedPalette = saved.asset!.materialPalette.map((binding) => ({
    ...binding,
    paletteEntryId: `voxel-material/authored-${binding.voxelMaterial}`,
    displayName: `Authored material ${binding.voxelMaterial}`,
    materialAssetId: `material/authored-${binding.voxelMaterial}`,
    materialCatalogBindingId: `catalog-binding/authored-${binding.voxelMaterial}`,
  }));
  const paletteUpdate = bridge.updateVoxelVolumeAssetPalette({
    asset: saved.asset!,
    materialPalette: editedPalette,
    targetProjectBundle: 'asha-testing-consumer-proof',
    targetAssetPath: 'assets/voxels/persisted-consumer-proof.avxl.json',
    expectedCanonicalJsonHash: saved.asset!.contentHashes.canonicalJson,
    expectedVoxelDataHash: saved.asset!.contentHashes.voxelData,
    maxMaterialBindings: 16,
  });
  assert.equal(paletteUpdate.updated, true, JSON.stringify(paletteUpdate.diagnostics));
  assert.ok(paletteUpdate.asset !== null);
  assert.equal(paletteUpdate.asset.materialPalette[0]?.displayName, 'Authored material 3');
  assert.equal(paletteUpdate.asset.materialPalette[0]?.paletteEntryId, 'voxel-material/authored-3');
  assert.equal(paletteUpdate.asset.materialPalette[0]?.materialAssetId, 'material/authored-3');
  assert.equal(
    paletteUpdate.asset.materialPalette[0]?.materialCatalogBindingId,
    'catalog-binding/authored-3',
  );
  assert.equal(paletteUpdate.voxelDataHash, saved.voxelDataHash);
  assert.notEqual(paletteUpdate.canonicalJsonHash, saved.canonicalJsonHash);

  const reloaded = bridge.loadVoxelVolumeAsset(loadRequest(paletteUpdate.asset));
  assert.equal(reloaded.loaded, true);
  assert.equal(reloaded.voxelCount, modelInfo.voxelCount);
  assert.deepEqual(reloaded.materialCounts, modelInfo.materialCounts);

  const readback = bridge.readVoxelModelInfo({
    grid: 7,
    volumeAssetId: 'voxel/generated',
    includeMaterialCounts: true,
  });
  assert.equal(readback.resident, true);
  assert.equal(readback.source?.assetId, 'voxel-volume/persisted-consumer-proof');
  assert.equal(readback.latestOutputHash, saved.voxelDataHash);

  const badContentHash: VoxelVolumeAsset = {
    ...cloneAsset(paletteUpdate.asset),
    contentHashes: {
      ...paletteUpdate.asset.contentHashes,
      canonicalJson: 'fnv1a64:0000000000000000',
    },
  };
  const badCoordinateSystem: VoxelVolumeAsset = {
    ...cloneAsset(paletteUpdate.asset),
    grid: {
      ...paletteUpdate.asset.grid,
      coordinateSystem: 'left_handed_test',
    },
  };
  const invalidMaterialRef: VoxelVolumeAsset = {
    ...cloneAsset(paletteUpdate.asset),
    materialPalette: [{
      voxelMaterial: 3,
      paletteEntryId: 'voxel-material/not-material',
      displayName: 'Not material',
      materialAssetId: 'texture/not-material',
      materialCatalogBindingId: 'catalog-binding/not-material',
    }],
  };
  const unsupportedSchema: VoxelVolumeAsset = {
    ...cloneAsset(paletteUpdate.asset),
    schemaVersion: 999,
  };

  const negativeMatrix = [
    {
      caseId: 'bad_content_hash',
      receipt: bridge.loadVoxelVolumeAsset(loadRequest(badContentHash)),
      expectedCode: 'content_hash_mismatch',
    },
    {
      caseId: 'bad_coordinate_system',
      receipt: bridge.loadVoxelVolumeAsset(loadRequest(badCoordinateSystem)),
      expectedCode: 'invalid_grid',
    },
    {
      caseId: 'invalid_material_ref',
      receipt: bridge.loadVoxelVolumeAsset(loadRequest(invalidMaterialRef)),
      expectedCode: 'invalid_material_reference',
    },
    {
      caseId: 'unsupported_schema',
      receipt: bridge.loadVoxelVolumeAsset(loadRequest(unsupportedSchema)),
      expectedCode: 'unsupported_schema_version',
    },
  ];
  for (const item of negativeMatrix) {
    assert.equal(item.receipt.loaded, false, item.caseId);
    assert.equal(item.receipt.diagnostics[0]?.code, item.expectedCode, item.caseId);
  }

  const staleSave = bridge.saveVoxelVolumeAsset({
    exportRequest: {
      ...exported.request,
      expectedSessionHash: 'fnv1a64:0000000000000000',
      targetAssetId: 'voxel-volume/stale-persisted-consumer-proof',
    },
    targetProjectBundle: 'asha-testing-consumer-proof',
    targetAssetPath: 'assets/voxels/stale-persisted-consumer-proof.avxl.json',
    representationKind: 'sparse_runs',
    expectedExistingCanonicalJsonHash: null,
    expectedCanonicalJsonHash: null,
    expectedVoxelDataHash: null,
  });
  assert.equal(staleSave.saved, false);
  assert.equal(staleSave.diagnostics[0]?.code, 'stale_runtime_snapshot');

  const stalePaletteUpdate = bridge.updateVoxelVolumeAssetPalette({
    asset: paletteUpdate.asset,
    materialPalette: paletteUpdate.asset.materialPalette,
    targetProjectBundle: 'asha-testing-consumer-proof',
    targetAssetPath: 'assets/voxels/persisted-consumer-proof.avxl.json',
    expectedCanonicalJsonHash: 'fnv1a64:0000000000000000',
    expectedVoxelDataHash: paletteUpdate.asset.contentHashes.voxelData,
    maxMaterialBindings: 16,
  });
  assert.equal(stalePaletteUpdate.updated, false);
  assert.equal(stalePaletteUpdate.diagnostics[0]?.code, 'content_hash_mismatch');

  const duplicatePaletteUpdate = bridge.updateVoxelVolumeAssetPalette({
    asset: paletteUpdate.asset,
    materialPalette: [
      ...paletteUpdate.asset.materialPalette,
      paletteUpdate.asset.materialPalette[0]!,
    ],
    targetProjectBundle: 'asha-testing-consumer-proof',
    targetAssetPath: 'assets/voxels/persisted-consumer-proof.avxl.json',
    expectedCanonicalJsonHash: paletteUpdate.asset.contentHashes.canonicalJson,
    expectedVoxelDataHash: paletteUpdate.asset.contentHashes.voxelData,
    maxMaterialBindings: 16,
  });
  assert.equal(duplicatePaletteUpdate.updated, false);
  assert.equal(duplicatePaletteUpdate.diagnostics[0]?.code, 'duplicate_material_binding');

  assert.throws(
    () =>
      bridge.exportVoxelConversionEvidence([
        { kind: 'source_snapshot', uri: 'asha://missing/source', contentHash: 'fnv1a64:0000000000000000' },
      ]),
    (error: unknown) => error instanceof RuntimeBridgeError && error.kind === 'invalid_input',
  );

  const proof = {
    schemaVersion: 1,
    project: 'asha',
    consumer: '@asha/smoke',
    publicImports: ['@asha/contracts', '@asha/runtime-bridge', '@asha/runtime-session'],
    engineCommit: gitValue(['rev-parse', 'HEAD']),
    engineRef: gitValue(['rev-parse', '--abbrev-ref', 'HEAD']),
    assetPath: saved.diff?.assetPath,
    assetId: saved.asset?.assetId,
    canonicalJsonHash: saved.canonicalJsonHash,
    voxelDataHash: saved.voxelDataHash,
    paletteUpdate: {
      canonicalJsonHash: paletteUpdate.canonicalJsonHash,
      voxelDataHash: paletteUpdate.voxelDataHash,
      materialPalette: paletteUpdate.asset.materialPalette,
    },
    diagnostics: saved.diagnostics,
    evidenceRefs: [...plan.evidence, ...preview.evidence, ...receipt.evidence, ...modelInfo.evidence],
    negativeMatrix: [
      ...negativeMatrix.map((item) => ({
        caseId: item.caseId,
        code: item.receipt.diagnostics[0]?.code,
      })),
      { caseId: 'stale_runtime_snapshot', code: staleSave.diagnostics[0]?.code },
      { caseId: 'stale_palette_hash', code: stalePaletteUpdate.diagnostics[0]?.code },
      { caseId: 'duplicate_palette_binding', code: duplicatePaletteUpdate.diagnostics[0]?.code },
      { caseId: 'missing_source_evidence', bridgeErrorKind: 'invalid_input' },
    ],
    readback: {
      modelId: readback.modelId,
      voxelCount: readback.voxelCount,
      materialCounts: readback.materialCounts,
      sessionHash: readback.sessionHash,
      replayHash: readback.replayHash,
    },
  };
  mkdirSync(dirname(proofPath), { recursive: true });
  writeFileSync(proofPath, `${JSON.stringify(proof, null, 2)}\n`);

  assert.deepEqual(proof.publicImports, [
    '@asha/contracts',
    '@asha/runtime-bridge',
    '@asha/runtime-session',
  ]);
  assert.match(proof.engineCommit, /^[0-9a-f]{40}$/u);
  assert.equal(proof.negativeMatrix.length, 8);
});
