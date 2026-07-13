#!/usr/bin/env node
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { spawnSync } from 'node:child_process';

import { auditRustSourceShapePolicy } from './ts-source-shape-policy-audit.mjs';

const POLICY_REL = 'harness/depgraph/rust-source-shape-policy.json';
const args = process.argv.slice(2);
const repoRoot = resolve(args.shift() ?? process.cwd());
let basePolicyPath;
let currentPolicyPath = resolve(repoRoot, POLICY_REL);
let baseRef = normalizeBaseRef(process.env.ASHA_SOURCE_SHAPE_BASE_REF);

while (args.length > 0) {
  const option = args.shift();
  const value = args.shift();
  if (value === undefined) {
    throw new Error(`${option} requires a value`);
  }
  if (option === '--base-policy') {
    basePolicyPath = resolve(value);
  } else if (option === '--current-policy') {
    currentPolicyPath = resolve(value);
  } else if (option === '--base-ref') {
    baseRef = normalizeBaseRef(value);
  } else {
    throw new Error(`unknown option ${option}`);
  }
}

function normalizeBaseRef(value) {
  const trimmed = value?.trim();
  if (!trimmed || /^0+$/u.test(trimmed)) {
    return undefined;
  }
  return trimmed;
}

function runGit(gitArgs) {
  return spawnSync('git', gitArgs, { cwd: repoRoot, encoding: 'utf8' });
}

function inferBaseRef() {
  const workingTreeDiff = runGit(['diff', '--quiet', 'HEAD', '--', POLICY_REL]);
  if (workingTreeDiff.status === 1) {
    return 'HEAD';
  }
  const parent = runGit(['rev-parse', '--verify', 'HEAD^']);
  return parent.status === 0 ? 'HEAD^' : undefined;
}

function readPolicyAtRef(ref) {
  const result = runGit(['show', `${ref}:${POLICY_REL}`]);
  if (result.status !== 0) {
    throw new Error(`cannot read Rust source-shape policy at ${ref}: ${result.stderr.trim()}`);
  }
  return JSON.parse(result.stdout);
}

const currentPolicy = JSON.parse(readFileSync(currentPolicyPath, 'utf8'));
let basePolicy;
let baseLabel;
if (basePolicyPath !== undefined) {
  basePolicy = JSON.parse(readFileSync(basePolicyPath, 'utf8'));
  baseLabel = basePolicyPath;
} else {
  const selectedBaseRef = baseRef ?? inferBaseRef();
  if (selectedBaseRef === undefined) {
    console.log('Rust source-shape policy audit skipped: no base revision is available.');
    process.exit(0);
  }
  basePolicy = readPolicyAtRef(selectedBaseRef);
  baseLabel = selectedBaseRef;
}

const failures = [];
auditRustSourceShapePolicy(basePolicy, currentPolicy, failures);
if (failures.length > 0) {
  for (const failure of failures) {
    console.error(failure);
  }
  process.exit(1);
}

console.log(`Rust source-shape policy audit: OK (base ${baseLabel})`);
