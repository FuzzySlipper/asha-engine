import { randomUUID } from 'node:crypto';
import {
  cp,
  lstat,
  mkdir,
  readFile,
  rename,
  rm,
  writeFile,
} from 'node:fs/promises';
import { basename, dirname, isAbsolute, join, relative, resolve } from 'node:path';

import {
  ASHA_PROJECT_BUNDLE_MANIFEST_PATH,
  compareAshaProjectPaths,
  loadAshaProjectSource,
  type AshaLoadedProjectSource,
  type ProjectArtifactExpectation,
  type ProjectStoreIdentity,
  type ProjectWriteCandidate,
  type ProjectWritePublication,
  type ProjectWriteResourceRef,
} from '@asha/game-workspace';

import { createAshaProjectDirectorySource } from './project-directory-source.js';

// Keep editor/tool state out of ProjectBundle runtime closure.
export const ASHA_PROJECT_STORE_STATE_PATH = '.asha/project-store.json';
export const ASHA_PROJECT_INDEX_PATH = '.asha/project-index.json';

export interface AshaProjectWriteTransactionOptions {
  readonly projectRoot: string;
  readonly candidate: ProjectWriteCandidate;
  readonly readResource: (resource: ProjectWriteResourceRef) => Promise<Uint8Array>;
  readonly releaseResource?: (resource: ProjectWriteResourceRef) => void | Promise<void>;
  readonly confirm: (publication: ProjectWritePublication) => boolean | Promise<boolean>;
}

export interface AshaProjectWriteTransactionReceipt {
  readonly candidateHash: string;
  readonly published: ProjectStoreIdentity;
}

/**
 * Apply one Rust candidate through copy-on-write staging and a reversible
 * directory swap. Confirmation happens only after the staged tree matches the
 * complete expected identity; a rejected/failed confirmation restores the old
 * tree.
 */
export async function applyAshaProjectWriteCandidate(
  options: AshaProjectWriteTransactionOptions,
): Promise<AshaProjectWriteTransactionReceipt> {
  const projectRoot = resolve(options.projectRoot);
  const rootStat = await lstat(projectRoot);
  if (!rootStat.isDirectory()) throw new Error('project write root must be a directory');
  assertCandidatePaths(options.candidate);
  const indexPath = options.candidate.indexReplacement?.path ?? ASHA_PROJECT_INDEX_PATH;
  const prior = await observeStore(projectRoot, options.candidate.expectedPriorArtifacts, indexPath);
  assertIdentity('stale project write candidate', options.candidate.expectedPrior, prior);

  const resources = await borrowCandidateResources(options);
  await releaseCandidateResources(options);
  const parent = dirname(projectRoot);
  const rootName = basename(projectRoot);
  const transactionId = randomUUID();
  const staging = join(parent, `.${rootName}.asha-stage-${transactionId}`);
  const backup = join(parent, `.${rootName}.asha-backup-${transactionId}`);
  let published = false;
  try {
    await cp(projectRoot, staging, { recursive: true, force: false, errorOnExist: true });
    await applyMoves(staging, options.candidate);
    await applyWrites(staging, options.candidate, resources);
    await applyDeletes(staging, options.candidate);
    await writeFile(join(staging, ASHA_PROJECT_BUNDLE_MANIFEST_PATH), options.candidate.manifestJson);
    await writeStoreState(staging, options.candidate);

    const staged = await observeStore(staging, options.candidate.expectedNextArtifacts, indexPath);
    assertIdentity('staged project write differs from Rust candidate', options.candidate.expectedNext, staged);

    await rename(projectRoot, backup);
    try {
      await rename(staging, projectRoot);
      published = true;
    } catch (error) {
      await rename(backup, projectRoot);
      throw error;
    }

    const publication: ProjectWritePublication = {
      candidateHash: options.candidate.candidateHash,
      published: staged,
    };
    let confirmed = false;
    try {
      confirmed = await options.confirm(publication);
    } catch (error) {
      await rollbackPublication(projectRoot, backup, staging);
      published = false;
      throw error;
    }
    if (!confirmed) {
      await rollbackPublication(projectRoot, backup, staging);
      published = false;
      throw new Error('Rust rejected the published project write candidate');
    }
    await rm(backup, { recursive: true, force: true });
    return { candidateHash: options.candidate.candidateHash, published: staged };
  } finally {
    if (!published) {
      await rm(staging, { recursive: true, force: true });
      await rm(backup, { recursive: true, force: true });
    }
  }
}

