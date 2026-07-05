import { test } from 'node:test';
import assert from 'node:assert/strict';
import { sceneId, sceneNodeId, } from '@asha/contracts';
import { applyProposalToDraft, proposeAddGroup, proposeAddSprite, proposeAddStaticMesh, proposeReparent, proposeSetMetadata, proposeSetTransform, summarizeValidation, } from './scene-authoring.js';
const IDENTITY = { translation: [0, 0, 0], rotation: [0, 0, 0, 1], scale: [1, 1, 1] };
function ref(id) {
    return { id, version: { req: 'any' }, hash: null };
}
function node(id, kind, parent = null, childOrder = 0) {
    return {
        id: sceneNodeId(id),
        parent: parent === null ? null : sceneNodeId(parent),
        childOrder,
        label: null,
        tags: [],
        transform: IDENTITY,
        kind,
    };
}
// A small valid base scene: root group (1) with a static-mesh child (2).
function baseScene() {
    return {
        schemaVersion: 1,
        id: sceneId(1001),
        metadata: { name: 'base', authoringFormatVersion: 1 },
        dependencies: [],
        nodes: [node(1, { kind: 'emptyGroup' }), node(2, { kind: 'staticMesh', asset: ref('mesh/wall') }, 1, 0)],
    };
}
// ── Proposal builders are pure DTO constructors ───────────────────────────────────
void test('proposeAddStaticMesh / Sprite / Group build typed addNode proposals', () => {
    const mesh = proposeAddStaticMesh(sceneNodeId(10), ref('mesh/crate'), { parent: sceneNodeId(1), label: 'crate' });
    assert.equal(mesh.op, 'addNode');
    if (mesh.op === 'addNode') {
        assert.deepEqual(mesh.node.kind, { kind: 'staticMesh', asset: ref('mesh/crate') });
        assert.equal(mesh.node.parent, 1);
        assert.equal(mesh.node.label, 'crate');
        assert.deepEqual(mesh.node.transform, IDENTITY);
    }
    const sprite = proposeAddSprite(sceneNodeId(11), ref('sprite/icon'));
    assert.equal(sprite.op === 'addNode' && sprite.node.kind.kind, 'sprite');
    const group = proposeAddGroup(sceneNodeId(12), { parent: sceneNodeId(1), childOrder: 3 });
    assert.equal(group.op === 'addNode' && group.node.kind.kind, 'emptyGroup');
    assert.equal(group.op === 'addNode' && group.node.childOrder, 3);
});
void test('reparent / setTransform / setMetadata builders are typed proposals', () => {
    assert.deepEqual(proposeReparent(sceneNodeId(2), sceneNodeId(1), 1), {
        op: 'reparent',
        node: sceneNodeId(2),
        newParent: sceneNodeId(1),
        childOrder: 1,
    });
    assert.deepEqual(proposeReparent(sceneNodeId(2), null), {
        op: 'reparent',
        node: sceneNodeId(2),
        newParent: null,
        childOrder: 0,
    });
    const t = proposeSetTransform(sceneNodeId(2), { translation: [1, 2, 3], rotation: [0, 0, 0, 1], scale: [1, 1, 1] });
    assert.equal(t.op === 'setTransform' && t.transform.translation[0], 1);
    assert.deepEqual(proposeSetMetadata(sceneNodeId(2), 'label', ['a']), {
        op: 'setMetadata',
        node: sceneNodeId(2),
        label: 'label',
        tags: ['a'],
    });
});
// ── Draft application is pure and never mutates authority ──────────────────────────
void test('applyProposalToDraft does not mutate the input document', () => {
    const doc = baseScene();
    const before = JSON.stringify(doc);
    const draft = applyProposalToDraft(doc, proposeAddGroup(sceneNodeId(3), { parent: sceneNodeId(1) }));
    assert.equal(JSON.stringify(doc), before); // input untouched
    assert.equal(draft.nodes.length, 3);
    assert.notEqual(draft, doc);
});
void test('applyProposalToDraft applies add / reparent / setTransform / setMetadata', () => {
    let draft = applyProposalToDraft(baseScene(), proposeAddGroup(sceneNodeId(3), { parent: sceneNodeId(1), childOrder: 1 }));
    draft = applyProposalToDraft(draft, proposeReparent(sceneNodeId(2), sceneNodeId(3), 0));
    draft = applyProposalToDraft(draft, proposeSetTransform(sceneNodeId(2), { translation: [5, 0, 0], rotation: [0, 0, 0, 1], scale: [1, 1, 1] }));
    draft = applyProposalToDraft(draft, proposeSetMetadata(sceneNodeId(2), 'moved', ['x']));
    const n2 = draft.nodes.find((n) => n.id === 2);
    assert.equal(n2.parent, 3);
    assert.deepEqual(n2.transform.translation, [5, 0, 0]);
    assert.equal(n2.label, 'moved');
    assert.deepEqual(n2.tags, ['x']);
});
void test('applyProposalToDraft leaves an unknown-target proposal a no-op (authority rejects)', () => {
    const draft = applyProposalToDraft(baseScene(), proposeReparent(sceneNodeId(404), sceneNodeId(1)));
    // The draft never invents node 404.
    assert.equal(draft.nodes.some((n) => n.id === 404), false);
});
// ── End-to-end: proposal → (mocked authority validation) → UI readout ─────────────
//
// The validator stands in for Rust authority over the facade. editor-tools holds no
// validation authority of its own; it builds the proposal, drafts it, and reflects
// whatever the authoritative report says.
function kindFromAssetId(id) {
    if (id.startsWith('mesh/'))
        return 'mesh';
    if (id.startsWith('sprite/'))
        return 'sprite';
    return 'other';
}
/** A faithful-enough mock of Rust scene validation for the end-to-end wiring test. */
function mockAuthorityValidate(doc) {
    const errors = [];
    const ids = new Set(doc.nodes.map((n) => n.id));
    for (const n of doc.nodes) {
        if (n.parent !== null && !ids.has(n.parent)) {
            errors.push({ code: 'unknown-parent', node: n.id, parent: n.parent, expectedKind: null, actualKind: null, transformReason: null, cyclePath: [] });
        }
        if (n.kind.kind === 'staticMesh' && kindFromAssetId(n.kind.asset.id) !== 'mesh') {
            errors.push({ code: 'asset-kind-mismatch', node: n.id, parent: null, expectedKind: 'mesh', actualKind: kindFromAssetId(n.kind.asset.id), transformReason: null, cyclePath: [] });
        }
    }
    // Cycle detection over parent pointers.
    const parentOf = new Map(doc.nodes.map((n) => [n.id, n.parent === null ? null : n.parent]));
    for (const startNode of doc.nodes) {
        const seen = new Set();
        let cur = startNode.id;
        const path = [];
        while (cur !== null && ids.has(cur)) {
            if (seen.has(cur)) {
                const from = path.indexOf(cur);
                errors.push({ code: 'cycle', node: null, parent: null, expectedKind: null, actualKind: null, transformReason: null, cyclePath: path.slice(from).map((id) => sceneNodeId(id)) });
                break;
            }
            seen.add(cur);
            path.push(cur);
            cur = parentOf.get(cur) ?? null;
        }
    }
    return { errors };
}
function submitProposal(doc, proposal) {
    const draft = applyProposalToDraft(doc, proposal);
    const report = mockAuthorityValidate(draft);
    return summarizeValidation(report);
}
void test('a valid add-node proposal is accepted by authority validation', () => {
    const feedback = submitProposal(baseScene(), proposeAddStaticMesh(sceneNodeId(3), ref('mesh/floor'), { parent: sceneNodeId(1) }));
    assert.equal(feedback.accepted, true);
    assert.equal(feedback.issues.length, 0);
});
void test('a reparent that forms a cycle is rejected with a classified cycle issue', () => {
    // Make the root (1) a child of its descendant (2): 1 → 2 → 1.
    const feedback = submitProposal(baseScene(), proposeReparent(sceneNodeId(1), sceneNodeId(2)));
    assert.equal(feedback.accepted, false);
    assert.ok(feedback.issues.some((i) => i.code === 'cycle'));
});
void test('a wrong-kind static-mesh proposal is rejected with asset-kind-mismatch', () => {
    const feedback = submitProposal(baseScene(), proposeAddStaticMesh(sceneNodeId(3), ref('material/concrete'), { parent: sceneNodeId(1) }));
    assert.equal(feedback.accepted, false);
    const issue = feedback.issues.find((i) => i.code === 'asset-kind-mismatch');
    assert.match(issue.detail, /expected mesh, found other/);
});
void test('a reparent under an absent parent is rejected with unknown-parent', () => {
    const feedback = submitProposal(baseScene(), proposeReparent(sceneNodeId(2), sceneNodeId(99)));
    assert.equal(feedback.accepted, false);
    assert.ok(feedback.issues.some((i) => i.code === 'unknown-parent' && i.node === 2));
});
void test('summarizeValidation maps every classified code to a readout', () => {
    const report = {
        errors: [
            { code: 'duplicate-node-id', node: sceneNodeId(2), parent: null, expectedKind: null, actualKind: null, transformReason: null, cyclePath: [] },
            { code: 'invalid-transform', node: sceneNodeId(3), parent: null, expectedKind: null, actualKind: null, transformReason: 'zero-scale-axis', cyclePath: [] },
        ],
    };
    const feedback = summarizeValidation(report);
    assert.equal(feedback.accepted, false);
    assert.equal(feedback.issues.length, 2);
    assert.match(feedback.issues[1].detail, /zero-scale-axis/);
});
//# sourceMappingURL=scene-authoring.test.js.map