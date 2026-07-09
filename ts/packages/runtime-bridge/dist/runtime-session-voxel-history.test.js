import { test } from 'node:test';
import assert from 'node:assert/strict';
import { RuntimeBridgeError, createRuntimeSessionFacade } from './index.js';
import { createMockRuntimeBridge } from './mock.js';
import { createMockRuntimeSession } from './reference.js';
function sessionInput() {
    return {
        sessionId: 'runtime-session.voxel-history.test',
        seed: 5283,
        project: {
            gameId: 'asha-history-test',
            workspaceId: 'workspace.history-test',
        },
        projectBundle: {
            bundleSchemaVersion: 1,
            protocolVersion: 1,
            sceneId: 5283,
        },
    };
}
const cursor = {
    cursorId: 'cursor/0',
    cursorKind: 'applied',
    appliedTransactionId: null,
    parentCursorId: null,
    historyHash: 'fnv1a64:history',
    voxelStateHash: 'fnv1a64:voxel',
    materialCatalogHash: 'fnv1a64:materials',
    undoDepth: 0,
    redoDepth: 1,
    entryCount: 1,
    checkpointCount: 0,
};
const readRequest = {
    historyId: 'history/test',
    cursorId: null,
    maxEntries: 8,
    includeRedoTail: true,
    expectedHistoryHash: null,
};
const summary = {
    historyId: 'history/test',
    schemaVersion: 1,
    mediaType: 'application/vnd.asha.voxel-edit-history+json;version=1',
    targetGrid: 1,
    targetVoxelVolumeAssetId: 'voxel/generated',
    baseVoxelHash: 'fnv1a64:base',
    materialCatalogHash: 'fnv1a64:materials',
    cursor,
    entries: [],
    retainedRedoTransactionIds: ['tx/2'],
    historyHash: 'fnv1a64:history',
    diagnostics: [],
};
const revertRequest = {
    historyId: 'history/test',
    mode: 'preview_revert',
    target: {
        transactionId: null,
        cursorId: 'cursor/0',
        cursorIndex: 0,
    },
    expectedHistoryHash: 'fnv1a64:history',
    expectedCursorHash: 'fnv1a64:cursor',
    maxReplaySteps: 16,
    maxDiffVoxels: 32,
    includeSampleWindow: false,
};
const undoRequest = {
    historyId: 'history/test',
    expectedHistoryHash: 'fnv1a64:history',
    expectedCursorHash: 'fnv1a64:cursor',
    maxReplaySteps: 16,
    maxDiffVoxels: 32,
};
const redoRequest = {
    historyId: 'history/test',
    expectedHistoryHash: 'fnv1a64:history',
    expectedCursorHash: 'fnv1a64:cursor',
    maxReplaySteps: 16,
    maxDiffVoxels: 32,
};
function revertReceipt(request, applied) {
    return {
        request,
        applied,
        preview: request.mode === 'preview_revert',
        historyId: request.historyId,
        cursorBefore: cursor,
        cursorAfter: cursor,
        durableEntry: null,
        previewEvidence: null,
        diffSummary: null,
        replayHash: 'fnv1a64:replay',
        historyHashBefore: 'fnv1a64:history',
        historyHashAfter: applied ? 'fnv1a64:history-after' : 'fnv1a64:history',
        diagnostics: [],
    };
}
function historyBridgeDouble() {
    const bridge = createMockRuntimeBridge();
    return new Proxy(bridge, {
        get(target, property, receiver) {
            if (property === 'readVoxelEditHistory') {
                return (request) => {
                    assert.deepEqual(request, readRequest);
                    return summary;
                };
            }
            if (property === 'previewVoxelEditRevert') {
                return (request) => {
                    assert.deepEqual(request, revertRequest);
                    return revertReceipt(request, false);
                };
            }
            if (property === 'applyVoxelEditRevert') {
                return (request) => {
                    assert.deepEqual(request, { ...revertRequest, mode: 'apply_revert' });
                    return revertReceipt(request, true);
                };
            }
            if (property === 'undoVoxelEdit') {
                return (request) => {
                    assert.deepEqual(request, undoRequest);
                    return { request, receipt: revertReceipt({ ...revertRequest, mode: 'undo' }, true) };
                };
            }
            if (property === 'redoVoxelEdit') {
                return (request) => {
                    assert.deepEqual(request, redoRequest);
                    return { request, receipt: revertReceipt({ ...revertRequest, mode: 'redo' }, true) };
                };
            }
            const value = Reflect.get(target, property, receiver);
            if (typeof value === 'function') {
                const method = value;
                return (...args) => method.apply(target, args);
            }
            return value;
        },
    });
}
void test('reference RuntimeSession fails closed for voxel edit history authority', () => {
    const session = createMockRuntimeSession();
    session.initialize(sessionInput());
    for (const call of [
        () => session.readVoxelEditHistory(readRequest),
        () => session.previewVoxelEditRevert(revertRequest),
        () => session.applyVoxelEditRevert({ ...revertRequest, mode: 'apply_revert' }),
        () => session.undoVoxelEdit(undoRequest),
        () => session.redoVoxelEdit(redoRequest),
    ]) {
        assert.throws(call, (error) => error instanceof RuntimeBridgeError && error.kind === 'operation_unimplemented');
    }
});
void test('rust-backed RuntimeSession forwards voxel edit history operations to bridge authority', () => {
    const session = createRuntimeSessionFacade({ bridge: historyBridgeDouble(), mode: 'rust' });
    session.initialize(sessionInput());
    assert.equal(session.readVoxelEditHistory(readRequest), summary);
    assert.equal(session.previewVoxelEditRevert(revertRequest).preview, true);
    assert.equal(session.applyVoxelEditRevert({ ...revertRequest, mode: 'apply_revert' }).applied, true);
    assert.equal(session.undoVoxelEdit(undoRequest).receipt.request.mode, 'undo');
    assert.equal(session.redoVoxelEdit(redoRequest).receipt.request.mode, 'redo');
});
//# sourceMappingURL=runtime-session-voxel-history.test.js.map