import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { test } from 'node:test';
import assert from 'node:assert/strict';

import {
  ASHA_GAME_WORKSPACE_COMPATIBILITY,
  buildAshaGamePublishAssetManifest,
  parseAshaGameManifestToml,
  resolveAshaGameAssetForDev,
  validateAshaGameAssetCatalog,
  validateAshaConsumerCompatibility,
  type AshaGameAssetCatalog,
} from './index.js';

const fixturesRoot = resolve(import.meta.dirname, '../src/fixtures');

function fixture(name: string): string {
  return readFileSync(resolve(fixturesRoot, name), 'utf8');
}

test('validates the golden asha.game.toml manifest', () => {
  const result = parseAshaGameManifestToml(fixture('asha.game.toml'));
  assert.equal(result.ok, true);
  if (!result.ok) {
    throw new Error('golden manifest should validate');
  }
  assert.equal(result.manifest.asha.engineVersion, '0.1.0');
  assert.equal(result.manifest.runtime.devtoolsEndpoint, 'ws://127.0.0.1:7391');
  assert.deepEqual(result.manifest.studio.allowedSourceWrites, ['scenes', 'assets', 'packages/game-catalogs']);
});

test('fails closed when required workspace roots are missing', () => {
  const result = parseAshaGameManifestToml(fixture('invalid-missing-roots.toml'));
  assert.equal(result.ok, false);
  if (result.ok) {
    throw new Error('missing roots should fail validation');
  }
  assert.equal(result.diagnostics.some((diagnostic) => diagnostic.code === 'missing_root' && diagnostic.path === 'workspace.scene_roots'), true);
  assert.equal(result.diagnostics.some((diagnostic) => diagnostic.code === 'invalid_write_scope'), true);
});

test('fails closed on disallowed Studio source-write roots', () => {
  const result = parseAshaGameManifestToml(fixture('invalid-source-write.toml'));
  assert.equal(result.ok, false);
  if (result.ok) {
    throw new Error('private write scope should fail validation');
  }
  assert.equal(result.diagnostics.some((diagnostic) => diagnostic.code === 'invalid_path'), true);
  assert.equal(result.diagnostics.some((diagnostic) => diagnostic.code === 'invalid_write_scope'), true);
});

test('classifies bad versions and unsupported devtools endpoints', () => {
  const manifest = fixture('asha.game.toml')
    .replace('engine_version = "0.1.0"', 'engine_version = "latest"')
    .replace('devtools_endpoint = "ws://127.0.0.1:7391"', 'devtools_endpoint = "https://example.com/devtools"');
  const result = parseAshaGameManifestToml(manifest);
  assert.equal(result.ok, false);
  if (result.ok) {
    throw new Error('bad version and endpoint should fail validation');
  }
  assert.equal(result.diagnostics.some((diagnostic) => diagnostic.code === 'bad_version'), true);
  assert.equal(result.diagnostics.some((diagnostic) => diagnostic.code === 'unsupported_endpoint'), true);
});

test('validates compatible ASHA consumer metadata against the manifest', () => {
  const result = parseAshaGameManifestToml(fixture('asha.game.toml'));
  assert.equal(result.ok, true);
  if (!result.ok) {
    throw new Error('golden manifest should validate');
  }

  const compatibility = validateAshaConsumerCompatibility(result.manifest, ASHA_GAME_WORKSPACE_COMPATIBILITY);
  assert.equal(compatibility.ok, true);
  if (!compatibility.ok) {
    throw new Error('golden compatibility metadata should validate');
  }
  assert.equal(compatibility.metadata.runtimeBridge.compatibilityVersion, 'runtime-bridge.v0');
});

