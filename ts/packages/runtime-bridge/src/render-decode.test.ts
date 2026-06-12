// Runtime tests for the render-diff decode path, run with `node --test`.

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import {
  decodeRenderDiff,
  decodeRenderFrameDiff,
  decodeMeshPayloadDescriptor,
  decodeStaticMeshAsset,
  decodeSpriteInstance,
  RenderDecodeError,
  RenderDiffStream,
  FrameMemory,
} from './render-decode.js';

const fixturesRoot = resolve(import.meta.dirname, '../../../../harness/fixtures/render-diffs');

function loadFixture(name: string): unknown {
  return JSON.parse(readFileSync(resolve(fixturesRoot, `${name}.json`), 'utf8'));
}

test('decodes the Rust-shaped sample frame (create/update/destroy)', () => {
  const frame = decodeRenderFrameDiff(loadFixture('sample-frame'));
  assert.equal(frame.ops.length, 3);

  const create = frame.ops[0]!;
  const update = frame.ops[1]!;
  const destroy = frame.ops[2]!;
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
  assert.throws(
    () => decodeRenderDiff({ op: 'teleport', handle: 1 }),
    (e: unknown) => e instanceof RenderDecodeError && /unknown render diff op/.test((e as Error).message),
  );
});

test('rejects an unknown geometry shape', () => {
  assert.throws(
    () =>
      decodeRenderDiff({
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
      }),
    (e: unknown) => e instanceof RenderDecodeError && /unknown geometry shape/.test((e as Error).message),
  );
});

test('rejects malformed payloads with a path-bearing error', () => {
  // Missing node.transform.
  assert.throws(
    () =>
      decodeRenderDiff({
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
      }),
    (e: unknown) => e instanceof RenderDecodeError && (e as RenderDecodeError).path.includes('transform'),
  );
  // Wrong tuple length.
  assert.throws(
    () => decodeRenderDiff({ op: 'destroy', handle: 'not-a-number' }),
    RenderDecodeError,
  );
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
  assert.equal(frames[0]!.ops.length, 3);
  assert.equal(frames[1]!.ops.length, 0);
  assert.equal(stream.pending, 0);
});

test('decodes the Rust render-bridge fixture sequence', () => {
  // The same fixture the Rust render bridge emits and the Three.js renderer
  // applies — proving the decode boundary on a real Rust-produced artifact.
  const frames = loadFixture('bridge-sequence') as unknown[];
  const decoded = frames.map((f) => decodeRenderFrameDiff(f));
  assert.equal(decoded.length, 2);
  assert.equal(decoded[0]!.ops.length, 2);
  assert.equal(decoded[1]!.ops.length, 3);
  assert.equal(decoded[0]!.ops[0]!.op, 'create');
  assert.equal(decoded[1]!.ops[2]!.op, 'destroy');
});

test('FrameMemory enforces its single-frame lifetime', () => {
  const mem = new FrameMemory(new Uint8Array([1, 2, 3]));
  assert.ok(mem.valid);
  assert.deepEqual([...mem.bytes()], [1, 2, 3]);

  mem.invalidate();
  assert.ok(!mem.valid);
  assert.throws(() => mem.bytes(), RenderDecodeError);
});

// ── mesh payload descriptor (ADR 0007 / #2262) ────────────────────────────────

