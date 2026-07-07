#!/usr/bin/env node
import { readFileSync, statSync } from 'node:fs';
import { dirname, join, relative } from 'node:path';

const repoRoot = process.argv[2] ?? process.cwd();
const packageRoot = join(repoRoot, 'ts/packages/runtime-bridge/src');
const entrypoint = join(packageRoot, 'index.ts');
const forbidden = new Set([
  'mock.ts',
  'mock-session.ts',
  'reference.ts',
  'reference-browser.ts',
  'reference-launcher.ts',
]);
const visited = new Set();
const failures = [];

function normalizeFile(path) {
  if (path.endsWith('.js')) {
    return `${path.slice(0, -3)}.ts`;
  }
  return path;
}

function resolveLocal(fromFile, specifier) {
  if (!specifier.startsWith('./') && !specifier.startsWith('../')) {
    return null;
  }
  const resolved = normalizeFile(join(dirname(fromFile), specifier));
  try {
    statSync(resolved);
    return resolved;
  } catch {
    return null;
  }
}

function stripTypeOnlySpecifiers(specifiers) {
  return specifiers
    .split(',')
    .map((specifier) => specifier.trim())
    .filter((specifier) => specifier.length > 0 && !specifier.startsWith('type '));
}

function moduleSpecifiers(text) {
  const specs = [];
  for (const match of text.matchAll(/^\s*import\s+(?!type\b)(?:[^'"]+?\s+from\s+)?['"]([^'"]+)['"];?/gm)) {
    specs.push(match[1]);
  }
  for (const match of text.matchAll(/^\s*export\s+\*\s+from\s+['"]([^'"]+)['"];?/gm)) {
    specs.push(match[1]);
  }
  for (const match of text.matchAll(/^\s*export\s+(?!type\b)\{([^}]*)\}\s+from\s+['"]([^'"]+)['"];?/gm)) {
    if (stripTypeOnlySpecifiers(match[1]).length > 0) {
      specs.push(match[2]);
    }
  }
  return specs;
}

function visit(file, via) {
  const rel = relative(packageRoot, file).replaceAll('\\', '/');
  if (forbidden.has(rel)) {
    failures.push(`FAIL: runtime-bridge root graph reaches ${rel} via ${via.join(' -> ')}`);
    return;
  }
  if (visited.has(file)) {
    return;
  }
  visited.add(file);
  const text = readFileSync(file, 'utf8');
  for (const specifier of moduleSpecifiers(text)) {
    const next = resolveLocal(file, specifier);
    if (next !== null) {
      visit(next, [...via, relative(packageRoot, next).replaceAll('\\', '/')]);
    }
  }
}

visit(entrypoint, ['index.ts']);

if (failures.length > 0) {
  for (const failure of failures) {
    console.error(failure);
  }
  process.exit(1);
}

console.log('Runtime bridge root isolation check: OK');
