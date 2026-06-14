import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createMockRuntimeBridge } from '@asha/runtime-bridge';
import { cameraPointerRay, defaultCamera } from '@asha/ui-dom';
import { EditorStore, VoxelEditController, bridgeCommandSink, bridgePicker, pickAndSelect } from './index.js';
function controller() {
    const submitted = [];
    const ctrl = new VoxelEditController((cmds) => submitted.push([...cmds]));
    // A place tool with a selection so there is something to commit.
    ctrl.store.dispatch({ type: 'setTool', tool: 'place' });
    ctrl.store.dispatch({ type: 'setMaterial', material: 4 });
    ctrl.store.dispatch({ type: 'setSelection', selection: { voxel: { x: 5, y: 0, z: 0 }, face: 'negX' } });
    return { ctrl, submitted };
}
test('commit submits the proposed command through the sink (generated VoxelCommand)', () => {
    const { ctrl, submitted } = controller();
    const cmd = ctrl.commit();
    assert.deepEqual(cmd, {
        op: 'setVoxel',
        grid: 0,
        coord: { x: 4, y: 0, z: 0 },
        value: { kind: 'solid', material: 4 },
    });
    assert.equal(submitted.length, 1);
    assert.deepEqual(submitted[0], [cmd]);
});
test('preview does not submit / mutate authority', () => {
    const { ctrl, submitted } = controller();
    const targets = ctrl.preview();
    assert.deepEqual(targets, [{ x: 4, y: 0, z: 0 }]);
    // Reading the proposal does not submit either.
    ctrl.proposal();
    assert.equal(submitted.length, 0, 'no submission happens until commit()');
});
test('commit through bridgeCommandSink reaches submitCommands and returns a classified result', () => {
    const bridge = createMockRuntimeBridge();
    bridge.initializeEngine({ seed: 1 });
    const results = [];
    const ctrl = new VoxelEditController(bridgeCommandSink(bridge, (r) => results.push(r)));
    ctrl.store.dispatch({ type: 'setTool', tool: 'place' });
    ctrl.store.dispatch({ type: 'setMaterial', material: 1 });
    ctrl.store.dispatch({ type: 'setSelection', selection: { voxel: { x: 0, y: 0, z: 0 }, face: 'posX' } });
    const cmd = ctrl.commit();
    assert.equal(cmd?.op, 'setVoxel');
    // The generated VoxelCommand reached the facade and authority classified it.
    assert.deepEqual(results, [{ accepted: 1, rejected: 0, rejections: [] }]);
});
test('commit with nothing to do does not call the sink', () => {
    const submitted = [];
    const ctrl = new VoxelEditController((cmds) => submitted.push([...cmds]));
    // No selection → nothing to commit.
    assert.equal(ctrl.commit(), null);
    // A non-editing tool with a selection also proposes nothing.
    ctrl.store.dispatch({ type: 'setTool', tool: 'inspect' });
    ctrl.store.dispatch({ type: 'setSelection', selection: { voxel: { x: 1, y: 1, z: 1 }, face: 'posX' } });
    assert.equal(ctrl.commit(), null);
    assert.equal(submitted.length, 0);
});
// ── Picking → selection (launch path) ──────────────────────────────────────────
test('pickAndSelect selects the struck voxel + face on an authority hit (pure action)', () => {
    const store = new EditorStore();
    // A stub authority picker returning a hit — the renderer never owns these coords.
    const hit = {
        outcome: 'hit',
        hit: {
            grid: 1,
            voxel: { x: 2, y: 0, z: 0 },
            chunk: { x: 1, y: 0, z: 0 },
            face: 'negX',
            point: [2, 0.5, 0.5],
            distance: 4,
        },
    };
    const ray = { grid: 1, origin: [-5, 0.5, 0.5], direction: [1, 0, 0], maxDistance: 100 };
    const result = pickAndSelect(store, () => hit, ray);
    assert.equal(result.outcome, 'hit');
    assert.deepEqual(store.getState().selection, { voxel: { x: 2, y: 0, z: 0 }, face: 'negX' });
});
test('pickAndSelect clears selection on a classified miss', () => {
    const store = new EditorStore();
    store.dispatch({ type: 'setSelection', selection: { voxel: { x: 9, y: 9, z: 9 }, face: 'posX' } });
    const miss = { outcome: 'miss', rejection: { reason: 'noHit' } };
    const ray = { grid: 1, origin: [0, 0, 0], direction: [1, 0, 0], maxDistance: 100 };
    const result = pickAndSelect(store, () => miss, ray);
    assert.equal(result.outcome, 'miss');
    assert.equal(store.getState().selection, null);
});
test('pointer + camera → ray → pickVoxel reaches the facade launch path', () => {
    const bridge = createMockRuntimeBridge();
    bridge.initializeEngine({ seed: 1 });
    const store = new EditorStore();
    // The renderer/UI builds the ray from camera + pointer; authority casts it.
    const ray = cameraPointerRay(defaultCamera(), [0, 0], 1, 1);
    const result = pickAndSelect(store, bridgePicker(bridge), ray);
    // The mock hosts no geometry, so the launch path returns a classified miss.
    assert.deepEqual(result, { outcome: 'miss', rejection: { reason: 'noHit' } });
    assert.equal(store.getState().selection, null);
});
//# sourceMappingURL=controller.test.js.map