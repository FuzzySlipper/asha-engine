import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { test } from 'node:test';
import assert from 'node:assert/strict';

import {
  ASHA_GAME_WORKSPACE_COMPATIBILITY,
  buildAshaAuthoringPersistenceContract,
  buildAshaGamePublishAssetManifest,
  parseAshaGameManifestToml,
  resolveAshaAuthoringWriteTarget,
  resolveAshaGameAssetForDev,
  validateAshaGameAssetCatalog,
  validateAshaConsumerCompatibility,
  type AshaGameAssetCatalog,
} from './index.js';

const fixturesRoot = resolve(import.meta.dirname, '../src/fixtures');

function fixture(name: string): string {
  return readFileSync(resolve(fixturesRoot, name), 'utf8');
}

void test('validates the golden asha.game.toml manifest', () => {
  const result = parseAshaGameManifestToml(fixture('asha.game.toml'));
  assert.equal(result.ok, true);
  if (!result.ok) {
    throw new Error('golden manifest should validate');
  }
  assert.equal(result.manifest.asha.engineVersion, '0.1.0');
  assert.equal(result.manifest.runtime.devtoolsEndpoint, 'ws://127.0.0.1:7391');
  assert.equal(result.manifest.runtime.backendMode, 'reference');
  assert.equal(result.manifest.runtime.backendProfile, 'reference');
  assert.deepEqual(result.manifest.runtime.backendProofRefs, []);
  assert.deepEqual(result.manifest.studio.allowedSourceWrites, ['scenes', 'assets', 'packages/game-catalogs']);
  assert.deepEqual(result.manifest.devResourceProfile.localRoots, ['assets', 'packages/game-catalogs']);
  assert.equal(result.manifest.devResourceProfile.cacheDir, 'dist/dev-cache');
  assert.equal(result.manifest.devResourceProfile.resolutionPolicy, 'prefer-source');
  assert.equal(result.manifest.publishResourceProfile.outputDir, 'dist/resources');
  assert.equal(result.manifest.publishResourceProfile.archiveDir, 'dist/archive');
  assert.equal(result.manifest.publishResourceProfile.resolutionPolicy, 'locked');
});

void test('fails closed when required workspace roots are missing', () => {
  const result = parseAshaGameManifestToml(fixture('invalid-missing-roots.toml'));
  assert.equal(result.ok, false);
  if (result.ok) {
    throw new Error('missing roots should fail validation');
  }
  assert.equal(result.diagnostics.some((diagnostic) => diagnostic.code === 'missing_root' && diagnostic.path === 'workspace.scene_roots'), true);
  assert.equal(result.diagnostics.some((diagnostic) => diagnostic.code === 'invalid_write_scope'), true);
});

void test('fails closed on disallowed Studio source-write roots', () => {
  const result = parseAshaGameManifestToml(fixture('invalid-source-write.toml'));
  assert.equal(result.ok, false);
  if (result.ok) {
    throw new Error('private write scope should fail validation');
  }
  assert.equal(result.diagnostics.some((diagnostic) => diagnostic.code === 'invalid_path'), true);
  assert.equal(result.diagnostics.some((diagnostic) => diagnostic.code === 'invalid_write_scope'), true);
});

