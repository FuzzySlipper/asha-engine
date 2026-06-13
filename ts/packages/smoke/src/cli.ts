// asha-smoke CLI — runs the smoke harness, writes structured artifacts, and exits
// non-zero on failure so CI/agents get an unambiguous pass/fail signal (#2395/#2398).

import { mkdirSync, writeFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { runSmoke } from './harness.js';
import { formatResult } from './result.js';

/** Repo-relative artifact directory (harness convention; Den can link to it). */
function artifactDir(): string {
  // dist/cli.js → package dir is two up; repo root is four up from there.
  const here = dirname(fileURLToPath(import.meta.url));
  return resolve(here, '../../../../harness/smoke-out');
}

function main(): void {
  const result = runSmoke();
  const text = formatResult(result);
  process.stdout.write(text);

  const dir = artifactDir();
  mkdirSync(dir, { recursive: true });
  writeFileSync(resolve(dir, 'asha-smoke.txt'), text);
  writeFileSync(resolve(dir, 'asha-smoke.json'), JSON.stringify(result, null, 2) + '\n');
  process.stdout.write(`artifacts: ${dir}/asha-smoke.{txt,json}\n`);

  process.exit(result.ok ? 0 : 1);
}

main();
