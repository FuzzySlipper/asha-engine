import { test } from 'node:test';
import assert from 'node:assert/strict';
import { entityId, makeWorldView, tagId, worldCommands, } from '@asha/script-sdk';
import { defineWorldPolicy } from './sandbox.js';
import { runPolicyTickStage } from './tick.js';
function spatial(id) {
    return {
        id: entityId(id),
        lifecycle: 'active',
        transform: { translation: [id, 0, 0], rotation: [0, 0, 0, 1], scale: [1, 1, 1] },
        source: { kind: 'runtime' },
        labels: [],
        spatial: true,
    };
}
const labelAll = defineWorldPolicy('label-all', (v) => v.entities.filter((e) => e.spatial).map((e) => worldCommands.addLabel(e.id, tagId(9))));
const crashes = defineWorldPolicy('boom', () => {
    throw new Error('crash');
});
const noop = defineWorldPolicy('noop', () => [worldCommands.noop('marker')]);
function input(over = {}) {
    return {
        tick: over.tick ?? 1,
        seed: over.seed ?? 100,
        view: over.view ?? makeWorldView({ entities: [spatial(1), spatial(2)] }),
        policies: over.policies ?? [labelAll, noop],
    };
}
void test('the tick stage collects proposals from all policies in order with metadata', () => {
    const result = runPolicyTickStage(input());
    assert.equal(result.tick, 1);
    // label-all proposes 2, noop proposes 1.
    assert.equal(result.proposed.length, 3);
    assert.equal(result.violations.length, 0);
    assert.deepEqual(result.executions.map((e) => [e.policy, e.proposedCount]), [
        ['label-all', 2],
        ['noop', 1],
    ]);
    // Each policy got its own deterministic seed (seed + index).
    assert.deepEqual(result.executions.map((e) => e.seed), [100, 101]);
});
void test('a crashing policy is isolated: others still run and the crash is classified', () => {
    const result = runPolicyTickStage(input({ policies: [crashes, labelAll] }));
    // label-all still produced its proposals despite the earlier crash.
    assert.equal(result.proposed.length, 2);
    assert.equal(result.violations.length, 1);
    assert.equal(result.violations[0].code, 'policyThrew');
    assert.equal(result.violations[0].policy, 'boom');
});
void test('the stage reproduces identical proposals across runs with the same input', () => {
    const a = runPolicyTickStage(input());
    const b = runPolicyTickStage(input());
    assert.deepEqual(a.proposed, b.proposed);
    assert.deepEqual(a.executions, b.executions);
});
void test('the stage proposes only — it never mutates the projected view', () => {
    const view = makeWorldView({ entities: [spatial(1)] });
    const before = JSON.stringify(view);
    runPolicyTickStage(input({ view }));
    assert.equal(JSON.stringify(view), before);
});
//# sourceMappingURL=tick.test.js.map