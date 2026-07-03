import { test } from 'node:test';
import assert from 'node:assert/strict';

import type { VoxelCommand } from '@asha/contracts';
import { RuntimeBridgeError, createMockRuntimeSession } from './index.js';

function sessionInput() {
  return {
    sessionId: 'runtime-session.asha-demo.reference',
    seed: 17,
    project: {
      gameId: 'asha-demo',
      workspaceId: 'workspace.local',
    },
    projectBundle: {
      bundleSchemaVersion: 1,
      protocolVersion: 1,
      sceneId: 42,
    },
  };
}

test('RuntimeSession initializes, ticks, reads projection and telemetry, then restarts', () => {
  const session = createMockRuntimeSession();
  const initialized = session.initialize(sessionInput());

  assert.equal(initialized.identity.sessionId, 'runtime-session.asha-demo.reference');
  assert.equal(initialized.identity.mode, 'reference');
  assert.equal(initialized.composition.loadedWorld, 42);
  assert.ok(initialized.identity.nonClaims.includes('not_raw_state_store'));
  assert.ok(initialized.identity.nonClaims.includes('not_arbitrary_json_bridge'));

  const command: VoxelCommand = {
    op: 'setVoxel',
    grid: 1,
    coord: { x: 0, y: 0, z: 0 },
    value: { kind: 'solid', material: 1 },
  };
  const receipt = session.submitCommands({ commands: [command] });
  assert.equal(receipt.result.accepted, 1);
  assert.equal(receipt.result.rejected, 0);
  assert.notEqual(receipt.sessionHashAfter, receipt.sessionHashBefore);

  const tick = session.tick();
  assert.equal(tick.tick, 1);
  assert.equal(tick.composition.loadedWorld, 42);

  const projection = session.readProjection();
  assert.equal(projection.sequenceId, tick.sequenceId);
  assert.equal(projection.renderDiffCount, 0);
  assert.ok(projection.projectionHash.startsWith('fnv1a64:'));

  const telemetry = session.readTelemetry();
  assert.equal(telemetry.acceptedCommandCount, 1);
  assert.equal(telemetry.rejectedCommandCount, 0);
  assert.equal(telemetry.replayRecords.map((record) => record.kind).join(','), 'initialize,submitCommands,tick');

  const restarted = session.restart();
  assert.equal(restarted.tick, 0);
  assert.equal(restarted.restartCount, 1);
  assert.equal(restarted.composition.loadedWorld, 42);

  const afterRestart = session.readTelemetry();
  assert.equal(afterRestart.acceptedCommandCount, 0);
  assert.equal(afterRestart.rejectedCommandCount, 0);
  assert.equal(afterRestart.replayRecords.at(-1)?.kind, 'restart');
});

test('RuntimeSession fails closed before initialize and on unsupported ProjectBundle', () => {
  const session = createMockRuntimeSession();
  assert.throws(
    () => session.tick(),
    (error: unknown) => error instanceof RuntimeBridgeError && error.kind === 'not_initialized',
  );

  assert.throws(
    () =>
      session.initialize({
        ...sessionInput(),
        projectBundle: {
          bundleSchemaVersion: 99,
          protocolVersion: 1,
          sceneId: 42,
        },
      }),
    (error: unknown) => error instanceof RuntimeBridgeError && error.kind === 'invalid_input',
  );
});
