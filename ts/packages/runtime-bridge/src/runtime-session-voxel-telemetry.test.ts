import { test } from 'node:test';
import assert from 'node:assert/strict';

import type { RuntimeSessionFacade, RuntimeSessionMode } from '@asha/runtime-session';

import { createMockRuntimeBridge } from './mock.js';
import {
  RuntimeBridgeError,
  createNativeRuntimeBridge,
  createRuntimeSessionFacade,
} from './index.js';

function createSession(
  mode: RuntimeSessionMode,
  bridge = createMockRuntimeBridge(),
): RuntimeSessionFacade {
  const session = createRuntimeSessionFacade({ bridge, mode });
  session.initialize({
    sessionId: `runtime-session.voxel-telemetry.${mode}.test`,
    seed: 7,
    project: { gameId: 'asha-test', workspaceId: 'workspace.local' },
  });
  return session;
}

function submitVoxel(session: RuntimeSessionFacade, x: number) {
  return session.submitCommands({
    commands: [{
      op: 'setVoxel',
      grid: 1,
      coord: { x, y: 1, z: 1 },
      value: { kind: 'solid', material: 2 },
    }],
  });
}

function assertProjectionIdentityScenario(session: RuntimeSessionFacade): void {
  const firstCommand = submitVoxel(session, 1);
  const firstProjection = session.readProjection();
  const firstWork = session.readVoxelUpdateTelemetry({
    grid: 1,
    projectionCursor: firstProjection.cursor,
  });

  assert.equal(firstCommand.result.accepted, 1);
  assert.equal(firstWork.projectionCursor, firstProjection.cursor);
  assert.equal(firstWork.committedCommandBatchCount, 1);
  assert.equal(firstWork.acceptedCommandCount, 1);
  assert.equal(firstWork.touchedVoxelCount, 1);

  const repeatedProjection = session.readProjection();
  const repeatedWork = session.readVoxelUpdateTelemetry({
    grid: 1,
    projectionCursor: repeatedProjection.cursor,
  });
  assert.notEqual(repeatedProjection.cursor, firstProjection.cursor);
  assert.equal(repeatedWork.committedCommandBatchCount, 0);
  assert.equal(repeatedWork.acceptedCommandCount, 0);
  assert.throws(
    () => session.readVoxelUpdateTelemetry({
      grid: 1,
      projectionCursor: firstProjection.cursor,
    }),
    (error: unknown) => error instanceof RuntimeBridgeError && error.kind === 'invalid_input',
  );

  const secondCommand = submitVoxel(session, 2);
  const secondProjection = session.readProjection();
  const secondWork = session.readVoxelUpdateTelemetry({
    grid: 1,
    projectionCursor: secondProjection.cursor,
  });
  assert.equal(secondCommand.result.accepted, 1);
  assert.notEqual(secondProjection.cursor, repeatedProjection.cursor);
  assert.equal(secondWork.committedCommandBatchCount, 1);
  assert.equal(secondWork.acceptedCommandCount, 1);
  assert.equal(secondWork.touchedVoxelCount, 1);
  assert.equal(firstWork.authorityTick, repeatedWork.authorityTick);
  assert.equal(repeatedWork.authorityTick, secondWork.authorityTick);
}

void test('reference RuntimeSession gives repeated reads and same-tick batches unique telemetry identities', () => {
  assertProjectionIdentityScenario(createSession('reference'));
});

void test('Rust-backed RuntimeSession gives repeated reads and same-tick batches unique telemetry identities', () => {
  assertProjectionIdentityScenario(createSession('rust'));
});

void test('native RuntimeSession gives repeated reads and same-tick batches unique telemetry identities', (t) => {
  let session: RuntimeSessionFacade;
  try {
    session = createSession('rust', createNativeRuntimeBridge());
  } catch (error) {
    if (error instanceof RuntimeBridgeError && error.kind === 'native_unavailable') {
      t.skip('native addon not built (run harness/ci/check-native.sh)');
      return;
    }
    throw error;
  }
  assertProjectionIdentityScenario(session);
});
