import { test } from 'node:test';
import assert from 'node:assert/strict';

import { cameraHandle } from '@asha/contracts';
import {
  RuntimeBridgeError,
  createRuntimeSessionFacade,
  type FpsPrimaryFireRequest,
  type FpsPrimaryFireResult,
} from './index.js';
import { MockRuntimeBridge } from './mock.js';

class CameraFireBridgeDouble extends MockRuntimeBridge {
  readonly fireRequests: FpsPrimaryFireRequest[] = [];

  override applyFpsPrimaryFire(request: FpsPrimaryFireRequest): FpsPrimaryFireResult {
    this.fireRequests.push(request);
    return super.applyFpsPrimaryFire(request);
  }
}

void test('Rust RuntimeSession resolves primary fire from the current bridge-owned camera', () => {
  const bridge = new CameraFireBridgeDouble();
  const session = createRuntimeSessionFacade({ bridge, mode: 'rust' });
  session.initialize({
    sessionId: 'runtime-session.camera-fire',
    seed: 17,
    project: { gameId: 'asha-demo', workspaceId: 'workspace.local' },
    projectBundle: { bundleSchemaVersion: 1, protocolVersion: 1, sceneId: 42 },
  });
  const camera = session.createCamera({
    initialPose: { position: [3, 1.7, 4], yawDegrees: 25, pitchDegrees: -5 },
    projection: { fovYDegrees: 55, near: 0.1, far: 100 },
    viewport: { width: 1280, height: 720 },
  }).snapshot.camera;
  session.applyFirstPersonCameraInput({
    camera,
    tick: 6,
    input: {
      moveForward: 1,
      moveRight: 0.25,
      moveUp: 0,
      yawDeltaDegrees: 12,
      pitchDeltaDegrees: 3,
      dtSeconds: 0.1,
      moveSpeedUnitsPerSecond: 3,
    },
  });
  const authoritativeCamera = session.readCameraControllerState({ camera }).snapshot;

  const receipt = session.submitRuntimeActionIntent({
    kind: 'runtime_action_intent.v0',
    action: 'primary_fire',
    phase: 'pressed',
    camera,
    tick: 7,
    source: 'programmatic',
    pressed: true,
  });

  assert.equal(receipt.accepted, true);
  assert.deepEqual(bridge.fireRequests, [{
    tick: 7,
    origin: authoritativeCamera.pose.position,
    direction: authoritativeCamera.basis.forward,
  }]);
});

void test('unknown camera rejects before primary-fire authority or facade progress mutates', () => {
  const bridge = new CameraFireBridgeDouble();
  const session = createRuntimeSessionFacade({ bridge, mode: 'rust' });
  session.initialize({
    sessionId: 'runtime-session.camera-fire.invalid',
    seed: 19,
    project: { gameId: 'asha-demo', workspaceId: 'workspace.local' },
    projectBundle: { bundleSchemaVersion: 1, protocolVersion: 1, sceneId: 43 },
  });
  const before = session.readTelemetry();

  assert.throws(
    () => session.submitRuntimeActionIntent({
      kind: 'runtime_action_intent.v0',
      action: 'primary_fire',
      phase: 'pressed',
      camera: cameraHandle(999),
      tick: 7,
      source: 'programmatic',
      pressed: true,
    }),
    (error: unknown) => error instanceof RuntimeBridgeError && error.kind === 'unknown_handle',
  );
  assert.deepEqual(bridge.fireRequests, []);
  assert.deepEqual(session.readTelemetry(), before);
});
