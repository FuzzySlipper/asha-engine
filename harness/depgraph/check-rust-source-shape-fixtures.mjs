#!/usr/bin/env node
import { mkdtempSync, mkdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { spawnSync } from 'node:child_process';

const repoRoot = resolve(process.argv[2] ?? process.cwd());
const checkerPath = join(repoRoot, 'harness/depgraph/check-rust-source-shape.mjs');
const tempRoot = mkdtempSync(join(tmpdir(), 'asha-rust-source-shape-'));

function sourceLines(count) {
  return Array.from({ length: count }, (_, index) => `pub fn line_${index}() {}`).join('\n');
}

function makeFixture(name, sourceLineCount, policy) {
  const root = join(tempRoot, name);
  const sourcePath = join(root, 'engine-rs/crates/foundation/core-a/src/lib.rs');
  const policyPath = join(root, 'harness/depgraph/rust-source-shape-policy.json');
  mkdirSync(dirname(sourcePath), { recursive: true });
  mkdirSync(dirname(policyPath), { recursive: true });
  writeFileSync(sourcePath, sourceLines(sourceLineCount));
  writeFileSync(policyPath, `${JSON.stringify(policy, null, 2)}\n`);
  return root;
}

function runChecker(root) {
  return spawnSync(process.execPath, [checkerPath, root], { encoding: 'utf8' });
}

function expectPass(label, root) {
  const result = runChecker(root);
  if (result.status !== 0) {
    throw new Error(`${label} unexpectedly failed:\n${result.stdout}${result.stderr}`);
  }
  console.log(`source-shape fixture OK: ${label}`);
}

function expectFailure(label, root, expected) {
  const result = runChecker(root);
  const output = `${result.stdout}${result.stderr}`;
  if (result.status === 0) {
    throw new Error(`${label} unexpectedly passed`);
  }
  if (!output.includes(expected)) {
    throw new Error(`${label} did not mention expected text '${expected}':\n${output}`);
  }
  console.log(`source-shape negative fixture OK: ${label}`);
}

try {
  const sourcePath = 'engine-rs/crates/foundation/core-a/src/lib.rs';
  expectPass(
    'exempt source may shrink below its baseline',
    makeFixture('shrink-pass', 4, {
      maxSourceLines: 3,
      fileLineExemptions: {
        [sourcePath]: {
          maxLines: 5,
          justification: 'Temporary fixture exemption with explicit shrink-only headroom.',
        },
      },
    }),
  );

  expectFailure(
    'unlisted oversized Rust source',
    makeFixture('unlisted-oversize', 4, {
      maxSourceLines: 3,
      fileLineExemptions: {},
    }),
    'has 4 lines; limit is 3',
  );

  expectFailure(
    'malformed Rust exemption',
    makeFixture('malformed-exemption', 4, {
      maxSourceLines: 3,
      fileLineExemptions: {
        [sourcePath]: 'legacy prose-only exemption',
      },
    }),
    'entry must be an object with maxLines and justification fields',
  );

  expectFailure(
    'invalid Rust exemption baseline',
    makeFixture('invalid-baseline', 4, {
      maxSourceLines: 3,
      fileLineExemptions: {
        [sourcePath]: {
          maxLines: 0,
          justification: 'This fixture deliberately supplies an invalid numeric baseline.',
        },
      },
    }),
    'entry maxLines must be a positive integer',
  );

  expectFailure(
    'string Rust exemption baseline',
    makeFixture('string-baseline', 4, {
      maxSourceLines: 3,
      fileLineExemptions: {
        [sourcePath]: {
          maxLines: '4',
          justification: 'This fixture proves numeric-looking strings are not numeric baselines.',
        },
      },
    }),
    'entry maxLines must be a positive integer',
  );

  expectFailure(
    'vague Rust exemption justification',
    makeFixture('vague-justification', 4, {
      maxSourceLines: 3,
      fileLineExemptions: {
        [sourcePath]: {
          maxLines: 4,
          justification: 'temporary',
        },
      },
    }),
    'entry must include a specific justification',
  );

  expectFailure(
    'stale Rust exemption path',
    makeFixture('stale-exemption', 2, {
      maxSourceLines: 3,
      fileLineExemptions: {
        'engine-rs/crates/foundation/missing/src/lib.rs': {
          maxLines: 4,
          justification: 'This fixture entry deliberately points at a missing Rust source file.',
        },
      },
    }),
    'stale fileLineExemptions entry for missing file',
  );

  expectFailure(
    'one-line growth above Rust exemption baseline',
    makeFixture('exemption-growth', 5, {
      maxSourceLines: 3,
      fileLineExemptions: {
        [sourcePath]: {
          maxLines: 4,
          justification: 'Temporary fixture exemption whose numeric baseline must not grow.',
        },
      },
    }),
    'has 5 lines; fileLineExemptions baseline is 4',
  );
} finally {
  rmSync(tempRoot, { recursive: true, force: true });
}

console.log('Rust source-shape fixtures: OK');
