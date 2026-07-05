import { test } from 'node:test';
import assert from 'node:assert/strict';
import { entityId, sceneId, sceneNodeId, } from '@asha/contracts';
import { buildOutlinerModel, inspectEntity, inspectNode, } from './scene-outliner.js';
const IDENTITY = {
    translation: [0, 0, 0],
    rotation: [0, 0, 0, 1],
    scale: [1, 1, 1],
};
function at(x, y, z) {
    return { translation: [x, y, z], rotation: [0, 0, 0, 1], scale: [1, 1, 1] };
}
const meshAsset = { id: 'mesh:crate', version: { req: 'any' }, hash: null };
function node(over) {
    return {
        id: sceneNodeId(over.id),
        parent: over.parent ?? null,
        childOrder: over.childOrder ?? 0,
        label: over.label ?? null,
        tags: over.tags ?? [],
        transform: over.transform ?? IDENTITY,
        kind: over.kind ?? { kind: 'emptyGroup' },
    };
}
// A small abstract scene: a root group with two children (a mesh + a group), plus
// a deeper grandchild. No product nouns — purely structural.
function fixtureScene() {
    return {
        schemaVersion: 1,
        id: sceneId(1001),
        metadata: { name: 'fixture', authoringFormatVersion: 1 },
        dependencies: [meshAsset],
        nodes: [
            node({ id: 10, label: 'root', childOrder: 0 }),
            node({ id: 12, parent: sceneNodeId(10), childOrder: 1, label: 'group-b' }),
            node({
                id: 11,
                parent: sceneNodeId(10),
                childOrder: 0,
                label: 'mesh-a',
                transform: at(1, 0, 0),
                kind: { kind: 'staticMesh', asset: meshAsset },
            }),
            node({ id: 13, parent: sceneNodeId(12), childOrder: 0, label: 'leaf' }),
        ],
    };
}
function input(over = {}) {
    return {
        scene: over.scene ?? fixtureScene(),
        entities: over.entities ?? [],
        sourceTraces: over.sourceTraces ?? [],
    };
}
void test('buildOutlinerModel builds a tree ordered by childOrder then id', () => {
    const model = buildOutlinerModel(input());
    assert.equal(model.roots.length, 1);
    const root = model.roots[0];
    assert.equal(root.node.id, 10);
    // childOrder 0 (mesh-a, id 11) before childOrder 1 (group-b, id 12).
    assert.deepEqual(root.children.map((c) => c.node.id), [11, 12]);
    // group-b has the leaf.
    assert.deepEqual(root.children[1].children.map((c) => c.node.id), [13]);
});
void test('a scene node with a live source trace correlates as matched', () => {
    const traces = [
        { sceneNodeId: sceneNodeId(11), runtimeEntityId: entityId(500) },
    ];
    const entities = [
        { entityId: entityId(500), lifecycle: 'active', transform: at(1, 0, 0), sourceNode: sceneNodeId(11) },
    ];
    const model = buildOutlinerModel(input({ sourceTraces: traces, entities }));
    const mesh = model.roots[0].children.find((c) => c.node.id === 11);
    assert.deepEqual(mesh.correlation, { kind: 'matched', entityId: entityId(500), lifecycle: 'active' });
    assert.equal(model.diagnostics.length, 0);
});
void test('a tombstoned scene-sourced entity is surfaced as destroyed, never hidden', () => {
    const traces = [
        { sceneNodeId: sceneNodeId(11), runtimeEntityId: entityId(500) },
    ];
    const entities = [
        { entityId: entityId(500), lifecycle: 'tombstoned', transform: null, sourceNode: sceneNodeId(11) },
    ];
    const model = buildOutlinerModel(input({ sourceTraces: traces, entities }));
    const mesh = model.roots[0].children.find((c) => c.node.id === 11);
    assert.deepEqual(mesh.correlation, { kind: 'destroyed', entityId: entityId(500) });
    assert.deepEqual(model.diagnostics.map((d) => d.code), ['destroyedSceneEntity']);
});
void test('a trace to an absent entity is reported as a dangling trace', () => {
    const traces = [
        { sceneNodeId: sceneNodeId(11), runtimeEntityId: entityId(999) },
    ];
    const model = buildOutlinerModel(input({ sourceTraces: traces }));
    const mesh = model.roots[0].children.find((c) => c.node.id === 11);
    assert.deepEqual(mesh.correlation, { kind: 'danglingTrace', entityId: entityId(999) });
    assert.equal(model.diagnostics[0].code, 'danglingSourceTrace');
});
void test('runtime-created entities (no scene source) are listed separately', () => {
    const entities = [
        { entityId: entityId(700), lifecycle: 'active', transform: at(5, 5, 5), sourceNode: null },
        { entityId: entityId(701), lifecycle: 'disabled', transform: null, sourceNode: null },
    ];
    const model = buildOutlinerModel(input({ entities }));
    assert.deepEqual(model.runtimeOnly, [
        { entityId: entityId(700), lifecycle: 'active', hasTransform: true },
        { entityId: entityId(701), lifecycle: 'disabled', hasTransform: false },
    ]);
});
void test('an orphaned node (absent parent) is shown explicitly with a diagnostic', () => {
    const scene = fixtureScene();
    const withOrphan = {
        ...scene,
        nodes: [...scene.nodes, node({ id: 20, parent: sceneNodeId(99), label: 'orphan' })],
    };
    const model = buildOutlinerModel(input({ scene: withOrphan }));
    assert.deepEqual(model.orphans.map((o) => o.node.id), [20]);
    assert.ok(model.diagnostics.some((d) => d.code === 'orphanedNode' && d.sceneNode === 20));
});
void test('an entity naming an absent source node yields danglingEntitySource', () => {
    const entities = [
        { entityId: entityId(800), lifecycle: 'active', transform: null, sourceNode: sceneNodeId(99) },
    ];
    const model = buildOutlinerModel(input({ entities }));
    assert.ok(model.diagnostics.some((d) => d.code === 'danglingEntitySource' && d.entityId === 800));
});
void test('inspectNode distinguishes authored transform from runtime override', () => {
    const traces = [
        { sceneNodeId: sceneNodeId(11), runtimeEntityId: entityId(500) },
    ];
    const entities = [
        // Runtime moved the entity away from its authored (1,0,0).
        { entityId: entityId(500), lifecycle: 'active', transform: at(3, 0, 0), sourceNode: sceneNodeId(11) },
    ];
    const view = inspectNode(input({ sourceTraces: traces, entities }), sceneNodeId(11));
    assert.deepEqual(view.transform.authored.translation, [1, 0, 0]);
    assert.deepEqual(view.transform.runtime?.translation, [3, 0, 0]);
    assert.equal(view.transform.diverged, true);
    assert.equal(view.assetRefs.kindTag, 'staticMesh');
    assert.equal(view.assetRefs.asset?.id, 'mesh:crate');
});
void test('inspectNode reports no divergence when transforms match, null runtime when none', () => {
    const matched = inspectNode(input({
        sourceTraces: [{ sceneNodeId: sceneNodeId(11), runtimeEntityId: entityId(500) }],
        entities: [{ entityId: entityId(500), lifecycle: 'active', transform: at(1, 0, 0), sourceNode: sceneNodeId(11) }],
    }), sceneNodeId(11));
    assert.equal(matched.transform.diverged, false);
    const noRuntime = inspectNode(input(), sceneNodeId(11));
    assert.equal(noRuntime.transform.runtime, null);
    assert.equal(noRuntime.transform.diverged, false);
    assert.deepEqual(noRuntime.correlation, { kind: 'authoredOnly' });
});
void test('inspectNode/inspectEntity return null for unknown ids', () => {
    assert.equal(inspectNode(input(), sceneNodeId(404)), null);
    assert.equal(inspectEntity(input(), entityId(404)), null);
});
void test('inspectEntity resolves a scene source and flags a dangling source', () => {
    const resolved = inspectEntity(input({ entities: [{ entityId: entityId(500), lifecycle: 'active', transform: null, sourceNode: sceneNodeId(11) }] }), entityId(500));
    assert.equal(resolved.sourceNode?.id, 11);
    assert.equal(resolved.danglingSource, false);
    const dangling = inspectEntity(input({ entities: [{ entityId: entityId(501), lifecycle: 'active', transform: null, sourceNode: sceneNodeId(99) }] }), entityId(501));
    assert.equal(dangling.sourceNode, null);
    assert.equal(dangling.danglingSource, true);
});
//# sourceMappingURL=scene-outliner.test.js.map