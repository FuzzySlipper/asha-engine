import { test } from 'node:test';
import assert from 'node:assert/strict';
import { entityId, tagId } from '@asha/contracts';
import { deriveSummary, emptyWorldView, makeWorldView, worldQuery } from './world-view.js';
function spatial(id, label) {
    return {
        id: entityId(id),
        lifecycle: 'active',
        transform: { translation: [id, 0, 0], rotation: [0, 0, 0, 1], scale: [1, 1, 1] },
        source: { kind: 'runtime' },
        labels: label === undefined ? [] : [tagId(label)],
        spatial: true,
    };
}
function logical(id, lifecycle = 'active') {
    return { id: entityId(id), lifecycle, transform: null, source: { kind: 'runtime' }, labels: [], spatial: false };
}
const crate = { id: 'mesh/crate', kind: 'mesh', status: 'resolved' };
const missing = { id: 'mesh/absent', kind: 'mesh', status: 'missing' };
void test('emptyWorldView has a derived, self-consistent summary', () => {
    const view = emptyWorldView(7);
    assert.equal(view.tick, 7);
    assert.deepEqual(view.summary, { tick: 7, activeEntities: 0, spatialEntities: 0, assetCount: 0, missingAssets: 0 });
});
void test('makeWorldView derives the summary so it cannot disagree with contents', () => {
    const view = makeWorldView({ tick: 3, entities: [spatial(1, 9), logical(2), logical(3, 'disabled')], assets: [crate, missing] });
    assert.deepEqual(view.summary, { tick: 3, activeEntities: 2, spatialEntities: 1, assetCount: 2, missingAssets: 1 });
    // deriveSummary is the same computation, exposed for fixtures.
    assert.deepEqual(deriveSummary(3, view.entities, view.assets), view.summary);
});
void test('worldQuery reads entities, spatiality, labels, and asset status without mutating', () => {
    const view = makeWorldView({ entities: [spatial(1, 9), logical(2)], assets: [crate] });
    const before = JSON.stringify(view);
    assert.equal(worldQuery.entity(view, entityId(1))?.id, entityId(1));
    assert.equal(worldQuery.hasEntity(view, entityId(99)), false);
    assert.deepEqual(worldQuery.spatialEntities(view).map((e) => e.id), [entityId(1)]);
    assert.deepEqual(worldQuery.activeEntities(view).map((e) => e.id), [entityId(1), entityId(2)]);
    assert.equal(worldQuery.entityHasLabel(view, entityId(1), tagId(9)), true);
    assert.equal(worldQuery.entityHasLabel(view, entityId(2), tagId(9)), false);
    assert.equal(worldQuery.assetStatus(view, 'mesh/crate'), 'resolved');
    assert.equal(worldQuery.assetStatus(view, 'mesh/nope'), undefined);
    assert.equal(JSON.stringify(view), before); // queries never mutate the view
});
//# sourceMappingURL=world-view.test.js.map