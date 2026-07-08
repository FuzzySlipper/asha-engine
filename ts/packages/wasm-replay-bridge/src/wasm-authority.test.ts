// WASM replay authority — the real sim-replay divergence logic compiled to wasm32
// and run from Node (task #2251 followup). Runs when the module is built (via
// harness/ci/check-wasm-replay.sh), otherwise skips so offline check-all stays green.

import { test } from 'node:test';
import assert from 'node:assert/strict';

import { WasmReplayUnavailable, loadWasmReplayAuthority } from './index.js';

const WASM_AUTHORITY_SKIP_MESSAGE =
  'WASM authority unavailable: wasm-api module not built; ' +
  'this is a classified opt-in skip, not coverage. ' +
  'Run harness/ci/check-wasm-replay.sh';

// A real sim-replay artifact (same text format as harness/goldens/replays/*.replay).
const GOLDEN = [
  'replay 1',
  'init 0000000000000abc',
  'step 0',
  'cmd input entity.create 5',
  'event entity.created 5',
  'post 0000000000000011',
  '',
].join('\n');

function authorityOrSkip(t: { skip: (m: string) => void }) {
  try {
    return loadWasmReplayAuthority();
  } catch (e) {
    if (e instanceof WasmReplayUnavailable) {
      t.skip(WASM_AUTHORITY_SKIP_MESSAGE);
      return null;
    }
    throw e;
  }
}

void test('WASM authority: identical artifacts match', (t) => {
  const wasm = authorityOrSkip(t);
  if (!wasm) return;
  assert.deepEqual(wasm.classifyRecords(GOLDEN, GOLDEN), {
    class: 'match',
    matched: true,
    step: null,
  });
});

void test('WASM authority: tampered post hash is classified at the step', (t) => {
  const wasm = authorityOrSkip(t);
  if (!wasm) return;
  const tampered = GOLDEN.replace('0000000000000011', '00000000000000ff');
  const d = wasm.classifyRecords(GOLDEN, tampered);
  assert.equal(d.matched, false);
  assert.equal(d.class, 'hash-checkpoint-mismatch');
  assert.equal(d.step, 0);
});

void test('WASM authority: malformed artifact is classified', (t) => {
  const wasm = authorityOrSkip(t);
  if (!wasm) return;
  assert.equal(wasm.classifyRecords(GOLDEN, 'garbage').class, 'malformed-artifact');
});

void test('WASM authority: emits the expected class labels', (t) => {
  const wasm = authorityOrSkip(t);
  if (!wasm) return;
  assert.deepEqual(wasm.classLabels(), [
    'match',
    'command-mismatch',
    'accepted-event-mismatch',
    'rejection-mismatch',
    'hash-checkpoint-mismatch',
    'structural-mismatch',
    'malformed-artifact',
  ]);
});
