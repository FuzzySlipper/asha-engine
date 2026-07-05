import { test } from 'node:test';
import assert from 'node:assert/strict';

import {
  entityId,
  makeWorldView,
  tagId,
  worldCommands,
  type PolicyEntityView,
  type PolicyWorldView,
} from '@asha/script-sdk';

import { labelSpatialEntities, labelSpatialPolicy } from './index.js';

function spatial(id: number, labels: number[] = []): PolicyEntityView {
  return {
    id: entityId(id),
    lifecycle: 'active',
    transform: { translation: [id, 0, 0], rotation: [0, 0, 0, 1], scale: [1, 1, 1] },
    source: { kind: 'runtime' },
    labels: labels.map(tagId),
    spatial: true,
  };
}

function logical(id: number): PolicyEntityView {
  return { id: entityId(id), lifecycle: 'active', transform: null, source: { kind: 'runtime' }, labels: [], spatial: false };
}

function disabledSpatial(id: number): PolicyEntityView {
  return { ...spatial(id), lifecycle: 'disabled' };
}

void test('labelSpatialPolicy proposes a label only for active, spatial, unlabelled entities', () => {
  const view = makeWorldView({ entities: [spatial(1), logical(2), disabledSpatial(3), spatial(4, [9])] });
  const proposed = labelSpatialEntities(view);
  // Entity 1 needs it; 2 is non-spatial; 3 is disabled; 4 already has it.
  assert.deepEqual(proposed, [worldCommands.addLabel(entityId(1), tagId(9))]);
});

void test('labelSpatialPolicy proposes only — it never mutates the view', () => {
  const view = makeWorldView({ entities: [spatial(1), spatial(2)] });
  const before = JSON.stringify(view);
  const proposed = labelSpatialEntities(view);
  assert.equal(proposed.length, 2);
  assert.equal(JSON.stringify(view), before); // proposing does not touch the view
});

void test('labelSpatialPolicy is idempotent once every spatial entity is labelled', () => {
  const labelled: PolicyWorldView = makeWorldView({ entities: [spatial(1, [9]), spatial(2, [9])] });
  assert.deepEqual(labelSpatialEntities(labelled), []);
});

void test('a differently-configured policy honors its own label deterministically', () => {
  const policy = labelSpatialPolicy({ label: tagId(3) });
  const view = makeWorldView({ entities: [spatial(1)] });
  assert.deepEqual(policy(view), [worldCommands.addLabel(entityId(1), tagId(3))]);
  assert.deepEqual(policy(view), policy(view)); // deterministic
});
