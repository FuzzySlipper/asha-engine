import { test } from 'node:test';
import assert from 'node:assert/strict';

import type { CollisionConstrainedCameraInputEnvelope } from '@asha/contracts';

import { createMockRuntimeSession } from './reference.js';

void test('collision camera free flight is explicit and grounded vertical input fails closed', () => {
  const session = createMockRuntimeSession();
  session.initialize({
    sessionId: 'runtime-session.camera-movement.reference',
    seed: 17,
    project: { gameId: 'asha-demo', workspaceId: 'workspace.local' },
  });
  const camera = session.createCamera({
    initialPose: { position: [20, 20, 20], yawDegrees: 40, pitchDegrees: -45 },
    projection: { fovYDegrees: 60, near: 0.1, far: 100 },
    viewport: { width: 1280, height: 720 },
  }).snapshot.camera;
  const envelope: CollisionConstrainedCameraInputEnvelope = {
    camera,
    grid: 1,
    movementMode: 'freeFlight',
    input: {
      moveForward: 1,
      moveRight: 0,
      moveUp: 0,
      yawDeltaDegrees: 0,
      pitchDeltaDegrees: 0,
      dtSeconds: 1,
      moveSpeedUnitsPerSecond: 3,
    },
    tick: 1,
    shape: { halfExtents: [0.25, 0.7, 0.25] },
    policy: { mode: 'axis_separable_slide', maxIterations: 3 },
  };

  const freeFlight = session.applyCollisionConstrainedCameraInput(envelope);
  assert.equal(freeFlight.snapshot.collision.movementMode, 'freeFlight');
  assert.ok(freeFlight.snapshot.attempted.pose.position[1] < 20);
  assert.throws(
    () => session.applyCollisionConstrainedCameraInput({
      ...envelope,
      movementMode: 'grounded',
      input: { ...envelope.input, moveForward: 0, moveUp: 1 },
      tick: 2,
    }),
    /select freeFlight for vertical locomotion/u,
  );
});
