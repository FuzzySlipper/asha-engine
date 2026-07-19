import assert from 'node:assert/strict';
import test from 'node:test';

import {
  ASHA_PROJECT_BUNDLE_MANIFEST_PATH,
  createMemoryAshaProjectSource,
  createPackagedAshaProjectSource,
  decodeAshaProjectPackage,
  encodeAshaProjectPackage,
  loadAshaProjectSource,
} from './project-source.js';

const text = (value: string): Uint8Array => new TextEncoder().encode(value);

function projectFiles(): ReadonlyMap<string, Uint8Array> {
  const scene = text('{"scene":"main"}\n');
  const lock = text('{"assets":[]}\n');
  const second = text('{"scene":"second"}\n');
  const manifest = {
    bundleSchemaVersion: 2,
    protocolVersion: 1,
    project: { id: 7, name: 'adapter-equivalence' },
    entryScene: 100,
    scenes: [
      { id: 100, schemaVersion: 1, artifact: 'scenes/main.json' },
      { id: 200, schemaVersion: 1, artifact: 'scenes/second.json' },
    ],
    assetLock: { artifact: 'assets/lock.json', assetCount: 0 },
    generationProvenance: null,
    artifacts: [
      { path: 'assets/lock.json', class: 'durable', role: 'assetLock', contentHash: '0000000000000001' },
      { path: 'cache/preview.bin', class: 'cache', role: 'cache', contentHash: null },
      { path: 'scenes/main.json', class: 'durable', role: 'sceneDocument', contentHash: '0000000000000002' },
      { path: 'scenes/second.json', class: 'durable', role: 'sceneDocument', contentHash: '0000000000000003' },
    ],
  };
  return new Map([
    [ASHA_PROJECT_BUNDLE_MANIFEST_PATH, text(`${JSON.stringify(manifest)}\n`)],
    ['scenes/main.json', scene],
    ['assets/lock.json', lock],
    ['scenes/second.json', second],
  ]);
}

void test('memory and packaged archive adapters materialize the same manifest-owned closure', async () => {
  const files = projectFiles();
  const memory = await loadAshaProjectSource(createMemoryAshaProjectSource('memory:test', files));
  const archiveBytes = encodeAshaProjectPackage(files);
  const packaged = await loadAshaProjectSource(createPackagedAshaProjectSource('package:test', archiveBytes));

  assert.equal(memory.materializationHash, packaged.materializationHash);
  assert.deepEqual(memory.files, packaged.files);
  assert.deepEqual(memory.files.map((file) => file.path), [
    'assets/lock.json',
    'scenes/main.json',
    'scenes/second.json',
  ]);
  assert.equal(packaged.sourceKind, 'packaged-archive');
});

void test('archive decoding rejects trailing data and duplicate/traversing paths', () => {
  const encoded = encodeAshaProjectPackage(projectFiles());
  const trailing = new Uint8Array(encoded.byteLength + 1);
  trailing.set(encoded);
  assert.throws(() => decodeAshaProjectPackage(trailing), /trailing bytes/);
  assert.throws(
    () => encodeAshaProjectPackage(new Map([['../outside', text('bad')]])),
    /invalid canonical project-relative path/,
  );
});

void test('reader follows only manifest paths and fails when a declared file is missing', async () => {
  const files = new Map(projectFiles());
  files.set('unlisted/private.json', text('must-not-be-read'));
  files.delete('scenes/second.json');
  const reads: string[] = [];
  const memory = createMemoryAshaProjectSource('memory:missing', files);
  await assert.rejects(
    loadAshaProjectSource({
      kind: memory.kind,
      identity: memory.identity,
      read: async (path) => {
        reads.push(path);
        return memory.read(path);
      },
    }),
    /missing "scenes\/second.json"/,
  );
  assert.equal(reads.includes('unlisted/private.json'), false);
});
