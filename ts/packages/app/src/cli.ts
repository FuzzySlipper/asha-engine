// asha-shell launch CLI — composes the shared app shell, drives load → projection, and
// writes a deterministic readout artifact (task #2439). This is the documented CI-safe
// composition target: it proves runtime + renderer + UI + devtools assemble into ONE
// navigable shell without a real window.
//
// Modes:
//   pnpm --filter @asha/app dev:asha-shell                       → reference (mock) shell
//   ASHA_SHELL_MODE=authority pnpm --filter @asha/app dev:asha-shell → real native path
// Authority mode reports `unavailable` (non-zero exit) when the native addon is missing;
// it is never silently downgraded to a mock success.

import { mkdirSync, writeFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { formatReadout } from './shell.js';
import { runHeadlessLaunch, type LaunchMode } from './launch.js';

/** Repo-relative artifact directory (harness convention; Den can link to it). */
function artifactDir(): string {
  // dist/cli.js → package dir is two up; repo root is four up from there.
  const here = dirname(fileURLToPath(import.meta.url));
  return resolve(here, '../../../../harness/shell-out');
}

function selectedMode(): LaunchMode {
  return process.env['ASHA_SHELL_MODE'] === 'authority' ? 'authority' : 'reference';
}

function main(): void {
  const mode = selectedMode();
  const readout = runHeadlessLaunch({ mode });
  const text = formatReadout(readout);
  process.stdout.write(text);

  const dir = artifactDir();
  mkdirSync(dir, { recursive: true });
  const suffix = mode === 'authority' ? '-authority' : '';
  writeFileSync(resolve(dir, `asha-shell${suffix}.txt`), text);
  writeFileSync(resolve(dir, `asha-shell${suffix}.json`), JSON.stringify(readout, null, 2) + '\n');
  process.stdout.write(`artifacts: ${dir}/asha-shell${suffix}.{txt,json}\n`);

  // The shell "launched"; only a truly unavailable runtime (boot failed closed) is a
  // non-zero exit. A visible `degraded` still launched and is reported honestly.
  process.exit(readout.runtime.availability === 'unavailable' ? 1 : 0);
}

main();
