// Runtime tests for the script host, run with `node --test`. They prove the
// host invokes the no-op policy and collects an empty buffer, preserves command
// order across policies, and reports a structured diagnostic when a policy
// throws — all without validating or mutating.

import { test } from 'node:test';
import assert from 'node:assert/strict';

import {
  commands,
  makeView,
  entityId,
  tagId,
  type Policy,
  type PolicyCommand,
} from '@asha/script-sdk';
import { noopPolicy } from '@asha/policy-core';

import {
  CommandBuffer,
  definePolicy,
  invokePolicy,
  invokePolicies,
} from './index.js';

test('host invokes the no-op policy and collects an empty command buffer', () => {
  const result = invokePolicy(definePolicy('noop', noopPolicy), makeView());
  assert.deepEqual(result.commands, []);
  assert.deepEqual(result.diagnostics, []);
});

test('command collection preserves order across policies', () => {
  const first: Policy = () => [commands.createEntity(entityId(1))];
  const second: Policy = () => [
    commands.addTag(entityId(1), tagId(2)),
    commands.deleteEntity(entityId(1)),
  ];

  const result = invokePolicies(
    [definePolicy('first', first), definePolicy('second', second)],
    makeView(),
  );

  assert.equal(result.diagnostics.length, 0);
  assert.deepEqual(result.commands, [
    { domain: 'entity', command: { kind: 'create', id: entityId(1) } },
    { domain: 'entity', command: { kind: 'addTag', id: entityId(1), tag: tagId(2) } },
    { domain: 'entity', command: { kind: 'delete', id: entityId(1) } },
  ]);
});

test('a throwing policy yields a structured diagnostic, not a crash', () => {
  const ok: Policy = () => [commands.createEntity(entityId(1))];
  const boom: Policy = () => {
    throw new Error('policy exploded');
  };

  const result = invokePolicies(
    [definePolicy('ok', ok), definePolicy('boom', boom)],
    makeView(),
  );

  // The healthy policy's command is still collected, in order.
  assert.deepEqual(result.commands, [
    { domain: 'entity', command: { kind: 'create', id: entityId(1) } },
  ]);
  // The throwing policy is reported with its name and message.
  assert.equal(result.diagnostics.length, 1);
  assert.deepEqual(result.diagnostics[0], {
    kind: 'policy-threw',
    policy: 'boom',
    message: 'policy exploded',
  });
});

test('collected() returns a defensive copy that cannot mutate the buffer', () => {
  const buffer = new CommandBuffer();
  buffer.push(commands.createEntity(entityId(1)));

  const snapshot = buffer.collected();
  // Mutating the returned (mutable-typed) array must not affect the buffer.
  (snapshot as PolicyCommand[]).push(commands.deleteEntity(entityId(2)));

  assert.equal(buffer.length, 1);
  assert.equal(buffer.collected().length, 1);
});
