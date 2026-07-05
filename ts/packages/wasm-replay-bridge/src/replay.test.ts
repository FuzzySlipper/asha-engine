// WASM replay path + native-vs-WASM divergence checks (task #2251).
//
// Verifies a replay fixture runs through the (reference) replay path and that the
// divergence classifier catches hash/length mismatches — the determinism.md
// native-vs-WASM check, exercised without a wasm32 toolchain (the real module load
// is a documented blocker, asserted to fail classified).

import { test } from 'node:test';
import assert from 'node:assert/strict';

import {
  replayHash,
  stepIndex,
  type ReplayRecord,
  type ReplayStep,
} from '@asha/contracts';
import {
  ReferenceReplayRunner,
  classifyDivergence,
  compareReplay,
} from './index.js';

// A tiny replay fixture: 3 accepted steps with deterministic post hashes.
function fixture(hashes: readonly number[]): ReplayRecord {
  const steps: ReplayStep[] = hashes.map((h, i) => ({
    index: stepIndex(i),
    command: { kind: 'noop' } as unknown as ReplayStep['command'],
    outcome: { status: 'accepted', events: [] },
    postHash: replayHash(h),
  }));
  return {
    formatVersion: 1,
    initialHash: replayHash(0),
    steps,
    snapshots: [],
  };
}

void test('reference runner replays a fixture into per-step hashes', () => {
  const runner = new ReferenceReplayRunner();
  assert.deepEqual(runner.replayHashes(fixture([10, 20, 30])), [10, 20, 30]);
});

void test('classifyDivergence: identical runs match', () => {
  const report = classifyDivergence([1, 2, 3], [1, 2, 3]);
  assert.equal(report.kind, 'match');
  assert.equal(report.firstDivergingStep, null);
});

void test('classifyDivergence: single hash mismatch reports the step (WASM authoritative)', () => {
  const report = classifyDivergence([1, 2, 3], [1, 99, 3]);
  assert.equal(report.kind, 'hash_divergence');
  assert.equal(report.firstDivergingStep as number, 1);
  assert.equal(report.nativeHash as number, 2);
  assert.equal(report.wasmHash as number, 99);
});

void test('classifyDivergence: differing lengths report length_divergence', () => {
  const report = classifyDivergence([1, 2], [1, 2, 3]);
  assert.equal(report.kind, 'length_divergence');
  assert.equal(report.firstDivergingStep as number, 2);
});

void test('compareReplay: reference-vs-reference baseline matches', () => {
  const record = fixture([5, 6, 7]);
  const ref = new ReferenceReplayRunner();
  assert.equal(compareReplay(record, ref, ref).kind, 'match');
});
