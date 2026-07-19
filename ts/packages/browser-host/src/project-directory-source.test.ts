import assert from 'node:assert/strict';
import { mkdtemp, mkdir, rm, symlink, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';

import {
  ASHA_PROJECT_BUNDLE_MANIFEST_PATH,
  createMemoryAshaProjectSource,
  encodeAshaProjectPackage,
  createPackagedAshaProjectSource,
  loadAshaProjectSource,
} from '@asha/game-workspace';

import { createAshaProjectDirectorySource } from './project-directory-source.js';

const text = (value: string): Uint8Array => new TextEncoder().encode(value);

function projectFiles(): ReadonlyMap<string, Uint8Array> {
  const manifest = {
    bundleSchemaVersion: 2,
    protocolVersion: 1,
    project: { id: 9, name: 'directory-equivalence' },
    entryScene: 1,
    scenes: [{ id: 1, schemaVersion: 1, artifact: 'scenes/main.json' }],
    assetLock: { artifact: 'assets/lock.json', assetCount: 0 },
    generationProvenance: null,
    artifacts: [
      { path: 'assets/lock.json', class: 'durable', role: 'assetLock', contentHash: '0000000000000001' },
      { path: 'scenes/main.json', class: 'durable', role: 'sceneDocument', contentHash: '0000000000000002' },
    ],
  };
  return new Map([
    [ASHA_PROJECT_BUNDLE_MANIFEST_PATH, text(`${JSON.stringify(manifest)}\n`)],
    ['assets/lock.json', text('{"assets":[]}\n')],
    ['scenes/main.json', text('{"scene":"main"}\n')],
  ]);
}

void test('development directory, packaged directory, archive, and memory produce one source batch', async (context) => {
  const root = await mkdtemp(join(tmpdir(), 'asha-project-source-'));
  context.after(async () => rm(root, { recursive: true, force: true }));
  const files = projectFiles();
  for (const [path, bytes] of files) {
    const target = join(root, path);
    await mkdir(join(target, '..'), { recursive: true });
    await writeFile(target, bytes);
  }

  const sources = [
    await createAshaProjectDirectorySource(root),
    await createAshaProjectDirectorySource(root, 'packaged-directory'),
    createPackagedAshaProjectSource('archive:test', encodeAshaProjectPackage(files)),
    createMemoryAshaProjectSource('memory:test', files),
  ];
  const loaded = await Promise.all(sources.map(loadAshaProjectSource));
  assert.deepEqual(new Set(loaded.map((source) => source.materializationHash)).size, 1);
  assert.deepEqual(loaded.map((source) => source.sourceKind), [
    'development-directory',
    'packaged-directory',
    'packaged-archive',
    'memory',
  ]);
});

void test('directory reader rejects a symlink that leaves the selected project root', async (context) => {
  const root = await mkdtemp(join(tmpdir(), 'asha-project-root-'));
  const outside = await mkdtemp(join(tmpdir(), 'asha-project-outside-'));
  context.after(async () => {
    await rm(root, { recursive: true, force: true });
    await rm(outside, { recursive: true, force: true });
  });
  await writeFile(join(outside, 'secret.json'), '{}');
  await symlink(join(outside, 'secret.json'), join(root, 'secret.json'));
  const source = await createAshaProjectDirectorySource(root);
  await assert.rejects(source.read('secret.json'), /escapes root/);
});
