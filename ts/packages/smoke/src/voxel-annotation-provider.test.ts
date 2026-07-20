import { test } from 'node:test';
import assert from 'node:assert/strict';

import type {
  VoxelAnnotationLayer,
  VoxelAnnotationLayerDraft,
  VoxelAnnotationLayerValidationInput,
  VoxelAnnotationLayerValidationRequest,
  VoxelAnnotationRegion,
  VoxelAnnotationSparseRun,
  VoxelVolumeAsset,
} from '@asha/contracts';
import {
  RuntimeBridgeError,
  createNativeRuntimeBridge,
  createRuntimeSessionFacade,
  type RuntimeBridge,
} from '@asha/runtime-bridge';
import type { RuntimeSessionFacade } from '@asha/runtime-session';

const TARGET_GRID = 2;

function bootNativeSession(t: { skip: (reason?: string) => void }): RuntimeSessionFacade | null {
  let bridge: RuntimeBridge;
  try {
    bridge = createNativeRuntimeBridge();
  } catch (error) {
    if (error instanceof RuntimeBridgeError && error.kind === 'native_unavailable') {
      t.skip('native addon not built; run harness/ci/check-native.sh for this provider regression');
      return null;
    }
    throw error;
  }

  const session = createRuntimeSessionFacade({ bridge, mode: 'rust' });
  session.initialize({
    sessionId: 'runtime-session.voxel-annotation.provider-regression',
    seed: 5278,
    project: {
      gameId: 'asha-provider-regression',
      workspaceId: 'workspace.local',
    },
  });
  return session;
}

function createVoxelAsset(session: RuntimeSessionFacade): VoxelVolumeAsset {
  const registration = session.registerVoxelConversionMeshAsset({
    source: {
      assetId: 'mesh/annotation-provider-quad',
      assetKind: 'mesh',
      assetVersion: 1,
      sourceHash: 'sha256:annotation-provider-quad',
      meshPrimitive: 'default',
    },
    meshAsset: {
      assetId: 'mesh/annotation-provider-quad',
      sourcePath: 'assets/meshes/annotation-provider-quad.mesh.json',
      positions: [[0, 0, 0], [2, 0, 0], [2, 1, 0], [0, 1, 0]],
      normals: [],
      indices: [0, 1, 2, 0, 2, 3],
      groups: [{ materialSlot: 0, start: 0, count: 6 }],
      materialSlots: [{ sourceMaterialSlot: 0, sourceMaterialId: 'material/surface-a' }],
    },
  });
  assert.equal(registration.registered, true, JSON.stringify(registration.diagnostics));

  const plan = session.planVoxelConversion({
    source: registration.source,
    target: {
      grid: TARGET_GRID,
      volumeAssetId: 'voxel/generated',
      origin: { x: 0, y: 0, z: 0 },
    },
    settings: {
      mode: 'surface',
      fitPolicy: 'contain',
      originPolicy: 'target_min',
      resolution: [4, 4, 1],
      voxelSize: 1,
      maxOutputVoxels: 32,
      transform: [1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1],
      materialMap: {
        entries: [{ sourceMaterialSlot: 0, sourceMaterialId: 'material/surface-a', voxelMaterial: 3 }],
        textureAssets: [],
        textureBindings: [],
        defaultVoxelMaterial: null,
      },
    },
  });
  assert.equal(plan.diagnostics.length, 0, JSON.stringify(plan.diagnostics));

  const preview = session.previewVoxelConversion({
    planId: plan.planId,
    expectedPlanHash: plan.planHash,
  });
  assert.equal(preview.diagnostics.length, 0, JSON.stringify(preview.diagnostics));

  const receipt = session.applyVoxelConversion({
    planId: plan.planId,
    expectedPlanHash: plan.planHash,
    expectedPreviewHash: preview.outputHash,
  });
  assert.equal(receipt.applied, true, JSON.stringify(receipt.diagnostics));

  const modelInfo = session.readVoxelModelInfo({
    grid: TARGET_GRID,
    volumeAssetId: 'voxel/generated',
    includeMaterialCounts: true,
  });
  assert.equal(modelInfo.resident, true);

  const exported = session.exportVoxelVolumeAsset({
    grid: TARGET_GRID,
    volumeAssetId: 'voxel/generated',
    targetAssetId: 'voxel-volume/annotation-provider-regression',
    label: 'Voxel annotation consumer proof',
    createdBy: '@asha/smoke',
    sourceTool: '@asha/runtime-bridge',
    maxSparseRuns: 32,
    expectedSessionHash: modelInfo.sessionHash,
  });
  assert.equal(exported.exported, true, JSON.stringify(exported.diagnostics));
  assert.ok(exported.asset !== null);
  return exported.asset;
}

