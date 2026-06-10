// Runtime tests for the threshold policy fixture, run with `node --test`.
//
// Inputs and expected outputs are loaded from the named, inspectable golden
// fixtures under harness/fixtures/policy-{inputs,outputs}/ so the documented
// fixture and the asserted behavior cannot drift apart.

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import {
  makeView,
  entityId,
  signalId,
  tagId,
  type PolicyView,
  type PolicyCommand,
} from '@asha/script-sdk';
import { definePolicy, invokePolicy } from '@asha/script-host';

import { tagCountThreshold, thresholdPolicy } from './index.js';

// Compiled test lives at packages/policy-examples/dist/; the repo root is four
// directories up.
const fixturesRoot = resolve(import.meta.dirname, '../../../../harness/fixtures');

function loadInput(name: string): PolicyView {
  const path = resolve(fixturesRoot, 'policy-inputs', `${name}.json`);
  return JSON.parse(readFileSync(path, 'utf8')) as PolicyView;
}

function loadOutput(name: string): readonly PolicyCommand[] {
  const path = resolve(fixturesRoot, 'policy-outputs', `${name}.json`);
  return JSON.parse(readFileSync(path, 'utf8')) as PolicyCommand[];
}

test('threshold not met returns no commands', () => {
  const view = loadInput('threshold-below');
  const out = tagCountThreshold(view);
  assert.deepEqual(out, []);
  assert.deepEqual(out, loadOutput('threshold-below'));
});

test('threshold met returns a generated PolicyCommand union value', () => {
  const view = loadInput('threshold-met');
  const out = tagCountThreshold(view);

  assert.equal(out.length, 1);
  assert.deepEqual(out[0], {
    domain: 'signal',
    command: { kind: 'define', id: signalId(1) },
  });
  assert.deepEqual(out, loadOutput('threshold-met'));
});

test('policy is idempotent once the signal is defined', () => {
  const view = makeView({
    entities: [
      { id: entityId(1), tags: [tagId(1)] },
      { id: entityId(2), tags: [tagId(1)] },
      { id: entityId(3), tags: [tagId(1)] },
    ],
    signals: [signalId(1)],
    tags: [tagId(1)],
  });
  assert.deepEqual(tagCountThreshold(view), []);
});

test('a different threshold config is honored deterministically', () => {
  const strict = thresholdPolicy({
    watchTag: tagId(1),
    threshold: 2,
    raiseSignal: signalId(7),
  });
  const view = makeView({
    entities: [
      { id: entityId(1), tags: [tagId(1)] },
      { id: entityId(2), tags: [tagId(1)] },
    ],
    tags: [tagId(1)],
  });
  assert.deepEqual(strict(view), [
    { domain: 'signal', command: { kind: 'define', id: signalId(7) } },
  ]);
});

test('script host invokes the threshold policy and collects the buffer in order', () => {
  const view = loadInput('threshold-met');
  const result = invokePolicy(definePolicy('threshold', tagCountThreshold), view);

  assert.deepEqual(result.diagnostics, []);
  assert.deepEqual(result.commands, loadOutput('threshold-met'));
});
