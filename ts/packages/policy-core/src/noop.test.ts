// Runtime tests for the no-op policy, run with `node --test`.

import { test } from 'node:test';
import assert from 'node:assert/strict';

import { makeView, runPolicy, entityId, tagId } from '@asha/script-sdk';
import { noopPolicy } from './index.js';

void test('no-op policy receives a read-only view and returns an empty command list', () => {
  const view = makeView({
    entities: [{ id: entityId(1), tags: [tagId(1)] }],
    tags: [tagId(1)],
  });

  const out = runPolicy(noopPolicy, view);
  assert.deepEqual(out, []);
});

void test('no-op policy does not mutate the view it is given', () => {
  const view = makeView({
    entities: [{ id: entityId(1), tags: [tagId(2)] }],
    tags: [tagId(2)],
  });
  const snapshot = JSON.stringify(view);

  runPolicy(noopPolicy, view);

  assert.equal(JSON.stringify(view), snapshot, 'view must be unchanged');
});

void test('no-op policy is deterministic across the same and different views', () => {
  assert.deepEqual(runPolicy(noopPolicy, makeView()), []);
  assert.deepEqual(
    runPolicy(noopPolicy, makeView({ entities: [{ id: entityId(9), tags: [] }] })),
    [],
  );
});
