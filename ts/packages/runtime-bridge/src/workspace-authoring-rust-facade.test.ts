import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { FpsRuntimeSessionLoadRequest } from '@asha/runtime-session';
import {
  RuntimeBridgeError,
  createNativeRuntimeBridge,
  createWorkspaceAuthoringFacade,
} from './index.js';
import { MockRuntimeBridge } from './mock.js';

const OPEN_INPUT = {
  authoringId: 'workspace-authoring.test',
  seed: 29,
  project: {
    gameId: 'authoring-consumer',
    workspaceId: 'workspace.local',
  },
  projectBundle: {
    bundleSchemaVersion: 1,
    protocolVersion: 1,
    sceneId: 42,
  },
} as const;

class GameplayRejectingBridge extends MockRuntimeBridge {
  gameplayLoadCount = 0;

  override loadFpsRuntimeSession(request: FpsRuntimeSessionLoadRequest): never {
    void request;
    this.gameplayLoadCount += 1;
    throw new Error('workspace authoring must not load gameplay runtime authority');
  }
}

void test('workspace authoring has a distinct generation-bound lifecycle and never loads gameplay', () => {
  const bridge = new GameplayRejectingBridge();
  const authoring = createWorkspaceAuthoringFacade({ bridge });

  const opened = authoring.open(OPEN_INPUT);
  assert.equal(opened.status, 'open');
  assert.equal(opened.identity.generation, 1);
  assert.equal(opened.identity.project.workspaceId, 'workspace.local');
  assert.equal(opened.dirty, false);
  assert.equal(bridge.gameplayLoadCount, 0);

  assert.throws(
    () => authoring.confirmStored({
      expectedWorkspaceId: 'workspace.other',
      expectedGeneration: 1,
      hostPath: 'assets/voxels/test.avxl.json',
      canonicalJsonHash: 'sha256:test',
    }),
    (error: unknown) => error instanceof RuntimeBridgeError
      && error.kind === 'stale_authority_snapshot',
  );
  assert.throws(
    () => authoring.close({
      expectedWorkspaceId: 'workspace.local',
      expectedGeneration: 2,
    }),
    (error: unknown) => error instanceof RuntimeBridgeError
      && error.kind === 'stale_authority_snapshot',
  );

  const closed = authoring.close({
    expectedWorkspaceId: 'workspace.local',
    expectedGeneration: 1,
  });
  assert.equal(closed.closed, true);
  assert.equal(authoring.readState().status, 'closed');
  assert.throws(
    () => authoring.readProjection(),
    (error: unknown) => error instanceof RuntimeBridgeError
      && error.kind === 'not_initialized',
  );
  assert.equal(bridge.gameplayLoadCount, 0);

  const reopened = authoring.open({
    ...OPEN_INPUT,
    authoringId: 'workspace-authoring.test.reopened',
  });
  assert.equal(reopened.identity.generation, 2);
  assert.equal(bridge.gameplayLoadCount, 0);
});

