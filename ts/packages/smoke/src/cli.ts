// asha-smoke CLI — runs the smoke harness, writes structured artifacts, and exits
// non-zero on failure so CI/agents get an unambiguous pass/fail signal (#2395/#2398).
//
// Modes (#2424):
//   pnpm --filter @asha/smoke dev:asha-smoke                  → reference (mock) smoke
//   ASHA_SMOKE_MODE=authority pnpm --filter @asha/smoke dev:asha-smoke → real authority path
// The authority mode fails closed (non-zero) when the native addon is unavailable;
// it is never silently downgraded to a mock success.

import { mkdirSync, writeFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { bootForMode, runSmoke } from './harness.js';
import { formatResult, type SmokeMode } from './result.js';

/** Repo-relative artifact directory (harness convention; Den can link to it). */
function artifactDir(): string {
  // dist/cli.js → package dir is two up; repo root is four up from there.
  const here = dirname(fileURLToPath(import.meta.url));
  return resolve(here, '../../../../harness/smoke-out');
}

function selectedMode(): SmokeMode {
  return process.env['ASHA_SMOKE_MODE'] === 'authority' ? 'authority' : 'reference';
}

async function main(): Promise<void> {
  const mode = selectedMode();
  const result = await runSmoke({ bootBridge: () => bootForMode(mode) });
  const text = formatResult(result);
  process.stdout.write(text);

  const dir = artifactDir();
  mkdirSync(dir, { recursive: true });
  const suffix = mode === 'authority' ? '-authority' : '';
  writeFileSync(resolve(dir, `asha-smoke${suffix}.txt`), text);
  writeFileSync(resolve(dir, `asha-smoke${suffix}.json`), JSON.stringify(result, null, 2) + '\n');
  process.stdout.write(`artifacts: ${dir}/asha-smoke${suffix}.{txt,json}\n`);

  process.exit(result.ok ? 0 : 1);
}

await main();
