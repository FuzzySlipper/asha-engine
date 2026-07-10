import assert from 'node:assert/strict';
import { test } from 'node:test';
import { RuntimeBridgeError, createNativeRuntimeBridge, createRuntimeSessionFacade, } from '@asha/runtime-bridge';
const VALUES_BY_KIND = {
    empty: { kind: 'empty' },
    solid: { kind: 'solid', material: 2 },
};
const COMMANDS_BY_OP = {
    generateChunk: {
        op: 'generateChunk',
        grid: 1,
        chunk: { x: 0, y: 0, z: 0 },
        seed: 77,
        generatorVersion: 1,
    },
    fillRegion: {
        op: 'fillRegion',
        grid: 1,
        min: { x: 0, y: 0, z: 0 },
        max: { x: 2, y: 2, z: 2 },
        value: VALUES_BY_KIND.empty,
    },
    setVoxel: {
        op: 'setVoxel',
        grid: 1,
        coord: { x: 1, y: 1, z: 1 },
        value: VALUES_BY_KIND.empty,
    },
};
void test('public RuntimeSession submits the exhaustive generated voxel command union to native authority', (t) => {
    let session;
    try {
        session = createRuntimeSessionFacade({
            bridge: createNativeRuntimeBridge(),
            mode: 'rust',
        });
    }
    catch (error) {
        if (error instanceof RuntimeBridgeError && error.kind === 'native_unavailable') {
            t.skip('native addon not built; run harness/ci/check-native.sh for this proof');
            return;
        }
        throw error;
    }
    session.initialize({
        sessionId: 'runtime-session.voxel-command.consumer-proof',
        seed: 77,
        project: { gameId: 'asha-voxel-command-proof', workspaceId: 'workspace.local' },
        projectBundle: { bundleSchemaVersion: 1, protocolVersion: 1, sceneId: 5547 },
    });
    const result = session.submitCommands({
        commands: [
            COMMANDS_BY_OP.generateChunk,
            COMMANDS_BY_OP.fillRegion,
            { ...COMMANDS_BY_OP.fillRegion, value: VALUES_BY_KIND.solid },
            COMMANDS_BY_OP.setVoxel,
        ],
    });
    assert.deepEqual(result.result, { accepted: 4, rejected: 0, rejections: [] });
    assert.equal(session.readTelemetry().acceptedCommandCount, 4);
    const history = session.readVoxelEditHistory({
        historyId: 'history/default',
        cursorId: null,
        maxEntries: 8,
        includeRedoTail: true,
        expectedHistoryHash: null,
    });
    assert.equal(history.entries.length, 1);
    assert.equal(history.entries[0]?.commandCount, 4);
    assert.equal(history.cursor.entryCount, 1);
});
//# sourceMappingURL=native-voxel-command-consumer-proof.test.js.map