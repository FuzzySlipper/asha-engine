import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import {
  entityId,
  renderHandle,
  type AnimatedMeshAsset,
  type MeshPayloadDescriptor,
  type RenderDiff,
  type RenderFrameDiff,
  type RenderNode,
  type SpriteInstanceDescriptor,
  type StaticMeshAsset,
} from '@asha/contracts';
import {
  RenderProjection,
  RenderProjectionError,
  createGeneratedTunnelRoomFrame,
  createGeneratedTunnelViewportFrame,
  type GeneratedTunnelFrameReadout,
} from './index.js';

const repoRoot = resolve(import.meta.dirname, '../../../..');

function fixturePath(name: string): string {
  return resolve(repoRoot, 'harness/fixtures/render-projection', name);
}

function renderDiffFixturePath(name: string): string {
  return resolve(repoRoot, 'harness/fixtures/render-diffs', name);
}

function goldenPath(name: string): string {
  return resolve(repoRoot, 'harness/goldens/render-projection', name);
}

function tinyGeneratedTunnelReadout(): GeneratedTunnelFrameReadout {
  return {
    generator: {
      presetId: 'tiny-enclosed',
      seed: 17,
      generationHash: 'fnv1a64:0821a0c2aea17dff',
      outputHash: '1471496d88d70647',
    },
    volume: {
      tunnelDims: [5, 4, 9],
      solidVoxels: 178,
    },
    spawnMarkers: [
      { id: 'player_start', kind: 'player', world: [2.5, 2.5, 2.5] },
      { id: 'exit_hint', kind: 'exit', world: [4.5, 2.5, 8.5] },
    ],
    materials: [
      { role: 'wall', material: 1 },
      { role: 'floor', material: 2 },
      { role: 'accent', material: 3 },
    ],
    renderProjection: {
      hash: 'fnv1a64:21eb8696f6f3b5c4',
    },
    collisionProjection: {
      hash: 'fnv1a64:627389be013a3154',
    },
    runtimeFrame: {
      worldOffset: [-3.5, -1, -5.5],
      playableMin: [-2.5, 0, -4.5],
      playableMax: [2.5, 4, 4.5],
    },
    replayHash: 'fnv1a64:0821a0c2aea17dff',
  };
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

function animatedMeshAsset(asset = 'mesh-animation/kenney-retro-character-medium'): AnimatedMeshAsset {
  return {
    asset,
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

void test('retains and resets handle-targeted material feedback parameters', () => {
  const projection = new RenderProjection();
  projection.applyDiff({ op: 'defineStaticMesh', asset: meshAsset() });
  projection.applyDiff({
    op: 'createStaticMeshInstance',
    handle: renderHandle(1),
    parent: null,
    instance: {
      asset: 'mesh/crate',
      transform: { translation: [0, 0, 0], rotation: [0, 0, 0, 1], scale: [1, 1, 1] },
      materialOverrides: [],
      metadata: { source: entityId(1), tags: [], label: 'warning-light' },
    },
  });
  const parameters = {
    textureTint: [1, 0.2, 0.2, 1] as const,
    emissionColor: [1, 0, 0] as const,
    emissionIntensity: 2,
  };
  projection.applyDiff({
    op: 'setMaterialInstanceParameters',
    handle: renderHandle(1),
    slot: 1,
    parameters,
  });
  const active = projection.node(renderHandle(1));
  assert.equal(active?.kind, 'staticMesh');
  if (active?.kind === 'staticMesh') {
    assert.deepEqual(active.materialParameters, [{ slot: 1, parameters }]);
  }

  projection.applyDiff({
    op: 'setMaterialInstanceParameters',
    handle: renderHandle(1),
    slot: 1,
    parameters: null,
  });
  const reset = projection.node(renderHandle(1));
  assert.equal(reset?.kind, 'staticMesh');
  if (reset?.kind === 'staticMesh') {
    assert.deepEqual(reset.materialParameters, []);
  }
  assert.throws(
    () => projection.applyDiff({
      op: 'setMaterialInstanceParameters',
      handle: renderHandle(1),
      slot: 9,
      parameters,
    }),
    /unbound slot 9/,
  );
});

void test('tracks animated mesh definitions and command-selected named clip playback', () => {
  const projection = new RenderProjection();
  projection.applyDiff({ op: 'defineAnimatedMesh', asset: animatedMeshAsset() });
  projection.applyDiff({
    op: 'createAnimatedMeshInstance',
    handle: renderHandle(12),
    parent: null,
    instance: {
      asset: 'mesh-animation/kenney-retro-character-medium',
      transform: { translation: [0, 0, 0], rotation: [0, 0, 0, 1], scale: [1, 1, 1] },
      materialOverrides: [],
      playback: null,
      metadata: { source: entityId(12), tags: [], label: 'animated-proof' },
    },
  });

  assert.equal(projection.animatedMeshRefCount('mesh-animation/kenney-retro-character-medium'), 1);
  const instructions = projection.applyDiff({
    op: 'setAnimatedMeshPlayback',
    handle: renderHandle(12),
    playback: {
      action: 'play',
      clip: 'run',
      loop: 'repeat',
      speed: 1,
      weight: 1,
      restart: true,
      fadeSeconds: 0.1,
    },
  });

  assert.equal(instructions[0]?.op, 'upsertNode');
  const node = projection.node(renderHandle(12));
  assert.equal(node?.kind, 'animatedMesh');
  if (node?.kind === 'animatedMesh') {
    assert.equal(node.playback?.action, 'play');
    if (node.playback?.action === 'play') {
      assert.equal(node.playback.clip, 'run');
    }
  }

  assert.throws(
    () =>
      projection.applyDiff({
        op: 'setAnimatedMeshPlayback',
        handle: renderHandle(12),
        playback: {
          action: 'play',
          clip: 'dance',
          loop: 'repeat',
          speed: 1,
          weight: 1,
          restart: true,
          fadeSeconds: null,
        },
      }),
    RenderProjectionError,
  );

  assert.throws(
    () => projection.applyDiff({ op: 'defineAnimatedMesh', asset: animatedMeshAsset() }),
    RenderProjectionError,
  );
  projection.applyDiff({ op: 'destroy', handle: renderHandle(12) });
  assert.equal(projection.animatedMeshRefCount('mesh-animation/kenney-retro-character-medium'), 0);
});

void test('animated mesh render-diff fixture registers asset and starts the run clip', () => {
  const frame = JSON.parse(readFileSync(renderDiffFixturePath('animated-mesh.json'), 'utf8')) as RenderFrameDiff;
  const projection = new RenderProjection();
  projection.applyFrame(frame);

  const node = projection.node(renderHandle(41));
  assert.equal(node?.kind, 'animatedMesh');
  if (node?.kind === 'animatedMesh') {
    assert.equal(node.instance.asset, 'mesh-animation/kenney-retro-character-medium');
    assert.equal(node.playback?.action, 'play');
    if (node.playback?.action === 'play') {
      assert.equal(node.playback.clip, 'run');
    }
  }
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

void test('generated tunnel room frame is renderer-neutral and structurally stable', () => {
  const tunnel = tinyGeneratedTunnelReadout();
  const viewportFrame = createGeneratedTunnelViewportFrame(tunnel);
  const roomFrame = createGeneratedTunnelRoomFrame({
    tunnel,
    enemy: {
      label: 'generated-tunnel-enemy',
      position: [0, 1.1, -1.35],
      scale: [0.7, 1.8, 0.7],
    },
  });
  const signature = renderFrameSignature(roomFrame);

  assert.equal(viewportFrame.ops.length, 18);
  assert.equal(roomFrame.ops.length, 31);
  assert.deepEqual(signature.labels, [
    'generated-tunnel-floor',
    'generated-tunnel-ceiling',
    'generated-tunnel-wall-west',
    'generated-tunnel-wall-east',
    'generated-tunnel-entrance-cap',
    'generated-tunnel-exit-cap',
    'generated-tunnel-spawn-player_start',
    'generated-tunnel-spawn-exit_hint',
    'generated-tunnel-wall-rib-west-1',
    'generated-tunnel-wall-rib-east-1',
    'generated-tunnel-wall-rib-west-2',
    'generated-tunnel-wall-rib-east-2',
    'generated-tunnel-wall-rib-west-3',
    'generated-tunnel-wall-rib-east-3',
    'generated-tunnel-wall-rib-west-4',
    'generated-tunnel-wall-rib-east-4',
    'generated-tunnel-low-cover-west',
    'generated-tunnel-low-cover-east',
    'generated-tunnel-ceiling-crossbeam',
    'generated-tunnel-enemy',
    'generated-tunnel-centerline',
  ]);
  assert.equal(signature.hash, 'fnv1a64:cf70df6dccdf1758');
});

function renderFrameSignature(frame: RenderFrameDiff): {
  readonly hash: string;
  readonly labels: readonly string[];
} {
  const labels = frame.ops.flatMap((op) => {
    if (op.op === 'create') {
      return [op.node.metadata.label ?? ''];
    }
    if (op.op === 'createStaticMeshInstance') {
      return [op.instance.metadata.label ?? ''];
    }
    return [];
  });
  return {
    hash: stableHash({ opCount: frame.ops.length, labels }),
    labels,
  };
}

type StableHashPrimitive = string | number | boolean | null;
type StableHashValue = StableHashPrimitive | readonly StableHashValue[] | StableHashRecord;
interface StableHashRecord {
  readonly [key: string]: StableHashValue | undefined;
}

function stableHash(value: StableHashValue): string {
  return `fnv1a64:${fnv1a64(stableStringify(value))}`;
}

function stableStringify(value: StableHashValue | undefined): string {
  if (value === undefined) {
    return 'undefined';
  }
  if (value === null || typeof value !== 'object') {
    return JSON.stringify(value);
  }
  if (Array.isArray(value)) {
    const entries = value as readonly StableHashValue[];
    return `[${entries.map((entry) => stableStringify(entry)).join(',')}]`;
  }
  const record = value as StableHashRecord;
  return `{${Object.keys(record)
    .sort()
    .map((key) => `${JSON.stringify(key)}:${stableStringify(record[key])}`)
    .join(',')}}`;
}

function fnv1a64(text: string): string {
  let hash = 0xcbf29ce484222325n;
  const prime = 0x100000001b3n;
  const mask = 0xffffffffffffffffn;
  for (let index = 0; index < text.length; index += 1) {
    hash ^= BigInt(text.charCodeAt(index));
    hash = (hash * prime) & mask;
  }
  return hash.toString(16).padStart(16, '0');
}
