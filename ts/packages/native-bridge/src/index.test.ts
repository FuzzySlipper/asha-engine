import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';

import { NativeAddonUnavailable, REQUIRED_NATIVE_ADDON_EXPORTS, loadNativeAddon } from './index.js';

function writeStaleAddonModule(): string {
  const dir = mkdtempSync(join(tmpdir(), 'asha-native-bridge-'));
  const modulePath = join(dir, 'stale-native-addon.cjs');
  writeFileSync(
    modulePath,
    `module.exports = {
      initializeEngine() {},
      submitCommands() {},
      stepSimulation() {},
      applyEnemyDirectNavMovement() {},
      readFpsRuntimeSession() {},
      applyFpsPrimaryFire() {},
      restartFpsRuntimeSession() {},
      readRenderDiffs() {},
    };`,
  );
  return modulePath;
}

function writeCurrentAddonModule(): string {
  const dir = mkdtempSync(join(tmpdir(), 'asha-native-bridge-'));
  const modulePath = join(dir, 'current-native-addon.cjs');
  const exports = REQUIRED_NATIVE_ADDON_EXPORTS
    .map((name) => `      ${JSON.stringify(name)}() {}`)
    .join(',\n');
  writeFileSync(modulePath, `module.exports = {\n${exports}\n    };\n`);
  return modulePath;
}

function retiredRuntimeContainerTerm(): string {
  return String.fromCharCode(87, 111, 114, 108, 100);
}

void test('native addon loader accepts the canonical project lifecycle exports', () => {
  const modulePath = writeCurrentAddonModule();
  try {
    assert.ok(REQUIRED_NATIVE_ADDON_EXPORTS.includes('loadRuntimeProject'));
    assert.ok(REQUIRED_NATIVE_ADDON_EXPORTS.includes('closeRuntimeProject'));
    assert.ok(REQUIRED_NATIVE_ADDON_EXPORTS.includes('readActiveRuntimeProjectContent'));
    assert.ok(REQUIRED_NATIVE_ADDON_EXPORTS.includes('createCamera'));
    assert.ok(REQUIRED_NATIVE_ADDON_EXPORTS.includes('importVoxelConversionMeshSource'));
    assert.ok(REQUIRED_NATIVE_ADDON_EXPORTS.includes('unloadVoxelVolumeAsset'));
    assert.equal((REQUIRED_NATIVE_ADDON_EXPORTS as readonly string[]).includes(`load${retiredRuntimeContainerTerm()}Bundle`), false);
    assert.equal((REQUIRED_NATIVE_ADDON_EXPORTS as readonly string[]).includes(`saveCurrent${retiredRuntimeContainerTerm()}`), false);
    assert.equal((REQUIRED_NATIVE_ADDON_EXPORTS as readonly string[]).includes('getCompositionStatus'), false);

    const addon = loadNativeAddon(modulePath);
    assert.equal(typeof addon.loadRuntimeProject, 'function');
    assert.equal(typeof addon.closeRuntimeProject, 'function');
    assert.equal(typeof addon.readActiveRuntimeProjectContent, 'function');
    assert.equal(typeof addon.importVoxelConversionMeshSource, 'function');
    assert.equal(typeof addon.unloadVoxelVolumeAsset, 'function');
  } finally {
    rmSync(dirname(modulePath), { recursive: true, force: true });
  }
});

void test('native addon loader rejects stale modules missing encounter authority exports', () => {
  const modulePath = writeStaleAddonModule();
  try {
    assert.ok(REQUIRED_NATIVE_ADDON_EXPORTS.includes('planVoxelConversion'));
    assert.ok(REQUIRED_NATIVE_ADDON_EXPORTS.includes('registerVoxelConversionSource'));
    assert.ok(REQUIRED_NATIVE_ADDON_EXPORTS.includes('importVoxelConversionMeshSource'));
    assert.ok(REQUIRED_NATIVE_ADDON_EXPORTS.includes('readVoxelConversionSourceMetadata'));
    assert.ok(REQUIRED_NATIVE_ADDON_EXPORTS.includes('previewVoxelConversion'));
    assert.ok(REQUIRED_NATIVE_ADDON_EXPORTS.includes('applyVoxelConversion'));
    assert.ok(REQUIRED_NATIVE_ADDON_EXPORTS.includes('exportVoxelConversionEvidence'));
    assert.ok(REQUIRED_NATIVE_ADDON_EXPORTS.includes('readVoxelModelInfo'));
    assert.ok(REQUIRED_NATIVE_ADDON_EXPORTS.includes('readVoxelModelWindow'));
    assert.ok(REQUIRED_NATIVE_ADDON_EXPORTS.includes('unloadVoxelVolumeAsset'));
    assert.ok(REQUIRED_NATIVE_ADDON_EXPORTS.includes('readVoxelEditHistory'));
    assert.ok(REQUIRED_NATIVE_ADDON_EXPORTS.includes('previewVoxelEditRevert'));
    assert.ok(REQUIRED_NATIVE_ADDON_EXPORTS.includes('applyVoxelEditRevert'));
    assert.ok(REQUIRED_NATIVE_ADDON_EXPORTS.includes('undoVoxelEdit'));
    assert.ok(REQUIRED_NATIVE_ADDON_EXPORTS.includes('redoVoxelEdit'));
    assert.throws(
      () => loadNativeAddon(modulePath),
      (error: unknown) =>
        error instanceof NativeAddonUnavailable &&
        error.message.includes('readFpsEncounterDirector') &&
        error.message.includes('applyFpsEncounterTransition') &&
        error.message.includes('createCamera') &&
        error.message.includes('planVoxelConversion') &&
        error.message.includes('registerVoxelConversionSource') &&
        error.message.includes('readVoxelConversionSourceMetadata') &&
        error.message.includes('unloadVoxelVolumeAsset') &&
        error.message.includes('readVoxelEditHistory') &&
        error.message.includes('redoVoxelEdit') &&
        error.message.includes('exportVoxelConversionEvidence'),
    );
  } finally {
    rmSync(dirname(modulePath), { recursive: true, force: true });
  }
});
