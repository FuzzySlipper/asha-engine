import { test } from 'node:test';
import assert from 'node:assert/strict';

import type { CommandResult, PickRay, PickResult, VoxelCommand } from '@asha/contracts';
import { createMockRuntimeBridge } from '@asha/runtime-bridge/reference';
import { cameraPointerRay, defaultCamera } from '@asha/ui-dom';
import {
  EditorStore,
  VoxelEditController,
  bridgeCommandSink,
  bridgePicker,
  pickAndSelect,
  revalidatePickHint,
} from './index.js';

function controller() {
  const submitted: VoxelCommand[][] = [];
  const ctrl = new VoxelEditController((cmds) => submitted.push([...cmds]));
  // A place tool with a selection so there is something to commit.
  ctrl.store.dispatch({ type: 'setTool', tool: 'place' });
  ctrl.store.dispatch({ type: 'setMaterial', material: 4 });
  ctrl.store.dispatch({ type: 'setSelection', selection: { voxel: { x: 5, y: 0, z: 0 }, face: 'negX' } });
  return { ctrl, submitted };
}

void test('commit submits the proposed command through the sink (generated VoxelCommand)', () => {
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

void test('preview does not submit / mutate authority', () => {
  const { ctrl, submitted } = controller();
  const targets = ctrl.preview();
  assert.deepEqual(targets, [{ x: 4, y: 0, z: 0 }]);
  // Reading the proposal does not submit either.
  ctrl.proposal();
  assert.equal(submitted.length, 0, 'no submission happens until commit()');
});

void test('commit through bridgeCommandSink reaches submitCommands and returns a classified result', () => {
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

void test('commit with nothing to do does not call the sink', () => {
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

void test('cancel clears the draft selection without submitting (symmetric with commit)', () => {
  const { ctrl, submitted } = controller();
  assert.ok(ctrl.proposal(), 'there is a draft to cancel');
  ctrl.cancel();
  assert.equal(ctrl.store.getState().selection, null, 'the draft/preview is cleared');
  assert.equal(ctrl.proposal(), null, 'nothing remains to commit');
  assert.equal(submitted.length, 0, 'cancel never calls the command sink');
});

// ── Picking → selection (launch path) ──────────────────────────────────────────

void test('pickAndSelect selects the struck voxel + face on an authority hit (pure action)', () => {
  const store = new EditorStore();
  // A stub authority picker returning a hit — the renderer never owns these coords.
  const hit: PickResult = {
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
  const ray: PickRay = { grid: 1, origin: [-5, 0.5, 0.5], direction: [1, 0, 0], maxDistance: 100 };
  const result = pickAndSelect(store, () => hit, ray);
  assert.equal(result.outcome, 'hit');
  assert.deepEqual(store.getState().selection, { voxel: { x: 2, y: 0, z: 0 }, face: 'negX' });
});

void test('pickAndSelect clears selection on a classified miss', () => {
  const store = new EditorStore();
  store.dispatch({ type: 'setSelection', selection: { voxel: { x: 9, y: 9, z: 9 }, face: 'posX' } });
  const miss: PickResult = { outcome: 'miss', rejection: { reason: 'noHit' } };
  const ray: PickRay = { grid: 1, origin: [0, 0, 0], direction: [1, 0, 0], maxDistance: 100 };
  const result = pickAndSelect(store, () => miss, ray);
  assert.equal(result.outcome, 'miss');
  assert.equal(store.getState().selection, null);
});

void test('pointer + camera → ray → pickVoxel reaches the facade launch path', () => {
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

// ── Renderer-hint revalidation (authority is the source of truth) ──────────────

const authorityHit: PickResult = {
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

void test('revalidatePickHint passes a confirmed renderer hint through unchanged', () => {
  const result = revalidatePickHint(authorityHit, { voxel: { x: 2, y: 0, z: 0 }, face: 'negX' });
  assert.deepEqual(result, authorityHit);
});

void test('revalidatePickHint classifies a stale renderer hint as hitMismatch', () => {
  // The renderer claims a different cell/face than authority hit → stale metadata.
  const result = revalidatePickHint(authorityHit, { voxel: { x: 9, y: 9, z: 9 }, face: 'posX' });
  assert.deepEqual(result, {
    outcome: 'miss',
    rejection: {
      reason: 'hitMismatch',
      authoritativeVoxel: { x: 2, y: 0, z: 0 },
      authoritativeFace: 'negX',
      claimedVoxel: { x: 9, y: 9, z: 9 },
      claimedFace: 'posX',
    },
  });
});

void test('revalidatePickHint passes an authority miss through (nothing to reconcile)', () => {
  const miss: PickResult = { outcome: 'miss', rejection: { reason: 'noHit' } };
  assert.deepEqual(revalidatePickHint(miss, { voxel: { x: 0, y: 0, z: 0 }, face: 'posX' }), miss);
});
