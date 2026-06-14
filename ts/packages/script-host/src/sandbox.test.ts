import { test } from 'node:test';
import assert from 'node:assert/strict';

import {
  entityId,
  makeEnv,
  makeWorldView,
  tagId,
  worldCommands,
  type PolicyEntityView,
  type PolicyWorldCommand,
  type WorldPolicyWithEnv,
} from '@asha/script-sdk';

import { defineWorldPolicy, isWellFormedCommand, runWorldPolicySandboxed } from './sandbox.js';

function spatial(id: number): PolicyEntityView {
  return {
    id: entityId(id),
    lifecycle: 'active',
    transform: { translation: [id, 0, 0], rotation: [0, 0, 0, 1], scale: [1, 1, 1] },
    source: { kind: 'runtime' },
    labels: [],
    spatial: true,
  };
}

const view = makeWorldView({ entities: [spatial(1), spatial(2)] });

test('a well-behaved policy runs cleanly with no violations', () => {
  const policy = defineWorldPolicy('label-all', (v) => v.entities.map((e) => worldCommands.addLabel(e.id, tagId(9))));
  const result = runWorldPolicySandboxed(policy, view, makeEnv(1, 1));
  assert.equal(result.violations.length, 0);
  assert.equal(result.commands.length, 2);
});

test('an env-seeded policy produces identical proposals across runs with the same envelope', () => {
  // The policy uses ONLY the deterministic envelope for its "random" choice.
  const policy: WorldPolicyWithEnv = (v, env) => {
    const pick = env.rng.nextInRange(0, v.entities.length);
    const chosen = v.entities[pick];
    return chosen ? [worldCommands.disable(chosen.id)] : [];
  };
  const named = defineWorldPolicy('pick-one', policy);
  const a = runWorldPolicySandboxed(named, view, makeEnv(5, 1234));
  const b = runWorldPolicySandboxed(named, view, makeEnv(5, 1234));
  assert.deepEqual(a.commands, b.commands);
  // A different seed may choose differently, but is itself reproducible.
  const c = runWorldPolicySandboxed(named, view, makeEnv(5, 1234));
  assert.deepEqual(a.commands, c.commands);
});

test('a throwing policy is classified as policyThrew, never a silent failure', () => {
  const policy = defineWorldPolicy('boom', () => {
    throw new Error('kaboom');
  });
  const result = runWorldPolicySandboxed(policy, view, makeEnv(1, 1));
  assert.deepEqual(result.commands, []);
  assert.equal(result.violations.length, 1);
  assert.equal(result.violations[0]!.code, 'policyThrew');
  assert.match(result.violations[0]!.detail, /kaboom/);
});

test('a policy touching a forbidden ambient global surfaces as a classified violation', () => {
  // Simulates a policy that reached for an absent ambient capability at runtime
  // (the lint/depgraph block such code statically; the host still fails closed).
  const policy = defineWorldPolicy('ambient', () => {
    // @ts-expect-error — intentionally touching an undefined ambient global.
    return [worldCommands.noop(String(globalThis.__no_such_capability__.read()))];
  });
  const result = runWorldPolicySandboxed(policy, view, makeEnv(1, 1));
  assert.equal(result.violations[0]?.code, 'policyThrew');
});

test('a non-array result is classified as nonArrayResult', () => {
  const policy = defineWorldPolicy('weird', () => ({ kind: 'requestDisable', entity: entityId(1) }) as unknown as readonly PolicyWorldCommand[]);
  const result = runWorldPolicySandboxed(policy, view, makeEnv(1, 1));
  assert.equal(result.commands.length, 0);
  assert.equal(result.violations[0]!.code, 'nonArrayResult');
});

test('malformed command elements are dropped and classified, well-formed ones kept', () => {
  const policy = defineWorldPolicy('mixed', () =>
    [
      worldCommands.disable(entityId(1)),
      { kind: 'notACommand' },
      { entity: entityId(2) },
      worldCommands.noop('ok'),
    ] as unknown as readonly PolicyWorldCommand[],
  );
  const result = runWorldPolicySandboxed(policy, view, makeEnv(1, 1));
  assert.equal(result.commands.length, 2); // disable + noop
  assert.equal(result.violations.filter((v) => v.code === 'malformedCommand').length, 2);
});

test('isWellFormedCommand checks shape, not semantics', () => {
  assert.equal(isWellFormedCommand(worldCommands.noop('x')), true);
  assert.equal(isWellFormedCommand({ kind: 'requestDisable', entity: entityId(1) }), true);
  assert.equal(isWellFormedCommand({ kind: 'nope' }), false);
  assert.equal(isWellFormedCommand(null), false);
  assert.equal(isWellFormedCommand('requestDisable'), false);
});
