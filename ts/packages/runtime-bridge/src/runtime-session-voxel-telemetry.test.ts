import { test } from 'node:test';
import assert from 'node:assert/strict';

import { createMockRuntimeBridge } from './mock.js';
import { createRuntimeSessionFacade } from './runtime-session-adapter.js';

void test('RuntimeSession correlates command projection and bounded voxel work through public methods', () => {
  const session = createRuntimeSessionFacade({
    bridge: createMockRuntimeBridge(),
    mode: 'reference',
  });
  session.initialize({
    sessionId: 'runtime-session.voxel-telemetry.test',
    seed: 7,
    project: { gameId: 'asha-test', workspaceId: 'workspace.local' },
  });
  const command = session.submitCommands({
    commands: [{
      op: 'setVoxel',
      grid: 1,
      coord: { x: 2, y: 1, z: 0 },
      value: { kind: 'solid', material: 2 },
    }],
  });
  const projection = session.readProjection();
  const work = session.readVoxelUpdateTelemetry({
    grid: 1,
    projectionCursor: projection.cursor,
  });

  assert.equal(command.result.accepted, 1);
  assert.equal(work.projectionCursor, projection.cursor);
  assert.equal(work.committedCommandBatchCount, 1);
  assert.equal(work.acceptedCommandCount, command.result.accepted);
  assert.equal(work.touchedVoxelCount, 1);
  assert.equal(work.emittedRenderOpCount, projection.frame.ops.length);
});
