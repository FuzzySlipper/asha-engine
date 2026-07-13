import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { EditorResolvedInputFrame } from '@asha/editor-tools';
import { createMockRuntimeBridge } from '@asha/runtime-bridge/reference';

import { composeAppShell } from './shell.js';

void test('app shell composes resolved editor input into camera and real tool behavior', () => {
  const bridge = createMockRuntimeBridge();
  const cameraFrames: EditorResolvedInputFrame[] = [];
  const shell = composeAppShell({
    host: { name: 'browser', accessibility: true },
    bootBridge: () => ({
      bridge,
      mode: 'mock',
      intent: 'reference',
      nativeAvailable: false,
    }),
    fixtures: [{
      id: 'editor-input',
      label: 'Editor input',
      materials: [1],
      request: { bundleSchemaVersion: 1, protocolVersion: 1, sceneId: 1 },
    }],
  });
  shell.controller.store.dispatch({
    type: 'setSelection',
    selection: { voxel: { x: 0, y: 0, z: 0 }, face: 'posX' },
  });
  const input = shell.createEditorInput({ apply: (frame) => cameraFrames.push(frame) });
  assert.ok(input);
  input.host.setPointerLockActive(true);

  input.host.handleKeyDown({ code: 'KeyD' });
  input.host.handleMouseMove({ movementX: 5, movementY: -2 });
  input.host.handlePointerDown({ button: 0 });
  const applied = input.drain();

  assert.equal(applied.committed?.op, 'setVoxel');
  assert.equal(applied.cancelled, false);
  assert.deepEqual(cameraFrames[0], {
    cameraForward: 0,
    cameraRight: 1,
    lookDelta: [5, -2],
    primaryToolPressed: true,
    cancelPressed: false,
  });
  assert.equal(shell.readout().lastCommandResult?.accepted, 1);

  shell.controller.store.dispatch({
    type: 'setSelection',
    selection: { voxel: { x: 2, y: 0, z: 0 }, face: 'negX' },
  });
  input.host.handleKeyDown({ code: 'Escape' });
  const cancelled = input.drain();
  assert.equal(cancelled.cancelled, true);
  assert.equal(cancelled.committed, null);
  assert.equal(shell.controller.store.getState().selection, null);
  assert.deepEqual(input.host.readout().activeContexts, ['editor']);
});
