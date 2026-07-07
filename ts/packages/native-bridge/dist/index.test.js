import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { NativeAddonUnavailable, REQUIRED_NATIVE_ADDON_EXPORTS, loadNativeAddon } from './index.js';
function writeStaleAddonModule() {
    const dir = mkdtempSync(join(tmpdir(), 'asha-native-bridge-'));
    const modulePath = join(dir, 'stale-native-addon.cjs');
    writeFileSync(modulePath, `module.exports = {
      initializeEngine() {},
      loadWorldBundle() {}, // vocab-allow: stale native-addon fixture must name the legacy bridge operation.
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
    };`);
    return modulePath;
}
void test('native addon loader rejects stale modules missing encounter authority exports', () => {
    const modulePath = writeStaleAddonModule();
    try {
        assert.ok(REQUIRED_NATIVE_ADDON_EXPORTS.includes('planVoxelConversion'));
        assert.ok(REQUIRED_NATIVE_ADDON_EXPORTS.includes('previewVoxelConversion'));
        assert.ok(REQUIRED_NATIVE_ADDON_EXPORTS.includes('applyVoxelConversion'));
        assert.ok(REQUIRED_NATIVE_ADDON_EXPORTS.includes('exportVoxelConversionEvidence'));
        assert.throws(() => loadNativeAddon(modulePath), (error) => error instanceof NativeAddonUnavailable &&
            error.message.includes('readFpsEncounterDirector') &&
            error.message.includes('applyFpsEncounterTransition') &&
            error.message.includes('planVoxelConversion') &&
            error.message.includes('exportVoxelConversionEvidence'));
    }
    finally {
        rmSync(dirname(modulePath), { recursive: true, force: true });
    }
});
//# sourceMappingURL=index.test.js.map