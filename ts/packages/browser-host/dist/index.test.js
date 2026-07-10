import { mkdtemp, mkdir, rm, writeFile } from 'node:fs/promises';
import { request } from 'node:http';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';
import { runInNewContext } from 'node:vm';
import assert from 'node:assert/strict';
import { MANIFEST_OPERATIONS, NativeRuntimeBridge, } from '@asha/runtime-bridge';
import { ASHA_BROWSER_HOST_BRIDGE_METHODS, ASHA_BROWSER_HOST_COMMAND, ASHA_BROWSER_HOST_COMPATIBILITY_VERSION, describeNativeBrowserHostCommand, installNativeBrowserHostProvider, launchNativeBrowserHost, readNativeBrowserHostProviderStatus, } from './index.js';
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
void test('browser host RPC surface follows every generated RuntimeBridge manifest operation', () => {
    assert.deepEqual(ASHA_BROWSER_HOST_BRIDGE_METHODS, MANIFEST_OPERATIONS.map(({ facadeMethod }) => facadeMethod));
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
            const scriptText = await script.text();
            assert.match(scriptText, /globalThis\.ashaRuntimeBridge/);
            const browserScope = {};
            runInNewContext(scriptText, browserScope);
            const browserProvider = browserScope['ashaRuntimeBridge'];
            const browserBridge = browserProvider.createRuntimeBridge();
            for (const { facadeMethod } of MANIFEST_OPERATIONS) {
                assert.equal(typeof browserBridge[facadeMethod], 'function', `served native provider must install ${facadeMethod}`);
            }
            const invocation = await fetch(`${host.url}/asha/browser-host/runtime-bridge/initializeEngine`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ args: [{ seed: 17 }] }),
            });
            assert.equal(invocation.status, 200);
            assert.deepEqual(await invocation.json(), { result: { called: true } });
        }
        finally {
            await host.close();
        }
    }
    finally {
        await rm(tempRoot, { recursive: true, force: true });
    }
});
void test('browser host preserves native RuntimeBridge receiver binding over HTTP', async () => {
    const tempRoot = await mkdtemp(join(tmpdir(), 'asha-browser-host-'));
    const calls = [];
    try {
        await writeFile(join(tempRoot, 'index.html'), '<!doctype html><title>ASHA demo</title>');
        const host = await launchNativeBrowserHost({
            uiRoot: tempRoot,
            host: '127.0.0.1',
            port: 0,
            provider: {
                globalScope: {},
                createRuntimeBridge: () => createFakeNativeRuntimeBridge(calls),
            },
        });
        try {
            const invocation = await fetch(`${host.url}/asha/browser-host/runtime-bridge/initializeEngine`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ args: [{ seed: 23 }] }),
            });
            assert.equal(invocation.status, 200);
            assert.deepEqual(await invocation.json(), { result: 123 });
            assert.deepEqual(calls, ['initialize:23']);
            const cameraInvocation = await fetch(`${host.url}/asha/browser-host/runtime-bridge/createCamera`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    args: [{
                            initialPose: { position: [0, 1.6, 0], yawDegrees: 0, pitchDegrees: 0 },
                            projection: { fovYDegrees: 60, near: 0.1, far: 1000 },
                            viewport: { width: 1280, height: 720 },
                        }],
                }),
            });
            assert.equal(cameraInvocation.status, 200);
            const cameraPayload = await cameraInvocation.json();
            assert.deepEqual(cameraPayload['result'], {
                camera: 1,
                tick: 0,
                pose: { position: [0, 1.6, 0], yawDegrees: 0, pitchDegrees: 0 },
                basis: {
                    forward: [0, 0, -1],
                    right: [1, 0, 0],
                    up: [0, 1, 0],
                },
                projection: { fovYDegrees: 60, near: 0.1, far: 1000 },
                viewport: { width: 1280, height: 720 },
            });
            const compositionInvocation = await fetch(`${host.url}/asha/browser-host/runtime-bridge/getProjectBundleCompositionStatus`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ args: [] }),
            });
            assert.equal(compositionInvocation.status, 200);
            assert.deepEqual(await compositionInvocation.json(), {
                result: {
                    loadedProjectBundle: 4103,
                    fatalCount: 0,
                    totalCount: 0,
                    blocksLoad: false,
                },
            });
            assert.deepEqual(calls, ['initialize:23', 'createCamera', 'compositionStatus']);
        }
        finally {
            await host.close();
        }
    }
    finally {
        await rm(tempRoot, { recursive: true, force: true });
    }
});
void test('browser host contains native RuntimeBridge invocation failures', async () => {
    const tempRoot = await mkdtemp(join(tmpdir(), 'asha-browser-host-'));
    const calls = [];
    try {
        await writeFile(join(tempRoot, 'index.html'), '<!doctype html><title>ASHA demo</title>');
        const host = await launchNativeBrowserHost({
            uiRoot: tempRoot,
            host: '127.0.0.1',
            port: 0,
            provider: {
                globalScope: {},
                createRuntimeBridge: () => createFakeNativeRuntimeBridge(calls, true),
            },
        });
        try {
            const initializeInvocation = await fetch(`${host.url}/asha/browser-host/runtime-bridge/initializeEngine`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ args: [{ seed: 23 }] }),
            });
            assert.equal(initializeInvocation.status, 200);
            const failedInvocation = await fetch(`${host.url}/asha/browser-host/runtime-bridge/getProjectBundleCompositionStatus`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ args: [] }),
            });
            assert.equal(failedInvocation.status, 500);
            assert.deepEqual(await failedInvocation.json(), {
                error: {
                    message: 'runtime bridge error [internal]: native composition status failed',
                },
            });
            assert.deepEqual(calls, ['initialize:23', 'compositionStatus']);
            const health = await fetch(`${host.url}/health`);
            assert.equal(health.status, 200);
            assert.deepEqual(await health.json(), {
                ok: true,
                project: 'asha-game-project',
                compatibilityVersion: ASHA_BROWSER_HOST_COMPATIBILITY_VERSION,
            });
        }
        finally {
            await host.close();
        }
    }
    finally {
        await rm(tempRoot, { recursive: true, force: true });
    }
});
void test('browser host rejects raw traversal into sibling directories outside ui root', async () => {
    const tempRoot = await mkdtemp(join(tmpdir(), 'asha-browser-host-'));
    try {
        const uiRoot = join(tempRoot, 'ui');
        const secretRoot = join(tempRoot, 'ui-secret');
        await mkdir(uiRoot);
        await mkdir(secretRoot);
        await writeFile(join(uiRoot, 'index.html'), '<!doctype html><title>ASHA demo</title>');
        await writeFile(join(secretRoot, 'secret.txt'), 'outside-ui-root');
        const host = await launchNativeBrowserHost({
            uiRoot,
            host: '127.0.0.1',
            port: 0,
            provider: {
                globalScope: {},
                createRuntimeBridge: createFakeRuntimeBridge,
            },
        });
        try {
            const response = await readRawHttpPath(host.url, '/%2e%2e/ui-secret/secret.txt');
            assert.equal(response.statusCode, 403);
            assert.equal(response.body, 'Forbidden');
        }
        finally {
            await host.close();
        }
    }
    finally {
        await rm(tempRoot, { recursive: true, force: true });
    }
});
async function readJson(url) {
    const response = await fetch(url);
    assert.equal(response.status, 200);
    return await response.json();
}
function readRawHttpPath(baseUrl, path) {
    const url = new URL(baseUrl);
    return new Promise((resolveRead, rejectRead) => {
        const requestHandle = request({
            hostname: url.hostname,
            method: 'GET',
            path,
            port: Number(url.port),
        }, (response) => {
            const chunks = [];
            response.on('data', (chunk) => {
                chunks.push(chunk);
            });
            response.on('end', () => {
                resolveRead({
                    statusCode: response.statusCode ?? 0,
                    body: Buffer.concat(chunks).toString('utf8'),
                });
            });
        });
        requestHandle.on('error', rejectRead);
        requestHandle.end();
    });
}
function createFakeRuntimeBridge() {
    const operation = () => ({ called: true });
    return {
        initializeEngine: operation,
        loadProjectBundle: operation, // vocab-allow: fake bridge must satisfy the legacy RuntimeBridge method name.
        getProjectBundleCompositionStatus: operation,
        submitCommands: operation,
        stepSimulation: operation,
        createCamera: operation,
        applyFirstPersonCameraInput: operation,
        readCameraProjection: operation,
        pickVoxel: operation,
        selectVoxel: operation,
        readVoxelMeshEvidence: operation,
        readRenderDiffs: operation,
        saveProjectBundle: operation,
        applyCollisionConstrainedCameraInput: operation,
        applyGeneratedTunnelToRuntimeWorld: operation,
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
        registerVoxelConversionMeshAsset: operation,
        readVoxelConversionSourceMetadata: operation,
        previewVoxelConversion: operation,
        applyVoxelConversion: operation,
        exportVoxelConversionEvidence: operation,
        readVoxelModelInfo: operation,
        readVoxelModelWindow: operation,
        exportVoxelVolumeAsset: operation,
        saveVoxelVolumeAsset: operation,
        updateVoxelVolumeAssetPalette: operation,
        loadVoxelVolumeAsset: operation,
        validateVoxelAnnotationLayer: operation,
        loadVoxelAnnotationLayer: operation,
        readVoxelAnnotationQuery: operation,
        applyVoxelAnnotationEdit: operation,
        exportVoxelAnnotationLayer: operation,
        readVoxelEditHistory: operation,
        previewVoxelEditRevert: operation,
        applyVoxelEditRevert: operation,
        undoVoxelEdit: operation,
        redoVoxelEdit: operation,
        getBuffer: operation,
        releaseBuffer: operation,
        unloadProjectBundle: operation,
        loadReplayFixture: operation,
        runReplayStep: operation,
    };
}
function createFakeNativeRuntimeBridge(calls, failCompositionStatus = false) {
    const addon = {
        initializeEngine: (seed) => {
            calls.push(`initialize:${seed}`);
            return seed + 100;
        },
        createCamera: (_handle, request) => {
            calls.push('createCamera');
            return {
                camera: 1,
                tick: 0,
                pose: request.initialPose,
                basis: {
                    forward: [0, 0, -1],
                    right: [1, 0, 0],
                    up: [0, 1, 0],
                },
                projection: request.projection,
                viewport: request.viewport,
            };
        },
        getProjectBundleCompositionStatus: (handle) => {
            void handle;
            calls.push('compositionStatus');
            if (failCompositionStatus) {
                throw new Error('native composition status failed');
            }
            return {
                loadedProjectBundle: 4103,
                fatalCount: 0,
                totalCount: 0,
                blocksLoad: false,
            };
        },
    };
    return new NativeRuntimeBridge(addon);
}
//# sourceMappingURL=index.test.js.map