function oneTriangleInline(): unknown {
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

test('decodes a valid inline mesh payload and the replaceMeshPayload diff', () => {
  const d = decodeMeshPayloadDescriptor(oneTriangleInline());
  assert.equal(d.layout.vertexCount, 3);
  assert.equal(d.groups.length, 1);
  assert.equal(d.source.kind, 'inline');

  const diff = decodeRenderDiff({ op: 'replaceMeshPayload', handle: 5, payload: oneTriangleInline() });
  assert.equal(diff.op, 'replaceMeshPayload');
});

test('decodes a handle-source mesh payload', () => {
  const p = oneTriangleInline() as Record<string, unknown>;
  p.source = { kind: 'handle', buffer: 7, positionsByteOffset: 0, normalsByteOffset: 36, indicesByteOffset: 72 };
  const d = decodeMeshPayloadDescriptor(p);
  assert.equal(d.source.kind, 'handle');
});

test('rejects malformed mesh payloads with path-bearing errors', () => {
  // wrong positions length
  const badPos = oneTriangleInline() as { source: { positions: number[] } };
  badPos.source.positions = [0, 0, 0];
  assert.throws(() => decodeMeshPayloadDescriptor(badPos), RenderDecodeError);

  // index out of range
  const badIdx = oneTriangleInline() as { source: { indices: number[] } };
  badIdx.source.indices = [0, 1, 9];
  assert.throws(() => decodeMeshPayloadDescriptor(badIdx), RenderDecodeError);

  // groups do not tile
  const badGroup = oneTriangleInline() as { groups: { count: number }[] };
  badGroup.groups[0]!.count = 2;
  assert.throws(() => decodeMeshPayloadDescriptor(badGroup), RenderDecodeError);

  // unknown attribute name
  const badAttr = oneTriangleInline() as { layout: { attributes: { name: string }[] } };
  badAttr.layout.attributes[0]!.name = 'tangent';
  assert.throws(() => decodeMeshPayloadDescriptor(badAttr), RenderDecodeError);

  // unknown provenance
  const badProv = oneTriangleInline() as { provenance: string };
  badProv.provenance = 'mystery';
  assert.throws(() => decodeMeshPayloadDescriptor(badProv), RenderDecodeError);
});

// ── static mesh + sprite decode (render-asset-04/05/06) ───────────────────────

function crateAssetRaw(): Record<string, unknown> {
  return {
    asset: 'mesh/crate',
    payload: { ...(oneTriangleInline() as object), provenance: 'staticAsset' },
    materialSlots: [{ slot: 1, material: 'material/wood' }],
    collision: { kind: 'aabbFallback' },
  };
}

test('decodes a static mesh asset + instance diff, validating slot bindings', () => {
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

test('rejects a static mesh whose group references an unbound material slot', () => {
  const bad = crateAssetRaw();
  bad.materialSlots = [{ slot: 9, material: 'material/wood' }]; // group uses slot 1
  assert.throws(() => decodeStaticMeshAsset(bad), RenderDecodeError);
});

test('rejects a proxy collision policy with an empty proxy asset', () => {
  const bad = crateAssetRaw();
  bad.collision = { kind: 'proxy', proxyAsset: '' };
  assert.throws(() => decodeStaticMeshAsset(bad), RenderDecodeError);
});

function sparkSpriteRaw(): Record<string, unknown> {
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

test('decodes a sprite instance + a deterministic updateSprite diff', () => {
  const s = decodeSpriteInstance(sparkSpriteRaw());
  assert.equal(s.asset, 'sprite/spark');
  assert.equal(s.attachment.attachmentPoint, 'muzzle');

  const diff = decodeRenderDiff({ op: 'updateSprite', handle: 1, frame: 3, tint: null, renderOrder: null, visible: false });
  assert.equal(diff.op, 'updateSprite');
});

test('rejects a sprite with out-of-range pivot or non-positive size', () => {
  const badPivot = sparkSpriteRaw();
  badPivot.pivot = [1.5, 0];
  assert.throws(() => decodeSpriteInstance(badPivot), RenderDecodeError);

  const badSize = sparkSpriteRaw();
  badSize.size = [0, 1];
  assert.throws(() => decodeSpriteInstance(badSize), RenderDecodeError);

  // reserved lit shading is accepted (not rejected as unlit-only).
  const lit = sparkSpriteRaw();
  lit.shading = 'lit';
  assert.equal(decodeSpriteInstance(lit).shading, 'lit');
});
