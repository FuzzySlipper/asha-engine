import {
  validateGeneratedWireValue,
  type GeneratedWireValue,
  type ProjectBundleManifest,
} from '@asha/contracts';

export type {
  ProjectArtifactExpectation,
  ProjectBundleManifest,
  ProjectStoreIdentity,
  ProjectWriteCandidate,
  ProjectWritePublication,
  ProjectWriteResourceRef,
} from '@asha/contracts';

export const ASHA_PROJECT_BUNDLE_MANIFEST_PATH = 'asha.project-bundle.json';
export const ASHA_PROJECT_PACKAGE_MAGIC = 'ASHAPKG2';
export const ASHA_PROJECT_SOURCE_MAX_FILES = 16_384;
export const ASHA_PROJECT_SOURCE_MAX_TOTAL_BYTES = 512 * 1024 * 1024;
export const ASHA_PROJECT_SOURCE_MAX_PATH_BYTES = 4_096;

export type AshaProjectSourceKind =
  | 'development-directory'
  | 'packaged-directory'
  | 'packaged-archive'
  | 'memory';

/**
 * A deliberately small host adapter. The manifest owns the file closure;
 * readers do not supply roles, hashes, or a second path manifest.
 */
export interface AshaProjectSourceReader {
  readonly kind: AshaProjectSourceKind;
  readonly identity: string;
  read(relativePath: string): Promise<Uint8Array>;
}

export interface AshaProjectSourceFile {
  readonly path: string;
  readonly bytes: Uint8Array;
}

export interface AshaLoadedProjectSource {
  readonly sourceKind: AshaProjectSourceKind;
  readonly sourceIdentity: string;
  readonly manifestJson: string;
  readonly manifest: ProjectBundleManifest;
  readonly files: readonly AshaProjectSourceFile[];
  readonly materializationHash: string;
}

/** Read exactly the canonical manifest and the paths it declares. */
export async function loadAshaProjectSource(
  reader: AshaProjectSourceReader,
): Promise<AshaLoadedProjectSource> {
  const manifestBytes = await reader.read(ASHA_PROJECT_BUNDLE_MANIFEST_PATH);
  const manifestJson = decodeUtf8(manifestBytes, ASHA_PROJECT_BUNDLE_MANIFEST_PATH);
  const manifest = decodeProjectBundleManifest(manifestJson);
  if (manifest.artifacts.length > ASHA_PROJECT_SOURCE_MAX_FILES) {
    throw new Error(`ProjectBundle declares too many artifacts: ${manifest.artifacts.length}`);
  }
  const seen = new Set<string>();
  let totalBytes = manifestBytes.byteLength;
  const files: AshaProjectSourceFile[] = [];
  // Cache rows are inspectable manifest metadata, not runtime source closure.
  // Rust admission requires exactly durable/generated bodies and rejects a
  // cache body whose manifest intentionally carries no authority hash.
  for (const artifact of manifest.artifacts.filter((entry) => entry.class !== 'cache')) {
    assertCanonicalProjectPath(artifact.path);
    if (seen.has(artifact.path)) {
      throw new Error(`ProjectBundle repeats artifact path "${artifact.path}"`);
    }
    seen.add(artifact.path);
    const bytes = await reader.read(artifact.path);
    totalBytes += bytes.byteLength;
    if (totalBytes > ASHA_PROJECT_SOURCE_MAX_TOTAL_BYTES) {
      throw new Error('ProjectBundle source exceeds the shared adapter byte limit');
    }
    files.push({ path: artifact.path, bytes: bytes.slice() });
  }
  files.sort((left, right) => compareAshaProjectPaths(left.path, right.path));
  return {
    sourceKind: reader.kind,
    sourceIdentity: reader.identity,
    manifestJson,
    manifest,
    files,
    materializationHash: hashMaterialization(manifestBytes, files),
  };
}

export function createMemoryAshaProjectSource(
  identity: string,
  files: ReadonlyMap<string, Uint8Array>,
): AshaProjectSourceReader {
  const owned = new Map<string, Uint8Array>();
  for (const [path, bytes] of files) {
    assertCanonicalProjectPath(path, path === ASHA_PROJECT_BUNDLE_MANIFEST_PATH);
    if (owned.has(path)) throw new Error(`memory project source repeats "${path}"`);
    owned.set(path, bytes.slice());
  }
  return {
    kind: 'memory',
    identity,
    read: async (relativePath) => {
      const value = owned.get(relativePath);
      if (value === undefined) throw new Error(`project source is missing "${relativePath}"`);
      return value.slice();
    },
  };
}

