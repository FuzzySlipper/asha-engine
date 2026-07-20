import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
  BrowserInputHost,
  ResolvedCameraNavigationConsumer,
  createRuntimeSessionFacade,
  type ResolvedCameraActionReceipt,
} from './index.js';
import { MockRuntimeBridge } from './mock.js';

function createCameraSession() {
  const bridge = new MockRuntimeBridge();
  const session = createRuntimeSessionFacade({ bridge, mode: 'reference' });
  session.initialize({
    sessionId: 'camera-modes.test',
    seed: 31,
    project: { gameId: 'camera-modes', workspaceId: 'workspace.camera' },
  });
  const camera = session.createCamera({
    initialPose: { position: [0, 1.6, 0], yawDegrees: 0, pitchDegrees: 0 },
    projection: { fovYDegrees: 60, near: 0.1, far: 500 },
    viewport: { width: 1280, height: 720 },
  }).snapshot.camera;
  return { bridge, camera, session };
}

void test('one resolved-input path switches FPS, orbit, and top-down without simultaneous controllers', () => {
  const { bridge, camera, session } = createCameraSession();
  let tick = 1;
  const receipts: ResolvedCameraActionReceipt[] = [];
  const consumer = new ResolvedCameraNavigationConsumer({
    session,
    camera,
    selectedPivot: () => [3, 1, -5],
    nextTick: () => tick++,
  });
  const host = new BrowserInputHost({
    session,
    onResolvedAction: (action) => {
      const receipt = consumer.consume(action);
      if (receipt !== null) receipts.push(receipt);
    },
  });
  host.setPointerLockActive(true);

  const orbit = host.handleKeyDown({ code: 'KeyO' });
  assert.equal(orbit.receipt.action?.actionId, 'camera.mode.orbit');
  assert.equal(receipts.at(-1)?.kind, 'mode');
  assert.equal(session.readCameraControllerState({ camera }).mode, 'orbit');
  assert.deepEqual(host.readout().activeContexts, ['gameplay', 'cameraNavigation']);
  assert.throws(
    () => bridge.applyFirstPersonCameraInput({
      camera,
      tick: 2,
      input: {
        moveForward: 1,
        moveRight: 0,
        moveUp: 0,
        yawDeltaDegrees: 0,
        pitchDeltaDegrees: 0,
        dtSeconds: 1 / 60,
        moveSpeedUnitsPerSecond: 3,
      },
    }),
    /requires firstPerson camera mode/u,
  );

  const pan = host.handleKeyDown({ code: 'KeyW' });
  assert.equal(pan.receipt.action?.actionId, 'camera.navigation.panForward');
  assert.notEqual(pan.receipt.action?.actionId, 'gameplay.move.forward');
  const rotated = host.handleMouseMove({ movementX: 10, movementY: -4 });
  assert.equal(rotated?.receipt.action?.actionId, 'camera.navigation.rotate');
  const zoomed = host.handleWheel({ deltaY: 100 });
  assert.equal(zoomed?.receipt.action?.actionId, 'camera.navigation.zoom');
  const navigated = session.readCameraControllerState({ camera });
  assert.equal(navigated.mode, 'orbit');
  assert.equal(navigated.revision, 4);
  assert.equal(navigated.distance, 7);
  assert.notDeepEqual(navigated.pivot, [3, 1, -5]);

  host.handleKeyDown({ code: 'KeyT' });
  const topDown = session.readCameraControllerState({ camera });
  assert.equal(topDown.mode, 'topDown');
  assert.equal(topDown.pivot?.[0], 3);

  const firstPerson = host.handleKeyDown({ code: 'KeyF' });
  assert.equal(firstPerson.receipt.action?.actionId, 'camera.mode.firstPerson');
  assert.equal(session.readCameraControllerState({ camera }).mode, 'firstPerson');
  assert.deepEqual(host.readout().activeContexts, ['gameplay']);
  assert.equal(host.handleKeyDown({ code: 'KeyW' }).receipt.action?.actionId, 'gameplay.move.forward');
});

void test('missing pivot and stale revisions fail without changing camera or input context', () => {
  const { camera, session } = createCameraSession();
  const consumer = new ResolvedCameraNavigationConsumer({
    session,
    camera,
    selectedPivot: () => null,
    nextTick: () => 1,
  });
  const host = new BrowserInputHost({ session });
  const resolved = host.handleKeyDown({ code: 'KeyO' }).receipt.action;
  const rejected = consumer.consume(resolved!);
  assert.equal(rejected?.kind, 'mode');
  assert.equal(rejected?.kind === 'mode' ? rejected.rejection : null, 'missingSelectedPivot');
  assert.equal(session.readCameraControllerState({ camera }).revision, 0);
  assert.deepEqual(host.readout().activeContexts, ['gameplay']);

  const stale = session.applyCameraModeCommand({
    camera,
    expectedRevision: 99,
    target: { mode: 'firstPerson', pose: { position: [0, 1.6, 0], yawDegrees: 0, pitchDegrees: 0 } },
    transition: null,
    tick: 2,
  });
  assert.equal(stale.accepted, false);
  assert.equal(stale.rejection, 'staleRevision');
  assert.equal(stale.before.stateHash, stale.after.stateHash);
});
