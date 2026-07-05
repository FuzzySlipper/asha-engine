#!/usr/bin/env node
import { lstatSync, readFileSync, readdirSync, statSync } from 'node:fs';
import { join, relative } from 'node:path';

const repoRoot = process.argv[2] ?? process.cwd();
const policyPath = join(repoRoot, 'harness/depgraph/ts-source-shape-policy.json');
const policy = JSON.parse(readFileSync(policyPath, 'utf8'));
const maxSourceLines = Number(policy.maxSourceLines);
const fileLineExemptions = policy.fileLineExemptions ?? {};
const rootBarrelExemptions = policy.rootBarrelExemptions ?? {};
const failures = [];

if (!Number.isSafeInteger(maxSourceLines) || maxSourceLines <= 0) {
  failures.push('FAIL: ts-source-shape-policy.json maxSourceLines must be a positive integer');
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

const packageRoot = join(repoRoot, 'ts/packages');
for (const file of walk(packageRoot)) {
  const rel = relative(repoRoot, file).replaceAll('\\', '/');
  const lineCount = readFileSync(file, 'utf8').split(/\r?\n/).length;
  const exemption = fileLineExemptions[rel];
  if (lineCount > maxSourceLines && exemption === undefined) {
    failures.push(
      `FAIL: ${rel} has ${lineCount} lines; limit is ${maxSourceLines}. ` +
        'Split the file or add a justified fileLineExemptions entry.',
    );
  }
  if (lineCount > maxSourceLines && typeof exemption === 'string' && exemption.trim().length === 0) {
    failures.push(`FAIL: ${rel} fileLineExemptions entry must include a justification.`);
  }

  if (!rel.endsWith('/src/index.ts')) {
    continue;
  }
  if (isExportsOnlyBarrel(readFileSync(file, 'utf8'))) {
    continue;
  }
  const barrelExemption = rootBarrelExemptions[rel];
  if (barrelExemption === undefined) {
    failures.push(
      `FAIL: ${rel} is a package root barrel with implementation logic. ` +
        'Move implementation into focused modules and keep src/index.ts exports-only, ' +
        'or add a justified rootBarrelExemptions entry.',
    );
    continue;
  }
  if (typeof barrelExemption !== 'string' || barrelExemption.trim().length < 20) {
    failures.push(`FAIL: ${rel} rootBarrelExemptions entry must include a specific justification.`);
  }
}

for (const rel of Object.keys(fileLineExemptions)) {
  try {
    statSync(join(repoRoot, rel));
  } catch {
    failures.push(`FAIL: stale fileLineExemptions entry for missing file ${rel}`);
  }
}

for (const rel of Object.keys(rootBarrelExemptions)) {
  try {
    statSync(join(repoRoot, rel));
  } catch {
    failures.push(`FAIL: stale rootBarrelExemptions entry for missing file ${rel}`);
  }
}

if (failures.length > 0) {
  for (const failure of failures) {
    console.error(failure);
  }
  process.exit(1);
}

console.log('TypeScript source shape check: OK');
