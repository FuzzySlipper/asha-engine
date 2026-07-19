import assert from 'node:assert/strict';
import { mkdtemp, mkdir, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test, { type TestContext } from 'node:test';

import {
  ASHA_PROJECT_BUNDLE_MANIFEST_PATH,
  loadAshaProjectSource,
  type ProjectArtifactExpectation,
  type ProjectBundleManifest,
  type ProjectStoreIdentity,
  type ProjectWriteCandidate,
} from '@asha/game-workspace';

import { createAshaProjectDirectorySource } from './project-directory-source.js';
import {
  applyAshaProjectWriteCandidate,
  observeAshaProjectStore,
} from './project-write-transaction.js';

const text = (value: string): Uint8Array => new TextEncoder().encode(value);

interface ProjectFixture {
  readonly root: string;
  readonly priorManifestJson: string;
  readonly nextManifestJson: string;
  readonly candidate: ProjectWriteCandidate;
  readonly resources: ReadonlyMap<number, Uint8Array>;
}

void test('one Rust candidate saves add move delete and index changes then reloads normally', async (context) => {
  const fixture = await createFixture(context);
  const observed = await observeAshaProjectStore(fixture.root);
  assert.deepEqual(observed.identity, fixture.candidate.expectedPrior);
  assert.equal(observed.manifestJson, fixture.priorManifestJson);
  let confirmations = 0;
  const receipt = await applyAshaProjectWriteCandidate({
    projectRoot: fixture.root,
    candidate: fixture.candidate,
    readResource: async (resource) => {
      const bytes = fixture.resources.get(resource.handle);
      if (bytes === undefined) throw new Error('missing test resource');
      return bytes;
    },
    confirm: (publication) => {
      confirmations += 1;
      assert.deepEqual(publication.published, fixture.candidate.expectedNext);
      assert.equal(publication.candidateHash, fixture.candidate.candidateHash);
      return true;
    },
  });
  assert.equal(confirmations, 1);
  assert.deepEqual(receipt.published, fixture.candidate.expectedNext);

  const reloaded = await loadAshaProjectSource(await createAshaProjectDirectorySource(fixture.root));
  assert.equal(reloaded.manifestJson, fixture.nextManifestJson);
  assert.deepEqual(reloaded.files.map((file) => file.path), [
    'assets/lock.json',
    'scenes/added.json',
    'scenes/archive/main-renamed.json',
  ]);
  await assert.rejects(readFile(join(fixture.root, 'scenes/removed.json')), /ENOENT/);
});

void test('stale host content rejects before staging and never calls Rust confirmation', async (context) => {
  const fixture = await createFixture(context);
  await writeFile(join(fixture.root, 'scenes/main.json'), 'drifted');
  let confirmations = 0;
  await assert.rejects(
    applyAshaProjectWriteCandidate({
      projectRoot: fixture.root,
      candidate: fixture.candidate,
      readResource: async () => {
        throw new Error('resources must not be borrowed for stale state');
      },
      confirm: () => {
        confirmations += 1;
        return true;
      },
    }),
    /content drift/,
  );
  assert.equal(confirmations, 0);
  assert.equal(await readFile(join(fixture.root, 'scenes/main.json'), 'utf8'), 'drifted');
});

void test('a rejected confirmation rolls the published directory back exactly', async (context) => {
  const fixture = await createFixture(context);
  let confirmations = 0;
  await assert.rejects(
    applyAshaProjectWriteCandidate({
      projectRoot: fixture.root,
      candidate: fixture.candidate,
      readResource: async (resource) => fixture.resources.get(resource.handle) ?? new Uint8Array(),
      confirm: () => {
        confirmations += 1;
        return false;
      },
    }),
    /Rust rejected/,
  );
  assert.equal(confirmations, 1);
  assert.equal(await readFile(join(fixture.root, ASHA_PROJECT_BUNDLE_MANIFEST_PATH), 'utf8'), fixture.priorManifestJson);
  assert.equal(await readFile(join(fixture.root, 'scenes/main.json'), 'utf8'), 'main-old');
  await assert.rejects(readFile(join(fixture.root, 'scenes/added.json')), /ENOENT/);
});

async function createFixture(context: TestContext): Promise<ProjectFixture> {
  const root = await mkdtemp(join(tmpdir(), 'asha-project-write-'));
  context.after(async () => rm(root, { recursive: true, force: true }));
  const priorFiles = new Map<string, Uint8Array>([
    ['scenes/main.json', text('main-old')],
    ['scenes/removed.json', text('removed-old')],
    ['assets/lock.json', text('{"assets":[]}')],
  ]);
  const nextFiles = new Map<string, Uint8Array>([
    ['scenes/archive/main-renamed.json', priorFiles.get('scenes/main.json')!],
    ['scenes/added.json', text('added-new')],
    ['assets/lock.json', priorFiles.get('assets/lock.json')!],
  ]);
  const priorManifest = manifest([
    [1, 'scenes/main.json', priorFiles.get('scenes/main.json')!],
    [2, 'scenes/removed.json', priorFiles.get('scenes/removed.json')!],
  ], priorFiles.get('assets/lock.json')!);
  const nextManifest = manifest([
    [1, 'scenes/archive/main-renamed.json', nextFiles.get('scenes/archive/main-renamed.json')!],
    [3, 'scenes/added.json', nextFiles.get('scenes/added.json')!],
  ], nextFiles.get('assets/lock.json')!);
  const priorManifestJson = `${JSON.stringify(priorManifest)}\n`;
  const nextManifestJson = `${JSON.stringify(nextManifest)}\n`;
  await writeProject(root, priorManifestJson, priorFiles);

  const indexBytes = text('{"scenes":2}\n');
  const resources = new Map<number, Uint8Array>([
    [1, nextFiles.get('scenes/added.json')!],
    [2, indexBytes],
  ]);
  const priorExpectations = expectations(priorManifest);
  const nextExpectations = expectations(nextManifest);
  const candidate: ProjectWriteCandidate = {
    candidateHash: 'candidate:fixture',
    expectedPrior: identity(0, priorManifestJson, priorManifest, null),
    expectedNext: identity(1, nextManifestJson, nextManifest, hash(indexBytes)),
    expectedPriorArtifacts: priorExpectations,
    expectedNextArtifacts: nextExpectations,
    manifestJson: nextManifestJson,
    writes: [{
      path: 'scenes/added.json',
      contentHash: hash(resources.get(1)!),
      resource: { handle: 1, version: 1, byteLen: resources.get(1)!.byteLength },
    }],
    moves: [{
      from: 'scenes/main.json',
      to: 'scenes/archive/main-renamed.json',
      expectedContentHash: hash(priorFiles.get('scenes/main.json')!),
    }],
    deletes: [{
      path: 'scenes/removed.json',
      expectedContentHash: hash(priorFiles.get('scenes/removed.json')!),
    }],
    indexReplacement: {
      path: '.asha/project-index.json',
      contentHash: hash(indexBytes),
      resource: { handle: 2, version: 1, byteLen: indexBytes.byteLength },
    },
  };
  return { root, priorManifestJson, nextManifestJson, candidate, resources };
}

function manifest(
  scenes: readonly (readonly [number, string, Uint8Array])[],
  lock: Uint8Array,
): ProjectBundleManifest {
  return {
    bundleSchemaVersion: 2,
    protocolVersion: 1,
    project: { id: 7 as ProjectBundleManifest['project']['id'], name: 'write-host' },
    entryScene: scenes[0]![0] as ProjectBundleManifest['entryScene'],
    scenes: scenes.map(([id, artifact]) => ({
      id: id as ProjectBundleManifest['entryScene'],
      schemaVersion: 1,
      artifact,
    })),
    assetLock: { artifact: 'assets/lock.json', assetCount: 0 },
    generationProvenance: null,
    artifacts: [
      { path: 'assets/lock.json', class: 'durable' as const, role: 'assetLock', contentHash: hash(lock) },
      ...scenes.map(([, path, bytes]) => ({
        path,
        class: 'durable' as const,
        role: 'sceneDocument',
        contentHash: hash(bytes),
      })),
    ].sort((left, right) => left.path.localeCompare(right.path)),
  };
}

function expectations(manifestValue: ProjectBundleManifest): readonly ProjectArtifactExpectation[] {
  return manifestValue.artifacts.map((artifact) => ({
    path: artifact.path,
    contentHash: artifact.contentHash,
  }));
}

function identity(
  revision: number,
  manifestJson: string,
  manifestValue: ProjectBundleManifest,
  indexHash: string | null,
): ProjectStoreIdentity {
  let content = '';
  for (const artifact of [...manifestValue.artifacts].sort((left, right) => left.path.localeCompare(right.path))) {
    content += `${artifact.path}\0${artifact.class}\0${artifact.role}\0${artifact.contentHash ?? '-'}\n`;
  }
  return {
    revision,
    manifestHash: hash(text(manifestJson)),
    contentSetHash: hash(text(content)),
    indexHash,
  };
}

async function writeProject(
  root: string,
  manifestJson: string,
  files: ReadonlyMap<string, Uint8Array>,
): Promise<void> {
  await writeFile(join(root, ASHA_PROJECT_BUNDLE_MANIFEST_PATH), manifestJson);
  for (const [path, bytes] of files) {
    const target = join(root, path);
    await mkdir(join(target, '..'), { recursive: true });
    await writeFile(target, bytes);
  }
}

function hash(bytes: Uint8Array): string {
  let value = 14_695_981_039_346_656_037n;
  for (const byte of bytes) {
    value ^= BigInt(byte);
    value = BigInt.asUintN(64, value * 1_099_511_628_211n);
  }
  return value.toString(16).padStart(16, '0');
}
