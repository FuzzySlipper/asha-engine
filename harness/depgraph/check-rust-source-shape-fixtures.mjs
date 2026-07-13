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
  writeFileSync(
    policyPath,
    `${JSON.stringify({ warningSourceLines: 2, ...policy }, null, 2)}\n`,
  );
  return root;
}

function entry(maxLines, overrides = {}) {
  return {
    maxLines,
    warningLines: typeof maxLines === 'number' && maxLines > 1 ? maxLines - 1 : 1,
    owner: 'rust-foundation',
    rationale: 'Temporary fixture exception retained until its focused source split lands.',
    introducedBy: '#5761',
    reviewBy: '2026-10-15',
    reviewTrigger: 'Review when the fixture source changes or its recorded baseline changes.',
    removalCondition: 'Remove when the fixture source is split below the global source cap.',
    ...overrides,
  };
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
        [sourcePath]: entry(5),
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
    'entry must be an object with governed metadata',
  );

  expectFailure(
    'invalid Rust exemption baseline',
    makeFixture('invalid-baseline', 4, {
      maxSourceLines: 3,
      fileLineExemptions: {
        [sourcePath]: entry(0),
      },
    }),
    'entry maxLines must be a positive integer',
  );

  expectFailure(
    'string Rust exemption baseline',
    makeFixture('string-baseline', 4, {
      maxSourceLines: 3,
      fileLineExemptions: {
        [sourcePath]: entry('4'),
      },
    }),
    'entry maxLines must be a positive integer',
  );

  expectFailure(
    'vague Rust exemption rationale',
    makeFixture('vague-justification', 4, {
      maxSourceLines: 3,
      fileLineExemptions: {
        [sourcePath]: entry(4, { rationale: 'temporary' }),
      },
    }),
    'rationale must explain the structural exception',
  );

  expectFailure(
    'stale Rust exemption path',
    makeFixture('stale-exemption', 2, {
      maxSourceLines: 3,
      fileLineExemptions: {
        'engine-rs/crates/foundation/missing/src/lib.rs': entry(4),
      },
    }),
    'stale fileLineExemptions entry for missing file',
  );

  expectFailure(
    'one-line growth above Rust exemption baseline',
    makeFixture('exemption-growth', 5, {
      maxSourceLines: 3,
      fileLineExemptions: {
        [sourcePath]: entry(4),
      },
    }),
    'has 5 lines; fileLineExemptions baseline is 4',
  );

  expectFailure(
    'expired Rust exemption metadata',
    makeFixture('expired-review', 4, {
      maxSourceLines: 3,
      fileLineExemptions: {
        [sourcePath]: entry(4, { reviewBy: '2026-07-12' }),
      },
    }),
    'review metadata expired on 2026-07-12',
  );
} finally {
  rmSync(tempRoot, { recursive: true, force: true });
}

console.log('Rust source-shape fixtures: OK');
