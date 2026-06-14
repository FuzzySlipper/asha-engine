import { test } from 'node:test';
import assert from 'node:assert/strict';

import type { CommandResult, VoxelCommand } from '@asha/contracts';
import { createMockRuntimeBridge } from '@asha/runtime-bridge';
import { VoxelEditController, bridgeCommandSink } from './index.js';

function controller() {
  const submitted: VoxelCommand[][] = [];
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
  const results: CommandResult[] = [];
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
  const submitted: VoxelCommand[][] = [];
  const ctrl = new VoxelEditController((cmds) => submitted.push([...cmds]));
  // No selection → nothing to commit.
  assert.equal(ctrl.commit(), null);
  // A non-editing tool with a selection also proposes nothing.
  ctrl.store.dispatch({ type: 'setTool', tool: 'inspect' });
  ctrl.store.dispatch({ type: 'setSelection', selection: { voxel: { x: 1, y: 1, z: 1 }, face: 'posX' } });
  assert.equal(ctrl.commit(), null);
  assert.equal(submitted.length, 0);
});
