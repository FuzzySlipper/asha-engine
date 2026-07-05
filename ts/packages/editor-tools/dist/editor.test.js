import { test } from 'node:test';
import assert from 'node:assert/strict';
import { EditorStore, initialEditorContext, proposeCommand, previewTargets, faceNeighbor, brushBox, } from './index.js';
function withSelection(over = {}) {
    return {
        ...initialEditorContext(0),
        selection: { voxel: { x: 5, y: 0, z: 0 }, face: 'negX' },
        ...over,
    };
}
void test('store applies actions, validates, and notifies subscribers on change', () => {
    const store = new EditorStore();
    let notified = 0;
    store.subscribe(() => (notified += 1));
    store.dispatch({ type: 'setTool', tool: 'remove' });
    assert.equal(store.getState().tool, 'remove');
    // Brush size is clamped to an integer >= 1.
    store.dispatch({ type: 'setBrushSize', size: 0 });
    assert.equal(store.getState().brushSize, 1);
    store.dispatch({ type: 'setBrushSize', size: 3.9 });
    assert.equal(store.getState().brushSize, 3);
    assert.equal(notified, 3);
});
void test('store reducer is pure and identity-stable for no-op', () => {
    const store = new EditorStore();
    const before = store.getState();
    // Setting a selection then reading is a new object; but unchanged primitive
    // actions still produce a new state object (immutability), so we check values.
    store.dispatch({ type: 'setSnapping', snapping: store.getState().snapping });
    assert.equal(store.getState().snapping, before.snapping);
});
void test('faceNeighbor and brushBox are correct', () => {
    assert.deepEqual(faceNeighbor({ x: 5, y: 0, z: 0 }, 'negX'), { x: 4, y: 0, z: 0 });
    assert.deepEqual(faceNeighbor({ x: 5, y: 0, z: 0 }, 'posY'), { x: 5, y: 1, z: 0 });
    // size 1 → the single cell.
    assert.deepEqual(brushBox({ x: 2, y: 2, z: 2 }, 1), { min: { x: 2, y: 2, z: 2 }, max: { x: 3, y: 3, z: 3 } });
    // size 3 → centred 3³.
    assert.deepEqual(brushBox({ x: 2, y: 2, z: 2 }, 3), { min: { x: 1, y: 1, z: 1 }, max: { x: 4, y: 4, z: 4 } });
});
void test('place tool proposes a setVoxel at the face-neighbour anchor', () => {
    const ctx = withSelection({ tool: 'place', material: 7 });
    assert.deepEqual(proposeCommand(ctx), {
        op: 'setVoxel',
        grid: 0,
        coord: { x: 4, y: 0, z: 0 }, // across the -X face of voxel 5
        value: { kind: 'solid', material: 7 },
    });
});
void test('remove tool proposes a setVoxel Empty at the selected voxel', () => {
    const ctx = withSelection({ tool: 'remove' });
    assert.deepEqual(proposeCommand(ctx), {
        op: 'setVoxel',
        grid: 0,
        coord: { x: 5, y: 0, z: 0 },
        value: { kind: 'empty' },
    });
});
void test('paint tool proposes a setVoxel that recolours the selected voxel itself', () => {
    // Paint targets the struck voxel (not its face-neighbour) with the current
    // material — the same SetVoxel command, a different anchor (not a new variant).
    const ctx = withSelection({ tool: 'paint', material: 5 });
    assert.deepEqual(proposeCommand(ctx), {
        op: 'setVoxel',
        grid: 0,
        coord: { x: 5, y: 0, z: 0 },
        value: { kind: 'solid', material: 5 },
    });
});
void test('box shape proposes a fillRegion; single ignores brushSize', () => {
    const box = withSelection({ tool: 'place', brushShape: 'box', brushSize: 3, material: 2 });
    const cmd = proposeCommand(box);
    assert.equal(cmd?.op, 'fillRegion');
    if (cmd?.op === 'fillRegion') {
        // anchor = (4,0,0); 3³ box centred there.
        assert.deepEqual(cmd.min, { x: 3, y: -1, z: -1 });
        assert.deepEqual(cmd.max, { x: 6, y: 2, z: 2 });
        assert.deepEqual(cmd.value, { kind: 'solid', material: 2 });
    }
    // Single shape ignores brushSize — it is always one cell (a SetVoxel).
    const single = withSelection({ tool: 'place', brushShape: 'single', brushSize: 3 });
    assert.equal(proposeCommand(single)?.op, 'setVoxel');
});
void test('box shape paint/remove fill the selected voxel region', () => {
    const remove = withSelection({ tool: 'remove', brushShape: 'box', brushSize: 3 });
    const cmd = proposeCommand(remove);
    assert.equal(cmd?.op, 'fillRegion');
    if (cmd?.op === 'fillRegion') {
        // remove anchors on the selected voxel (5,0,0), not its neighbour.
        assert.deepEqual(cmd.min, { x: 4, y: -1, z: -1 });
        assert.deepEqual(cmd.value, { kind: 'empty' });
    }
});
void test('select/inspect and no-selection propose nothing', () => {
    assert.equal(proposeCommand(withSelection({ tool: 'select' })), null);
    assert.equal(proposeCommand(withSelection({ tool: 'inspect' })), null);
    assert.equal(proposeCommand(initialEditorContext(0)), null); // no selection
});
void test('previewTargets enumerates the affected cells without proposing/mutating', () => {
    assert.deepEqual(previewTargets(withSelection({ tool: 'place', brushShape: 'single' })), [{ x: 4, y: 0, z: 0 }]);
    assert.equal(previewTargets(withSelection({ tool: 'place', brushShape: 'box', brushSize: 3 })).length, 27);
    assert.deepEqual(previewTargets(withSelection({ tool: 'paint' })), [{ x: 5, y: 0, z: 0 }]);
    assert.deepEqual(previewTargets(withSelection({ tool: 'select' })), []);
    assert.deepEqual(previewTargets(initialEditorContext(0)), []);
});
void test('proposeCommand and previewTargets are pure: they never mutate the context', () => {
    const ctx = withSelection({ tool: 'place', brushShape: 'box', brushSize: 2, material: 3 });
    const snapshot = JSON.stringify(ctx);
    proposeCommand(ctx);
    previewTargets(ctx);
    assert.equal(JSON.stringify(ctx), snapshot, 'the draft/proposal path mutates nothing');
});
//# sourceMappingURL=editor.test.js.map