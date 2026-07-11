import assert from 'node:assert/strict';
import { mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';
import { RuntimeBridgeError, } from '@asha/runtime-bridge';
import { launchNativeBrowserHost } from './index.js';
const FPS_LOAD_REQUEST = {
    projectBundle: 'browser-host-native-lifecycle',
    definitions: [
        {
            entity: 101,
            stableId: 'actor/native-host-player',
            displayName: 'Native Host Player',
            sourcePath: 'catalogs/actors/native-host-player.entity.json',
            tags: ['player'],
            role: 'player',
            transform: { translation: [0, 1.5, 0], rotation: [0, 0, 0, 1], scale: [1, 1, 1] },
            bounds: { min: [2.2, 1, 1], max: [2.8, 2, 2] },
            renderVisible: true,
            staticCollider: false,
            health: { current: 100, max: 100 },
            weapon: {
                weaponId: 'weapon.native-host-player',
                damage: 40,
                rangeUnits: 16,
                ammo: 64,
                cooldownTicksAfterFire: 1,
            },
            policyBinding: null,
        },
        {
            entity: 777,
            stableId: 'actor/native-host-enemy',
            displayName: 'Native Host Enemy',
            sourcePath: 'catalogs/actors/native-host-enemy.entity.json',
            tags: ['enemy'],
            role: 'enemy',
            transform: { translation: [0, 1.5, 3], rotation: [0, 0, 0, 1], scale: [1, 1, 1] },
            bounds: { min: [2.2, 1, 5], max: [2.8, 2, 5.8] },
            renderVisible: true,
            staticCollider: false,
            health: { current: 40, max: 40 },
            weapon: {
                weaponId: 'weapon.native-host-enemy',
                damage: 100,
                rangeUnits: 16,
                ammo: 64,
                cooldownTicksAfterFire: 1,
            },
            policyBinding: null,
        },
    ],
    gameRuleModules: [],
};
void test('native browser host survives sustained movement defeat restart and invocation errors', async (t) => {
    const uiRoot = await mkdtemp(join(tmpdir(), 'asha-native-lifecycle-'));
    await writeFile(join(uiRoot, 'index.html'), '<!doctype html><title>Native lifecycle</title>');
    let host;
    try {
        try {
            host = await launchNativeBrowserHost({ uiRoot, host: '127.0.0.1', port: 0 });
        }
        catch (error) {
            if (error instanceof RuntimeBridgeError && error.kind === 'native_unavailable') {
                t.skip('native addon not built (run harness/ci/check-native.sh)');
                return;
            }
            throw error;
        }
        await invokeBridge(host.url, 'initializeEngine', [{ seed: 17 }]);
        await invokeBridge(host.url, 'loadProjectBundle', [{
                bundleSchemaVersion: 1,
                protocolVersion: 1,
                sceneId: 4103,
            }]);
        const loaded = await invokeBridge(host.url, 'loadFpsRuntimeSession', [FPS_LOAD_REQUEST]);
        assert.equal(loaded.sessionEpoch, 1);
        const camera = await invokeBridge(host.url, 'createCamera', [{
                initialPose: { position: [0, 1.62, 1.5], yawDegrees: 0, pitchDegrees: 0 },
                projection: { fovYDegrees: 60, near: 0.1, far: 1000 },
                viewport: { width: 1280, height: 720 },
            }]);
        let epoch = loaded.sessionEpoch;
        for (let cycle = 0; cycle < 64; cycle += 1) {
            const movement = await invokeBridge(host.url, 'applyCollisionConstrainedCameraInput', [{
                    camera: camera.camera,
                    grid: 1,
                    movementMode: 'grounded',
                    input: {
                        moveForward: cycle % 2 === 0 ? 1 : 0,
                        moveRight: cycle % 2 === 0 ? 0 : 1,
                        moveUp: 0,
                        yawDeltaDegrees: 0.25,
                        pitchDeltaDegrees: 0,
                        dtSeconds: 0.016,
                        moveSpeedUnitsPerSecond: 3,
                    },
                    tick: cycle + 1,
                    shape: { halfExtents: [0.25, 0.7, 0.25] },
                    policy: { mode: 'axis_separable_slide', maxIterations: 3 },
                }]);
            assert.equal(movement.collision.movementMode, 'grounded');
            const defeat = await invokeBridge(host.url, 'applyFpsPrimaryFire', [{
                    tick: cycle + 1,
                    origin: [2.5, 1.5, 5.5],
                    direction: [0, 0, -1],
                    shooterRole: 'enemy',
                    targetRole: 'player',
                }]);
            assert.equal(defeat.target, 101, JSON.stringify(defeat));
            assert.equal(defeat.targetHealthAfter?.current, 0, JSON.stringify(defeat));
            const restarted = await invokeBridge(host.url, 'restartFpsRuntimeSession', [{ expectedEpoch: epoch }]);
            epoch += 1;
            assert.equal(restarted.sessionEpoch, epoch);
            assert.equal(restarted.lifecycleStatus.state, 'active');
        }
        const rejected = await invokeBridgeResponse(host.url, 'restartFpsRuntimeSession', [{ expectedEpoch: 1 }]);
        assert.equal(rejected.status, 500);
        assert.match(rejected.errorMessage, /stale|epoch/iu);
        const health = await fetch(`${host.url}/health`);
        assert.equal(health.status, 200);
        const finalSnapshot = await invokeBridge(host.url, 'readFpsRuntimeSession', []);
        assert.equal(finalSnapshot.sessionEpoch, epoch);
        assert.equal(finalSnapshot.lifecycleStatus.state, 'active');
    }
    finally {
        await host?.close();
        await rm(uiRoot, { recursive: true, force: true });
    }
});
async function invokeBridge(baseUrl, method, args) {
    const response = await invokeBridgeResponse(baseUrl, method, args);
    assert.equal(response.status, 200, response.errorMessage);
    return response.result;
}
async function invokeBridgeResponse(baseUrl, method, args) {
    const response = await fetch(`${baseUrl}/asha/browser-host/runtime-bridge/${encodeURIComponent(method)}`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ args }),
    });
    const payload = await response.json();
    return {
        status: response.status,
        result: payload.result ?? null,
        errorMessage: payload.error?.message ?? '',
    };
}
//# sourceMappingURL=native-lifecycle.test.js.map