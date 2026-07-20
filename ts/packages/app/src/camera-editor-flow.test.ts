import { test } from 'node:test';
import assert from 'node:assert/strict';
import { EditorStore, editorCameraPivot } from '@asha/editor-tools';
import {
  BrowserInputHost,
  ResolvedCameraNavigationConsumer,
  createRuntimeSessionFacade,
} from '@asha/runtime-bridge';
import { createMockRuntimeBridge } from '@asha/runtime-bridge/reference';

void test('editor selection drives the public orbit pan zoom and FPS return flow', () => {
  const editor = new EditorStore();
  editor.dispatch({
    type: 'setSelection',
    selection: { voxel: { x: 3, y: 1, z: -5 }, face: 'posY' },
  });
  const session = createRuntimeSessionFacade({ bridge: createMockRuntimeBridge(), mode: 'reference' });
  session.initialize({
    sessionId: 'app.camera-editor-flow',
    seed: 41,
    project: { gameId: 'camera-editor-flow', workspaceId: 'workspace.editor' },
  });
  const camera = session.createCamera({
    initialPose: { position: [0, 1.6, 0], yawDegrees: 0, pitchDegrees: 0 },
    projection: { fovYDegrees: 60, near: 0.1, far: 500 },
    viewport: { width: 1280, height: 720 },
  }).snapshot.camera;
  let tick = 1;
  const cameraConsumer = new ResolvedCameraNavigationConsumer({
    session,
    camera,
    selectedPivot: () => editorCameraPivot(editor.getState()),
    nextTick: () => tick++,
  });
  const input = new BrowserInputHost({
    session,
    onResolvedAction: (action) => { cameraConsumer.consume(action); },
  });
  input.setPointerLockActive(true);

  input.handleKeyDown({ code: 'KeyO' });
  assert.deepEqual(session.readCameraControllerState({ camera }).pivot, [3.5, 1.5, -4.5]);
  input.handleMouseMove({ movementX: 12, movementY: -4 });
  input.handleKeyDown({ code: 'KeyD' });
  input.handleWheel({ deltaY: 100 });
  const orbit = session.readCameraControllerState({ camera });
  assert.equal(orbit.mode, 'orbit');
  assert.equal(orbit.distance, 7);
  assert.notDeepEqual(orbit.pivot, [3.5, 1.5, -4.5]);

  input.handleKeyDown({ code: 'KeyF' });
  assert.equal(session.readCameraControllerState({ camera }).mode, 'firstPerson');
  assert.equal(input.handleKeyDown({ code: 'KeyW' }).receipt.action?.actionId, 'gameplay.move.forward');
});
