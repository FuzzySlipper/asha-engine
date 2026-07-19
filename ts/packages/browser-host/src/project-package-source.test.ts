import assert from 'node:assert/strict';
import test from 'node:test';
import { mkdtemp, rm, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { tmpdir } from 'node:os';

import {
  ASHA_PROJECT_BUNDLE_MANIFEST_PATH,
  encodeAshaProjectPackage,
  loadAshaProjectSource,
} from '@asha/game-workspace';

import { createAshaProjectPackageFileSource } from './project-package-source.js';

const text = (value: string): Uint8Array => new TextEncoder().encode(value);

void test('package-file adapter reads one server-local archive through the shared loader', async () => {
  const root = await mkdtemp(join(tmpdir(), 'asha-package-source-'));
  try {
    const manifest = {
      bundleSchemaVersion: 2,
      protocolVersion: 1,
      project: { id: 4, name: 'package-file' },
      entryScene: 9,
      scenes: [{ id: 9, schemaVersion: 4, artifact: 'scenes/entry.json' }],
      assetLock: { artifact: 'assets/lock.json', assetCount: 0 },
      generationProvenance: null,
      artifacts: [
        { path: 'assets/lock.json', class: 'durable', role: 'assetLock', contentHash: '0000000000000001' },
        { path: 'scenes/entry.json', class: 'durable', role: 'sceneDocument', contentHash: '0000000000000002' },
      ],
    };
    const archive = encodeAshaProjectPackage(new Map([
      [ASHA_PROJECT_BUNDLE_MANIFEST_PATH, text(JSON.stringify(manifest))],
      ['assets/lock.json', text('{}')],
      ['scenes/entry.json', text('{}')],
    ]));
    const packagePath = join(root, 'game.asha');
    await writeFile(packagePath, archive);

    const loaded = await loadAshaProjectSource(
      await createAshaProjectPackageFileSource(packagePath),
    );
    assert.equal(loaded.sourceKind, 'packaged-archive');
    assert.match(loaded.sourceIdentity, /^packaged-project:/);
    assert.deepEqual(loaded.files.map((file) => file.path), [
      'assets/lock.json',
      'scenes/entry.json',
    ]);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});
