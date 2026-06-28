import { mkdir, readFile, rm, writeFile } from 'node:fs/promises';
import { basename, resolve } from 'node:path';
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';

const root = resolve(import.meta.dirname, '..');
const outDir = resolve(root, 'artifacts/public-packages');

const publicPackages = [
  '@asha/contracts',
  '@asha/runtime-bridge',
  '@asha/devtools',
  '@asha/game-workspace',
];

function packageDir(packageName) {
  return resolve(root, 'packages', packageName.replace('@asha/', ''));
}

function run(command, args) {
  execFileSync(command, args, { cwd: root, stdio: 'inherit' });
}

async function readJson(path) {
  return JSON.parse(await readFile(path, 'utf8'));
}

async function sha256(path) {
  const data = await readFile(path);
  return createHash('sha256').update(data).digest('hex');
}

await rm(outDir, { recursive: true, force: true });
await mkdir(outDir, { recursive: true });

const artifacts = [];

for (const packageName of publicPackages) {
  run('pnpm', ['--filter', packageName, 'build']);
  const dir = packageDir(packageName);
  const packageJson = await readJson(resolve(dir, 'package.json'));
  run('pnpm', ['--filter', packageName, 'pack', '--pack-destination', outDir]);

  const tarballName = `${packageName.replace('@', '').replace('/', '-')}-${packageJson.version}.tgz`;
  const tarballPath = resolve(outDir, tarballName);
  const compatibilityPath = resolve(dir, 'compatibility.json');
  let compatibility = null;
  try {
    compatibility = await readJson(compatibilityPath);
  } catch (error) {
    if (error && typeof error === 'object' && 'code' in error && error.code !== 'ENOENT') {
      throw error;
    }
  }

  artifacts.push({
    package: packageName,
    version: packageJson.version,
    tarball: basename(tarballPath),
    sha256: await sha256(tarballPath),
    compatibility,
  });
}

const manifest = {
  schemaVersion: 1,
  generatedAt: new Date().toISOString(),
  artifactKind: 'asha_public_package_bundle',
  packages: artifacts,
  consumption: {
    localInstall: 'Use the package tarballs from artifacts/public-packages/ as file: dependencies in a downstream game repo.',
    privateImportPolicy: 'Downstream consumers must import package roots only and must not import ASHA src/** paths, generated file paths, Rust crates, raw native/WASM transports, or arbitrary JSON hatches.',
  },
};

await writeFile(resolve(outDir, 'manifest.json'), `${JSON.stringify(manifest, null, 2)}\n`);
console.log(`wrote ${resolve(outDir, 'manifest.json')}`);
