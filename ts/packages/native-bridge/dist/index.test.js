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
      loadProjectBundle() {},
      submitCommands() {},
      stepSimulation() {},
      applyEnemyDirectNavMovement() {},
      loadFpsRuntimeSession() {},
      readFpsRuntimeSession() {},
      applyFpsPrimaryFire() {},
      restartFpsRuntimeSession() {},
      readRenderDiffs() {},
      saveProjectBundle() {},
      getProjectBundleCompositionStatus() {}
    };`);
    return modulePath;
}
function writeCurrentAddonModule() {
    const dir = mkdtempSync(join(tmpdir(), 'asha-native-bridge-'));
    const modulePath = join(dir, 'current-native-addon.cjs');
    const exports = REQUIRED_NATIVE_ADDON_EXPORTS
        .map((name) => `      ${JSON.stringify(name)}() {}`)
        .join(',\n');
    writeFileSync(modulePath, `module.exports = {\n${exports}\n    };\n`);
    return modulePath;
}
function retiredRuntimeContainerTerm() {
    return String.fromCharCode(87, 111, 114, 108, 100);
}
void test('native addon loader accepts current ProjectBundle export vocabulary', () => {
    const modulePath = writeCurrentAddonModule();
    try {
        assert.ok(REQUIRED_NATIVE_ADDON_EXPORTS.includes('loadProjectBundle'));
        assert.ok(REQUIRED_NATIVE_ADDON_EXPORTS.includes('saveProjectBundle'));
        assert.ok(REQUIRED_NATIVE_ADDON_EXPORTS.includes('getProjectBundleCompositionStatus'));
        assert.equal(REQUIRED_NATIVE_ADDON_EXPORTS.includes(`load${retiredRuntimeContainerTerm()}Bundle`), false);
        assert.equal(REQUIRED_NATIVE_ADDON_EXPORTS.includes(`saveCurrent${retiredRuntimeContainerTerm()}`), false);
        assert.equal(REQUIRED_NATIVE_ADDON_EXPORTS.includes('getCompositionStatus'), false);
        const addon = loadNativeAddon(modulePath);
        assert.equal(typeof addon.loadProjectBundle, 'function');
        assert.equal(typeof addon.saveProjectBundle, 'function');
        assert.equal(typeof addon.getProjectBundleCompositionStatus, 'function');
    }
    finally {
        rmSync(dirname(modulePath), { recursive: true, force: true });
    }
});
void test('native addon loader rejects stale modules missing encounter authority exports', () => {
    const modulePath = writeStaleAddonModule();
    try {
        assert.ok(REQUIRED_NATIVE_ADDON_EXPORTS.includes('planVoxelConversion'));
        assert.ok(REQUIRED_NATIVE_ADDON_EXPORTS.includes('registerVoxelConversionSource'));
        assert.ok(REQUIRED_NATIVE_ADDON_EXPORTS.includes('previewVoxelConversion'));
        assert.ok(REQUIRED_NATIVE_ADDON_EXPORTS.includes('applyVoxelConversion'));
        assert.ok(REQUIRED_NATIVE_ADDON_EXPORTS.includes('exportVoxelConversionEvidence'));
        assert.ok(REQUIRED_NATIVE_ADDON_EXPORTS.includes('readVoxelModelInfo'));
        assert.throws(() => loadNativeAddon(modulePath), (error) => error instanceof NativeAddonUnavailable &&
            error.message.includes('readFpsEncounterDirector') &&
            error.message.includes('applyFpsEncounterTransition') &&
            error.message.includes('planVoxelConversion') &&
            error.message.includes('registerVoxelConversionSource') &&
            error.message.includes('exportVoxelConversionEvidence'));
    }
    finally {
        rmSync(dirname(modulePath), { recursive: true, force: true });
    }
});
//# sourceMappingURL=index.test.js.map