function firstRegionFromAsset(asset: VoxelVolumeAsset): {
  readonly region: VoxelAnnotationRegion;
  readonly queryCell: { readonly x: number; readonly y: number; readonly z: number };
} {
  const run = asset.representation.sparseRuns[0];
  assert.ok(run, 'proof asset must contain at least one sparse run');
  const annotationRun: VoxelAnnotationSparseRun = {
    start: run.start,
    length: run.length,
  };
  const region: VoxelAnnotationRegion = {
    regionId: 'region/annotation-proof-room',
    label: 'Annotation proof room',
    kind: 'room',
    tags: ['provider-regression'],
    parentRegionId: null,
    bounds: {
      min: run.start,
      max: { x: run.start.x + run.length - 1, y: run.start.y, z: run.start.z },
    },
    selection: { sparseRuns: [annotationRun] },
  };
  return { region, queryCell: run.start };
}

function annotationLayer(asset: VoxelVolumeAsset): {
  readonly layer: VoxelAnnotationLayerDraft;
  readonly queryCell: { readonly x: number; readonly y: number; readonly z: number };
} {
  const { region, queryCell } = firstRegionFromAsset(asset);
  return {
    queryCell,
    layer: {
      layerId: 'voxel-annotation/annotation-provider-regression',
      schemaVersion: 1,
      mediaType: 'application/vnd.asha.voxel-annotation+json;version=1',
      targetVoxelVolumeAssetId: asset.assetId,
      targetVoxelDataHash: asset.contentHashes.voxelData,
      targetBounds: asset.bounds,
      regions: [region],
      provenance: [{
        kind: 'authored',
        uri: 'asha://smoke/voxel-annotation-provider-regression',
        contentHash: 'fnv1a64:annotation-provider-regression-source',
      }],
    },
  };
}

function validationRequest(
  input: VoxelAnnotationLayerValidationInput,
  targetVoxelVolumeAssetId: string,
  targetVoxelDataHash: string,
): VoxelAnnotationLayerValidationRequest {
  return {
    input,
    expectedTargetVoxelVolumeAssetId: targetVoxelVolumeAssetId,
    expectedTargetVoxelDataHash: targetVoxelDataHash,
    maxRegions: 16,
    maxSparseRunsPerRegion: 16,
    maxTotalAssignedCells: 32,
  };
}

