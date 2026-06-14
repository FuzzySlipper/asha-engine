// Runtime isolation negative tests (#2427): prove that view mutation and ambient
// capability escapes are blocked or classified at runtime, not just by lint.

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
} from '@asha/script-sdk';

import { defineWorldPolicy, runWorldPolicySandboxed } from './sandbox.js';
import { runPolicyTickStage } from './tick.js';
import { deepFreeze, PolicyCapabilityError, runQuarantined } from './isolation.js';

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

/** A loosely-typed global accessor so a test can mimic a hostile policy without
 *  using lint-restricted bare identifiers. */
const G = globalThis as unknown as Record<string, unknown>;

// ── View immutability ─────────────────────────────────────────────────────────

test('a policy cannot mutate a top-level view field (frozen → classified throw)', () => {
  const view = makeWorldView({ entities: [spatial(1)] });
  const policy = defineWorldPolicy('mutate-tick', (v) => {
    (v as unknown as { tick: number }).tick = 999;
    return [];
  });
  const result = runWorldPolicySandboxed(policy, view, makeEnv(1, 1));
  assert.equal(result.violations[0]?.code, 'policyThrew');
  assert.equal(view.tick, 0, 'view is unchanged');
});

test('a policy cannot mutate a nested view value (deep freeze)', () => {
  const view = makeWorldView({ entities: [spatial(1)] });
  const policy = defineWorldPolicy('mutate-nested', (v) => {
    (v.entities[0]!.labels as unknown as TagPush).push(tagId(7));
    return [];
  });
  type TagPush = { push: (t: unknown) => number };
  const result = runWorldPolicySandboxed(policy, view, makeEnv(1, 1));
  assert.equal(result.violations[0]?.code, 'policyThrew');
  assert.equal(view.entities[0]!.labels.length, 0, 'nested array is unchanged');
});

test('one policy cannot contaminate the view observed by a later policy', () => {
  const view = makeWorldView({ entities: [spatial(1)] });
  const mutator = defineWorldPolicy('mutator', (v) => {
    // Hostile attempt to plant a label the next policy would observe.
    (v.entities[0]!.labels as unknown as { push: (t: unknown) => number }).push(tagId(42));
    return [];
  });
  const reader = defineWorldPolicy('reader', (v) =>
    v.entities[0]!.labels.length > 0 ? [worldCommands.noop('contaminated')] : [],
  );
  const result = runPolicyTickStage({ tick: 1, seed: 1, view, policies: [mutator, reader] });

  assert.ok(
    result.violations.some((vio) => vio.policy === 'mutator' && vio.code === 'policyThrew'),
    'the mutating policy is classified',
  );
  assert.equal(result.proposed.length, 0, 'the reader saw a pristine view');
});

// ── Ambient capability quarantine ─────────────────────────────────────────────

const view = makeWorldView({ entities: [spatial(1)] });

test('process access is quarantined and classified', () => {
  const policy = defineWorldPolicy('process-probe', () => {
    const proc = G['process'] as { platform: string };
    return [worldCommands.noop(String(proc.platform))];
  });
  const result = runWorldPolicySandboxed(policy, view, makeEnv(1, 1));
  assert.equal(result.violations[0]?.code, 'policyThrew');
  assert.match(result.violations[0]!.detail, /process/);
});

test("Function('return process') escape is quarantined", () => {
  const policy = defineWorldPolicy('fn-escape', () => {
    const getProc = Function('return process') as () => { pid: number };
    return [worldCommands.noop(String(getProc().pid))];
  });
  const result = runWorldPolicySandboxed(policy, view, makeEnv(1, 1));
  assert.equal(result.violations[0]?.code, 'policyThrew');
});

test('timers are quarantined and classified', () => {
  const policy = defineWorldPolicy('timer', () => {
    (G['setTimeout'] as (cb: () => void, ms: number) => unknown)(() => undefined, 0);
    return [];
  });
  const result = runWorldPolicySandboxed(policy, view, makeEnv(1, 1));
  assert.equal(result.violations[0]?.code, 'policyThrew');
});

test('Math.random is quarantined (determinism)', () => {
  const policy = defineWorldPolicy('rand', () => {
    const m = Math as unknown as Record<string, () => number>;
    return [worldCommands.noop(String(m['random']!()))];
  });
  const result = runWorldPolicySandboxed(policy, view, makeEnv(1, 1));
  assert.equal(result.violations[0]?.code, 'policyThrew');
});

// ── Restoration ───────────────────────────────────────────────────────────────

test('quarantined globals are restored after the call, even on throw', () => {
  const before = G['process'];
  const ok = runWorldPolicySandboxed(
    defineWorldPolicy('noop', () => [] as readonly PolicyWorldCommand[]),
    view,
    makeEnv(1, 1),
  );
  assert.equal(ok.violations.length, 0);
  assert.equal(G['process'], before, 'process is restored');
  assert.equal(typeof G['setTimeout'], 'function', 'setTimeout is restored');
  // A throwing policy must still restore globals.
  runWorldPolicySandboxed(
    defineWorldPolicy('boom', () => {
      throw new Error('x');
    }),
    view,
    makeEnv(1, 1),
  );
  assert.equal(G['process'], before, 'process restored after a throwing policy');
});

// ── Unit-level checks ─────────────────────────────────────────────────────────

test('runQuarantined surfaces PolicyCapabilityError for a quarantined global', () => {
  assert.throws(
    () =>
      runQuarantined(() => {
        const proc = G['process'] as { pid: number };
        return proc.pid;
      }),
    (e: unknown) => e instanceof PolicyCapabilityError && e.capability === 'process',
  );
  // Restored outside the quarantine.
  assert.notEqual(G['process'], undefined);
});

test('deepFreeze freezes nested structures and is idempotent', () => {
  const obj = { a: { b: [1, 2] } };
  const frozen = deepFreeze(obj);
  assert.ok(Object.isFrozen(frozen));
  assert.ok(Object.isFrozen(frozen.a));
  assert.ok(Object.isFrozen(frozen.a.b));
  assert.equal(deepFreeze(obj), obj, 'returns the same reference');
});
