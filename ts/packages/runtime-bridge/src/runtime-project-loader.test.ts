import assert from 'node:assert/strict';
import test from 'node:test';

import {
  ASHA_PROJECT_BUNDLE_MANIFEST_PATH,
  createMemoryAshaProjectSource,
  createPackagedAshaProjectSource,
  encodeAshaProjectPackage,
} from '@asha/game-workspace';
import type { RuntimeSessionProjectSource } from '@asha/runtime-session';

import { createRuntimeSessionFacade } from './runtime-session-adapter.js';
import { createMockRuntimeBridge } from './mock.js';

const text = (value: string): Uint8Array => new TextEncoder().encode(value);

function projectFiles(): ReadonlyMap<string, Uint8Array> {
  const manifest = {
    bundleSchemaVersion: 2,
    protocolVersion: 1,
    project: { id: 71, name: 'public-loader-fixture' },
    entryScene: 101,
    scenes: [{ id: 101, schemaVersion: 4, artifact: 'scenes/entry.scene.json' }],
    assetLock: { artifact: 'assets/lock.json', assetCount: 0 },
    generationProvenance: null,
    artifacts: [
      { path: 'assets/lock.json', class: 'durable', role: 'assetLock', contentHash: '0000000000000001' },
      { path: 'scenes/entry.scene.json', class: 'durable', role: 'sceneDocument', contentHash: '0000000000000002' },
    ],
  };
  return new Map([
    [ASHA_PROJECT_BUNDLE_MANIFEST_PATH, text(`${JSON.stringify(manifest)}\n`)],
    ['assets/lock.json', text('{"assets":[]}\n')],
    ['scenes/entry.scene.json', text('{"schemaVersion":4,"id":101,"nodes":[]}\n')],
  ]);
}

function developmentDirectorySource(
  files: ReadonlyMap<string, Uint8Array>,
): RuntimeSessionProjectSource {
  return {
    kind: 'development-directory',
    identity: 'development-directory:/fixture/game',
    read: async (path) => {
      const bytes = files.get(path);
      if (bytes === undefined) throw new Error(`fixture source is missing ${path}`);
      return bytes.slice();
    },
  };
}

void test('loadProject uses one source-only call for development, packaged, and memory projects', async () => {
  const files = projectFiles();
  const sources: readonly RuntimeSessionProjectSource[] = [
    developmentDirectorySource(files),
    createPackagedAshaProjectSource('package:/fixture/game.asha', encodeAshaProjectPackage(files)),
    createMemoryAshaProjectSource('memory:public-loader', files),
  ];
  const expectedKinds = ['developmentDirectory', 'packagedProject', 'inMemory'];

  for (const [index, source] of sources.entries()) {
    const session = createRuntimeSessionFacade({ bridge: createMockRuntimeBridge(), mode: 'rust' });
    const initializeInput = {
      sessionId: `runtime-session.project-loader.${index}`,
      seed: 17,
      project: { gameId: 'loader-fixture', workspaceId: 'workspace.loader' },
    };
    assert.deepEqual(Object.keys(initializeInput).sort(), ['project', 'seed', 'sessionId']);
    session.initialize(initializeInput);

    const receipt = await session.loadProject({ source });
    assert.equal(receipt.accepted, true, JSON.stringify(receipt.diagnostics));
    assert.equal(receipt.source.kind, expectedKinds[index]);
    assert.equal(receipt.activeProject?.lifecycle.generation, 1);
    assert.equal(receipt.activeProject?.contentSetHash, 'mock-content-set');

    const closed = session.closeProject();
    assert.equal(closed.accepted, true);
    assert.equal(closed.lifecycle.revision, 2);
  }
});

void test('loadProject surfaces adapter failure without invoking runtime activation', async () => {
  const session = createRuntimeSessionFacade({ bridge: createMockRuntimeBridge(), mode: 'rust' });
  session.initialize({
    sessionId: 'runtime-session.project-loader.rejected',
    seed: 18,
    project: { gameId: 'loader-fixture', workspaceId: 'workspace.loader' },
  });
  const receipt = await session.loadProject({
    source: {
      kind: 'memory',
      identity: 'memory:missing-manifest',
      read: async () => { throw new Error('manifest is missing'); },
    },
  });
  assert.equal(receipt.accepted, false);
  assert.equal(receipt.diagnostics[0]?.phase, 'sourceAdapter');
  assert.equal(receipt.diagnostics[0]?.code, 'sourceAdapterRejected');
});
