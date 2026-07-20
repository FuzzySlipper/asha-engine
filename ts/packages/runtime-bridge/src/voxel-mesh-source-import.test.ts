import { test } from 'node:test';
import assert from 'node:assert/strict';

import type {
  NativeAddon,
  VoxelConversionMeshSourceImportRequest as NativeMeshSourceImportRequest,
} from '@asha/native-bridge';

import {
  NativeRuntimeBridge,
  RuntimeBridgeError,
  createRuntimeSessionFacade,
  type VoxelConversionMeshSourceImportReceipt,
  type VoxelConversionMeshSourceImportRequest,
} from './index.js';
import { MockRuntimeBridge } from './mock.js';
import {
  VOXEL_CONVERSION_MESH_SOURCE_IMPORT_REQUEST,
  createNativeVoxelMeshSourceHandlers,
  importedMeshReceipt,
} from './native-voxel-mesh-source.test-fixture.js';

const nativeRootRequest: NativeMeshSourceImportRequest = VOXEL_CONVERSION_MESH_SOURCE_IMPORT_REQUEST;

function sessionInput() {
  return {
    sessionId: 'runtime-session.voxel-mesh-source-import.test',
    seed: 17,
    project: {
      gameId: 'asha-test',
      workspaceId: 'workspace.local',
    },
  };
}

class CapturingVoxelMeshSourceImportBridge extends MockRuntimeBridge {
  request: VoxelConversionMeshSourceImportRequest | null = null;

  override importVoxelConversionMeshSource(
    request: VoxelConversionMeshSourceImportRequest,
  ): VoxelConversionMeshSourceImportReceipt {
    this.request = request;
    return importedMeshReceipt(request);
  }
}

void test('native RuntimeBridge routes mesh source import JSON and parses its typed receipt', () => {
  const calls: string[] = [];
  const addon = {
    initializeEngine: (seed: number) => {
      calls.push(`initialize:${seed}`);
      return 41;
    },
    ...createNativeVoxelMeshSourceHandlers(calls),
  } as unknown as NativeAddon;
  const bridge = new NativeRuntimeBridge(addon);
  bridge.initializeEngine({ seed: 7 });

  const receipt = bridge.importVoxelConversionMeshSource(nativeRootRequest);

  assert.equal(receipt.imported, true);
  assert.equal(receipt.source.assetId, nativeRootRequest.sourceAssetId);
  assert.equal(receipt.sourceByteCount, nativeRootRequest.sourceBytes.length);
  assert.equal(receipt.vertexCount, 4);
  assert.equal(receipt.triangleCount, 2);
  assert.deepEqual(calls, [
    'initialize:7',
    `voxelMeshSourceImport:${JSON.stringify(nativeRootRequest)}`,
  ]);
});

void test('Rust-backed RuntimeSession delegates mesh source import to its bridge', () => {
  const bridge = new CapturingVoxelMeshSourceImportBridge();
  const session = createRuntimeSessionFacade({ bridge, mode: 'rust' });
  session.initialize(sessionInput());

  const receipt = session.importVoxelConversionMeshSource(VOXEL_CONVERSION_MESH_SOURCE_IMPORT_REQUEST);

  assert.deepEqual(bridge.request, VOXEL_CONVERSION_MESH_SOURCE_IMPORT_REQUEST);
  assert.equal(receipt.imported, true);
  assert.equal(receipt.meshAsset?.indices.length, 6);
  assert.equal(receipt.groups[0]?.label, 'Wall');
});

void test('reference RuntimeSession and mock RuntimeBridge fail closed for mesh source import', () => {
  const mock = new MockRuntimeBridge();
  assert.throws(
    () => mock.importVoxelConversionMeshSource(VOXEL_CONVERSION_MESH_SOURCE_IMPORT_REQUEST),
    (error: unknown) => error instanceof RuntimeBridgeError && error.kind === 'not_initialized',
  );
  mock.initializeEngine({ seed: 5 });
  assert.throws(
    () => mock.importVoxelConversionMeshSource(VOXEL_CONVERSION_MESH_SOURCE_IMPORT_REQUEST),
    (error: unknown) => error instanceof RuntimeBridgeError && error.kind === 'operation_unimplemented',
  );

  const session = createRuntimeSessionFacade({
    bridge: new MockRuntimeBridge(),
    mode: 'reference',
  });
  session.initialize(sessionInput());
  assert.throws(
    () => session.importVoxelConversionMeshSource(VOXEL_CONVERSION_MESH_SOURCE_IMPORT_REQUEST),
    (error: unknown) => error instanceof RuntimeBridgeError && error.kind === 'operation_unimplemented',
  );
});
