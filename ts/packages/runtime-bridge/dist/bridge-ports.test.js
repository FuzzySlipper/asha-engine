import { strict as assert } from 'node:assert';
import test from 'node:test';
import { RUNTIME_BRIDGE_PORT_CONTRACTS, runtimeBridgePorts, } from './bridge.js';
void test('fixed port views bind one root without capability lookup', () => {
    const root = {};
    const ports = runtimeBridgePorts(root);
    assert.deepEqual(Object.keys(ports), [
        'input',
        'timeSimulation',
        'sceneEntities',
        'voxelAssetsBuffers',
        'camera',
        'gameplay',
        'projection',
        'bundleLifecycle',
        'replayEvidence',
    ]);
    for (const port of Object.values(ports)) {
        assert.equal(port, root);
    }
});
void test('a simulation consumer can use a capability subset test double', () => {
    const calls = [];
    const simulation = {
        applyTimeControlCommand: () => {
            throw new Error('not exercised');
        },
        readTimeControlState: () => {
            throw new Error('not exercised');
        },
        stepSimulation: ({ tick }) => {
            calls.push(tick);
            return { tick, diffCount: 0 };
        },
    };
    assert.equal(simulation.stepSimulation({ tick: 7 }).tick, 7);
    assert.deepEqual(calls, [7]);
});
void test('every fixed port records lifecycle hash and resource rules', () => {
    assert.deepEqual(Object.keys(RUNTIME_BRIDGE_PORT_CONTRACTS), [
        'input',
        'timeSimulation',
        'sceneEntities',
        'voxelAssetsBuffers',
        'camera',
        'gameplay',
        'projection',
        'bundleLifecycle',
        'replayEvidence',
    ]);
    assert.equal(RUNTIME_BRIDGE_PORT_CONTRACTS.bundleLifecycle.projectBundle, 'ownsLoadUnload');
    assert.equal(RUNTIME_BRIDGE_PORT_CONTRACTS.gameplay.projectBundle, 'retainedAcrossLoadUnload');
    assert.equal(RUNTIME_BRIDGE_PORT_CONTRACTS.voxelAssetsBuffers.resourceLifetime, 'mixedExplicitAndSession');
    assert.equal(RUNTIME_BRIDGE_PORT_CONTRACTS.projection.snapshotHash, 'projectionFrame');
    assert.equal(RUNTIME_BRIDGE_PORT_CONTRACTS.replayEvidence.snapshotHash, 'replayEvidence');
});
//# sourceMappingURL=bridge-ports.test.js.map