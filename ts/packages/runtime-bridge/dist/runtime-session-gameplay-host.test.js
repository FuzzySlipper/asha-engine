import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRuntimeSessionFacade, } from './index.js';
import { createMockRuntimeBridge } from './mock.js';
void test('Rust-backed RuntimeSession consumes a consumer-owned static gameplay host', () => {
    const readout = {
        kind: 'gameplay_runtime_host.readout.v1',
        gameplayRegistryDigest: 'fnv1a64:registry',
        bindingRegistryHash: 'fnv1a64:bindings',
        activationHash: 'fnv1a64:activation',
        moduleStateHash: 'fnv1a64:state',
        authorityStateHash: 'fnv1a64:authority',
        triggerRevision: 0,
        triggerSnapshotHash: 'fnv1a64:triggers',
        activeOverlapCount: 0,
        reactionFrameCount: 0,
        lastReactionFrameHash: null,
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
    const calls = [];
    const gameplayHost = {
        load(input) {
            calls.push(`load:${input.projectId}`);
            return {
                kind: 'gameplay_runtime_host.load_receipt.v1',
                accepted: true,
                diagnostics: [],
                readout,
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
                readout,
            };
        },
        read() { return readout; },
        save() {
            return {
                kind: 'gameplay_runtime_host.snapshot.v1',
                canonicalText: '{}',
                snapshotHash: 'fnv1a64:snapshot',
            };
        },
        restore(input) {
            calls.push(`restore:${input.projectId}`);
            return {
                kind: 'gameplay_runtime_host.load_receipt.v1',
                accepted: true,
                diagnostics: [],
                readout,
            };
        },
    };
    const session = createRuntimeSessionFacade({
        bridge: createMockRuntimeBridge(),
        mode: 'rust',
        gameplayHost,
    });
    session.initialize({
        sessionId: 'runtime-session.gameplay-host',
        seed: 17,
        project: { gameId: 'fixture', workspaceId: 'workspace.fixture' },
        projectBundle: { bundleSchemaVersion: 1, protocolVersion: 1, sceneId: 42 },
    });
    const load = {
        kind: 'gameplay_runtime_host.load.v1',
        projectId: 'fixture.gameplay',
        compositionHash: 'fnv1a64:composition',
        declaredReadPlanHash: 'fnv1a64:reads',
        bindings: {
            schemaVersion: 1,
            configurations: [],
            bindings: [],
            overrides: [],
            registryHash: 'fnv1a64:bindings',
        },
        triggers: [],
        scheduler: {
            owner: { ownerId: 'authority.scheduler', providerId: 'provider.scheduler' },
            declaredEvents: [],
            declaredProposals: [],
        },
    };
    assert.equal(session.loadGameplayRuntime(load).accepted, true);
    assert.equal(session.advanceGameplayRuntime({ kind: 'tick', tick: 1 }).accepted, true);
    assert.equal(session.advanceGameplayRuntime({
        kind: 'schedulerRoute',
        actionId: 'action.saved',
    }).accepted, true);
    assert.equal(session.readGameplayRuntime().runtimeHostHash, 'fnv1a64:host');
    const snapshot = session.saveGameplayRuntime();
    assert.equal(session.restoreGameplayRuntime(load, snapshot).accepted, true);
    assert.deepEqual(calls, [
        'load:fixture.gameplay',
        'advance:tick',
        'advance:schedulerRoute',
        'restore:fixture.gameplay',
    ]);
});
//# sourceMappingURL=runtime-session-gameplay-host.test.js.map