void test('classifies bad versions and unsupported devtools endpoints', () => {
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

void test('manifest accepts selected native backend mode with public proof refs', () => {
  const manifest = fixture('asha.game.toml')
    .replace('backend_mode = "reference"', 'backend_mode = "native"')
    .replace('backend_profile = "reference"', 'backend_profile = "native.napi.launcher.v1"')
    .replace('backend_proof_refs = []', 'backend_proof_refs = ["proof:dev-authority-smoke"]');
  const result = parseAshaGameManifestToml(manifest);
  assert.equal(result.ok, true);
  if (!result.ok) {
    throw new Error('native backend manifest should validate');
  }
  assert.equal(result.manifest.runtime.backendMode, 'native');
  assert.deepEqual(result.manifest.runtime.backendProofRefs, ['proof:dev-authority-smoke']);
});

void test('manifest fails closed on unsupported or unproved backend modes', () => {
  const wasm = parseAshaGameManifestToml(
    fixture('asha.game.toml').replace('backend_mode = "reference"', 'backend_mode = "wasm"'),
  );
  assert.equal(wasm.ok, false);
  assert.equal(
    !wasm.ok && wasm.diagnostics.some((diagnostic) => diagnostic.code === 'unsupported_backend_mode' && diagnostic.path === 'runtime.backend_mode'),
    true,
  );

  const nativeMissingProof = parseAshaGameManifestToml(
    fixture('asha.game.toml')
      .replace('backend_mode = "reference"', 'backend_mode = "native"')
      .replace('backend_profile = "reference"', 'backend_profile = "native.napi.launcher.v1"'),
  );
  assert.equal(nativeMissingProof.ok, false);
  assert.equal(
    !nativeMissingProof.ok && nativeMissingProof.diagnostics.some((diagnostic) => diagnostic.code === 'missing_backend_ref' && diagnostic.path === 'runtime.backend_proof_refs'),
    true,
  );
});

void test('manifest rejects private transport hints in backend selection', () => {
  const manifest = fixture('asha.game.toml')
    .replace('wasm_or_native_entry = "dist/runtime/index.js"', 'wasm_or_native_entry = "@asha/native-bridge/native-bridge.node"')
    .replace('backend_profile = "reference"', 'backend_profile = "@asha/native-bridge"');
  const result = parseAshaGameManifestToml(manifest);
  assert.equal(result.ok, false);
  assert.equal(
    !result.ok && result.diagnostics.some((diagnostic) => diagnostic.code === 'private_transport_hint' && diagnostic.path === 'runtime.wasm_or_native_entry'),
    true,
  );
  assert.equal(
    !result.ok && result.diagnostics.some((diagnostic) => diagnostic.code === 'private_transport_hint' && diagnostic.path === 'runtime.backend_profile'),
    true,
  );
});

void test('fails closed when the publish resource profile is missing', () => {
  const result = parseAshaGameManifestToml(fixture('invalid-missing-publish-profile.toml'));
  assert.equal(result.ok, false);
  if (result.ok) {
    throw new Error('missing publish resource profile should fail validation');
  }
  assert.equal(
    result.diagnostics.some((diagnostic) => diagnostic.code === 'missing_required_field' && diagnostic.path === 'publish_resource_profile'),
    true,
  );
});

void test('fails closed when publish resource paths point into dev-local roots', () => {
  const result = parseAshaGameManifestToml(fixture('invalid-dev-root-leakage.toml'));
  assert.equal(result.ok, false);
  if (result.ok) {
    throw new Error('publish resource paths inside dev roots should fail validation');
  }
  assert.equal(
    result.diagnostics.some((diagnostic) => diagnostic.code === 'invalid_resource_profile' && diagnostic.path === 'publish_resource_profile.output_dir'),
    true,
  );
  assert.equal(
    result.diagnostics.some((diagnostic) => diagnostic.code === 'invalid_resource_profile' && diagnostic.path === 'publish_resource_profile.archive_dir'),
    true,
  );
});

void test('validates compatible ASHA consumer metadata against the manifest', () => {
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

void test('fails closed on incompatible consumer metadata versions', () => {
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

void test('fails closed when compatibility metadata is missing', () => {
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

void test('authoring persistence contract exposes bounded public write scopes', () => {
  const manifest = validManifest();
  const contract = buildAshaAuthoringPersistenceContract(manifest);

  assert.equal(contract.contractVersion, 'authoring-persistence.v0');
  assert.deepEqual(
    contract.writeScopes.map((scope) => scope.operationKind),
    [
      'authoring.scene.save_source',
      'authoring.catalog.save_source',
      'authoring.asset.save_source',
      'authoring.policy.save_source',
    ],
  );
  assert.deepEqual(contract.writeScopes.find((scope) => scope.operationKind === 'authoring.scene.save_source')?.allowedRoots, ['scenes']);
  assert.deepEqual(contract.writeScopes.find((scope) => scope.operationKind === 'authoring.catalog.save_source')?.allowedRoots, ['packages/game-catalogs']);
  assert.deepEqual(contract.writeScopes.find((scope) => scope.operationKind === 'authoring.asset.save_source')?.allowedRoots, ['assets']);
  assert.deepEqual(contract.writeScopes.find((scope) => scope.operationKind === 'authoring.policy.save_source')?.allowedRoots, []);
  assert.ok(contract.forbiddenRoots.includes('harness/out'));
  assert.ok(contract.nonClaims.includes('not_repo_crawler'));
  assert.equal(contract.diagnostics.some((diagnostic) => diagnostic.code === 'unsupported_operation'), true);
});

void test('authoring write target resolver accepts normalized scene catalog and asset paths', () => {
  const manifest = validManifest();

  const scene = resolveAshaAuthoringWriteTarget(manifest, {
    operationKind: 'authoring.scene.save_source',
    relativePath: './scenes/demo.scene.json',
  });
  assert.equal(scene.ok, true);
  if (!scene.ok) throw new Error('scene authoring path should resolve');
  assert.equal(scene.normalizedPath, 'scenes/demo.scene.json');
  assert.equal(scene.allowedRoot, 'scenes');
  assert.equal(scene.requiredValidator, 'validateAshaProofSceneSourceDocument');

  const catalog = resolveAshaAuthoringWriteTarget(manifest, {
    operationKind: 'authoring.catalog.save_source',
    relativePath: 'packages/game-catalogs/catalog.json',
  });
  assert.equal(catalog.ok, true);
  if (!catalog.ok) throw new Error('catalog authoring path should resolve');
  assert.equal(catalog.format, 'asset-catalog-json.v1');
  assert.equal(catalog.requiredValidator, 'validateAshaGameAssetCatalog');

  const asset = resolveAshaAuthoringWriteTarget(manifest, {
    operationKind: 'authoring.asset.save_source',
    relativePath: 'assets/meshes/demo.mesh.json',
  });
  assert.equal(asset.ok, true);
  if (!asset.ok) throw new Error('asset authoring path should resolve');
  assert.equal(asset.format, 'inline-asset-json.v1');
});

void test('authoring write target resolver fails closed on disallowed paths and hatches', () => {
  const manifest = validManifest();

  const generated = resolveAshaAuthoringWriteTarget(manifest, {
    operationKind: 'authoring.scene.save_source',
    relativePath: 'harness/out/proof.scene.json',
  });
  assert.equal(generated.ok, false);
  assert.equal(
    !generated.ok && generated.diagnostics.some((diagnostic) => diagnostic.code === 'forbidden_generated_path'),
    true,
  );

  const traversal = resolveAshaAuthoringWriteTarget(manifest, {
    operationKind: 'authoring.catalog.save_source',
    relativePath: '../asha-engine/private/catalog.json',
  });
  assert.equal(traversal.ok, false);
  assert.equal(
    !traversal.ok && traversal.diagnostics.some((diagnostic) => diagnostic.code === 'disallowed_path'),
    true,
  );

  const wrongExtension = resolveAshaAuthoringWriteTarget(manifest, {
    operationKind: 'authoring.asset.save_source',
    relativePath: 'assets/meshes/demo.txt',
  });
  assert.equal(wrongExtension.ok, false);
  assert.equal(
    !wrongExtension.ok && wrongExtension.diagnostics.some((diagnostic) => diagnostic.code === 'invalid_extension'),
    true,
  );

  const privateTransport = resolveAshaAuthoringWriteTarget(manifest, {
    operationKind: 'authoring.asset.save_source',
    relativePath: 'assets/@asha/native-bridge/native-bridge.node',
  });
  assert.equal(privateTransport.ok, false);
  assert.equal(
    !privateTransport.ok && privateTransport.diagnostics.some((diagnostic) => diagnostic.code === 'private_transport_hint'),
    true,
  );
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
        importMetadata: { sourceHash: 'sha256:mesh', cacheKey: 'dev-cache/static_mesh/mesh.demo-cube/sha256-mesh', generatedArtifactVersion: 'asset-import.v1' },
        dependencies: ['material.demo-copper'],
        publish: { include: true, outputKey: 'meshes/demo-cube.mesh.json' },
        diagnostics: { owner: 'asha-demo', notes: [] },
      },
      {
        id: 'material.demo-copper',
        kind: 'material',
        source: 'assets/materials/demo-copper.material.json',
        importProfile: 'inline-material.v0',
        importMetadata: { sourceHash: 'sha256:material', cacheKey: 'dev-cache/material/material.demo-copper/sha256-material', generatedArtifactVersion: 'asset-import.v1' },
        dependencies: ['texture.demo-checker'],
        publish: { include: true, outputKey: 'materials/demo-copper.material.json' },
        diagnostics: { owner: 'asha-demo', notes: [] },
      },
      {
        id: 'texture.demo-checker',
        kind: 'texture',
        source: 'assets/textures/demo-checker.texture.json',
        importProfile: 'inline-texture.v0',
        importMetadata: { sourceHash: 'sha256:texture', cacheKey: 'dev-cache/texture/texture.demo-checker/sha256-texture', generatedArtifactVersion: 'asset-import.v1' },
        dependencies: [],
        publish: { include: true, outputKey: 'textures/demo-checker.texture.json' },
        diagnostics: { owner: 'asha-demo', notes: [] },
      },
    ],
  };
}

void test('asset catalog validates and resolves a dev resource by catalog id', () => {
  const catalog = validCatalog();
  const existingFiles = new Set(catalog.entries.map((entry) => entry.source));
  const sourceHashes = new Map(catalog.entries.map((entry) => [entry.source, entry.importMetadata!.sourceHash]));
  const validation = validateAshaGameAssetCatalog(
    catalog,
    validManifest(),
    (path) => existingFiles.has(path),
    { sourceHash: (path) => sourceHashes.get(path) ?? null },
  );
  assert.equal(validation.ok, true);
  const resolution = resolveAshaGameAssetForDev(catalog, 'mesh.demo-cube', sourceHashes.get('assets/meshes/demo-cube.mesh.json'));
  assert.deepEqual(resolution, {
    assetId: 'mesh.demo-cube',
    sourcePath: 'assets/meshes/demo-cube.mesh.json',
    sourceHash: 'sha256:mesh',
    devCacheKey: 'dev-cache/static_mesh/mesh.demo-cube/sha256-mesh',
    generatedArtifactVersion: 'asset-import.v1',
    importStatus: 'clean',
    publishOutputKey: 'meshes/demo-cube.mesh.json',
  });
  const publishManifest = buildAshaGamePublishAssetManifest(catalog);
  assert.deepEqual(publishManifest.dependencyOrder, [
    'texture.demo-checker',
    'material.demo-copper',
    'mesh.demo-cube',
  ]);
  assert.deepEqual(publishManifest.entries.map((entry) => entry.assetId), [
    'mesh.demo-cube',
    'material.demo-copper',
    'texture.demo-checker',
  ]);
});

void test('asset catalog reports stale import metadata in validation and dev resolution', () => {
  const catalog = validCatalog();
  const validation = validateAshaGameAssetCatalog(
    catalog,
    validManifest(),
    () => true,
    { sourceHash: (path) => (path === 'assets/meshes/demo-cube.mesh.json' ? 'sha256:changed' : catalog.entries.find((entry) => entry.source === path)?.importMetadata?.sourceHash ?? null) },
  );
  assert.equal(validation.ok, false);
  if (validation.ok) throw new Error('stale import metadata should fail validation');
  assert.equal(validation.diagnostics.some((diagnostic) => diagnostic.code === 'stale_import_metadata'), true);

  const resolution = resolveAshaGameAssetForDev(catalog, 'mesh.demo-cube', 'sha256:changed');
  assert.equal(resolution?.sourcePath, 'assets/meshes/demo-cube.mesh.json');
  assert.equal(resolution?.sourceHash, 'sha256:changed');
  assert.equal(resolution?.importStatus, 'stale');
});

void test('asset catalog fails closed for missing file, duplicate id, forbidden path, unsupported kind, and wrong kind profile', () => {
  const catalog: AshaGameAssetCatalog = {
    schemaVersion: 1,
    entries: [
      { ...validCatalog().entries[0]! },
      { ...validCatalog().entries[0]!, source: '../asha-engine/private.bin', kind: 'shader' as never },
      { ...validCatalog().entries[1]!, importProfile: 'inline-static-mesh.v0', publish: { include: true, outputKey: 'meshes/not-a-material.mesh.json' } },
    ],
  };
  const validation = validateAshaGameAssetCatalog(catalog, validManifest(), () => false);
  assert.equal(validation.ok, false);
  if (validation.ok) throw new Error('invalid catalog should fail validation');
  assert.equal(validation.diagnostics.some((diagnostic) => diagnostic.code === 'missing_asset_file'), true);
  assert.equal(validation.diagnostics.some((diagnostic) => diagnostic.code === 'duplicate_asset_id'), true);
  assert.equal(validation.diagnostics.some((diagnostic) => diagnostic.code === 'forbidden_asset_path'), true);
  assert.equal(validation.diagnostics.some((diagnostic) => diagnostic.code === 'unsupported_asset_kind'), true);
  assert.equal(validation.diagnostics.some((diagnostic) => diagnostic.code === 'invalid_asset_entry' && diagnostic.path.endsWith('.importProfile')), true);
  assert.equal(validation.diagnostics.some((diagnostic) => diagnostic.code === 'invalid_asset_entry' && diagnostic.path.endsWith('.publish.outputKey')), true);
});

void test('asset catalog dependency graph fails closed for missing dependency and cycles', () => {
  const missing: AshaGameAssetCatalog = {
    ...validCatalog(),
    entries: [
      { ...validCatalog().entries[0]!, dependencies: ['material.missing', 'material.missing'] },
      ...validCatalog().entries.slice(1),
    ],
  };
  const missingValidation = validateAshaGameAssetCatalog(missing, validManifest(), (path) => path.startsWith('assets/'));
  assert.equal(missingValidation.ok, false);
  if (missingValidation.ok) throw new Error('missing dependency should fail validation');
  assert.equal(missingValidation.diagnostics.some((diagnostic) => diagnostic.code === 'missing_asset_dependency'), true);
  assert.equal(missingValidation.diagnostics.some((diagnostic) => diagnostic.code === 'duplicate_asset_dependency'), true);

  const cyclic: AshaGameAssetCatalog = {
    ...validCatalog(),
    entries: validCatalog().entries.map((entry) =>
      entry.id === 'texture.demo-checker' ? { ...entry, dependencies: ['mesh.demo-cube'] } : entry,
    ),
  };
  const cyclicValidation = validateAshaGameAssetCatalog(cyclic, validManifest(), (path) => path.startsWith('assets/'));
  assert.equal(cyclicValidation.ok, false);
  if (cyclicValidation.ok) throw new Error('dependency cycle should fail validation');
  assert.equal(cyclicValidation.diagnostics.some((diagnostic) => diagnostic.code === 'asset_dependency_cycle'), true);
});
