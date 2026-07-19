import { lstat, readFile, realpath } from 'node:fs/promises';
import { isAbsolute, relative, resolve } from 'node:path';

import {
  type AshaProjectSourceKind,
  type AshaProjectSourceReader,
} from '@asha/game-workspace';

export type AshaProjectDirectorySourceKind = Extract<
  AshaProjectSourceKind,
  'development-directory' | 'packaged-directory'
>;

/**
 * Trusted Node host adapter for ordinary project directories. It reads a path
 * only when the shared manifest-driven loader asks for it; no directory crawl
 * or host-authored role table is involved.
 */
export async function createAshaProjectDirectorySource(
  root: string,
  kind: AshaProjectDirectorySourceKind = 'development-directory',
): Promise<AshaProjectSourceReader> {
  const canonicalRoot = await realpath(resolve(root));
  const rootStat = await lstat(canonicalRoot);
  if (!rootStat.isDirectory()) throw new Error(`ASHA project source root is not a directory: ${root}`);
  return {
    kind,
    identity: `${kind}:${canonicalRoot}`,
    read: async (relativePath) => {
      assertRelativeSourcePath(relativePath);
      const requested = resolve(canonicalRoot, relativePath);
      assertInsideRoot(canonicalRoot, requested);
      const canonicalFile = await realpath(requested);
      assertInsideRoot(canonicalRoot, canonicalFile);
      const fileStat = await lstat(canonicalFile);
      if (!fileStat.isFile()) throw new Error(`project source path is not a file: ${relativePath}`);
      const bytes = await readFile(canonicalFile);
      return new Uint8Array(bytes.buffer, bytes.byteOffset, bytes.byteLength).slice();
    },
  };
}

function assertRelativeSourcePath(path: string): void {
  if (
    path.length === 0
    || isAbsolute(path)
    || path.includes('\\')
    || path.split('/').some((segment) => segment.length === 0 || segment === '.' || segment === '..')
  ) {
    throw new Error(`invalid project source path: ${path}`);
  }
}

function assertInsideRoot(root: string, target: string): void {
  const relation = relative(root, target);
  if (relation === '..' || relation.startsWith('../') || isAbsolute(relation)) {
    throw new Error(`project source path escapes root: ${target}`);
  }
}
