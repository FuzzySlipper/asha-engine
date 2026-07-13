#!/usr/bin/env node
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { spawnSync } from 'node:child_process';

const repoRoot = resolve(process.argv[2] ?? process.cwd());
const checkerPath = join(repoRoot, 'harness/depgraph/check-ts-source-shape-policy-diff.mjs');
const tempRoot = mkdtempSync(join(tmpdir(), 'asha-ts-source-shape-policy-'));
const sourcePath = 'ts/packages/runtime-bridge/src/runtime-session.ts';
const barrelPath = 'ts/packages/app/src/index.ts';

function policy({ maxSourceLines = 1600, fileEntries = {}, barrelEntries = {} } = {}) {
  return {
    maxSourceLines,
    fileLineExemptions: fileEntries,
    rootBarrelExemptions: barrelEntries,
  };
}

function entry(maxLines, baselineChange) {
  return {
    maxLines,
    justification: 'Fixture exemption retained temporarily until its focused source split lands.',
    ...(baselineChange === undefined ? {} : { baselineChange }),
  };
}

function change(previousMaxLines, newMaxLines, overrides = {}) {
  return {
    changedAt: '2026-07-09',
    changeTask: '#5505',
    reason: 'A reviewed temporary increase is required while the focused split proceeds.',
    previousMaxLines,
    newMaxLines,
    ...overrides,
  };
}

function writePolicy(name, value) {
  const path = join(tempRoot, `${name}.json`);
  writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`);
  return path;
}

function runAudit(label, basePolicy, currentPolicy) {
  const basePath = writePolicy(`${label}-base`, basePolicy);
  const currentPath = writePolicy(`${label}-current`, currentPolicy);
  return spawnSync(
    process.execPath,
    [checkerPath, repoRoot, '--base-policy', basePath, '--current-policy', currentPath],
    { encoding: 'utf8' },
  );
}

function expectPass(label, basePolicy, currentPolicy) {
  const result = runAudit(label, basePolicy, currentPolicy);
  if (result.status !== 0) {
    throw new Error(`${label} unexpectedly failed:\n${result.stdout}${result.stderr}`);
  }
  console.log(`source-shape policy fixture OK: ${label}`);
}

function expectFailure(label, basePolicy, currentPolicy, expected) {
  const result = runAudit(label, basePolicy, currentPolicy);
  const output = `${result.stdout}${result.stderr}`;
  if (result.status === 0) {
    throw new Error(`${label} unexpectedly passed`);
  }
  if (!output.includes(expected)) {
    throw new Error(`${label} did not mention expected text '${expected}':\n${output}`);
  }
  console.log(`source-shape policy negative fixture OK: ${label}`);
}

try {
  expectPass(
    'unchanged and shrinking baselines need no ceremony',
    policy({ fileEntries: { [sourcePath]: entry(1810) } }),
    policy({ fileEntries: { [sourcePath]: entry(1800) } }),
  );

  expectPass(
    'reviewed baseline raise carries complete audit metadata',
    policy({ fileEntries: { [sourcePath]: entry(1810) } }),
    policy({ fileEntries: { [sourcePath]: entry(1820, change(1810, 1820)) } }),
  );

  expectPass(
    'reviewed new exemption records a null prior baseline',
    policy(),
    policy({ fileEntries: { [sourcePath]: entry(1620, change(null, 1620)) } }),
  );

  expectFailure(
    'silent baseline raise',
    policy({ fileEntries: { [sourcePath]: entry(1810) } }),
    policy({ fileEntries: { [sourcePath]: entry(1820) } }),
    'baseline increase requires baselineChange audit metadata',
  );

  expectFailure(
    'vague baseline raise reason',
    policy({ fileEntries: { [sourcePath]: entry(1810) } }),
    policy({
      fileEntries: {
        [sourcePath]: entry(1820, change(1810, 1820, { reason: 'needed' })),
      },
    }),
    'baselineChange.reason must explain the temporary raise',
  );

  expectFailure(
    'stale baseline raise metadata',
    policy({ fileEntries: { [sourcePath]: entry(1810, change(1800, 1810)) } }),
    policy({ fileEntries: { [sourcePath]: entry(1820, change(1800, 1810)) } }),
    'baselineChange.previousMaxLines must equal 1810',
  );

  expectFailure(
    'new source-cap exemption without an audit trail',
    policy(),
    policy({ fileEntries: { [sourcePath]: entry(1620) } }),
    'new exemption requires baselineChange audit metadata',
  );

  expectFailure(
    'new root-barrel exemption without an audit trail',
    policy(),
    policy({ barrelEntries: { [barrelPath]: entry(224) } }),
    'new exemption requires baselineChange audit metadata',
  );

  expectFailure(
    'global source cap increase',
    policy({ maxSourceLines: 1600 }),
    policy({ maxSourceLines: 1700 }),
    'global TypeScript source cap increased from 1600 to 1700',
  );
} finally {
  rmSync(tempRoot, { recursive: true, force: true });
}

console.log('TypeScript source-shape policy fixtures: OK');
