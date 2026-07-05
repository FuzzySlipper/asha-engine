import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { entityId, renderHandle, type MeshPayloadDescriptor, type RenderDiff, type RenderFrameDiff, type RenderNode, type SpriteInstanceDescriptor, type StaticMeshAsset } from '@asha/contracts';
import { RenderProjection, RenderProjectionError } from './index.js';

const repoRoot = resolve(import.meta.dirname, '../../../..');

function fixturePath(name: string): string {
  return resolve(repoRoot, 'harness/fixtures/render-projection', name);
}

function goldenPath(name: string): string {
  return resolve(repoRoot, 'harness/goldens/render-projection', name);
}

function cubeNode(label = 'cube'): RenderNode {
  return {
    geometry: { shape: 'cube' },
    material: { color: [1, 1, 1, 1], wireframe: false },
    transform: { translation: [0, 0, 0], rotation: [0, 0, 0, 1], scale: [1, 1, 1] },
    visible: true,
    layer: 'scene',
    metadata: { source: entityId(1), tags: [], label },
  };
}

function createPrimitive(handle: number, label = `node-${handle}`, parent: number | null = null): RenderDiff {
  return {
    op: 'create',
    handle: renderHandle(handle),
    parent: parent === null ? null : renderHandle(parent),
    node: cubeNode(label),
  };
}

function quadPayload(): MeshPayloadDescriptor {
  return {
    layout: {
      vertexCount: 4,
      indexCount: 6,
      indexWidth: 'u32',
      attributes: [
        { name: 'position', components: 3, kind: 'f32' },
        { name: 'normal', components: 3, kind: 'f32' },
      ],
    },
    groups: [{ materialSlot: 1, start: 0, count: 6 }],
    bounds: { min: [0, 0, 0], max: [1, 1, 0] },
    source: {
      kind: 'inline',
      positions: [0, 0, 0, 1, 0, 0, 1, 1, 0, 0, 1, 0],
      normals: [0, 0, 1, 0, 0, 1, 0, 0, 1, 0, 0, 1],
      indices: [0, 1, 2, 0, 2, 3],
    },
    provenance: 'staticAsset',
  };
}

function meshAsset(asset = 'mesh/crate'): StaticMeshAsset {
  return {
    asset,
    payload: quadPayload(),
    materialSlots: [{ slot: 1, material: 'material/wood' }],
    collision: { kind: 'aabbFallback' },
  };
}

function sprite(asset = 'sprite/ui', frame = 0): SpriteInstanceDescriptor {
  return {
    asset,
    frame,
    pivot: [0.5, 0.5],
    size: [2, 1],
    sizeMode: 'world',
    billboard: 'none',
    tint: [1, 1, 1, 1],
    renderOrder: 4,
    depth: 'default',
    shading: 'unlit',
    transform: { translation: [0, 0, 0], rotation: [0, 0, 0, 1], scale: [1, 1, 1] },
    attachment: { sourceEntity: entityId(7), sourceSceneNode: null, attachmentPoint: 'head' },
    metadata: { source: entityId(7), tags: [], label: 'sprite' },
  };
}

void test('applies frame ops in order and exposes neutral instructions', () => {
  const projection = new RenderProjection();
  const instructions = projection.applyFrame({
    ops: [
      createPrimitive(1),
      {
        op: 'update',
        handle: renderHandle(1),
        transform: { translation: [5, 0, 0], rotation: [0, 0, 0, 1], scale: [2, 2, 2] },
        material: null,
        visible: false,
        metadata: null,
      },
      { op: 'destroy', handle: renderHandle(1) },
    ],
  });

  assert.deepEqual(instructions.map((instruction) => instruction.op), [
    'upsertNode',
    'upsertNode',
    'removeNode',
  ]);
  assert.equal(projection.handleCount, 0);
});

void test('keeps stable parent/child ids and removes descendants before parents', () => {
  const projection = new RenderProjection();
  projection.applyFrame({ ops: [createPrimitive(10, 'parent'), createPrimitive(11, 'child', 10)] });

  assert.deepEqual(projection.node(renderHandle(10))?.children, [renderHandle(11)]);
  assert.equal(projection.node(renderHandle(11))?.parent, renderHandle(10));

  const instructions = projection.applyDiff({ op: 'destroy', handle: renderHandle(10) });
  assert.deepEqual(instructions, [
    { op: 'removeNode', handle: renderHandle(11) },
    { op: 'removeNode', handle: renderHandle(10) },
  ]);
  assert.equal(projection.handleCount, 0);
});

