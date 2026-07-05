import { test } from 'node:test';
import assert from 'node:assert/strict';
import { classifyEntity, movementEligibility, proposeAttachCapability, proposeCreateEntity, proposeMove, proposeSetContainment, proposeSetEntityTransform, summarizeAuthoringOutcome, transformEligibility, } from './entity-authoring.js';
const eid = (n) => n;
const tid = (n) => n;
function flags(over) {
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
void test('proposal builders produce typed, proposal-only commands', () => {
    const create = proposeCreateEntity(eid(1), { kind: 'runtimeCreated', by: null }, [tid(3)]);
    assert.equal(create.kind, 'create');
    assert.deepEqual(create.labels, [tid(3)]);
    const attach = proposeAttachCapability(eid(1), { kind: 'render', visible: true });
    assert.equal(attach.kind, 'attachCapability');
    const move = proposeMove(eid(1), [1, 0, 0]);
    assert.equal(move.kind, 'move');
    assert.deepEqual(move.delta, [1, 0, 0]);
    const contain = proposeSetContainment(eid(2), eid(1));
    assert.equal(contain.kind, 'setContainment');
    // A runtime-created source carries an optional `by` process id.
    const byProc = proposeCreateEntity(eid(5), { kind: 'runtimeCreated', by: 9 });
    assert.equal(byProc.kind, 'create');
});
void test('summarizeAuthoringOutcome reflects authority accept/reject, never decides', () => {
    const accepted = { status: 'accepted', event: { kind: 'created', entity: eid(1) } };
    assert.deepEqual(summarizeAuthoringOutcome(accepted), { accepted: true, detail: 'created', entity: eid(1) });
    const rejected = {
        status: 'rejected',
        rejection: { reason: 'notTransformEligible', entity: eid(1) },
    };
    assert.deepEqual(summarizeAuthoringOutcome(rejected), {
        accepted: false,
        detail: 'notTransformEligible',
        entity: eid(1),
    });
});
void test('classifyEntity buckets each fixture vocabulary class', () => {
    assert.deepEqual(classifyEntity(flags({ hasTransform: true, hasRender: true })), ['spatialRendered']);
    assert.deepEqual(classifyEntity(flags({ hasTransform: true, hasCollision: true })), ['spatialCollider']);
    assert.deepEqual(classifyEntity(flags({})), ['nonSpatialLogical']);
    assert.deepEqual(classifyEntity(flags({ containedIn: eid(2) })), ['nonSpatialLogical', 'contained']);
    assert.deepEqual(classifyEntity(flags({ hasTransform: true, hasRender: true, transformParent: eid(1) })), ['spatialRendered', 'attached']);
    assert.deepEqual(classifyEntity(flags({ lifecycle: 'tombstoned', hasTransform: true })), ['tombstoned']);
});
void test('eligibility mirrors capability discipline with a classified reason', () => {
    assert.deepEqual(transformEligibility(flags({})), { eligible: false, reason: 'notTransformEligible' });
    assert.deepEqual(transformEligibility(flags({ hasTransform: true })), { eligible: true, reason: null });
    assert.deepEqual(transformEligibility(flags({ hasTransform: true, hasCollision: true, staticCollider: true })), { eligible: false, reason: 'immovable' });
    assert.deepEqual(movementEligibility(flags({})), { eligible: false, reason: 'notSpatial' });
    assert.deepEqual(movementEligibility(flags({ hasTransform: true })), { eligible: false, reason: 'noCollider' });
    assert.deepEqual(movementEligibility(flags({ hasTransform: true, hasCollision: true })), { eligible: true, reason: null });
});
void test('setTransform proposal carries the transform verbatim (no local mutation)', () => {
    const t = { translation: [3, 0, 0], rotation: [0, 0, 0, 1], scale: [1, 1, 1] };
    const proposal = proposeSetEntityTransform(eid(1), t);
    assert.equal(proposal.kind, 'setTransform');
    if (proposal.kind === 'setTransform') {
        assert.deepEqual(proposal.transform.translation, [3, 0, 0]);
    }
});
//# sourceMappingURL=entity-authoring.test.js.map