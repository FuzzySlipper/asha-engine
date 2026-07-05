import { test } from 'node:test';
import assert from 'node:assert/strict';
import { buildEntityInspector, formatEntityInspector, } from './entity-inspector.js';
const eid = (n) => n;
function record(over) {
    return {
        lifecycle: 'active',
        hasTransform: false,
        hasRender: false,
        hasCollision: false,
        staticCollider: false,
        hasBounds: false,
        containedIn: null,
        transformParent: null,
        derivedFrom: null,
        source: { kind: 'runtimeCreated', by: null },
        labels: [],
        ...over,
    };
}
// A mixed world covering every fixture vocabulary class the panel must show.
function mixedRecords() {
    return [
        record({ id: eid(1), hasTransform: true, hasRender: true, source: { kind: 'sceneBootstrap', node: 10 } }),
        record({ id: eid(2), hasTransform: true, hasCollision: true, staticCollider: true }),
        record({ id: eid(3), source: { kind: 'policyProposed', by: 5 } }),
        record({ id: eid(4), containedIn: eid(2) }),
        record({ id: eid(5), hasTransform: true, hasRender: true, transformParent: eid(1), derivedFrom: eid(4) }),
    ];
}
void test('inspector classifies every fixture vocabulary class and counts them', () => {
    const view = buildEntityInspector(mixedRecords(), null);
    assert.equal(view.classCounts.spatialRendered, 2); // entities 1 and 5
    assert.equal(view.classCounts.spatialCollider, 1); // entity 2
    assert.equal(view.classCounts.nonSpatialLogical, 2); // entities 3 and 4
    assert.equal(view.classCounts.contained, 1); // entity 4
    assert.equal(view.classCounts.attached, 1); // entity 5
});
void test('inspector shows capabilities, relations, and eligibility distinctly', () => {
    const view = buildEntityInspector(mixedRecords(), null);
    const collider = view.rows.find((r) => r.id === 2);
    assert.deepEqual(collider.capabilities, ['transform', 'collision(static)']);
    assert.equal(collider.transformEligible, false); // static collider is immovable
    assert.equal(collider.movementEligible, false);
    const attached = view.rows.find((r) => r.id === 5);
    assert.ok(attached.relations.includes('transformParent=1'));
    assert.ok(attached.relations.includes('derivedFrom=4'));
    const logical = view.rows.find((r) => r.id === 3);
    assert.deepEqual(logical.capabilities, []); // non-spatial: no fake render handle
});
void test('inspector surfaces the last command result (accept and reject)', () => {
    const accepted = { status: 'accepted', event: { kind: 'created', entity: eid(9) } };
    const a = buildEntityInspector([], accepted);
    assert.deepEqual(a.lastResult, { accepted: true, detail: 'created', entity: eid(9) });
    const rejected = {
        status: 'rejected',
        rejection: { reason: 'notTransformEligible', entity: eid(3) },
    };
    const r = buildEntityInspector([], rejected);
    assert.deepEqual(r.lastResult, { accepted: false, detail: 'notTransformEligible', entity: eid(3) });
});
void test('every row exposes a stable accessibility/automation control label', () => {
    const view = buildEntityInspector(mixedRecords(), null);
    for (const row of view.rows) {
        assert.equal(row.controlLabel, `entity-${row.id}-authoring-controls`);
    }
});
void test('formatEntityInspector is deterministic and greppable', () => {
    const view = buildEntityInspector(mixedRecords(), {
        status: 'rejected',
        rejection: { reason: 'immovable', entity: eid(2) },
    });
    const lines = formatEntityInspector(view);
    assert.ok(lines[0].includes('entity 1 active source=sceneBootstrap'));
    assert.ok(lines.some((l) => l.includes('classes=[nonSpatialLogical]')));
    assert.ok(lines[lines.length - 1].includes('lastResult rejected immovable entity=2'));
});
//# sourceMappingURL=entity-inspector.test.js.map