async function borrowCandidateResources(
  options: AshaProjectWriteTransactionOptions,
): Promise<ReadonlyMap<number, Uint8Array>> {
  const resources = new Map<number, Uint8Array>();
  const borrowed: ProjectWriteResourceRef[] = [];
  try {
    for (const write of allWrites(options.candidate)) {
      if (resources.has(write.resource.handle)) {
        throw new Error(`project write candidate reuses resource handle ${write.resource.handle}`);
      }
      const bytes = (await options.readResource(write.resource)).slice();
      borrowed.push(write.resource);
      if (bytes.byteLength !== write.resource.byteLen) {
        throw new Error(`project write resource length mismatch for "${write.path}"`);
      }
      if (fnv1a64Hex(bytes) !== write.contentHash) {
        throw new Error(`project write resource hash mismatch for "${write.path}"`);
      }
      resources.set(write.resource.handle, bytes);
    }
  } catch (error) {
    if (options.releaseResource !== undefined) {
      for (const resource of borrowed) await options.releaseResource(resource);
    }
    throw error;
  }
  return resources;
}

async function releaseCandidateResources(options: AshaProjectWriteTransactionOptions): Promise<void> {
  if (options.releaseResource === undefined) return;
  for (const write of allWrites(options.candidate)) await options.releaseResource(write.resource);
}

function allWrites(candidate: ProjectWriteCandidate) {
  return candidate.indexReplacement === null
    ? candidate.writes
    : [...candidate.writes, candidate.indexReplacement];
}

async function applyMoves(root: string, candidate: ProjectWriteCandidate): Promise<void> {
  for (const movement of candidate.moves) {
    const from = projectPath(root, movement.from);
    const to = projectPath(root, movement.to);
    await mkdir(dirname(to), { recursive: true });
    await rename(from, to);
  }
}

async function applyWrites(
  root: string,
  candidate: ProjectWriteCandidate,
  resources: ReadonlyMap<number, Uint8Array>,
): Promise<void> {
  for (const write of allWrites(candidate)) {
    const bytes = resources.get(write.resource.handle);
    if (bytes === undefined) throw new Error(`missing borrowed resource ${write.resource.handle}`);
    const target = projectPath(root, write.path);
    await mkdir(dirname(target), { recursive: true });
    await writeFile(target, bytes);
  }
}

async function applyDeletes(root: string, candidate: ProjectWriteCandidate): Promise<void> {
  for (const deletion of candidate.deletes) {
    await rm(projectPath(root, deletion.path), { force: false });
  }
}

async function writeStoreState(root: string, candidate: ProjectWriteCandidate): Promise<void> {
  const target = projectPath(root, ASHA_PROJECT_STORE_STATE_PATH);
  await mkdir(dirname(target), { recursive: true });
  await writeFile(target, `${JSON.stringify({
    revision: candidate.expectedNext.revision,
    candidateHash: candidate.candidateHash,
  })}\n`);
}

async function observeStore(
  root: string,
  expectations: readonly ProjectArtifactExpectation[],
  indexPath: string,
): Promise<ProjectStoreIdentity> {
  const source = await loadAshaProjectSource(await createAshaProjectDirectorySource(root));
  verifyArtifactHashes(source, expectations);
  const revision = await readStoredRevision(root);
  const indexHash = await readOptionalHash(projectPath(root, indexPath));
  return {
    revision,
    manifestHash: fnv1a64Hex(new TextEncoder().encode(source.manifestJson)),
    contentSetHash: contentSetHash(source),
    indexHash,
  };
}

