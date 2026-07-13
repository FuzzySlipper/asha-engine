#!/usr/bin/env node
import { lstatSync, readFileSync, readdirSync, statSync } from 'node:fs';
import { join, relative } from 'node:path';

import {
  validateBaselineChange,
  validateExemptionMetadata,
} from './ts-source-shape-policy-audit.mjs';

const repoRoot = process.argv[2] ?? process.cwd();
const policyPath = join(repoRoot, 'harness/depgraph/ts-source-shape-policy.json');
const policy = JSON.parse(readFileSync(policyPath, 'utf8'));
const maxSourceLines = Number(policy.maxSourceLines);
const warningSourceLines = Number(policy.warningSourceLines);
const fileLineExemptions = policy.fileLineExemptions ?? {};
const rootBarrelExemptions = policy.rootBarrelExemptions ?? {};
const failures = [];
const warnings = [];
const today = process.env.ASHA_SOURCE_SHAPE_TODAY ?? new Date().toISOString().slice(0, 10);
const checkedFileLineExemptions = new Set();
const checkedRootBarrelExemptions = new Set();

if (!Number.isSafeInteger(maxSourceLines) || maxSourceLines <= 0) {
  failures.push('FAIL: ts-source-shape-policy.json maxSourceLines must be a positive integer');
}
if (
  !Number.isSafeInteger(warningSourceLines) ||
  warningSourceLines <= 0 ||
  warningSourceLines >= maxSourceLines
) {
  failures.push('FAIL: ts-source-shape-policy.json warningSourceLines must be below maxSourceLines');
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
      if (name !== 'dist' && name !== 'node_modules') {
        entries.push(...walk(path));
      }
      continue;
    }
    if (path.endsWith('.ts')) {
      entries.push(path);
    }
  }
  return entries;
}

function codeLines(text) {
  return text
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter((line) => line.length > 0 && !line.startsWith('//'));
}

function isExportsOnlyBarrel(text) {
  let exportDeclarationOpen = false;
  for (const line of codeLines(text)) {
    if (exportDeclarationOpen) {
      if (line.endsWith(';')) {
        exportDeclarationOpen = false;
      }
      continue;
    }
    if (line === 'export {};') {
      continue;
    }
    if (/^export\s+\*\s+from\s+['"][^'"]+['"];?$/.test(line)) {
      continue;
    }
    if (/^export\s+(type\s+)?\{/.test(line)) {
      if (!line.endsWith(';')) {
        exportDeclarationOpen = true;
      }
      continue;
    }
    return false;
  }
  return !exportDeclarationOpen;
}

function readExemption(kind, rel, value) {
  if (value === undefined) {
    return undefined;
  }
  if (value === null || typeof value !== 'object' || Array.isArray(value)) {
    failures.push(
      `FAIL: ${rel} ${kind} entry must be an object with governed metadata.`,
    );
    return undefined;
  }
  const maxLines = value.maxLines;
  if (typeof maxLines !== 'number' || !Number.isSafeInteger(maxLines) || maxLines <= 0) {
    failures.push(`FAIL: ${rel} ${kind} entry maxLines must be a positive integer.`);
  }
  validateExemptionMetadata(kind, rel, value, failures, today);
  validateBaselineChange(kind, rel, value.baselineChange, failures);
  return { maxLines, warningLines: value.warningLines };
}

function checkExemptionBaseline(kind, rel, lineCount, value) {
  const exemption = readExemption(kind, rel, value);
  if (exemption === undefined) {
    return;
  }
  if (lineCount > exemption.maxLines) {
    failures.push(
      `FAIL: ${rel} has ${lineCount} lines; ${kind} baseline is ${exemption.maxLines}. ` +
        'Shrink the file or update the reviewed source-shape policy baseline.',
    );
  }
  if (lineCount >= exemption.warningLines) {
    warnings.push(`${rel} ${lineCount}/${exemption.maxLines}`);
  }
}

const packageRoot = join(repoRoot, 'ts/packages');
for (const file of walk(packageRoot)) {
  const rel = relative(repoRoot, file).replaceAll('\\', '/');
  const text = readFileSync(file, 'utf8');
  const lineCount = text.split(/\r?\n/).length;
  const exemption = fileLineExemptions[rel];
  if (exemption !== undefined) {
    checkedFileLineExemptions.add(rel);
    checkExemptionBaseline('fileLineExemptions', rel, lineCount, exemption);
  }
  if (lineCount > maxSourceLines && exemption === undefined) {
    failures.push(
      `FAIL: ${rel} has ${lineCount} lines; limit is ${maxSourceLines}. ` +
        'Split the file or add a justified fileLineExemptions entry.',
    );
  }
  if (exemption === undefined && lineCount >= warningSourceLines) {
    warnings.push(`${rel} ${lineCount}/${maxSourceLines}`);
  }

  if (!rel.endsWith('/src/index.ts')) {
    continue;
  }
  const barrelExemption = rootBarrelExemptions[rel];
  if (barrelExemption !== undefined) {
    checkedRootBarrelExemptions.add(rel);
    checkExemptionBaseline('rootBarrelExemptions', rel, lineCount, barrelExemption);
  }
  if (isExportsOnlyBarrel(text)) {
    continue;
  }
  if (barrelExemption === undefined) {
    failures.push(
      `FAIL: ${rel} is a package root barrel with implementation logic. ` +
        'Move implementation into focused modules and keep src/index.ts exports-only, ' +
        'or add a justified rootBarrelExemptions entry.',
    );
    continue;
  }
}

for (const rel of Object.keys(fileLineExemptions)) {
  try {
    statSync(join(repoRoot, rel));
  } catch {
    failures.push(`FAIL: stale fileLineExemptions entry for missing file ${rel}`);
  }
  if (!checkedFileLineExemptions.has(rel)) {
    readExemption('fileLineExemptions', rel, fileLineExemptions[rel]);
  }
}

for (const rel of Object.keys(rootBarrelExemptions)) {
  try {
    statSync(join(repoRoot, rel));
  } catch {
    failures.push(`FAIL: stale rootBarrelExemptions entry for missing file ${rel}`);
  }
  if (!checkedRootBarrelExemptions.has(rel)) {
    readExemption('rootBarrelExemptions', rel, rootBarrelExemptions[rel]);
  }
}

if (failures.length > 0) {
  for (const failure of failures) {
    console.error(failure);
  }
  process.exit(1);
}

if (warnings.length > 0) {
  console.warn(`WARN: TypeScript source-shape hotspots: ${warnings.sort().join('; ')}`);
}

console.log(`TypeScript source shape check: OK (${warnings.length} near-cap hotspots)`);
