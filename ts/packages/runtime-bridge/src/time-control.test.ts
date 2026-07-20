import assert from 'node:assert/strict';
import test from 'node:test';
import type { ResolvedInputAction } from '@asha/contracts';
import { createMockRuntimeBridge } from './mock.js';
import { createMockRuntimeSession } from './mock-session.js';
import {
  ResolvedTimeControlConsumer,
  TIME_CONTROL_INPUT_ACTIONS,
} from './resolved-time-control.js';

function sessionInput() {
  return {
    sessionId: 'runtime-session.time-control',
    seed: 17,
    project: { gameId: 'time-control', workspaceId: 'workspace.local' },
  };
}

function pressed(actionId: string): ResolvedInputAction {
  return {
    sequence: 1,
    actionId,
    contextId: 'gameplay',
    bindingId: `binding.${actionId}`,
    phase: 'pressed',
    value: { kind: 'button', pressed: true },
  };
}

void test('headless RuntimeSession pause and exact stepping use validated authority commands', () => {
  const session = createMockRuntimeSession();
  session.initialize(sessionInput());

  assert.equal(session.applyTimeControlCommand({ operation: 'pause' }).accepted, true);
  assert.equal(session.tick({ tick: 9 }).tick, 0);
  const stepped = session.applyTimeControlCommand({ operation: 'stepTicks', ticks: 3 });
  assert.equal(stepped.exactTicksAdvanced, 3);
  assert.equal(stepped.after.mode, 'paused');
  assert.equal(session.tick({ tick: 10 }).tick, 3);
  assert.equal(session.readTimeControlState().authorityTick, 3);

  const rejected = session.applyTimeControlCommand({ operation: 'stepTicks', ticks: 0 });
  assert.equal(rejected.accepted, false);
  assert.equal(rejected.rejection, 'invalidStepCount');
  assert.deepEqual(rejected.before, rejected.after);
});

void test('resolved input actions route through the same RuntimeSession command surface', () => {
  const bridge = createMockRuntimeBridge();
  const session = createMockRuntimeSession({ bridge });
  session.initialize(sessionInput());
  const consumer = new ResolvedTimeControlConsumer(session);

  assert.equal(consumer.consume(pressed(TIME_CONTROL_INPUT_ACTIONS.pause))?.after.mode, 'paused');
  assert.equal(consumer.consume(pressed(TIME_CONTROL_INPUT_ACTIONS.stepOne))?.exactTicksAdvanced, 1);
  assert.equal(consumer.consume(pressed(TIME_CONTROL_INPUT_ACTIONS.resume))?.after.mode, 'running');
  assert.equal(consumer.consume(pressed('gameplay.primary_fire')), null);
});