void test('native workspace authoring creates, stores, closes, and reopens voxel state without gameplay RuntimeSession', (t) => {
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
  const authoring = createWorkspaceAuthoringFacade({ bridge });
  const opened = authoring.open(OPEN_INPUT);
  assert.equal(opened.identity.nonClaims.includes('not_gameplay_runtime_session'), true);
  assert.throws(
    () => bridge.readFpsRuntimeSession(),
    (error: unknown) => error instanceof RuntimeBridgeError && error.kind === 'not_initialized',
  );

  const initialized = authoring.initializeVoxelVolumeAuthoring({
    grid: 2,
    volumeAssetId: 'voxel/workspace-authoring-test',
    seedChunk: { x: 0, y: 0, z: 0 },
    materialPalette: [{
      voxelMaterial: 1,
      paletteEntryId: 'voxel-material/test',
      displayName: 'Test material',
      materialAssetId: 'material/test',
      materialCatalogBindingId: null,
    }],
    authoring: {
      label: 'Workspace authoring test',
      createdBy: 'runtime-bridge-test',
      sourceTool: 'runtime-bridge-test',
    },
    maxMaterialBindings: 16,
  });
  assert.equal(initialized.initialized, true, JSON.stringify(initialized.diagnostics));
  const emptyProjection = authoring.readProjection();
  assert.equal(emptyProjection.delivery, 'replace');
  assert.equal(
    emptyProjection.frame.ops.some((operation) => operation.op === 'replaceMeshPayload'),
    false,
    'an initialized empty authoring volume does not invent visible geometry',
  );

  const edit = authoring.submitCommands({
    commands: [{
      op: 'setVoxel',
      grid: 2,
      coord: { x: 0, y: 0, z: 0 },
      value: { kind: 'solid', material: 1 },
    }],
  });
  assert.equal(edit.accepted, 1);
  const model = authoring.readVoxelModelInfo({
    grid: 2,
    volumeAssetId: 'voxel/workspace-authoring-test',
    includeMaterialCounts: true,
  });
  assert.equal(model.resident, true);
  assert.equal(model.voxelCount, 1);

  const projection = authoring.readProjection();
  assert.equal(projection.delivery, 'apply');
  assert.equal(projection.workspaceId, 'workspace.local');
  assert.equal(projection.generation, 1);
  assert.equal(projection.workingRevision, 2);
  assert.equal(projection.renderDiffCount, projection.frame.ops.length);
  const created = projection.frame.ops.find((operation) => operation.op === 'create');
  const meshed = projection.frame.ops.find(
    (operation) => operation.op === 'replaceMeshPayload',
  );
  assert.notEqual(created, undefined, 'workspace projection creates a retained voxel chunk');
  assert.notEqual(meshed, undefined, 'workspace projection uploads real voxel mesh geometry');
  if (meshed?.op !== 'replaceMeshPayload') throw new Error('voxel mesh projection missing');
  assert.equal(meshed.payload.provenance, 'voxelChunk');
  assert.equal(meshed.payload.source.kind, 'inline');
  if (meshed.payload.source.kind !== 'inline') throw new Error('native proof expects inline mesh');
  assert.ok(meshed.payload.source.positions.length > 0);
  assert.ok(meshed.payload.source.indices.length > 0);
  assert.match(projection.projectionHash, /^fnv1a64:[0-9a-f]{16}$/);
  assert.deepEqual(
    authoring.readProjection().frame,
    { ops: [] },
    'unchanged workspace projection preserves retained handles without replaying geometry',
  );

  const exportReceipt = authoring.exportVoxelVolumeAsset({
    grid: 2,
    volumeAssetId: 'voxel/workspace-authoring-test',
    targetAssetId: 'voxel-volume/workspace-authoring-test',
    label: 'Workspace authoring test',
    createdBy: 'runtime-bridge-test',
    sourceTool: 'runtime-bridge-test',
    maxSparseRuns: 64,
    expectedSessionHash: model.sessionHash,
  });
  assert.equal(exportReceipt.exported, true, JSON.stringify(exportReceipt.diagnostics));
  assert.notEqual(exportReceipt.asset, null);
  const asset = exportReceipt.asset;
  if (asset === null) throw new Error('accepted export omitted asset');

  const saveReceipt = authoring.saveVoxelVolumeAsset({
    exportRequest: exportReceipt.request,
    targetProjectBundle: 'authoring-consumer',
    targetAssetPath: 'assets/voxels/workspace-authoring-test.avxl.json',
    representationKind: 'sparse_runs',
    expectedExistingCanonicalJsonHash: null,
    expectedCanonicalJsonHash: asset.contentHashes.canonicalJson,
    expectedVoxelDataHash: asset.contentHashes.voxelData,
  });
  assert.equal(saveReceipt.saved, true, JSON.stringify(saveReceipt.diagnostics));
  assert.equal(authoring.readState().dirty, true, 'save receipt is not host persistence confirmation');
  assert.throws(
    () => authoring.confirmStored({
      expectedWorkspaceId: 'workspace.local',
      expectedGeneration: 1,
      hostPath: 'assets/voxels/workspace-authoring-test.avxl.json',
      canonicalJsonHash: 'fnv1a64:not-the-save-candidate',
    }),
    (error: unknown) => error instanceof RuntimeBridgeError && error.kind === 'invalid_input',
    'host confirmation must be bound to the current Rust save candidate',
  );
  assert.throws(
    () => authoring.close({ expectedWorkspaceId: 'workspace.local', expectedGeneration: 1 }),
    (error: unknown) => error instanceof RuntimeBridgeError && error.kind === 'invalid_input',
    'an unpersisted save proposal must not be treated as stored truth',
  );
  authoring.confirmStored({
    expectedWorkspaceId: 'workspace.local',
    expectedGeneration: 1,
    hostPath: saveReceipt.diff?.assetPath ?? 'assets/voxels/workspace-authoring-test.avxl.json',
    canonicalJsonHash: asset.contentHashes.canonicalJson,
  });
  assert.equal(authoring.readState().dirty, false);
  authoring.close({ expectedWorkspaceId: 'workspace.local', expectedGeneration: 1 });

  const reopened = authoring.open({ ...OPEN_INPUT, authoringId: 'workspace-authoring.test.reopen' });
  assert.equal(reopened.identity.generation, 2);
  const loaded = authoring.loadVoxelVolumeAsset({
    asset,
    targetGrid: 2,
    targetVolumeAssetId: 'voxel/workspace-authoring-test',
    replaceExisting: true,
    includeMaterialCounts: true,
  });
  assert.equal(loaded.loaded, true, JSON.stringify(loaded.diagnostics));
  assert.equal(loaded.voxelCount, 1);
  assert.equal(loaded.canonicalJsonHash, asset.contentHashes.canonicalJson);
  assert.equal(authoring.readState().dirty, false);
  const reopenedProjection = authoring.readProjection();
  assert.equal(reopenedProjection.delivery, 'replace');
  assert.equal(reopenedProjection.generation, 2);
  const reopenedCreate = reopenedProjection.frame.ops.find(
    (operation) => operation.op === 'create',
  );
  assert.notEqual(reopenedCreate, undefined, 'reopened stored voxel projects geometry');
  if (created?.op !== 'create' || reopenedCreate?.op !== 'create') {
    throw new Error('retained projection create identity missing');
  }
  assert.ok(
    reopenedProjection.frame.ops.some((operation) => operation.op === 'replaceMeshPayload'),
  );
  assert.throws(
    () => bridge.readFpsRuntimeSession(),
    (error: unknown) => error instanceof RuntimeBridgeError && error.kind === 'not_initialized',
  );
});
