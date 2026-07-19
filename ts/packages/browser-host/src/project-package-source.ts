import { lstat, readFile, realpath } from 'node:fs/promises';
import { resolve } from 'node:path';

import {
  createPackagedAshaProjectSource,
  type AshaProjectSourceReader,
} from '@asha/game-workspace';

/** Trusted Node host adapter for one deterministic `.asha` package file. The
 * archive remains a transport container; its canonical ProjectBundle manifest
 * still determines every source path read by `loadProject({ source })`. */
export async function createAshaProjectPackageFileSource(
  packagePath: string,
): Promise<AshaProjectSourceReader> {
  const canonicalPath = await realpath(resolve(packagePath));
  const stat = await lstat(canonicalPath);
  if (!stat.isFile()) throw new Error(`ASHA packaged project is not a file: ${packagePath}`);
  const archive = await readFile(canonicalPath);
  const bytes = new Uint8Array(archive.buffer, archive.byteOffset, archive.byteLength).slice();
  return createPackagedAshaProjectSource(`packaged-project:${canonicalPath}`, bytes);
}
