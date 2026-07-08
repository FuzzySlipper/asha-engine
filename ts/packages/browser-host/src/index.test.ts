import { mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';
import assert from 'node:assert/strict';

import type { RuntimeBridge } from '@asha/runtime-bridge';

import {
  ASHA_BROWSER_HOST_COMMAND,
  ASHA_BROWSER_HOST_COMPATIBILITY_VERSION,
  describeNativeBrowserHostCommand,
  installNativeBrowserHostProvider,
  launchNativeBrowserHost,
  readNativeBrowserHostProviderStatus,
} from './index.js';

void test('browser host command shape documents public native provider boot', () => {
  assert.deepEqual(describeNativeBrowserHostCommand(), {
    command: ASHA_BROWSER_HOST_COMMAND,
    packageRoot: '@asha/browser-host',
    providerGlobal: 'globalThis.ashaRuntimeBridge',
    providerKind: 'asha.runtime_bridge.native_rust_provider.v1',
    bootstrapOrder: 'install_provider_before_app_boot',
    hostDefault: '0.0.0.0',
    portDefault: 5173,
    referenceFallback: false,
    privateImportsRequired: false,
  });
});

void test('browser host installs a public native provider and reports rust authority status', async () => {
  const globalScope = {};
  const installation = installNativeBrowserHostProvider({
    globalScope,
    createRuntimeBridge: createFakeRuntimeBridge,
  });

  assert.equal(installation.providerGlobal, 'globalThis.ashaRuntimeBridge');
  assert.equal(installation.profile.providerContract, 'asha.runtime_bridge.native_rust_provider.v1');
  assert.equal(installation.profile.referenceFallback, false);

  const status = await readNativeBrowserHostProviderStatus(globalScope);
  assert.equal(status.status, 'rust_authority');
  assert.equal(status.available, true);
  assert.equal(status.profile.providerGlobal, 'globalThis.ashaRuntimeBridge');
});

void test('browser host fails closed for missing and spoofed providers', async () => {
  const missing = await readNativeBrowserHostProviderStatus({});
  assert.equal(missing.status, 'missing_rust_backend');
  assert.equal(missing.available, false);
  assert.equal(missing.diagnostics[0]?.code, 'missing_rust_runtime_backend');

  const spoofed = await readNativeBrowserHostProviderStatus({
    ashaRuntimeBridge: {
      kind: 'asha.runtime_bridge.native_rust_provider.v1',
      backend: 'reference_bridge',
      productAuthority: true,
      referenceFallback: true,
      createRuntimeBridge: createFakeRuntimeBridge,
    },
  });
  assert.equal(spoofed.status, 'missing_rust_backend');
  assert.equal(spoofed.available, false);
  assert.equal(spoofed.diagnostics[0]?.code, 'invalid_rust_runtime_provider');
});

void test('browser host serves a downstream UI root with provider status evidence', async () => {
  const tempRoot = await mkdtemp(join(tmpdir(), 'asha-browser-host-'));
  try {
    await writeFile(join(tempRoot, 'index.html'), '<!doctype html><title>ASHA demo</title>');
  const globalScope = {};
    const host = await launchNativeBrowserHost({
      uiRoot: tempRoot,
      host: '127.0.0.1',
      port: 0,
      healthProject: 'asha-demo',
      provider: {
        globalScope,
        createRuntimeBridge: createFakeRuntimeBridge,
      },
    });
    try {
      assert.equal(host.kind, 'asha.browser_host.native_runtime_provider.v0');
      assert.equal(host.compatibilityVersion, ASHA_BROWSER_HOST_COMPATIBILITY_VERSION);
      assert.equal(host.provider.status, 'rust_authority');

      const health = await readJson(`${host.url}/health`);
      assert.deepEqual(health, {
        ok: true,
        project: 'asha-demo',
        compatibilityVersion: ASHA_BROWSER_HOST_COMPATIBILITY_VERSION,
      });

      const provider = await readJson(`${host.url}/asha/browser-host/runtime-provider.json`);
      assert.equal(provider['status'], 'rust_authority');
      assert.equal(provider['available'], true);

      const page = await fetch(host.url);
      assert.equal(page.status, 200);
      const html = await page.text();
      assert.match(html, /ASHA demo/);
      assert.match(html, /\/asha\/browser-host\/native-provider\.js/);

      const script = await fetch(`${host.url}/asha/browser-host/native-provider.js`);
      assert.equal(script.status, 200);
      assert.match(await script.text(), /globalThis\.ashaRuntimeBridge/);

      const invocation = await fetch(`${host.url}/asha/browser-host/runtime-bridge/initializeEngine`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ args: [{ seed: 17 }] }),
      });
      assert.equal(invocation.status, 200);
      assert.deepEqual(await invocation.json(), { result: { called: true } });
    } finally {
      await host.close();
    }
  } finally {
    await rm(tempRoot, { recursive: true, force: true });
  }
});

async function readJson(url: string): Promise<Record<string, unknown>> {
  const response = await fetch(url);
  assert.equal(response.status, 200);
  return await response.json() as Record<string, unknown>;
}

function createFakeRuntimeBridge(): RuntimeBridge {
  const operation = () => ({ called: true }) as never;
  return {
    initializeEngine: operation,
    loadWorldBundle: operation, // vocab-allow: fake bridge must satisfy the legacy RuntimeBridge method name.
    getCompositionStatus: operation,
    submitCommands: operation,
    stepSimulation: operation,
    createCamera: operation,
    applyFirstPersonCameraInput: operation,
    readCameraProjection: operation,
    pickVoxel: operation,
    selectVoxel: operation,
    readVoxelMeshEvidence: operation,
    readRenderDiffs: operation,
    saveCurrentWorld: operation,
    applyCollisionConstrainedCameraInput: operation,
    readModelMaterialPreview: operation,
    loadFpsRuntimeSession: operation,
    readFpsRuntimeSession: operation,
    applyFpsPrimaryFire: operation,
    invokeGameExtensionWeaponEffect: operation,
    validateGameRuleCatalog: operation,
    submitGameRuleEffectIntent: operation,
    readGameRuleRuntimeReadout: operation,
    restartFpsRuntimeSession: operation,
    readFpsEncounterDirector: operation,
    applyFpsEncounterTransition: operation,
    readSceneObjectSnapshot: operation,
    applySceneObjectCommand: operation,
    applyEnemyDirectNavMovement: operation,
    planVoxelConversion: operation,
    registerVoxelConversionSource: operation,
    previewVoxelConversion: operation,
    applyVoxelConversion: operation,
    exportVoxelConversionEvidence: operation,
    readVoxelModelInfo: operation,
    getBuffer: operation,
    releaseBuffer: operation,
    unloadWorld: operation,
    loadReplayFixture: operation,
    runReplayStep: operation,
  };
}
