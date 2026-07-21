import { randomUUID } from 'node:crypto';
import {
  link,
  lstat,
  mkdir,
  readdir,
  readFile,
  readlink,
  rename,
  rm,
  symlink,
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

export interface AshaProjectStoreObservation {
  readonly identity: ProjectStoreIdentity;
  readonly manifestJson: string;
}

/** Observe the current host store before asking Rust to prepare a write candidate. */
export async function observeAshaProjectStore(
  projectRoot: string,
): Promise<AshaProjectStoreObservation> {
  const root = resolve(projectRoot);
  const source = await loadAshaProjectSource(await createAshaProjectDirectorySource(root));
  const expectations = source.manifest.artifacts.map((artifact) => ({
    path: artifact.path,
    contentHash: artifact.contentHash,
  }));
  verifyArtifactHashes(source, expectations);
  return {
    identity: {
      revision: await readStoredRevision(root),
      manifestHash: fnv1a64Hex(new TextEncoder().encode(source.manifestJson)),
      contentSetHash: contentSetHash(source),
      indexHash: await readOptionalHash(projectPath(root, ASHA_PROJECT_INDEX_PATH)),
    },
    manifestJson: source.manifestJson,
  };
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
  const releaseStoreLock = await acquireProjectStoreLock(projectRoot);
  try {
    // The optimistic observation above avoids borrowing resources for an
    // already-stale candidate. This second observation is the real CAS: the
    // sibling lock remains held until Rust confirms or the host rolls back.
    let lockedPrior: ProjectStoreIdentity;
    try {
      lockedPrior = await observeStore(
        projectRoot,
        options.candidate.expectedPriorArtifacts,
        indexPath,
      );
    } catch (error) {
      throw new Error(`stale project write candidate: ${errorMessage(error)}`);
    }
    assertIdentity('stale project write candidate', options.candidate.expectedPrior, lockedPrior);
    return await publishAshaProjectWriteCandidate(
      options,
      projectRoot,
      indexPath,
      resources,
    );
  } finally {
    await releaseStoreLock();
  }
}

async function publishAshaProjectWriteCandidate(
  options: AshaProjectWriteTransactionOptions,
  projectRoot: string,
  indexPath: string,
  resources: ReadonlyMap<number, Uint8Array>,
): Promise<AshaProjectWriteTransactionReceipt> {
  const parent = dirname(projectRoot);
  const rootName = basename(projectRoot);
  const transactionId = randomUUID();
  const staging = join(parent, `.${rootName}.asha-stage-${transactionId}`);
  const backup = join(parent, `.${rootName}.asha-backup-${transactionId}`);
  let published = false;
  try {
    await cloneProjectTree(projectRoot, staging);
    await applyMoves(staging, options.candidate);
    await applyWrites(staging, options.candidate, resources);
    await applyDeletes(staging, options.candidate);
    await replaceFile(
      join(staging, ASHA_PROJECT_BUNDLE_MANIFEST_PATH),
      options.candidate.manifestJson,
    );
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
  if (candidate.moves.length === 0) return;
  const moveStaging = join(
    dirname(root),
    `.${basename(root)}.asha-moves-${randomUUID()}`,
  );
  await mkdir(moveStaging);
  const stagedMoves: Array<{ readonly temporary: string; readonly to: string }> = [];
  try {
    // Move every source out of the graph before populating any destination.
    // This preserves A->B,B->A swaps and A->B,B->C chains without letting an
    // early rename destroy a later source.
    for (const [index, movement] of candidate.moves.entries()) {
      const temporary = join(moveStaging, index.toString().padStart(8, '0'));
      await rename(projectPath(root, movement.from), temporary);
      stagedMoves.push({ temporary, to: projectPath(root, movement.to) });
    }
    for (const movement of stagedMoves) {
      await mkdir(dirname(movement.to), { recursive: true });
      await rename(movement.temporary, movement.to);
    }
  } finally {
    await rm(moveStaging, { recursive: true, force: true });
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
    await replaceFile(target, bytes);
  }
}

async function applyDeletes(root: string, candidate: ProjectWriteCandidate): Promise<void> {
  for (const deletion of candidate.deletes) {
    await rm(projectPath(root, deletion.path), { force: false });
  }
}

async function writeStoreState(root: string, candidate: ProjectWriteCandidate): Promise<void> {
  const target = projectPath(root, ASHA_PROJECT_STORE_STATE_PATH);
  await replaceFile(target, `${JSON.stringify({
    revision: candidate.expectedNext.revision,
    candidateHash: candidate.candidateHash,
  })}\n`);
}

/**
 * Build a copy-on-write sibling snapshot without copying build outputs or
 * dependency stores byte-for-byte. Every ordinary file begins as a hard link;
 * transaction writes replace their link with a new inode before publication.
 * The snapshot therefore preserves the complete checkout for the atomic root
 * swap while keeping staging cost proportional to file metadata and changed
 * project resources rather than repository byte size.
 */
async function cloneProjectTree(source: string, target: string): Promise<void> {
  const sourceStat = await lstat(source);
  if (!sourceStat.isDirectory()) throw new Error('project snapshot source must be a directory');
  await mkdir(target, { mode: sourceStat.mode });
  for (const entry of await readdir(source, { withFileTypes: true })) {
    const sourcePath = join(source, entry.name);
    const targetPath = join(target, entry.name);
    if (entry.isDirectory()) {
      await cloneProjectTree(sourcePath, targetPath);
    } else if (entry.isFile()) {
      await link(sourcePath, targetPath);
    } else if (entry.isSymbolicLink()) {
      await symlink(await readlink(sourcePath), targetPath);
    } else {
      throw new Error(`project snapshot contains unsupported entry "${sourcePath}"`);
    }
  }
}

async function replaceFile(path: string, bytes: string | Uint8Array): Promise<void> {
  await mkdir(dirname(path), { recursive: true });
  const temporary = join(dirname(path), `.${basename(path)}.asha-write-${randomUUID()}`);
  try {
    await writeFile(temporary, bytes);
    await rename(temporary, path);
  } finally {
    await rm(temporary, { force: true });
  }
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

async function acquireProjectStoreLock(
  projectRoot: string,
): Promise<() => Promise<void>> {
  const lockPath = join(dirname(projectRoot), `.${basename(projectRoot)}.asha-write-lock`);
  const deadline = Date.now() + 30_000;
  while (true) {
    try {
      await mkdir(lockPath);
      try {
        await writeFile(join(lockPath, 'owner.json'), `${JSON.stringify({
          pid: process.pid,
          acquiredAt: new Date().toISOString(),
        })}\n`);
      } catch (error) {
        await rm(lockPath, { recursive: true, force: true });
        throw error;
      }
      return async () => rm(lockPath, { recursive: true, force: true });
    } catch (error) {
      if (!isAlreadyExists(error)) throw error;
      if (Date.now() >= deadline) {
        throw new Error(`timed out waiting for project write lock at "${lockPath}"`);
      }
      await new Promise((resolveDelay) => setTimeout(resolveDelay, 10));
    }
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

function isAlreadyExists(error: unknown): boolean {
  return typeof error === 'object' && error !== null && 'code' in error && error.code === 'EEXIST';
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
