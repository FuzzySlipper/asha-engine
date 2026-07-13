import assert from 'node:assert/strict';
import { test } from 'node:test';

import { RuntimeSessionProgress } from './runtime-session-rust-progress.js';

void test('session and sparse projection ticks remain explicit across movement and fire', () => {
  const progress = new RuntimeSessionProgress();
  progress.initialize();

  progress.recordSimulationTick(5);
  progress.recordProjectionTick(2);
  assert.deepEqual(progress.snapshot(), {
    sequenceId: 1,
    sessionTick: 5,
    latestProjectionTick: 2,
    acceptedCommandCount: 0,
    rejectedCommandCount: 0,
    restartCount: 0,
  });

  progress.recordProjectedAuthorityTick(7);
  assert.equal(progress.snapshot().sessionTick, 7);
  assert.equal(progress.snapshot().latestProjectionTick, 7);
  assert.equal(progress.nextSimulationTick(), 8);
});

void test('command counters and restart reset the epoch without cursor drift', () => {
  const progress = new RuntimeSessionProgress();
  progress.recordCommandBatch(3, 1);
  progress.recordSimulationTick(9);
  progress.recordProjectionTick(6);
  progress.restart();

  assert.deepEqual(progress.snapshot(), {
    sequenceId: 3,
    sessionTick: 0,
    latestProjectionTick: 0,
    acceptedCommandCount: 0,
    rejectedCommandCount: 0,
    restartCount: 1,
  });
});
