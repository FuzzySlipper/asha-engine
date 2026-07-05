// Runtime tests for @asha/script-sdk, run with Node's built-in test runner
// (`node --test`). They prove the Phase 3 SDK contract: a policy accepts a
// read-only view and returns an array of proposed commands, the command/query
// helpers are well-formed, and the view is read-only at the type level.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { commands, query, makeView, emptyView, runPolicy, entityId, tagId, processId, modeId, } from './index.js';
void test('a policy accepts a read-only view and returns proposed commands', () => {
    // A policy that tags every untagged entity with a fixed tag.
    const tagAll = (view) => view.entities
        .filter((e) => !e.tags.includes(tagId(1)))
        .map((e) => commands.addTag(e.id, tagId(1)));
    const view = makeView({
        entities: [
            { id: entityId(1), tags: [] },
            { id: entityId(2), tags: [tagId(1)] },
        ],
        tags: [tagId(1)],
    });
    const out = runPolicy(tagAll, view);
    assert.equal(out.length, 1);
    assert.deepEqual(out[0], {
        domain: 'entity',
        command: { kind: 'addTag', id: entityId(1), tag: tagId(1) },
    });
});
void test('command builders produce well-formed generated Command values', () => {
    assert.deepEqual(commands.createEntity(entityId(7)), {
        domain: 'entity',
        command: { kind: 'create', id: entityId(7) },
    });
    assert.deepEqual(commands.setProcessMode(processId(3), modeId(9)), {
        domain: 'process',
        command: { kind: 'setMode', id: processId(3), mode: modeId(9) },
    });
});
void test('query helpers read the view without mutating it', () => {
    const view = makeView({
        entities: [{ id: entityId(1), tags: [tagId(5)] }],
        processes: [{ id: processId(2), mode: modeId(8) }],
        tags: [tagId(5)],
    });
    assert.equal(query.hasEntity(view, entityId(1)), true);
    assert.equal(query.hasEntity(view, entityId(99)), false);
    assert.equal(query.entityHasTag(view, entityId(1), tagId(5)), true);
    assert.equal(query.entityHasTag(view, entityId(1), tagId(6)), false);
    assert.equal(query.processMode(view, processId(2)), modeId(8));
    assert.equal(query.processMode(view, processId(404)), undefined);
});
void test('emptyView is the empty world', () => {
    const v = emptyView();
    assert.equal(v.entities.length, 0);
    assert.equal(v.tags.length, 0);
});
void test('a no-op policy proposes nothing', () => {
    const noop = () => [];
    assert.deepEqual(runPolicy(noop, emptyView()), []);
});
void test('view data is read-only at the type level', () => {
    // Type-only proof: this closure is never executed. Each line is rejected by
    // `tsc`, which is asserted by the `@ts-expect-error` directives. (At runtime
    // `readonly` is structural and would not throw, so we never run it.)
    const _readonlyProof = (v) => {
        // @ts-expect-error - entities is readonly
        v.entities = [];
        // @ts-expect-error - view fields are readonly
        v.tags = [];
    };
    void _readonlyProof;
    assert.ok(true);
});
//# sourceMappingURL=sdk.test.js.map