/**
 * Deterministic archive wrapper around the canonical directory files. This is
 * a transport container only: it carries no roles or runtime topology.
 */
export function encodeAshaProjectPackage(
  files: ReadonlyMap<string, Uint8Array>,
): Uint8Array {
  const entries = [...files.entries()]
    .map(([path, bytes]) => {
      assertCanonicalProjectPath(path, path === ASHA_PROJECT_BUNDLE_MANIFEST_PATH);
      return { path, pathBytes: encodeUtf8(path), bytes };
    })
    .sort((left, right) => compareAshaProjectPaths(left.path, right.path));
  if (entries.length > ASHA_PROJECT_SOURCE_MAX_FILES + 1) {
    throw new Error('project package contains too many files');
  }
  const seen = new Set<string>();
  let byteLength = 8 + 4;
  let contentBytes = 0;
  for (const entry of entries) {
    if (seen.has(entry.path)) throw new Error(`project package repeats "${entry.path}"`);
    seen.add(entry.path);
    if (entry.pathBytes.byteLength > ASHA_PROJECT_SOURCE_MAX_PATH_BYTES) {
      throw new Error(`project package path is too long: "${entry.path}"`);
    }
    contentBytes += entry.bytes.byteLength;
    if (contentBytes > ASHA_PROJECT_SOURCE_MAX_TOTAL_BYTES) {
      throw new Error('project package exceeds the shared adapter byte limit');
    }
    byteLength += 4 + 8 + entry.pathBytes.byteLength + entry.bytes.byteLength;
  }
  const output = new Uint8Array(byteLength);
  output.set(encodeUtf8(ASHA_PROJECT_PACKAGE_MAGIC), 0);
  const view = new DataView(output.buffer, output.byteOffset, output.byteLength);
  view.setUint32(8, entries.length, true);
  let cursor = 12;
  for (const entry of entries) {
    view.setUint32(cursor, entry.pathBytes.byteLength, true);
    view.setBigUint64(cursor + 4, BigInt(entry.bytes.byteLength), true);
    cursor += 12;
    output.set(entry.pathBytes, cursor);
    cursor += entry.pathBytes.byteLength;
    output.set(entry.bytes, cursor);
    cursor += entry.bytes.byteLength;
  }
  return output;
}

export function createPackagedAshaProjectSource(
  identity: string,
  archive: Uint8Array,
): AshaProjectSourceReader {
  const files = decodeAshaProjectPackage(archive);
  const memory = createMemoryAshaProjectSource(identity, files);
  return {
    kind: 'packaged-archive',
    identity,
    read: memory.read,
  };
}

export function decodeAshaProjectPackage(archive: Uint8Array): ReadonlyMap<string, Uint8Array> {
  if (archive.byteLength < 12 || decodeUtf8(archive.subarray(0, 8), 'archive magic') !== ASHA_PROJECT_PACKAGE_MAGIC) {
    throw new Error('invalid ASHA project package magic');
  }
  const view = new DataView(archive.buffer, archive.byteOffset, archive.byteLength);
  const count = view.getUint32(8, true);
  if (count > ASHA_PROJECT_SOURCE_MAX_FILES + 1) throw new Error('project package contains too many files');
  let cursor = 12;
  let totalBytes = 0;
  const files = new Map<string, Uint8Array>();
  for (let index = 0; index < count; index += 1) {
    if (cursor + 12 > archive.byteLength) throw new Error('truncated ASHA project package header');
    const pathLength = view.getUint32(cursor, true);
    const bodyLengthBig = view.getBigUint64(cursor + 4, true);
    cursor += 12;
    if (pathLength === 0 || pathLength > ASHA_PROJECT_SOURCE_MAX_PATH_BYTES) {
      throw new Error('invalid ASHA project package path length');
    }
    if (bodyLengthBig > BigInt(Number.MAX_SAFE_INTEGER)) throw new Error('ASHA project package body is too large');
    const bodyLength = Number(bodyLengthBig);
    const end = cursor + pathLength + bodyLength;
    if (!Number.isSafeInteger(end) || end > archive.byteLength) throw new Error('truncated ASHA project package body');
    const path = decodeUtf8(archive.subarray(cursor, cursor + pathLength), 'archive path');
    cursor += pathLength;
    assertCanonicalProjectPath(path, path === ASHA_PROJECT_BUNDLE_MANIFEST_PATH);
    if (files.has(path)) throw new Error(`project package repeats "${path}"`);
    totalBytes += bodyLength;
    if (totalBytes > ASHA_PROJECT_SOURCE_MAX_TOTAL_BYTES) {
      throw new Error('project package exceeds the shared adapter byte limit');
    }
    files.set(path, archive.slice(cursor, cursor + bodyLength));
    cursor += bodyLength;
  }
  if (cursor !== archive.byteLength) throw new Error('ASHA project package has trailing bytes');
  return files;
}

