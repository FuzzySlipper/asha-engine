import { test } from 'node:test';
import assert from 'node:assert/strict';
import { initialEditorContext } from '@asha/editor-tools';
import { summarizeScene, inspectEditor } from './index.js';
test('summarizeScene formats a projected report deterministically', () => {
    const report = {
        resident: 2, pending: 1, unloaded: 0, colliderChunks: 1, dirtyChunks: 0,
        queue: [{ kind: 'mesh', count: 1 }, { kind: 'collision-rebuild', count: 1 }, { kind: 'upload', count: 0 }],
    };
    assert.deepEqual(summarizeScene(report), [
        'chunks resident=2 pending=1 unloaded=0',
        'colliders=1 dirty=0',
        'queue mesh=1',
        'queue collision-rebuild=1', // zero-count lanes omitted
    ]);
});
test('inspectEditor is a pure read of the editor context (no authority copy)', () => {
    const ctx = {
        ...initialEditorContext(0),
        tool: 'place',
        brushSize: 3,
        selection: { voxel: { x: 5, y: 0, z: 0 }, face: 'negX' },
    };
    const view = inspectEditor(ctx);
    assert.equal(view.tool, 'place');
    assert.deepEqual(view.selectedVoxel, [5, 0, 0]);
    assert.equal(view.selectedFace, 'negX');
    assert.equal(view.affectedCells, 27);
    assert.deepEqual(inspectEditor(ctx), view); // pure
});
//# sourceMappingURL=devtools.test.js.map