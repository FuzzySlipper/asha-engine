import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';

import { NativeAddonUnavailable, loadNativeAddon } from './index.js';

function writeStaleAddonModule(): string {
  const dir = mkdtempSync(join(tmpdir(), 'asha-native-bridge-'));
  const modulePath = join(dir, 'stale-native-addon.cjs');
  writeFileSync(
    modulePath,
    `module.exports = {
      initializeEngine() {},
      loadWorldBundle() {},
      submitCommands() {},
      stepSimulation() {},
      applyEnemyDirectNavMovement() {},
      loadFpsRuntimeSession() {},
      readFpsRuntimeSession() {},
      applyFpsPrimaryFire() {},
      restartFpsRuntimeSession() {},
      readRenderDiffs() {},
      saveCurrentWorld() {},
      getCompositionStatus() {}
    };`,
  );
  return modulePath;
}

void test('native addon loader rejects stale modules missing encounter authority exports', () => {
  const modulePath = writeStaleAddonModule();
  try {
    assert.throws(
      () => loadNativeAddon(modulePath),
      (error: unknown) =>
        error instanceof NativeAddonUnavailable &&
        error.message.includes('readFpsEncounterDirector') &&
        error.message.includes('applyFpsEncounterTransition'),
    );
  } finally {
    rmSync(dirname(modulePath), { recursive: true, force: true });
  }
});