function decodeProjectBundleManifest(manifestJson: string): ProjectBundleManifest {
  const parsed: unknown = JSON.parse(manifestJson);
  if (!isGeneratedWireValue(parsed)) throw new Error('ProjectBundle manifest is not a JSON wire value');
  const validation = validateGeneratedWireValue('projectBundle.ProjectBundleManifest', parsed);
  if (!validation.valid) {
    throw new Error(`ProjectBundle manifest ${validation.issue.path}: ${validation.issue.message}`);
  }
  return parsed as unknown as ProjectBundleManifest;
}

function isGeneratedWireValue(value: unknown): value is GeneratedWireValue {
  if (value === null || typeof value === 'boolean' || typeof value === 'string') return true;
  if (typeof value === 'number') return Number.isFinite(value);
  if (Array.isArray(value)) return value.every(isGeneratedWireValue);
  if (typeof value !== 'object') return false;
  return Object.values(value).every(isGeneratedWireValue);
}

function assertCanonicalProjectPath(path: string, allowManifest = false): void {
  const segments = path.split('/');
  const encoded = encodeUtf8(path);
  if (
    path.length === 0
    || path.startsWith('/')
    || path.includes('\\')
    || segments.some((segment) => segment.length === 0 || segment === '.' || segment === '..')
    || encoded.byteLength > ASHA_PROJECT_SOURCE_MAX_PATH_BYTES
    || (!allowManifest && path === ASHA_PROJECT_BUNDLE_MANIFEST_PATH)
  ) {
    throw new Error(`invalid canonical project-relative path "${path}"`);
  }
}

function hashMaterialization(manifestBytes: Uint8Array, files: readonly AshaProjectSourceFile[]): string {
  let hash = fnv1a64(manifestBytes);
  for (const file of files) {
    hash = fnv1a64(encodeUtf8(`\n${file.path}\n`), hash);
    hash = fnv1a64(file.bytes, hash);
  }
  return `fnv1a64:${hash.toString(16).padStart(16, '0')}`;
}

function fnv1a64(bytes: Uint8Array, initial = 14_695_981_039_346_656_037n): bigint {
  let hash = initial;
  for (const byte of bytes) {
    hash ^= BigInt(byte);
    hash = BigInt.asUintN(64, hash * 1_099_511_628_211n);
  }
  return hash;
}

function encodeUtf8(value: string): Uint8Array {
  return new TextEncoder().encode(value);
}

/** Match Rust `str` ordering by comparing encoded bytes, not host locale. */
export function compareAshaProjectPaths(left: string, right: string): number {
  const leftBytes = encodeUtf8(left);
  const rightBytes = encodeUtf8(right);
  const sharedLength = Math.min(leftBytes.byteLength, rightBytes.byteLength);
  for (let index = 0; index < sharedLength; index += 1) {
    const difference = leftBytes[index]! - rightBytes[index]!;
    if (difference !== 0) return difference;
  }
  return leftBytes.byteLength - rightBytes.byteLength;
}

function decodeUtf8(value: Uint8Array, label: string): string {
  try {
    return new TextDecoder('utf-8', { fatal: true }).decode(value);
  } catch {
    throw new Error(`${label} is not valid UTF-8`);
  }
}
