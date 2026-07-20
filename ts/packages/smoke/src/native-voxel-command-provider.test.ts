import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { VoxelCommand, VoxelEditRejection, VoxelValue } from '@asha/contracts';
import {
  RuntimeBridgeError,
  createNativeRuntimeBridge,
  createRuntimeSessionFacade,
} from '@asha/runtime-bridge';
import type { RuntimeSessionFacade } from '@asha/runtime-session';

const VALUES_BY_KIND = {
  empty: { kind: 'empty' },
  solid: { kind: 'solid', material: 2 },
} satisfies Record<VoxelValue['kind'], VoxelValue>;

const COMMANDS_BY_OP = {
  generateChunk: {
    op: 'generateChunk',
    grid: 1,
    chunk: { x: 0, y: 0, z: 0 },
    seed: 77,
    generatorVersion: 1,
  },
  fillRegion: {
    op: 'fillRegion',
    grid: 1,
    min: { x: 0, y: 0, z: 0 },
    max: { x: 2, y: 2, z: 2 },
    value: VALUES_BY_KIND.empty,
  },
  setVoxel: {
    op: 'setVoxel',
    grid: 1,
    coord: { x: 1, y: 1, z: 1 },
    value: VALUES_BY_KIND.empty,
  },
} satisfies Record<VoxelCommand['op'], VoxelCommand>;

const REJECTIONS_BY_REASON = {
  unknownMaterial: { reason: 'unknownMaterial', material: 65535 },
  emptyRegion: {
    reason: 'emptyRegion',
    min: { x: 1, y: 1, z: 1 },
    max: { x: 1, y: 1, z: 1 },
  },
  chunkNotResident: { reason: 'chunkNotResident', chunk: { x: 50, y: 0, z: 0 } },
  generationDivergence: {
    reason: 'generationDivergence',
    chunk: { x: 0, y: 0, z: 0 },
    expected: 1,
    actual: 2,
  },
} satisfies Record<VoxelEditRejection['reason'], VoxelEditRejection>;

void test('native provider accepts the generated voxel command union and returns typed rejections', (t) => {
  let session: RuntimeSessionFacade;
  try {
    session = createRuntimeSessionFacade({
      bridge: createNativeRuntimeBridge(),
      mode: 'rust',
    });
  } catch (error) {
    if (error instanceof RuntimeBridgeError && error.kind === 'native_unavailable') {
      t.skip('native addon not built; run harness/ci/check-native.sh for this provider regression');
      return;
    }
    throw error;
  }
  session.initialize({
    sessionId: 'runtime-session.voxel-command.provider-regression',
    seed: 77,
    project: { gameId: 'asha-provider-regression', workspaceId: 'workspace.local' },
  });

  const result = session.submitCommands({
    commands: [
      COMMANDS_BY_OP.generateChunk,
      COMMANDS_BY_OP.fillRegion,
      { ...COMMANDS_BY_OP.fillRegion, value: VALUES_BY_KIND.solid },
      COMMANDS_BY_OP.setVoxel,
    ],
  });

  assert.deepEqual(result.result, { accepted: 4, rejected: 0, rejections: [] });
  assert.equal(session.readTelemetry().acceptedCommandCount, 4);

  const unknownMaterial = session.submitCommands({
    commands: [
      {
        op: 'setVoxel',
        grid: 1,
        coord: { x: 0, y: 0, z: 0 },
        value: { kind: 'solid', material: 65535 },
      },
    ],
  });
  assert.deepEqual(unknownMaterial.result, {
    accepted: 0,
    rejected: 1,
    rejections: [REJECTIONS_BY_REASON.unknownMaterial],
  });

  const emptyRegion = session.submitCommands({
    commands: [
      {
        op: 'fillRegion',
        grid: 1,
        min: { x: 1, y: 1, z: 1 },
        max: { x: 1, y: 1, z: 1 },
        value: { kind: 'empty' },
      },
    ],
  });
  assert.deepEqual(emptyRegion.result, {
    accepted: 0,
    rejected: 1,
    rejections: [REJECTIONS_BY_REASON.emptyRegion],
  });

  const nonResident = session.submitCommands({
    commands: [
      {
        op: 'setVoxel',
        grid: 1,
        coord: { x: 100, y: 0, z: 0 },
        value: { kind: 'empty' },
      },
    ],
  });
  assert.deepEqual(nonResident.result, {
    accepted: 0,
    rejected: 1,
    rejections: [REJECTIONS_BY_REASON.chunkNotResident],
  });
  const telemetry = session.readTelemetry();
  assert.equal(telemetry.acceptedCommandCount, 4);
  assert.equal(telemetry.rejectedCommandCount, 3);

  const history = session.readVoxelEditHistory({
    historyId: 'history/default',
    cursorId: null,
    maxEntries: 8,
    includeRedoTail: true,
    expectedHistoryHash: null,
  });
  assert.equal(history.entries.length, 1);
  assert.equal(history.entries[0]?.commandCount, 4);
  assert.equal(history.cursor.entryCount, 1);
});
