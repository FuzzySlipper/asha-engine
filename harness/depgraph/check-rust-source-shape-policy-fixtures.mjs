#!/usr/bin/env node
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { spawnSync } from 'node:child_process';

const repoRoot = resolve(process.argv[2] ?? process.cwd());
const checkerPath = join(repoRoot, 'harness/depgraph/check-rust-source-shape-policy-diff.mjs');
const tempRoot = mkdtempSync(join(tmpdir(), 'asha-rust-source-shape-policy-'));
const sourcePath = 'engine-rs/crates/rules/gameplay-runtime-host/src/lib.rs';

function entry(maxLines, baselineChange) {
  return {
    maxLines,
    warningLines: maxLines - 40,
    owner: 'rust-rule',
    rationale: 'Fixture exception retained while the public host is split into focused modules.',
    introducedBy: '#5761',
    reviewBy: '2026-10-15',
    reviewTrigger: 'Review when the host changes or the source baseline is adjusted.',
    removalCondition: 'Remove after the fixture host is split below the global source cap.',
    ...(baselineChange === undefined ? {} : { baselineChange }),
  };
}

function policy(entries = {}, maxSourceLines = 1600) {
  return { maxSourceLines, warningSourceLines: 1400, fileLineExemptions: entries };
}

function change(previousMaxLines, newMaxLines) {
  return {
    changedAt: '2026-07-13',
    changeTask: '#5761',
    reason: 'A reviewed exact baseline is required while the focused source split proceeds.',
    previousMaxLines,
    newMaxLines,
  };
}

function writePolicy(name, value) {
  const path = join(tempRoot, `${name}.json`);
  writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`);
  return path;
}

function runAudit(label, basePolicy, currentPolicy) {
  return spawnSync(
    process.execPath,
    [
      checkerPath,
      repoRoot,
      '--base-policy',
      writePolicy(`${label}-base`, basePolicy),
      '--current-policy',
      writePolicy(`${label}-current`, currentPolicy),
    ],
    { encoding: 'utf8' },
  );
}

function expect(label, basePolicy, currentPolicy, expectedStatus, expectedText = undefined) {
  const result = runAudit(label, basePolicy, currentPolicy);
  const output = `${result.stdout}${result.stderr}`;
  if (result.status !== expectedStatus || (expectedText && !output.includes(expectedText))) {
    throw new Error(`${label} produced unexpected result:\n${output}`);
  }
  console.log(`Rust source-shape policy fixture OK: ${label}`);
}

try {
  expect(
    'reviewed new exemption',
    policy(),
    policy({ [sourcePath]: entry(2795, change(null, 2795)) }),
    0,
  );
  expect(
    'unreviewed cap raise',
    policy({ [sourcePath]: entry(2795) }),
    policy({ [sourcePath]: entry(2800) }),
    1,
    'baseline increase requires baselineChange audit metadata',
  );
  expect(
    'global cap raise',
    policy({}, 1600),
    policy({}, 1700),
    1,
    'global Rust source cap increased from 1600 to 1700',
  );
} finally {
  rmSync(tempRoot, { recursive: true, force: true });
}

console.log('Rust source-shape policy fixtures: OK');