void test('native provider validates, loads, queries, edits, and exports voxel annotations', (t) => {
  const session = bootNativeSession(t);
  if (session === null) return;

  const asset = createVoxelAsset(session);
  const volumeLoad = session.loadVoxelVolumeAsset({
    asset,
    targetGrid: TARGET_GRID,
    targetVolumeAssetId: asset.assetId,
    replaceExisting: true,
    includeMaterialCounts: true,
  });
  assert.equal(volumeLoad.loaded, true, JSON.stringify(volumeLoad.diagnostics));
  assert.equal(volumeLoad.voxelDataHash, asset.contentHashes.voxelData);

  const { layer: draftLayer, queryCell } = annotationLayer(asset);
  const draftValidation = session.validateVoxelAnnotationLayer(validationRequest(
    { kind: 'draft', draft: draftLayer },
    draftLayer.targetVoxelVolumeAssetId,
    draftLayer.targetVoxelDataHash,
  ));
  assert.equal(draftValidation.valid, true, JSON.stringify(draftValidation.diagnostics));
  assert.ok(draftValidation.normalizedLayer !== null);
  assert.match(draftValidation.canonicalJsonHash ?? '', /^fnv1a64:/);
  assert.match(draftValidation.membershipDataHash ?? '', /^fnv1a64:/);

  const layer: VoxelAnnotationLayer = draftValidation.normalizedLayer;
  const validation = session.validateVoxelAnnotationLayer(validationRequest(
    { kind: 'finalized', layer },
    layer.targetVoxelVolumeAssetId,
    layer.targetVoxelDataHash,
  ));
  assert.equal(validation.valid, true, JSON.stringify(validation.diagnostics));
  assert.equal(validation.regionCount, 1);
  assert.ok(validation.assignedCellCount >= 1);
  assert.match(validation.canonicalJsonHash ?? '', /^fnv1a64:/);
  assert.match(validation.membershipDataHash ?? '', /^fnv1a64:/);

  const quotaReport = session.validateVoxelAnnotationLayer({
    ...validationRequest(
      { kind: 'finalized', layer },
      layer.targetVoxelVolumeAssetId,
      layer.targetVoxelDataHash,
    ),
    maxTotalAssignedCells: 0,
  });
  assert.equal(quotaReport.valid, false);
  assert.equal(quotaReport.diagnostics[0]?.code, 'quota_exceeded');

  const staleLoad = session.loadVoxelAnnotationLayer({
    layer,
    targetGrid: TARGET_GRID,
    replaceExisting: true,
    expectedSessionHash: 'fnv1a64:0000000000000000',
  });
  assert.equal(staleLoad.loaded, false);
  assert.equal(staleLoad.diagnostics[0]?.code, 'target_voxel_hash_mismatch');

  const load = session.loadVoxelAnnotationLayer({
    layer,
    targetGrid: TARGET_GRID,
    replaceExisting: true,
    expectedSessionHash: volumeLoad.sessionHash,
  });
  assert.equal(load.loaded, true, JSON.stringify(load.diagnostics));
  assert.ok(load.runtimeLayerId !== null);
  assert.match(load.layerHash ?? '', /^fnv1a64:/);

  const query = session.readVoxelAnnotationQuery({
    runtimeLayerId: load.runtimeLayerId,
    layerId: layer.layerId,
    mode: 'cell',
    cell: queryCell,
    bounds: null,
    regionId: null,
    maxRegions: 4,
    expectedLayerHash: load.layerHash,
  });
  assert.equal(query.diagnostics.length, 0, JSON.stringify(query.diagnostics));
  assert.equal(query.matchedRegions[0]?.regionId, 'region/annotation-proof-room');

  const staleEdit = session.applyVoxelAnnotationEdit({
    runtimeLayerId: load.runtimeLayerId,
    layerId: layer.layerId,
    expectedLayerHash: 'fnv1a64:0000000000000000',
    operation: 'set_label',
    regionId: 'region/annotation-proof-room',
    region: null,
    sparseRuns: [],
    tags: [],
    label: 'Edited annotation proof room',
    kind: null,
    parentRegionId: null,
  });
  assert.equal(staleEdit.edited, false);
  assert.equal(staleEdit.diagnostics[0]?.code, 'stale_layer_hash');

  const edit = session.applyVoxelAnnotationEdit({
    runtimeLayerId: load.runtimeLayerId,
    layerId: layer.layerId,
    expectedLayerHash: load.layerHash ?? '',
    operation: 'set_label',
    regionId: 'region/annotation-proof-room',
    region: null,
    sparseRuns: [],
    tags: [],
    label: 'Edited annotation proof room',
    kind: null,
    parentRegionId: null,
  });
  assert.equal(edit.edited, true, JSON.stringify(edit.diagnostics));
  assert.match(edit.layerHashAfter ?? '', /^fnv1a64:/);

  const exported = session.exportVoxelAnnotationLayer({
    runtimeLayerId: load.runtimeLayerId,
    layerId: layer.layerId,
    expectedLayerHash: edit.layerHashAfter ?? '',
    includeDiagnostics: true,
  });
  assert.equal(exported.exported, true, JSON.stringify(exported.diagnostics));
  assert.equal(exported.layer?.regions[0]?.label, 'Edited annotation proof room');
  assert.equal(exported.canonicalJsonHash, edit.layerHashAfter);
  assert.match(exported.membershipDataHash ?? '', /^fnv1a64:/);

  assert.deepEqual(
    query.matchedRegions.map((region) => region.regionId),
    ['region/annotation-proof-room'],
  );
  assert.match(exported.canonicalJsonHash ?? '', /^fnv1a64:/u);
  assert.match(exported.membershipDataHash ?? '', /^fnv1a64:/u);
});
