import { test } from 'node:test';
import assert from 'node:assert/strict';
import { sceneId, sceneNodeId, } from '@asha/contracts';
import { buildSceneObjectSnapshot, proposeRenameSceneObject, proposeReparentSceneObject, sceneObjectIdForNode, } from './scene-object-hierarchy.js';
const IDENTITY = {
    translation: [0, 0, 0],
    rotation: [0, 0, 0, 1],
    scale: [1, 1, 1],
};
function group(id, label, parent = null, childOrder = 0) {
    return {
        id: sceneNodeId(id),
        parent: parent === null ? null : sceneNodeId(parent),
        childOrder,
        label,
        tags: [],
        transform: IDENTITY,
        kind: { kind: 'emptyGroup' },
    };
}
function doc(nodes) {
    return {
        schemaVersion: 1,
        id: sceneId(1001),
        metadata: { name: 'hierarchy-test', authoringFormatVersion: 1 },
        dependencies: [],
        nodes,
    };
}
void test('scene object snapshot projects flat scene document parent links and renderable provenance', () => {
    const snapshot = buildSceneObjectSnapshot({
        document: doc([{ ...group(1, 'Root'), tags: ['studio-root'] }, group(2, 'Child', 1)]),
        renderableLinks: [{ sceneNodeId: sceneNodeId(2), renderableId: 'renderable-child' }],
    });
    assert.equal(snapshot.snapshotVersion, 'scene-object-snapshot.v0');
    assert.equal(snapshot.objects.length, 2);
    assert.equal(snapshot.objects[0]?.objectId, 'scene-node:1');
    assert.equal(snapshot.objects[1]?.parentObjectId, 'scene-node:1');
    assert.equal(snapshot.objects[1]?.provenance.renderableId, 'renderable-child');
    assert.ok(snapshot.sceneHash.startsWith('scene-object-'));
    assert.deepEqual(snapshot.diagnostics, []);
    assert.ok(snapshot.nonClaims.includes('not_authority_validation'));
    assert.equal(snapshot.objects[0]?.editability.transform, false);
    assert.equal(snapshot.objects[1]?.editability.transform, true);
});
void test('scene object rename proposal targets scene metadata without mutating authority', () => {
    const snapshot = buildSceneObjectSnapshot({ document: doc([group(1, 'Root')]) });
    const result = proposeRenameSceneObject({
        snapshot,
        objectId: sceneObjectIdForNode(sceneNodeId(1)),
        displayName: 'Renamed Root',
    });
    assert.equal(result.ok, true);
    if (result.ok) {
        assert.equal(result.proposal.op, 'setMetadata');
        assert.equal(result.proposal.op === 'setMetadata' && result.proposal.label, 'Renamed Root');
    }
});
void test('scene object reparent proposal rejects missing parents and cycles', () => {
    const snapshot = buildSceneObjectSnapshot({
        document: doc([group(1, 'Root'), group(2, 'Child', 1), group(3, 'Grandchild', 2)]),
    });
    const valid = proposeReparentSceneObject({
        snapshot,
        objectId: 'scene-node:3',
        parentObjectId: 'scene-node:1',
        childOrder: 2,
    });
    assert.equal(valid.ok, true);
    if (valid.ok) {
        assert.deepEqual(valid.proposal, {
            op: 'reparent',
            node: sceneNodeId(3),
            newParent: sceneNodeId(1),
            childOrder: 2,
        });
    }
    assert.equal(proposeReparentSceneObject({
        snapshot,
        objectId: 'scene-node:1',
        parentObjectId: 'scene-node:3',
    }).ok, false);
    assert.equal(proposeReparentSceneObject({
        snapshot,
        objectId: 'scene-node:2',
        parentObjectId: 'scene-node:99',
    }).ok, false);
});
//# sourceMappingURL=scene-object-hierarchy.test.js.map