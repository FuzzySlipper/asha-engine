// Runtime tests for the render-diff decode path, run with `node --test`.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { renderHandle } from '@asha/contracts';
import { decodeRenderDiff, decodeRenderFrameDiff, decodeMeshPayloadDescriptor, decodeStaticMeshAsset, decodeAnimatedMeshAsset, decodeAnimatedMeshPlaybackCommand, decodeSpriteInstance, RenderDecodeError, RenderDiffStream, FrameMemory, } from './render-decode.js';
const fixturesRoot = resolve(import.meta.dirname, '../../../../harness/fixtures/render-diffs');
function loadFixture(name) {
    return JSON.parse(readFileSync(resolve(fixturesRoot, `${name}.json`), 'utf8'));
}
void test('decodes the Rust-shaped sample frame (create/update/destroy)', () => {
    const frame = decodeRenderFrameDiff(loadFixture('sample-frame'));
    assert.equal(frame.ops.length, 3);
    const create = frame.ops[0];
    const update = frame.ops[1];
    const destroy = frame.ops[2];
    assert.equal(create.op, 'create');
    if (create.op === 'create') {
        assert.equal(create.handle, 1);
        assert.equal(create.parent, null);
        assert.equal(create.node.geometry.shape, 'cube');
        assert.equal(create.node.layer, 'scene');
        assert.deepEqual(create.node.material.color, [1, 1, 1, 1]);
        assert.equal(create.node.metadata.label, 'entity 1');
    }
    assert.equal(update.op, 'update');
    if (update.op === 'update') {
        assert.equal(update.visible, false);
        assert.equal(update.transform, null);
    }
    assert.equal(destroy.op, 'destroy');
});
void test('decodes a line-geometry debug node', () => {
    const diff = decodeRenderDiff({
        op: 'create',
        handle: 9,
        parent: null,
        node: {
            geometry: { shape: 'line', a: [0, 0, 0], b: [1, 1, 0] },
            material: { color: [1, 0, 0, 1], wireframe: true },
            transform: { translation: [0, 0, 0], rotation: [0, 0, 0, 1], scale: [1, 1, 1] },
            visible: true,
            layer: 'debug',
            metadata: { source: null, tags: [], label: '#9' },
        },
    });
    assert.equal(diff.op, 'create');
    if (diff.op === 'create' && diff.node.geometry.shape === 'line') {
        assert.deepEqual(diff.node.geometry.a, [0, 0, 0]);
        assert.deepEqual(diff.node.geometry.b, [1, 1, 0]);
    }
});
void test('rejects an unknown diff op', () => {
    assert.throws(() => decodeRenderDiff({ op: 'teleport', handle: 1 }), (e) => e instanceof RenderDecodeError && /unknown render diff op/.test(e.message));
});
void test('rejects an unknown geometry shape', () => {
    assert.throws(() => decodeRenderDiff({
        op: 'create',
        handle: 1,
        parent: null,
        node: {
            geometry: { shape: 'torus' },
            material: { color: [1, 1, 1, 1], wireframe: false },
            transform: { translation: [0, 0, 0], rotation: [0, 0, 0, 1], scale: [1, 1, 1] },
            visible: true,
            layer: 'scene',
            metadata: { source: null, tags: [], label: null },
        },
    }), (e) => e instanceof RenderDecodeError && /unknown geometry shape/.test(e.message));
});
void test('rejects malformed payloads with a path-bearing error', () => {
    // Missing node.transform.
    assert.throws(() => decodeRenderDiff({
        op: 'create',
        handle: 1,
        parent: null,
        node: {
            geometry: { shape: 'cube' },
            material: { color: [1, 1, 1, 1], wireframe: false },
            visible: true,
            layer: 'scene',
            metadata: { source: null, tags: [], label: null },
        },
    }), (e) => e instanceof RenderDecodeError && e.path.includes('transform'));
    // Wrong tuple length.
    assert.throws(() => decodeRenderDiff({ op: 'destroy', handle: 'not-a-number' }), RenderDecodeError);
    // Not an object.
    assert.throws(() => decodeRenderFrameDiff(42), RenderDecodeError);
});
void test('RenderDiffStream buffers and drains decoded frames in order', () => {
    const stream = new RenderDiffStream();
    assert.equal(stream.pending, 0);
    stream.push(loadFixture('sample-frame'));
    stream.push({ ops: [] });
    assert.equal(stream.pending, 2);
    const frames = stream.drain();
    assert.equal(frames.length, 2);
    assert.equal(frames[0].ops.length, 3);
    assert.equal(frames[1].ops.length, 0);
    assert.equal(stream.pending, 0);
});
void test('decodes the Rust render-bridge fixture sequence', () => {
    // The same fixture the Rust render bridge emits and the Three.js renderer
    // applies — proving the decode boundary on a real Rust-produced artifact.
    const frames = loadFixture('bridge-sequence');
    const decoded = frames.map((f) => decodeRenderFrameDiff(f));
    assert.equal(decoded.length, 2);
    assert.equal(decoded[0].ops.length, 2);
    assert.equal(decoded[1].ops.length, 3);
    assert.equal(decoded[0].ops[0].op, 'create');
    assert.equal(decoded[1].ops[2].op, 'destroy');
});
void test('decodes the animated mesh named-clip fixture', () => {
    const frame = decodeRenderFrameDiff(loadFixture('animated-mesh'));
    assert.deepEqual(frame.ops.map((op) => op.op), [
        'defineAnimatedMesh',
        'createAnimatedMeshInstance',
        'setAnimatedMeshPlayback',
    ]);
    const define = frame.ops[0];
    const playback = frame.ops[2];
    assert.equal(define.op, 'defineAnimatedMesh');
    assert.equal(playback.op, 'setAnimatedMeshPlayback');
    if (define.op === 'defineAnimatedMesh' && playback.op === 'setAnimatedMeshPlayback') {
        assert.equal(define.asset.asset, 'mesh-animation/kenney-retro-character-medium');
        assert.deepEqual(define.asset.clips.map((clip) => clip.id), ['idle', 'run', 'jump']);
        assert.equal(define.asset.defaultClip, 'idle');
        assert.equal(playback.playback.action, 'play');
        if (playback.playback.action === 'play') {
            assert.equal(playback.playback.clip, 'run');
            assert.equal(playback.playback.loop, 'repeat');
            assert.equal(playback.playback.restart, true);
        }
    }
});
void test('FrameMemory enforces its single-frame lifetime', () => {
    const mem = new FrameMemory(new Uint8Array([1, 2, 3]));
    assert.ok(mem.valid);
    assert.deepEqual([...mem.bytes()], [1, 2, 3]);
    mem.invalidate();
    assert.ok(!mem.valid);
    assert.throws(() => mem.bytes(), RenderDecodeError);
});
// ── mesh payload descriptor (ADR 0007 / #2262) ────────────────────────────────
function oneTriangleInline() {
    return {
        layout: {
            vertexCount: 3,
            indexCount: 3,
            indexWidth: 'u32',
            attributes: [
                { name: 'position', components: 3, kind: 'f32' },
                { name: 'normal', components: 3, kind: 'f32' },
            ],
        },
        groups: [{ materialSlot: 1, start: 0, count: 3 }],
        bounds: { min: [0, 0, 0], max: [1, 1, 0] },
        source: {
            kind: 'inline',
            positions: [0, 0, 0, 1, 0, 0, 1, 1, 0],
            normals: [0, 0, 1, 0, 0, 1, 0, 0, 1],
            indices: [0, 1, 2],
        },
        provenance: 'voxelChunk',
    };
}
void test('decodes a valid inline mesh payload and the replaceMeshPayload diff', () => {
    const d = decodeMeshPayloadDescriptor(oneTriangleInline());
    assert.equal(d.layout.vertexCount, 3);
    assert.equal(d.groups.length, 1);
    assert.equal(d.source.kind, 'inline');
    const diff = decodeRenderDiff({ op: 'replaceMeshPayload', handle: 5, payload: oneTriangleInline() });
    assert.equal(diff.op, 'replaceMeshPayload');
});
void test('decodes a handle-source mesh payload', () => {
    const p = oneTriangleInline();
    p['source'] = { kind: 'handle', buffer: 7, positionsByteOffset: 0, normalsByteOffset: 36, indicesByteOffset: 72 };
    const d = decodeMeshPayloadDescriptor(p);
    assert.equal(d.source.kind, 'handle');
});
void test('rejects malformed mesh payloads with path-bearing errors', () => {
    // wrong positions length
    const badPos = oneTriangleInline();
    badPos.source.positions = [0, 0, 0];
    assert.throws(() => decodeMeshPayloadDescriptor(badPos), RenderDecodeError);
    // index out of range
    const badIdx = oneTriangleInline();
    badIdx.source.indices = [0, 1, 9];
    assert.throws(() => decodeMeshPayloadDescriptor(badIdx), RenderDecodeError);
    // groups do not tile
    const badGroup = oneTriangleInline();
    badGroup.groups[0].count = 2;
    assert.throws(() => decodeMeshPayloadDescriptor(badGroup), RenderDecodeError);
    // unknown attribute name
    const badAttr = oneTriangleInline();
    badAttr.layout.attributes[0].name = 'tangent';
    assert.throws(() => decodeMeshPayloadDescriptor(badAttr), RenderDecodeError);
    // unknown provenance
    const badProv = oneTriangleInline();
    badProv.provenance = 'mystery';
    assert.throws(() => decodeMeshPayloadDescriptor(badProv), RenderDecodeError);
});
// ── static mesh + sprite decode (render-asset-04/05/06) ───────────────────────
function crateAssetRaw() {
    return {
        asset: 'mesh/crate',
        payload: { ...oneTriangleInline(), provenance: 'staticAsset' },
        materialSlots: [{ slot: 1, material: 'material/wood' }],
        collision: { kind: 'aabbFallback' },
    };
}
void test('decodes a static mesh asset + instance diff, validating slot bindings', () => {
    const asset = decodeStaticMeshAsset(crateAssetRaw());
    assert.equal(asset.asset, 'mesh/crate');
    assert.equal(asset.collision.kind, 'aabbFallback');
    const diff = decodeRenderDiff({
        op: 'createStaticMeshInstance',
        handle: 1,
        parent: null,
        instance: {
            asset: 'mesh/crate',
            transform: { translation: [0, 0, 0], rotation: [0, 0, 0, 1], scale: [1, 1, 1] },
            materialOverrides: [{ slot: 1, material: 'material/wood-red' }],
            metadata: { source: null, tags: [], label: 'crate' },
        },
    });
    assert.equal(diff.op, 'createStaticMeshInstance');
});
void test('decodes a defineMaterial diff (catalog material descriptor, visual only)', () => {
    const diff = decodeRenderDiff({
        op: 'defineMaterial',
        material: {
            id: 'material/wood',
            color: [0.6, 0.4, 0.2, 1],
            texture: null,
            roughness: 1,
            emissive: 0,
            uvStrategy: 'flat',
        },
    });
    assert.equal(diff.op, 'defineMaterial');
    if (diff.op === 'defineMaterial') {
        assert.equal(diff.material.schemaVersion, 2, 'legacy descriptor is normalized to v2');
        assert.equal(diff.material.id, 'material/wood');
        assert.deepEqual(diff.material.color, [0.6, 0.4, 0.2, 1]);
        assert.deepEqual(diff.material.textureTint, [1, 1, 1, 1]);
        assert.deepEqual(diff.material.emissionColor, [0.6, 0.4, 0.2]);
        assert.equal(diff.material.emissionIntensity, 0);
        assert.equal(diff.material.uvStrategy, 'flat');
        // The descriptor shape carries no collision/authority field by construction.
        assert.ok(!('solid' in diff.material) && !('structuralClass' in diff.material));
    }
});
void test('decodes v2 material feedback and a handle-targeted instance parameter update', () => {
    const define = decodeRenderDiff({
        op: 'defineMaterial',
        material: {
            schemaVersion: 2,
            id: 'material/warning',
            color: [0.4, 0.4, 0.4, 1],
            texture: null,
            roughness: 0.7,
            textureTint: [1, 0.8, 0.6, 1],
            emissionColor: [1, 0.1, 0],
            emissionIntensity: 2.5,
            uvStrategy: 'flat',
        },
    });
    assert.equal(define.op, 'defineMaterial');
    if (define.op === 'defineMaterial') {
        assert.deepEqual(define.material.emissionColor, [1, 0.1, 0]);
        assert.equal(define.material.emissionIntensity, 2.5);
    }
    const update = decodeRenderDiff({
        op: 'setMaterialInstanceParameters',
        handle: 17,
        slot: 0,
        parameters: {
            textureTint: [0.2, 1, 0.2, 1],
            emissionColor: [0, 1, 0.2],
            emissionIntensity: 1.25,
        },
    });
    assert.equal(update.op, 'setMaterialInstanceParameters');
    if (update.op === 'setMaterialInstanceParameters') {
        assert.equal(update.handle, renderHandle(17));
        assert.equal(update.slot, 0);
        assert.ok(update.parameters);
        assert.deepEqual(update.parameters.textureTint, [0.2, 1, 0.2, 1]);
    }
});
void test('rejects unsupported material descriptor schemas', () => {
    assert.throws(() => decodeRenderDiff({
        op: 'defineMaterial',
        material: {
            schemaVersion: 99,
            id: 'material/future',
            color: [1, 1, 1, 1],
            texture: null,
            roughness: 1,
            textureTint: [1, 1, 1, 1],
            emissionColor: [0, 0, 0],
            emissionIntensity: 0,
            uvStrategy: 'flat',
        },
    }), RenderDecodeError);
    assert.throws(() => decodeRenderDiff({
        op: 'setMaterialInstanceParameters',
        handle: 1,
        slot: 0,
        parameters: {
            textureTint: [1, 1, 1, 1],
            emissionColor: [1, 0, 0],
            emissionIntensity: -0.1,
        },
    }), /expected a non-negative number/);
});
void test('rejects a material descriptor with an unknown uv strategy', () => {
    assert.throws(() => decodeRenderDiff({
        op: 'defineMaterial',
        material: {
            id: 'material/x',
            color: [1, 1, 1, 1],
            texture: null,
            roughness: 1,
            emissive: 0,
            uvStrategy: 'holographic',
        },
    }), RenderDecodeError);
});
void test('decodes defineTexture and defineSpriteAtlas, validating frame rects', () => {
    const tex = decodeRenderDiff({
        op: 'defineTexture',
        texture: {
            id: 'texture/spark',
            width: 64,
            height: 32,
            filter: 'nearest',
            wrap: 'clamp',
            contentHash: null,
            version: 1,
        },
    });
    assert.equal(tex.op, 'defineTexture');
    const atlas = decodeRenderDiff({
        op: 'defineSpriteAtlas',
        atlas: {
            id: 'sprite/spark-sheet',
            texture: 'texture/spark',
            frames: [
                { frame: 0, uvMin: [0, 0], uvMax: [0.5, 1] },
                { frame: 3, uvMin: [0.5, 0], uvMax: [1, 1] },
            ],
        },
    });
    assert.equal(atlas.op, 'defineSpriteAtlas');
    if (atlas.op === 'defineSpriteAtlas') {
        assert.equal(atlas.atlas.frames.length, 2);
    }
});
void test('rejects a zero-dimension texture and a degenerate/out-of-range atlas frame', () => {
    assert.throws(() => decodeRenderDiff({
        op: 'defineTexture',
        texture: { id: 'texture/x', width: 0, height: 8, filter: 'nearest', wrap: 'clamp', contentHash: null, version: 1 },
    }), RenderDecodeError);
    assert.throws(() => decodeRenderDiff({
        op: 'defineSpriteAtlas',
        atlas: {
            id: 'sprite/x',
            texture: 'texture/x',
            frames: [{ frame: 0, uvMin: [0.5, 0], uvMax: [0.5, 1] }], // zero width
        },
    }), RenderDecodeError);
});
void test('rejects a static mesh whose group references an unbound material slot', () => {
    const bad = crateAssetRaw();
    bad['materialSlots'] = [{ slot: 9, material: 'material/wood' }]; // group uses slot 1
    assert.throws(() => decodeStaticMeshAsset(bad), RenderDecodeError);
});
function animatedMeshAssetRaw() {
    return {
        asset: 'mesh-animation/kenney-retro-character-medium',
        runtimeFormat: 'glb',
        contentHash: 'sha256-fixture-pending',
        clips: [
            { id: 'idle', name: 'Idle', durationSeconds: 1.2 },
            { id: 'run', name: 'Run', durationSeconds: 0.8 },
            { id: 'jump', name: 'Jump', durationSeconds: 0.6 },
        ],
        defaultClip: 'idle',
        materialSlots: [{ slot: 0, material: 'material/kenney-human-male-a' }],
        bounds: { min: [-0.5, 0, -0.5], max: [0.5, 1.8, 0.5] },
    };
}
void test('decodes animated mesh assets and projection-only playback commands', () => {
    const asset = decodeAnimatedMeshAsset(animatedMeshAssetRaw());
    assert.equal(asset.runtimeFormat, 'glb');
    assert.deepEqual(asset.clips.map((clip) => clip.id), ['idle', 'run', 'jump']);
    const playback = decodeAnimatedMeshPlaybackCommand({
        action: 'play',
        clip: 'run',
        loop: 'repeat',
        speed: 1,
        weight: 1,
        restart: true,
        fadeSeconds: 0.1,
    });
    assert.equal(playback.action, 'play');
    if (playback.action === 'play') {
        assert.equal(playback.clip, 'run');
    }
    const diff = decodeRenderDiff({
        op: 'createAnimatedMeshInstance',
        handle: 8,
        parent: null,
        instance: {
            asset: 'mesh-animation/kenney-retro-character-medium',
            transform: { translation: [0, 0, 0], rotation: [0, 0, 0, 1], scale: [1, 1, 1] },
            materialOverrides: [],
            playback: null,
            metadata: { source: null, tags: [], label: 'animated-proof' },
        },
    });
    assert.equal(diff.op, 'createAnimatedMeshInstance');
});
void test('rejects malformed animated mesh assets and playback actions', () => {
    const duplicateClip = animatedMeshAssetRaw();
    duplicateClip['clips'] = [
        { id: 'run', name: 'Run A', durationSeconds: 1 },
        { id: 'run', name: 'Run B', durationSeconds: 1 },
    ];
    assert.throws(() => decodeAnimatedMeshAsset(duplicateClip), RenderDecodeError);
    const missingDefault = animatedMeshAssetRaw();
    missingDefault['defaultClip'] = 'dance';
    assert.throws(() => decodeAnimatedMeshAsset(missingDefault), RenderDecodeError);
    assert.throws(() => decodeAnimatedMeshPlaybackCommand({
        action: 'play',
        clip: '',
        loop: 'repeat',
        speed: 1,
        weight: 1,
        restart: true,
        fadeSeconds: null,
    }), RenderDecodeError);
});
void test('rejects a proxy collision policy with an empty proxy asset', () => {
    const bad = crateAssetRaw();
    bad['collision'] = { kind: 'proxy', proxyAsset: '' };
    assert.throws(() => decodeStaticMeshAsset(bad), RenderDecodeError);
});
function sparkSpriteRaw() {
    return {
        asset: 'sprite/spark',
        frame: 0,
        pivot: [0.5, 0.5],
        size: [1, 1],
        sizeMode: 'world',
        billboard: 'spherical',
        tint: [1, 1, 1, 1],
        renderOrder: 0,
        depth: 'default',
        shading: 'unlit',
        transform: { translation: [0, 0, 0], rotation: [0, 0, 0, 1], scale: [1, 1, 1] },
        attachment: { sourceEntity: 7, sourceSceneNode: null, attachmentPoint: 'muzzle' },
        metadata: { source: 7, tags: [], label: 'spark' },
    };
}
void test('decodes a sprite instance + a deterministic updateSprite diff', () => {
    const s = decodeSpriteInstance(sparkSpriteRaw());
    assert.equal(s.asset, 'sprite/spark');
    assert.equal(s.attachment.attachmentPoint, 'muzzle');
    const diff = decodeRenderDiff({ op: 'updateSprite', handle: 1, frame: 3, tint: null, renderOrder: null, visible: false });
    assert.equal(diff.op, 'updateSprite');
});
void test('rejects a sprite with out-of-range pivot or non-positive size', () => {
    const badPivot = sparkSpriteRaw();
    badPivot['pivot'] = [1.5, 0];
    assert.throws(() => decodeSpriteInstance(badPivot), RenderDecodeError);
    const badSize = sparkSpriteRaw();
    badSize['size'] = [0, 1];
    assert.throws(() => decodeSpriteInstance(badSize), RenderDecodeError);
    // reserved lit shading is accepted (not rejected as unlit-only).
    const lit = sparkSpriteRaw();
    lit['shading'] = 'lit';
    assert.equal(decodeSpriteInstance(lit).shading, 'lit');
});
//# sourceMappingURL=render-decode.test.js.map