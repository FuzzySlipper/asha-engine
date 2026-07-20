import { test } from 'node:test';
import assert from 'node:assert/strict';

import { cameraHandle } from '@asha/contracts';
import {
  RuntimeBridgeError,
  createRuntimeSessionFacade,
  type FpsPrimaryFireRequest,
  type FpsPrimaryFireResult,
  type FpsRuntimeSessionSnapshot,
} from './index.js';
import { MockRuntimeBridge } from './mock.js';

class CameraFireBridgeDouble extends MockRuntimeBridge {
  readonly fireRequests: FpsPrimaryFireRequest[] = [];

  override applyFpsPrimaryFire(request: FpsPrimaryFireRequest): FpsPrimaryFireResult {
    this.fireRequests.push(request);
    return {
      backend: 'reference_bridge',
      authoritySurface: 'runtime_session.camera_fire_test.v0',
      mutationOwner: 'test-double',
      workspaceTrace: ['camera-authority-forwarded'],
      shooter: 1,
      target: 2,
      targetHealthBefore: { current: 10, max: 10 },
      targetHealthAfter: { current: 9, max: 10 },
      lifecycleStatus: { state: 'active' },
      targetRenderVisible: true,
      entityHash: 'fnv1a64:0000000000000001',
      healthHash: 'fnv1a64:0000000000000002',
      replayHash: 'fnv1a64:0000000000000003',
    };
  }

  override readFpsRuntimeSession(): FpsRuntimeSessionSnapshot {
    return {
      backend: 'reference_bridge',
      authoritySurface: 'runtime_session.camera_fire_test.v0',
      projectBundle: 'canonical-test-project',
      sessionEpoch: 1,
      lifecycleStatus: { state: 'active' },
      playerEntity: 1,
      enemyEntity: 2,
      health: [{ entity: 1, current: 10, max: 10 }, { entity: 2, current: 9, max: 10 }],
      policyBindings: [],
      replayRecords: [{
        replayUnit: 'runtime_session.camera_fire_test.v0',
        entityHash: 'fnv1a64:0000000000000001',
        healthHash: 'fnv1a64:0000000000000002',
        recordHash: 'fnv1a64:0000000000000003',
      }],
      readSets: [],
      entityHash: 'fnv1a64:0000000000000001',
      healthHash: 'fnv1a64:0000000000000002',
      replayHash: 'fnv1a64:0000000000000003',
    };
  }
}

void test('Rust RuntimeSession resolves primary fire from the current bridge-owned camera', () => {
  const bridge = new CameraFireBridgeDouble();
  const session = createRuntimeSessionFacade({ bridge, mode: 'rust' });
  session.initialize({
    sessionId: 'runtime-session.camera-fire',
    seed: 17,
    project: { gameId: 'asha-demo', workspaceId: 'workspace.local' },
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
