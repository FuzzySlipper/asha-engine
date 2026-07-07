#!/usr/bin/env node
import { lstatSync, readFileSync, readdirSync, statSync } from 'node:fs';
import { join, relative } from 'node:path';

const repoRoot = process.argv[2] ?? process.cwd();
const policyPath = join(repoRoot, 'harness/depgraph/rust-source-shape-policy.json');
const policy = JSON.parse(readFileSync(policyPath, 'utf8'));
const maxSourceLines = Number(policy.maxSourceLines);
const fileLineExemptions = policy.fileLineExemptions ?? {};
const failures = [];

if (!Number.isSafeInteger(maxSourceLines) || maxSourceLines <= 0) {
  failures.push('FAIL: rust-source-shape-policy.json maxSourceLines must be a positive integer');
}

function walk(dir) {
  const entries = [];
  for (const name of readdirSync(dir)) {
    const path = join(dir, name);
    const linkStat = lstatSync(path);
    if (linkStat.isSymbolicLink()) {
      continue;
    }
    const stat = statSync(path);
    if (stat.isDirectory()) {
      if (name !== 'target') {
        entries.push(...walk(path));
      }
      continue;
    }
    if (path.endsWith('.rs')) {
      entries.push(path);
    }
  }
  return entries;
}

const rustRoot = join(repoRoot, 'engine-rs');
for (const file of walk(rustRoot)) {
  const rel = relative(repoRoot, file).replaceAll('\\', '/');
  const lineCount = readFileSync(file, 'utf8').split(/\r?\n/).length;
  const exemption = fileLineExemptions[rel];
  if (lineCount > maxSourceLines && exemption === undefined) {
    failures.push(
      `FAIL: ${rel} has ${lineCount} lines; limit is ${maxSourceLines}. ` +
        'Split the file or add a justified fileLineExemptions entry.',
    );
  }
  if (lineCount > maxSourceLines && typeof exemption === 'string' && exemption.trim().length < 20) {
    failures.push(`FAIL: ${rel} fileLineExemptions entry must include a specific justification.`);
  }
}

for (const rel of Object.keys(fileLineExemptions)) {
  try {
    statSync(join(repoRoot, rel));
  } catch {
    failures.push(`FAIL: stale fileLineExemptions entry for missing file ${rel}`);
  }
}

if (failures.length > 0) {
  for (const failure of failures) {
    console.error(failure);
  }
  process.exit(1);
}

console.log('Rust source shape check: OK');
