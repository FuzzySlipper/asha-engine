import { test } from 'node:test';
import assert from 'node:assert/strict';

import type { EntityCapabilityFlags } from '@asha/editor-tools';
import type { EntityId } from '@asha/contracts';

import { buildEntityAuthoringControls, entityAuthoringControlToCommand } from './index.js';

const eid = (n: number): EntityId => n as EntityId;

function flags(over: Partial<EntityCapabilityFlags>): EntityCapabilityFlags {
  return {
    id: eid(1),
    lifecycle: 'active',
    hasTransform: false,
    hasRender: false,
    hasCollision: false,
    staticCollider: false,
    hasBounds: false,
    containedIn: null,
    transformParent: null,
    derivedFrom: null,
    ...over,
  };
}

function control(controls: ReturnType<typeof buildEntityAuthoringControls>, id: string) {
  const c = controls.find((x) => x.id === id);
  assert.ok(c, `missing control ${id}`);
  return c!;
}

test('authoring controls gate transform/move by eligibility, with the reason in the label', () => {
  const logical = buildEntityAuthoringControls(flags({})); // non-spatial
  assert.equal(control(logical, 'entity-set-transform').disabled, true);
  assert.match(control(logical, 'entity-set-transform').label, /notTransformEligible/);
  assert.equal(control(logical, 'entity-move').disabled, true);
  assert.match(control(logical, 'entity-move').label, /notSpatial/);

  const movable = buildEntityAuthoringControls(flags({ hasTransform: true, hasCollision: true }));
  assert.equal(control(movable, 'entity-set-transform').disabled, false);
  assert.equal(control(movable, 'entity-move').disabled, false);

  const immovable = buildEntityAuthoringControls(
    flags({ hasTransform: true, hasCollision: true, staticCollider: true }),
  );
  assert.match(control(immovable, 'entity-set-transform').label, /immovable/);
});

test('destroy is disabled only for a tombstone; create is always offered', () => {
  const tomb = buildEntityAuthoringControls(flags({ lifecycle: 'tombstoned' }));
  assert.equal(control(tomb, 'entity-destroy').disabled, true);
  assert.equal(control(tomb, 'entity-create').disabled, undefined);

  const live = buildEntityAuthoringControls(flags({}));
  assert.equal(control(live, 'entity-destroy').disabled, false);
});

test('every authoring control exposes a stable id and accessible label', () => {
  const controls = buildEntityAuthoringControls(flags({ hasTransform: true }));
  for (const c of controls) {
    assert.ok(c.id.startsWith('entity-'), c.id);
    assert.ok(c.label.length > 0);
    assert.equal(c.role, 'button');
  }
});

test('control interactions map to typed proposal commands (or null when a param is missing)', () => {
  assert.deepEqual(entityAuthoringControlToCommand('entity-destroy', eid(2)), { kind: 'destroy', id: eid(2) });
  assert.equal(entityAuthoringControlToCommand('entity-set-transform', eid(2))?.kind, 'setTransform');
  assert.equal(entityAuthoringControlToCommand('entity-attach-render', eid(2))?.kind, 'attachCapability');

  // Move needs a delta; containment needs a target — null until supplied.
  assert.equal(entityAuthoringControlToCommand('entity-move', eid(2)), null);
  assert.equal(entityAuthoringControlToCommand('entity-contain', eid(2)), null);
  assert.equal(entityAuthoringControlToCommand('entity-move', eid(2), { moveDelta: [1, 0, 0] })?.kind, 'move');
  assert.equal(
    entityAuthoringControlToCommand('entity-contain', eid(2), { container: eid(3) })?.kind,
    'setContainment',
  );

  // Create uses the allocated new id.
  const create = entityAuthoringControlToCommand('entity-create', eid(0), { newEntityId: eid(9) });
  assert.equal(create?.kind, 'create');
  if (create?.kind === 'create') {
    assert.equal(create.id, eid(9));
  }
});