test('fails closed on incompatible consumer metadata versions', () => {
  const result = parseAshaGameManifestToml(fixture('asha.game.toml'));
  assert.equal(result.ok, true);
  if (!result.ok) {
    throw new Error('golden manifest should validate');
  }

  const compatibility = validateAshaConsumerCompatibility(result.manifest, {
    ...ASHA_GAME_WORKSPACE_COMPATIBILITY,
    contracts: { compatibilityVersion: 'contracts.v0', packageVersion: '9.9.9' },
  });
  assert.equal(compatibility.ok, false);
  if (compatibility.ok) {
    throw new Error('incompatible metadata should fail validation');
  }
  assert.equal(compatibility.diagnostics.some((diagnostic) => diagnostic.code === 'incompatible_version' && diagnostic.path === 'asha.contracts_version'), true);
});

test('fails closed when compatibility metadata is missing', () => {
  const result = parseAshaGameManifestToml(fixture('asha.game.toml'));
  assert.equal(result.ok, true);
  if (!result.ok) {
    throw new Error('golden manifest should validate');
  }

  const compatibility = validateAshaConsumerCompatibility(result.manifest, {
    contracts: ASHA_GAME_WORKSPACE_COMPATIBILITY.contracts,
  });
  assert.equal(compatibility.ok, false);
  if (compatibility.ok) {
    throw new Error('missing metadata should fail validation');
  }
  assert.equal(compatibility.diagnostics.some((diagnostic) => diagnostic.code === 'missing_metadata' && diagnostic.path === 'runtimeBridge'), true);
});

function validManifest() {
  const result = parseAshaGameManifestToml(fixture('asha.game.toml'));
  assert.equal(result.ok, true);
  if (!result.ok) throw new Error('golden manifest should validate');
  return result.manifest;
}

function validCatalog(): AshaGameAssetCatalog {
  return {
    schemaVersion: 1,
    entries: [
      {
        id: 'mesh.demo-cube',
        kind: 'static_mesh',
        source: 'assets/meshes/demo-cube.mesh.json',
        importProfile: 'inline-static-mesh.v0',
        publish: { include: true, outputKey: 'meshes/demo-cube.mesh.json' },
        diagnostics: { owner: 'asha-demo', notes: [] },
      },
    ],
  };
}

test('asset catalog validates and resolves a dev resource by catalog id', () => {
  const catalog = validCatalog();
  const validation = validateAshaGameAssetCatalog(catalog, validManifest(), (path) => path === 'assets/meshes/demo-cube.mesh.json');
  assert.equal(validation.ok, true);
  const resolution = resolveAshaGameAssetForDev(catalog, 'mesh.demo-cube');
  assert.deepEqual(resolution, {
    assetId: 'mesh.demo-cube',
    sourcePath: 'assets/meshes/demo-cube.mesh.json',
    devCacheKey: 'dev-cache/static_mesh/mesh.demo-cube',
    publishOutputKey: 'meshes/demo-cube.mesh.json',
  });
  assert.deepEqual(buildAshaGamePublishAssetManifest(catalog).entries.map((entry) => entry.assetId), ['mesh.demo-cube']);
});

test('asset catalog fails closed for missing file, duplicate id, forbidden path, and unsupported kind', () => {
  const catalog: AshaGameAssetCatalog = {
    schemaVersion: 1,
    entries: [
      { ...validCatalog().entries[0]! },
      { ...validCatalog().entries[0]!, source: '../asha/private.bin', kind: 'shader' as never },
    ],
  };
  const validation = validateAshaGameAssetCatalog(catalog, validManifest(), () => false);
  assert.equal(validation.ok, false);
  if (validation.ok) throw new Error('invalid catalog should fail validation');
  assert.equal(validation.diagnostics.some((diagnostic) => diagnostic.code === 'missing_asset_file'), true);
  assert.equal(validation.diagnostics.some((diagnostic) => diagnostic.code === 'duplicate_asset_id'), true);
  assert.equal(validation.diagnostics.some((diagnostic) => diagnostic.code === 'forbidden_asset_path'), true);
  assert.equal(validation.diagnostics.some((diagnostic) => diagnostic.code === 'unsupported_asset_kind'), true);
});
