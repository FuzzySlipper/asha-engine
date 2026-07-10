#!/usr/bin/env node
import { lstatSync, readFileSync, readdirSync, statSync } from 'node:fs';
import { join, relative } from 'node:path';

const repoRoot = process.argv[2] ?? process.cwd();
const policyPath = join(repoRoot, 'harness/depgraph/rust-source-shape-policy.json');
const policy = JSON.parse(readFileSync(policyPath, 'utf8'));
const maxSourceLines = Number(policy.maxSourceLines);
const failures = [];
const rawFileLineExemptions = policy.fileLineExemptions ?? {};
const fileLineExemptions = readExemptionMap(rawFileLineExemptions);
const checkedFileLineExemptions = new Set();

if (!Number.isSafeInteger(maxSourceLines) || maxSourceLines <= 0) {
  failures.push('FAIL: rust-source-shape-policy.json maxSourceLines must be a positive integer');
}

function readExemptionMap(value) {
  if (value === null || typeof value !== 'object' || Array.isArray(value)) {
    failures.push('FAIL: rust-source-shape-policy.json fileLineExemptions must be an object');
    return {};
  }
  return value;
}

function readExemption(rel, value) {
  if (value === null || typeof value !== 'object' || Array.isArray(value)) {
    failures.push(
      `FAIL: ${rel} fileLineExemptions entry must be an object with maxLines and justification fields.`,
    );
    return undefined;
  }
  const maxLines = value.maxLines;
  if (typeof maxLines !== 'number' || !Number.isSafeInteger(maxLines) || maxLines <= 0) {
    failures.push(`FAIL: ${rel} fileLineExemptions entry maxLines must be a positive integer.`);
  }
  if (typeof value.justification !== 'string' || value.justification.trim().length < 20) {
    failures.push(`FAIL: ${rel} fileLineExemptions entry must include a specific justification.`);
  }
  return { maxLines };
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
  if (exemption !== undefined) {
    checkedFileLineExemptions.add(rel);
    const parsedExemption = readExemption(rel, exemption);
    if (parsedExemption !== undefined && lineCount > parsedExemption.maxLines) {
      failures.push(
        `FAIL: ${rel} has ${lineCount} lines; fileLineExemptions baseline is ` +
          `${parsedExemption.maxLines}. Shrink the file or update the reviewed Rust ` +
          'source-shape policy baseline.',
      );
    }
  }
  if (lineCount > maxSourceLines && exemption === undefined) {
    failures.push(
      `FAIL: ${rel} has ${lineCount} lines; limit is ${maxSourceLines}. ` +
        'Split the file or add a justified fileLineExemptions entry.',
    );
  }
}

for (const rel of Object.keys(fileLineExemptions)) {
  try {
    statSync(join(repoRoot, rel));
  } catch {
    failures.push(`FAIL: stale fileLineExemptions entry for missing file ${rel}`);
  }
  if (!checkedFileLineExemptions.has(rel)) {
    readExemption(rel, fileLineExemptions[rel]);
  }
}

if (failures.length > 0) {
  for (const failure of failures) {
    console.error(failure);
  }
  process.exit(1);
}

console.log('Rust source shape check: OK');
