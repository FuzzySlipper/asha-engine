import { test } from 'node:test';
import assert from 'node:assert/strict';

import { BrowserFpsInputCollector, createMockRuntimeSession } from './index.js';

function initializedSession() {
  const session = createMockRuntimeSession();
  session.initialize({
    sessionId: 'runtime-session.browser-fps.test',
    seed: 23,
    project: { gameId: 'asha-demo', workspaceId: 'workspace.local' },
    projectBundle: { bundleSchemaVersion: 1, protocolVersion: 1, sceneId: 7 },
  });
  const camera = session.createCamera({
    initialPose: { position: [0, 1.6, 0], yawDegrees: 0, pitchDegrees: 0 },
    projection: { fovYDegrees: 60, near: 0.1, far: 1000 },
    viewport: { width: 1280, height: 720 },
  });
  return { session, camera: camera.snapshot.camera };
}

test('BrowserFpsInputCollector maps WASD and mouse deltas to a RuntimeSession camera command', () => {
  const { session, camera } = initializedSession();
  const input = new BrowserFpsInputCollector({
    camera,
    moveSpeedUnitsPerSecond: 3,
    mouseSensitivityDegreesPerPixel: 0.1,
    pointerLocked: true,
  });

  input.handleKeyDown({ code: 'KeyW' });
  input.handleKeyDown({ code: 'KeyD' });
  input.handleMouseMove({ movementX: 12, movementY: -4 });

  const frame = input.drainFrame({ tick: 1, dtSeconds: 1 / 60 });
  assert.equal(frame.runtimeCommand.kind, 'runtime.apply_first_person_camera_input');
  assert.deepEqual(frame.runtimeCommand.envelope.input, {
    moveForward: 1,
    moveRight: 1,
    moveUp: 0,
    yawDeltaDegrees: 1.2000000000000002,
    pitchDeltaDegrees: 0.4,
    dtSeconds: 1 / 60,
    moveSpeedUnitsPerSecond: 3,
  });
  assert.deepEqual(frame.readout.pendingMouseDelta, [12, -4]);

  const receipt = session.applyFirstPersonCameraInput(frame.runtimeCommand.envelope);
  assert.equal(receipt.snapshot.tick, 1);
  assert.notDeepEqual(receipt.snapshot.pose, { position: [0, 1.6, 0], yawDegrees: 0, pitchDegrees: 0 });
  assert.deepEqual(input.readout().pendingMouseDelta, [0, 0]);
});

test('BrowserFpsInputCollector emits pointer lock request and Escape release readout', () => {
  const { camera } = initializedSession();
  const input = new BrowserFpsInputCollector({
    camera,
    moveSpeedUnitsPerSecond: 3,
    mouseSensitivityDegreesPerPixel: 0.1,
  });

  const request = input.handlePointerDown({ button: 0 });
  assert.deepEqual(request, [{ kind: 'request_pointer_lock', reason: 'primary_button' }]);
  input.setPointerLockActive(true);

  const release = input.handleKeyDown({ code: 'Escape' });
  assert.deepEqual(release, [{ kind: 'release_pointer_lock', reason: 'escape_key' }]);
  assert.equal(input.readout().releaseRequestedByEscape, true);

  const frame = input.drainFrame({ tick: 2, dtSeconds: 0 });
  assert.deepEqual(frame.pointerLockIntents, [
    { kind: 'request_pointer_lock', reason: 'primary_button' },
    { kind: 'release_pointer_lock', reason: 'escape_key' },
  ]);
  assert.deepEqual(input.drainFrame({ tick: 3, dtSeconds: 0 }).pointerLockIntents, []);
});

test('BrowserFpsInputCollector maps primary fire to a typed runtime action intent', () => {
  const { session, camera } = initializedSession();
  const input = new BrowserFpsInputCollector({
    camera,
    moveSpeedUnitsPerSecond: 3,
    mouseSensitivityDegreesPerPixel: 0.1,
    pointerLocked: true,
  });

  input.handlePointerDown({ button: 0 });
  const frame = input.drainFrame({ tick: 4, dtSeconds: 0 });

  assert.deepEqual(frame.runtimeActionIntents, [
    {
      kind: 'runtime.propose_runtime_action_intent',
      envelope: {
        kind: 'runtime_action_intent.v0',
        action: 'primary_fire',
        phase: 'pressed',
        camera,
        tick: 4,
        source: 'browser_fps_pointer',
        pressed: true,
      },
    },
  ]);
  assert.deepEqual(frame.unsupportedIntents, []);
  const primaryFireIntent = frame.runtimeActionIntents[0];
  assert.ok(primaryFireIntent);
  assert.equal('payload' in primaryFireIntent, false);

  const receipt = session.submitRuntimeActionIntent(primaryFireIntent.envelope);
  assert.equal(receipt.accepted, true);
  assert.equal(receipt.status, 'accepted');
  assert.equal(receipt.rejection, null);
  assert.equal(receipt.combatReadout?.outcome.kind, 'hit');
  assert.equal(receipt.combatReadout?.health[0]?.dead, true);

  input.handlePointerUp({ button: 0 });
  const releaseFrame = input.drainFrame({ tick: 5, dtSeconds: 0 });
  assert.deepEqual(releaseFrame.runtimeActionIntents, [
    {
      kind: 'runtime.propose_runtime_action_intent',
      envelope: {
        kind: 'runtime_action_intent.v0',
        action: 'primary_fire',
        phase: 'released',
        camera,
        tick: 5,
        source: 'browser_fps_pointer',
        pressed: false,
      },
    },
  ]);
  const emptyFrame = input.drainFrame({ tick: 6, dtSeconds: 0 });
  assert.deepEqual(emptyFrame.runtimeActionIntents, []);
  assert.deepEqual(emptyFrame.unsupportedIntents, []);
});
