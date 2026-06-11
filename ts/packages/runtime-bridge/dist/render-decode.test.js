// Runtime tests for the render-diff decode path, run with `node --test`.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { decodeRenderDiff, decodeRenderFrameDiff, RenderDecodeError, RenderDiffStream, FrameMemory, } from './render-decode.js';
const fixturesRoot = resolve(import.meta.dirname, '../../../../harness/fixtures/render-diffs');
function loadFixture(name) {
    return JSON.parse(readFileSync(resolve(fixturesRoot, `${name}.json`), 'utf8'));
}
test('decodes the Rust-shaped sample frame (create/update/destroy)', () => {
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
test('decodes a line-geometry debug node', () => {
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
test('rejects an unknown diff op', () => {
    assert.throws(() => decodeRenderDiff({ op: 'teleport', handle: 1 }), (e) => e instanceof RenderDecodeError && /unknown render diff op/.test(e.message));
});
test('rejects an unknown geometry shape', () => {
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
test('rejects malformed payloads with a path-bearing error', () => {
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
test('RenderDiffStream buffers and drains decoded frames in order', () => {
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
test('decodes the Rust render-bridge fixture sequence', () => {
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
test('FrameMemory enforces its single-frame lifetime', () => {
    const mem = new FrameMemory(new Uint8Array([1, 2, 3]));
    assert.ok(mem.valid);
    assert.deepEqual([...mem.bytes()], [1, 2, 3]);
    mem.invalidate();
    assert.ok(!mem.valid);
    assert.throws(() => mem.bytes(), RenderDecodeError);
});
//# sourceMappingURL=render-decode.test.js.map