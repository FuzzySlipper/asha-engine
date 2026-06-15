import { test } from 'node:test';
import assert from 'node:assert/strict';

import type { EntityAuthoringCommand, EntityAuthoringOutcome, EntityId } from '@asha/contracts';

import { EntityAuthoringController, type EntityAuthoringSink } from './index.js';

const eid = (n: number): EntityId => n as EntityId;

// A sink stand-in for the authority transport: it records what it was asked to
// validate and returns a scripted outcome. (The real sink routes to Rust
// `svc-entity-authoring`; this proves the controller's forward/record contract.)
function scriptedSink(outcome: EntityAuthoringOutcome): { sink: EntityAuthoringSink; seen: EntityAuthoringCommand[] } {
  const seen: EntityAuthoringCommand[] = [];
  const sink: EntityAuthoringSink = (command) => {
    seen.push(command);
    return outcome;
  };
  return { sink, seen };
}

test('controller forwards the proposal to the sink and records the accepted outcome', () => {
  const accepted: EntityAuthoringOutcome = { status: 'accepted', event: { kind: 'created', entity: eid(1) } };
  const { sink, seen } = scriptedSink(accepted);
  const controller = new EntityAuthoringController(sink);

  assert.equal(controller.lastOutcome(), null);
  const command: EntityAuthoringCommand = {
    kind: 'create',
    id: eid(1),
    source: { kind: 'runtimeCreated', by: null },
    labels: [],
  };
  const result = controller.submit(command);

  assert.deepEqual(result, accepted);
  assert.deepEqual(controller.lastOutcome(), accepted);
  assert.deepEqual(seen, [command]); // forwarded verbatim, exactly once
});

test('a rejected outcome is recorded for display; the controller mutates nothing locally', () => {
  const rejected: EntityAuthoringOutcome = {
    status: 'rejected',
    rejection: { reason: 'notTransformEligible', entity: eid(3) },
  };
  const { sink } = scriptedSink(rejected);
  const controller = new EntityAuthoringController(sink);

  const result = controller.submit({ kind: 'setTransform', id: eid(3), transform: { translation: [1, 0, 0], rotation: [0, 0, 0, 1], scale: [1, 1, 1] } });
  assert.equal(result.status, 'rejected');
  assert.deepEqual(controller.lastOutcome(), rejected);
});

test('the sink is the only mutation route — building a proposal does not submit it', () => {
  let calls = 0;
  const sink: EntityAuthoringSink = () => {
    calls += 1;
    return { status: 'accepted', event: { kind: 'destroyed', entity: eid(1) } };
  };
  const controller = new EntityAuthoringController(sink);
  // No submit yet → sink untouched.
  assert.equal(calls, 0);
  controller.submit({ kind: 'destroy', id: eid(1) });
  assert.equal(calls, 1);
});