function verifyArtifactHashes(
  source: AshaLoadedProjectSource,
  expectations: readonly ProjectArtifactExpectation[],
): void {
  const byPath = new Map(source.files.map((file) => [file.path, file.bytes]));
  for (const expectation of expectations) {
    const bytes = byPath.get(expectation.path);
    if (bytes === undefined && expectation.contentHash === null) continue;
    if (bytes === undefined) throw new Error(`project store is missing "${expectation.path}"`);
    if (expectation.contentHash !== null && fnv1a64Hex(bytes) !== expectation.contentHash) {
      throw new Error(`project store content drift at "${expectation.path}"`);
    }
  }
}

function contentSetHash(source: AshaLoadedProjectSource): string {
  let canonical = '';
  const artifacts = [...source.manifest.artifacts].sort((left, right) => compareAshaProjectPaths(left.path, right.path));
  for (const artifact of artifacts) {
    canonical += `${artifact.path}\0${artifact.class}\0${artifact.role}\0${artifact.contentHash ?? '-'}\n`;
  }
  return fnv1a64Hex(new TextEncoder().encode(canonical));
}

async function readStoredRevision(root: string): Promise<number> {
  try {
    const value: unknown = JSON.parse(await readFile(join(root, ASHA_PROJECT_STORE_STATE_PATH), 'utf8'));
    if (
      typeof value !== 'object'
      || value === null
      || !('revision' in value)
      || typeof value.revision !== 'number'
      || !Number.isSafeInteger(value.revision)
      || value.revision < 0
    ) throw new Error('invalid project store revision');
    return value.revision;
  } catch (error) {
    if (isMissingFile(error)) return 0;
    throw error;
  }
}

async function readOptionalHash(path: string): Promise<string | null> {
  try {
    return fnv1a64Hex(await readFile(path));
  } catch (error) {
    if (isMissingFile(error)) return null;
    throw error;
  }
}

function assertCandidatePaths(candidate: ProjectWriteCandidate): void {
  for (const path of [
    ...candidate.writes.map((write) => write.path),
    ...candidate.moves.flatMap((movement) => [movement.from, movement.to]),
    ...candidate.deletes.map((deletion) => deletion.path),
    ...(candidate.indexReplacement === null ? [] : [candidate.indexReplacement.path]),
  ]) projectPath('/', path);
}

function projectPath(root: string, relativePath: string): string {
  if (
    relativePath.length === 0
    || isAbsolute(relativePath)
    || relativePath.includes('\\')
    || relativePath.split('/').some((part) => part.length === 0 || part === '.' || part === '..')
  ) throw new Error(`invalid project write path "${relativePath}"`);
  const target = resolve(root, relativePath);
  const relation = relative(resolve(root), target);
  if (relation === '..' || relation.startsWith('../') || isAbsolute(relation)) {
    throw new Error(`project write path escapes root: ${relativePath}`);
  }
  return target;
}

function assertIdentity(label: string, expected: ProjectStoreIdentity, actual: ProjectStoreIdentity): void {
  if (
    expected.revision !== actual.revision
    || expected.manifestHash !== actual.manifestHash
    || expected.contentSetHash !== actual.contentSetHash
    || expected.indexHash !== actual.indexHash
  ) throw new Error(label);
}

async function rollbackPublication(projectRoot: string, backup: string, failed: string): Promise<void> {
  await rename(projectRoot, failed);
  try {
    await rename(backup, projectRoot);
  } finally {
    await rm(failed, { recursive: true, force: true });
  }
}

function fnv1a64Hex(bytes: Uint8Array): string {
  let hash = 14_695_981_039_346_656_037n;
  for (const byte of bytes) {
    hash ^= BigInt(byte);
    hash = BigInt.asUintN(64, hash * 1_099_511_628_211n);
  }
  return hash.toString(16).padStart(16, '0');
}

function isMissingFile(error: unknown): boolean {
  return typeof error === 'object' && error !== null && 'code' in error && error.code === 'ENOENT';
}
