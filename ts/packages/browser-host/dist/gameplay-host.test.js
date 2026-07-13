import assert from 'node:assert/strict';
import { mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';
import { runInNewContext } from 'node:vm';
import { ASHA_BROWSER_HOST_BRIDGE_METHODS, ASHA_BROWSER_HOST_GAMEPLAY_METHODS, launchNativeBrowserHost, } from './index.js';
const READOUT = {
    kind: 'gameplay_runtime_host.readout.v1',
    gameplayRegistryDigest: 'fnv1a64:registry',
    bindingRegistryHash: 'fnv1a64:bindings',
    activationHash: 'fnv1a64:activation',
    moduleStateHash: 'fnv1a64:state',
    authorityStateHash: 'fnv1a64:authority',
    triggerRevision: 3,
    triggerSnapshotHash: 'fnv1a64:triggers',
    activeOverlapCount: 1,
    reactionFrameCount: 2,
    lastReactionFrameHash: 'fnv1a64:frame',
    recentFrames: [],
    scheduler: {
        ownerId: 'authority.scheduler',
        stateHash: 'fnv1a64:scheduler',
        pendingActionCount: 0,
        outstandingDispatchCount: 0,
        factCount: 0,
        pendingActions: [],
        outstandingDispatches: [],
        truncated: false,
    },
    runtimeHostHash: 'fnv1a64:host',
};
void test('browser host preserves the closed gameplay RuntimeSession transport', async () => {
    const tempRoot = await mkdtemp(join(tmpdir(), 'asha-browser-gameplay-host-'));
    const calls = [];
    const gameplayHost = {
        load() {
            calls.push('load');
            return {
                kind: 'gameplay_runtime_host.load_receipt.v1',
                accepted: true,
                diagnostics: [],
                readout: READOUT,
            };
        },
        advance(moment) {
            calls.push(`advance:${moment.kind}`);
            return {
                kind: 'gameplay_runtime_host.advance_receipt.v1',
                accepted: true,
                diagnostics: [],
                moment,
                frames: [],
                readout: READOUT,
            };
        },
        read() {
            calls.push('read');
            return READOUT;
        },
        save() {
            calls.push('save');
            return {
                kind: 'gameplay_runtime_host.snapshot.v1',
                canonicalText: '{"kind":"fixture"}',
                snapshotHash: 'fnv1a64:snapshot',
            };
        },
        restore() {
            calls.push('restore');
            return {
                kind: 'gameplay_runtime_host.load_receipt.v1',
                accepted: true,
                diagnostics: [],
                readout: READOUT,
            };
        },
    };
    try {
        await writeFile(join(tempRoot, 'index.html'), '<!doctype html><title>Gameplay host</title>');
        const host = await launchNativeBrowserHost({
            uiRoot: tempRoot,
            host: '127.0.0.1',
            port: 0,
            provider: {
                globalScope: {},
                createRuntimeBridge: createCompleteFakeBridge,
                gameplayHost,
            },
        });
        try {
            const script = await fetch(`${host.url}/asha/browser-host/native-provider.js`);
            const scriptText = await script.text();
            const browserScope = {};
            runInNewContext(scriptText, browserScope);
            const provider = browserScope['ashaRuntimeBridge'];
            assert.deepEqual(Object.keys(provider.gameplayHost), ASHA_BROWSER_HOST_GAMEPLAY_METHODS);
            const advance = await invokeGameplayHost(host.url, 'advance', [{ kind: 'tick', tick: 7 }]);
            assert.equal(advance['result']['accepted'], true);
            const read = await invokeGameplayHost(host.url, 'read', []);
            assert.deepEqual(read, { result: READOUT });
            const save = await invokeGameplayHost(host.url, 'save', []);
            assert.deepEqual(save, {
                result: {
                    kind: 'gameplay_runtime_host.snapshot.v1',
                    canonicalText: '{"kind":"fixture"}',
                    snapshotHash: 'fnv1a64:snapshot',
                },
            });
            assert.deepEqual(calls, ['advance:tick', 'read', 'save']);
        }
        finally {
            await host.close();
        }
    }
    finally {
        await rm(tempRoot, { recursive: true, force: true });
    }
});
async function invokeGameplayHost(baseUrl, method, args) {
    const response = await fetch(`${baseUrl}/asha/browser-host/gameplay-runtime-host/${encodeURIComponent(method)}`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ args }),
    });
    assert.equal(response.status, 200);
    return await response.json();
}
function createCompleteFakeBridge() {
    const operation = () => ({ called: true });
    return Object.fromEntries(ASHA_BROWSER_HOST_BRIDGE_METHODS.map((method) => [method, operation]));
}
//# sourceMappingURL=gameplay-host.test.js.map