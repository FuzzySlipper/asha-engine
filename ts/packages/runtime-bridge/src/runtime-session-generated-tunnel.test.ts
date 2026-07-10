import assert from 'node:assert/strict';
import { test } from 'node:test';
import { RuntimeBridgeError } from './index.js';
import { createMockRuntimeSession } from './reference.js';

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

void test('RuntimeSession exposes the generated tunnel fixture readout and fail-closed operations', () => {
  const session = createMockRuntimeSession();
  session.initialize(sessionInput());

  const readout = session.readGeneratedTunnelReadout({ presetId: 'tiny-enclosed', seed: 17 });
  assert.equal(readout.status, 'available');
  assert.equal(readout.generator.generatorId, 'asha.tunnel.enclosed.v2');
  assert.equal(readout.generator.presetId, 'tiny-enclosed');
  assert.equal(readout.generator.seed, 17);
  assert.equal(readout.generator.configHash, 'e1d156c6b55137a7');
  assert.equal(readout.generator.outputHash, '1471496d88d70647');
  assert.equal(readout.replayHash, 'fnv1a64:0821a0c2aea17dff');
  assert.deepEqual(readout.volume.tunnelDims, [5, 4, 9]);
  assert.equal(readout.volume.solidVoxels, 282);
  assert.equal(readout.corridors.count, 1);
  assert.equal(readout.rooms.count, 0);
  assert.deepEqual(readout.spawnMarkers.map((marker) => marker.id), ['player_start', 'exit_hint']);
  assert.deepEqual(readout.materials.map((material) => `${material.role}:${material.material}`), [
    'wall:1',
    'floor:2',
    'accent:3',
  ]);
  assert.equal(readout.renderProjection.hash, 'fnv1a64:21eb8696f6f3b5c4');
  assert.equal(readout.collisionProjection.hash, 'fnv1a64:627389be013a3154');
  assert.deepEqual(readout.runtimeFrame, {
    worldOffset: [-3.5, -1, -5.5],
    playableMin: [-2.5, 0, -4.5],
    playableMax: [2.5, 4, 4.5],
  });

  const operation = session.requestGeneratedTunnelOperation({
    operation: 'regenerate',
    presetId: 'tiny-enclosed',
    seed: 17,
  });
  assert.equal(operation.status, 'unsupported');
  assert.equal(operation.reason, 'generated_tunnel_operation_not_wired');
  assert.equal('payload' in operation, false);
  assert.equal(session.readTelemetry().replayRecords.at(-1)?.kind, 'requestGeneratedTunnelOperation');

  assert.throws(
    () => session.readGeneratedTunnelReadout({ presetId: 'tiny-enclosed', seed: 18 }),
    (error: unknown) => error instanceof RuntimeBridgeError && error.kind === 'invalid_input',
  );
});