void test('tracks static mesh definitions and fails closed on in-use redefinition', () => {
  const projection = new RenderProjection();
  projection.applyDiff({ op: 'defineStaticMesh', asset: meshAsset() });
  projection.applyDiff({
    op: 'createStaticMeshInstance',
    handle: renderHandle(1),
    parent: null,
    instance: {
      asset: 'mesh/crate',
      transform: { translation: [1, 0, 0], rotation: [0, 0, 0, 1], scale: [1, 1, 1] },
      materialOverrides: [],
      metadata: { source: entityId(1), tags: [], label: 'crate' },
    },
  });

  assert.equal(projection.staticMeshRefCount('mesh/crate'), 1);
  assert.deepEqual(projection.pickMesh(renderHandle(1)), {
    handle: renderHandle(1),
    provenance: 'staticAsset',
  });
  assert.throws(
    () => projection.applyDiff({ op: 'defineStaticMesh', asset: meshAsset() }),
    RenderProjectionError,
  );

  projection.applyDiff({ op: 'destroy', handle: renderHandle(1) });
  assert.equal(projection.staticMeshRefCount('mesh/crate'), 0);
  assert.doesNotThrow(() => projection.applyDiff({ op: 'defineStaticMesh', asset: meshAsset() }));
});

void test('resolves sprite atlas frames and sprite pick hints without renderer types', () => {
  const projection = new RenderProjection();
  projection.applyFrame({
    ops: [
      {
        op: 'defineSpriteAtlas',
        atlas: {
          id: 'sprite/ui',
          texture: 'texture/ui',
          frames: [{ frame: 3, uvMin: [0.25, 0.5], uvMax: [0.5, 0.75] }],
        },
      },
      { op: 'createSprite', handle: renderHandle(2), parent: null, sprite: sprite('sprite/ui', 0) },
      {
        op: 'updateSprite',
        handle: renderHandle(2),
        frame: 3,
        tint: [1, 0, 0, 0.5],
        renderOrder: 8,
        visible: false,
      },
    ],
  });

  const node = projection.node(renderHandle(2));
  assert.equal(node?.kind, 'sprite');
  if (node?.kind === 'sprite') {
    assert.deepEqual(node.frameUv, [0.25, 0.5, 0.5, 0.75]);
    assert.deepEqual(node.sprite.tint, [1, 0, 0, 0.5]);
    assert.equal(node.renderOrder, 8);
    assert.equal(node.visible, false);
  }
  assert.deepEqual(projection.pickSprite(renderHandle(2)), {
    handle: renderHandle(2),
    sourceEntity: entityId(7),
    sourceSceneNode: null,
    asset: 'sprite/ui',
    attachmentPoint: 'head',
  });
});

void test('fails closed on unknown handles, unsupported ops, and malformed mesh payloads', () => {
  const projection = new RenderProjection();
  assert.throws(
    () =>
      projection.applyDiff({
        op: 'update',
        handle: renderHandle(99),
        transform: null,
        material: null,
        visible: null,
        metadata: null,
      }),
    RenderProjectionError,
  );
  assert.throws(
    () => projection.applyDiff({ op: 'teleport', handle: renderHandle(1) } as unknown as RenderDiff),
    RenderProjectionError,
  );

  projection.applyDiff(createPrimitive(1));
  const validPayload = quadPayload();
  const malformedPayload: MeshPayloadDescriptor = {
    ...validPayload,
    source: {
      kind: 'inline',
      positions: [0, 0, 0],
      normals: [0, 0, 1, 0, 0, 1, 0, 0, 1, 0, 0, 1],
      indices: [0, 1, 2, 0, 2, 3],
    },
  };
  assert.throws(
    () =>
      projection.applyDiff({
        op: 'replaceMeshPayload',
        handle: renderHandle(1),
        payload: malformedPayload,
      }),
    RenderProjectionError,
  );
});

void test('retained projection fixture matches the committed before/after goldens', () => {
  const frames = JSON.parse(readFileSync(fixturePath('retained-sequence.json'), 'utf8')) as RenderFrameDiff[];
  const before = JSON.parse(readFileSync(goldenPath('retained-sequence.before.json'), 'utf8')) as unknown;
  const after = JSON.parse(readFileSync(goldenPath('retained-sequence.after.json'), 'utf8')) as unknown;

  const projection = new RenderProjection();
  assert.deepEqual(projection.snapshot(), before);
  for (const frame of frames) {
    projection.applyFrame(frame);
  }
  assert.deepEqual(projection.snapshot(), after);
});
