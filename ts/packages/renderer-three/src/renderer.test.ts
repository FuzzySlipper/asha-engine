// Runtime tests for the Three.js renderer shell, run with `node --test`.
// The scene graph is built without a GL context (no rendering), so these assert
// registry/scene-graph state directly.

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { renderHandle, entityId, type RenderDiff, type RenderNode } from '@asha/contracts';
import {
  RuntimeBridgeError,
  type RuntimeBufferHandle,
  type RuntimeBufferView,
} from '@asha/runtime-bridge';
import { ThreeRenderer, RenderApplyError, type MeshBufferSource } from './index.js';

function cubeNode(label = 'cube'): RenderNode {
  return {
    geometry: { shape: 'cube' },
    material: { color: [1, 1, 1, 1], wireframe: false },
    transform: { translation: [2, 0, 0], rotation: [0, 0, 0, 1], scale: [1, 1, 1] },
    visible: true,
    layer: 'scene',
    metadata: { source: null, tags: [], label },
  };
}

function createDiff(handle: number, node: RenderNode): RenderDiff {
  return { op: 'create', handle: renderHandle(handle), parent: null, node };
}

void test('create places a node in the scene layer with its transform', () => {
  const r = new ThreeRenderer();
  r.applyDiff(createDiff(1, cubeNode()));

  assert.equal(r.handleCount, 1);
  assert.ok(r.has(renderHandle(1)));
  const obj = r.objectFor(renderHandle(1))!;
  assert.equal(obj.position.x, 2);
  assert.equal(obj.parent?.name, 'scene');
  assert.equal(obj.name, 'cube');
});

void test('update mutates transform and visibility', () => {
  const r = new ThreeRenderer();
  r.applyDiff(createDiff(1, cubeNode()));
  r.applyDiff({
    op: 'update',
    handle: renderHandle(1),
    transform: { translation: [5, 1, 0], rotation: [0, 0, 0, 1], scale: [2, 2, 2] },
    material: null,
    visible: false,
    metadata: null,
  });

  const obj = r.objectFor(renderHandle(1))!;
  assert.equal(obj.position.x, 5);
  assert.equal(obj.scale.x, 2);
  assert.equal(obj.visible, false);
});

void test('destroy removes the node and frees the handle', () => {
  const r = new ThreeRenderer();
  r.applyDiff(createDiff(1, cubeNode()));
  r.applyDiff({ op: 'destroy', handle: renderHandle(1) });

  assert.equal(r.handleCount, 0);
  assert.ok(!r.has(renderHandle(1)));
});

void test('duplicate create and stale/unknown handles throw', () => {
  const r = new ThreeRenderer();
  r.applyDiff(createDiff(1, cubeNode()));

  assert.throws(() => r.applyDiff(createDiff(1, cubeNode())), RenderApplyError);
  assert.throws(
    () =>
      r.applyDiff({
        op: 'update',
        handle: renderHandle(99),
        transform: null,
        material: null,
        visible: null,
        metadata: null,
      }),
    RenderApplyError,
  );
  assert.throws(
    () => r.applyDiff({ op: 'destroy', handle: renderHandle(42) }),
    RenderApplyError,
  );
});

void test('debug-layer nodes land in the debug group', () => {
  const r = new ThreeRenderer();
  const node: RenderNode = {
    ...cubeNode('#1'),
    geometry: { shape: 'point' },
    layer: 'debug',
  };
  r.applyDiff(createDiff(1, node));
  assert.equal(r.objectFor(renderHandle(1))?.parent?.name, 'debug');
});

void test('applyEncodedFrame decodes through runtime-bridge and sequences create→update→destroy', () => {
  const fixture: unknown = JSON.parse(
    readFileSync(
      resolve(import.meta.dirname, '../../../../harness/fixtures/render-diffs/sample-frame.json'),
      'utf8',
    ),
  );
  const r = new ThreeRenderer();
  r.applyEncodedFrame(fixture);
  // The fixture creates handle 1, updates it, then destroys it.
  assert.equal(r.handleCount, 0);
});

void test('applies the Rust render-bridge fixture sequence end-to-end', () => {
  // Rust render bridge → fixture → runtime-bridge decode → renderer apply.
  // Frame 1 creates handles 1 & 2; frame 2 creates 3, updates 1, destroys 2.
  const frames = JSON.parse(
    readFileSync(
      resolve(import.meta.dirname, '../../../../harness/fixtures/render-diffs/bridge-sequence.json'),
      'utf8',
    ),
  ) as unknown[];

  const r = new ThreeRenderer();
  for (const frame of frames) {
    r.applyEncodedFrame(frame);
  }

  assert.equal(r.handleCount, 2);
  assert.ok(r.has(renderHandle(1)));
  assert.ok(r.has(renderHandle(3)));
  assert.ok(!r.has(renderHandle(2)));
  // The update carried the new tag onto handle 1's scene object metadata.
  assert.deepEqual(r.objectFor(renderHandle(1))?.userData['tags'], [5]);
});

// ── mesh payload upload (ADR 0007 / #2263) ────────────────────────────────────

import * as THREE from 'three';
import type { MeshPayloadDescriptor } from '@asha/contracts';

function meshNode(): RenderNode {
  return {
    geometry: { shape: 'cube' },
    material: { color: [1, 1, 1, 1], wireframe: false },
    transform: { translation: [0, 0, 0], rotation: [0, 0, 0, 1], scale: [1, 1, 1] },
    visible: true,
    layer: 'scene',
    metadata: { source: null, tags: [], label: 'chunk' },
  };
}

// A quad (4 verts, 6 indices) split into two material-slot groups.
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
    groups: [
      { materialSlot: 1, start: 0, count: 3 },
      { materialSlot: 2, start: 3, count: 3 },
    ],
    bounds: { min: [0, 0, 0], max: [1, 1, 0] },
    source: {
      kind: 'inline',
      positions: [0, 0, 0, 1, 0, 0, 1, 1, 0, 0, 1, 0],
      normals: [0, 0, 1, 0, 0, 1, 0, 0, 1, 0, 0, 1],
      indices: [0, 1, 2, 0, 2, 3],
    },
    provenance: 'voxelChunk',
  };
}

void test('replaceMeshPayload uploads a BufferGeometry with groups and material slots', () => {
  const r = new ThreeRenderer();
  const h = renderHandle(1);
  r.applyDiff({ op: 'create', handle: h, parent: null, node: meshNode() });
  r.applyDiff({ op: 'replaceMeshPayload', handle: h, payload: quadPayload() });

  const mesh = r.objectFor(h) as THREE.Mesh;
  const geo = mesh.geometry;
  assert.equal(geo.getAttribute('position').count, 4);
  assert.equal(geo.getAttribute('normal').count, 4);
  assert.equal(geo.getIndex()!.count, 6);
  assert.equal(geo.groups.length, 2);
  assert.deepEqual(
    geo.groups.map((g) => [g.start, g.count, g.materialIndex]),
    [[0, 3, 0], [3, 3, 1]],
  );
  // Two materials, one per group.
  assert.ok(Array.isArray(mesh.material));
  assert.equal((mesh.material as THREE.Material[]).length, 2);
});

void test('pickMesh traces an uploaded mesh handle back to its authority provenance (#2437)', () => {
  const r = new ThreeRenderer();
  const h = renderHandle(1);
  r.applyDiff({ op: 'create', handle: h, parent: null, node: meshNode() });
  // No uploaded mesh yet → no source trace (missing metadata fails closed).
  assert.equal(r.pickMesh(h), undefined);

  r.applyDiff({ op: 'replaceMeshPayload', handle: h, payload: quadPayload() });
  // Now it maps back to the authority source (the voxel chunk that produced it).
  assert.deepEqual(r.pickMesh(h), { handle: h, provenance: 'voxelChunk' });
});

void test('pickMesh fails closed on a stale/missing handle (no invented source)', () => {
  const r = new ThreeRenderer();
  const h = renderHandle(1);
  // Unknown handle → undefined.
  assert.equal(r.pickMesh(renderHandle(99)), undefined);
  // Destroyed handle → undefined (stale): the renderer never invents a source.
  r.applyDiff({ op: 'create', handle: h, parent: null, node: meshNode() });
  r.applyDiff({ op: 'replaceMeshPayload', handle: h, payload: quadPayload() });
  r.applyDiff({ op: 'destroy', handle: h });
  assert.equal(r.pickMesh(h), undefined);
});

void test('registered slot colour maps to the group material; unregistered uses a fallback', () => {
  const r = new ThreeRenderer();
  r.registerSlotColor(1, 1, 0, 0); // slot 1 → red
  const h = renderHandle(1);
  r.applyDiff({ op: 'create', handle: h, parent: null, node: meshNode() });
  r.applyDiff({ op: 'replaceMeshPayload', handle: h, payload: quadPayload() });

  const mats = (r.objectFor(h) as THREE.Mesh).material as THREE.MeshBasicMaterial[];
  assert.deepEqual([mats[0]!.color.r, mats[0]!.color.g, mats[0]!.color.b], [1, 0, 0]);
  // Slot 2 was never registered → a deterministic non-red fallback colour.
  assert.notDeepEqual([mats[1]!.color.r, mats[1]!.color.g, mats[1]!.color.b], [1, 0, 0]);
});

void test('replaceMeshPayload disposes the previous geometry and material', () => {
  const r = new ThreeRenderer();
  const h = renderHandle(1);
  r.applyDiff({ op: 'create', handle: h, parent: null, node: meshNode() });
  const mesh = r.objectFor(h) as THREE.Mesh;
  const oldGeo = mesh.geometry;
  let disposed = false;
  oldGeo.addEventListener('dispose', () => { disposed = true; });

  r.applyDiff({ op: 'replaceMeshPayload', handle: h, payload: quadPayload() });
  assert.ok(disposed, 'old geometry should be disposed on replace');
  assert.notEqual(mesh.geometry, oldGeo);

  // A second replace disposes the first uploaded geometry too.
  const firstUpload = mesh.geometry;
  let secondDisposed = false;
  firstUpload.addEventListener('dispose', () => { secondDisposed = true; });
  r.applyDiff({ op: 'replaceMeshPayload', handle: h, payload: quadPayload() });
  assert.ok(secondDisposed);
});

void test('replaceMeshPayload on an unknown handle throws', () => {
  const r = new ThreeRenderer();
  assert.throws(
    () => r.applyDiff({ op: 'replaceMeshPayload', handle: renderHandle(9), payload: quadPayload() }),
    RenderApplyError,
  );
});

// ── Handle-backed mesh payloads (#2382) ──────────────────────────────────────

/** Pack the quad's inline streams into one `[positions|normals|indices]` blob. */
function quadHandleBytes(): Uint8Array {
  const positions = [0, 0, 0, 1, 0, 0, 1, 1, 0, 0, 1, 0];
  const normals = [0, 0, 1, 0, 0, 1, 0, 0, 1, 0, 0, 1];
  const indices = [0, 1, 2, 0, 2, 3];
  const bytes = new Uint8Array((positions.length + normals.length + indices.length) * 4);
  const dv = new DataView(bytes.buffer);
  let offset = 0;
  for (const v of positions) {
    dv.setFloat32(offset, v, true);
    offset += 4;
  }
  for (const v of normals) {
    dv.setFloat32(offset, v, true);
    offset += 4;
  }
  for (const v of indices) {
    dv.setUint32(offset, v, true);
    offset += 4;
  }
  return bytes;
}

/** The quad payload addressed by a buffer handle instead of inline arrays. */
function quadHandlePayload(buffer: number): MeshPayloadDescriptor {
  return {
    ...quadPayload(),
    source: {
      kind: 'handle',
      buffer,
      positionsByteOffset: 0,
      normalsByteOffset: 48,
      indicesByteOffset: 96,
    },
  };
}

/** A minimal in-memory mesh buffer source mirroring the runtime bridge contract,
 *  recording borrow/release calls so tests can assert the lifetime semantics. */
class MapBufferSource implements MeshBufferSource {
  readonly #buffers = new Map<number, Uint8Array>();
  #expired = new Set<number>();
  #failRelease = new Set<number>();
  /** Handles passed to getBuffer / releaseBuffer, in call order. */
  readonly borrowed: number[] = [];
  readonly released: number[] = [];

  set(handle: number, bytes: Uint8Array): void {
    this.#buffers.set(handle, bytes);
  }

  expire(handle: number): void {
    this.#expired.add(handle);
  }

  failReleaseOf(handle: number): void {
    this.#failRelease.add(handle);
  }

  /** Borrows minus releases — must return to zero after every upload. */
  get outstanding(): number {
    return this.borrowed.length - this.released.length;
  }

  getBuffer(handle: RuntimeBufferHandle): RuntimeBufferView {
    const raw = handle as number;
    if (this.#expired.has(raw)) {
      throw new RuntimeBridgeError('buffer_expired', `buffer ${raw} expired`);
    }
    const bytes = this.#buffers.get(raw);
    if (bytes === undefined) {
      throw new RuntimeBridgeError('unknown_handle', `no buffer for handle ${raw}`);
    }
    this.borrowed.push(raw);
    return { handle, bytes };
  }

  releaseBuffer(handle: RuntimeBufferHandle): void {
    const raw = handle as number;
    this.released.push(raw);
    if (this.#failRelease.has(raw)) {
      throw new RuntimeBridgeError('unknown_handle', `release: no buffer for handle ${raw}`);
    }
  }
}

void test('inline and handle-backed sources produce equivalent geometry', () => {
  const inlineRenderer = new ThreeRenderer();
  const hi = renderHandle(1);
  inlineRenderer.applyDiff({ op: 'create', handle: hi, parent: null, node: meshNode() });
  inlineRenderer.applyDiff({ op: 'replaceMeshPayload', handle: hi, payload: quadPayload() });
  const inlineGeo = (inlineRenderer.objectFor(hi) as THREE.Mesh).geometry;

  const source = new MapBufferSource();
  source.set(7, quadHandleBytes());
  const handleRenderer = new ThreeRenderer({ meshBufferSource: source });
  const hh = renderHandle(1);
  handleRenderer.applyDiff({ op: 'create', handle: hh, parent: null, node: meshNode() });
  handleRenderer.applyDiff({ op: 'replaceMeshPayload', handle: hh, payload: quadHandlePayload(7) });
  const handleGeo = (handleRenderer.objectFor(hh) as THREE.Mesh).geometry;

  assert.deepEqual(
    Array.from(handleGeo.getAttribute('position').array),
    Array.from(inlineGeo.getAttribute('position').array),
  );
  assert.deepEqual(
    Array.from(handleGeo.getAttribute('normal').array),
    Array.from(inlineGeo.getAttribute('normal').array),
  );
  assert.deepEqual(Array.from(handleGeo.getIndex()!.array), Array.from(inlineGeo.getIndex()!.array));
  assert.deepEqual(
    handleGeo.groups.map((g) => [g.start, g.count, g.materialIndex]),
    inlineGeo.groups.map((g) => [g.start, g.count, g.materialIndex]),
  );
});

void test('handle source with no buffer provider fails closed', () => {
  const r = new ThreeRenderer();
  const h = renderHandle(1);
  r.applyDiff({ op: 'create', handle: h, parent: null, node: meshNode() });
  assert.throws(
    () => r.applyDiff({ op: 'replaceMeshPayload', handle: h, payload: quadHandlePayload(7) }),
    RenderApplyError,
  );
});

void test('unknown and stale buffer handles produce a classified error, not an empty mesh', () => {
  const source = new MapBufferSource();
  const r = new ThreeRenderer({ meshBufferSource: source });
  const h = renderHandle(1);
  r.applyDiff({ op: 'create', handle: h, parent: null, node: meshNode() });

  // Unknown handle (provider has no buffer 7).
  assert.throws(
    () => r.applyDiff({ op: 'replaceMeshPayload', handle: h, payload: quadHandlePayload(7) }),
    /unavailable \[unknown_handle\]/,
  );

  // Stale handle (provider reports the buffer expired).
  source.set(7, quadHandleBytes());
  source.expire(7);
  assert.throws(
    () => r.applyDiff({ op: 'replaceMeshPayload', handle: h, payload: quadHandlePayload(7) }),
    /unavailable \[buffer_expired\]/,
  );
});

void test('a buffer too small for the declared layout fails closed', () => {
  const source = new MapBufferSource();
  source.set(7, quadHandleBytes().slice(0, 64)); // truncated: not enough for normals+indices
  const r = new ThreeRenderer({ meshBufferSource: source });
  const h = renderHandle(1);
  r.applyDiff({ op: 'create', handle: h, parent: null, node: meshNode() });
  assert.throws(
    () => r.applyDiff({ op: 'replaceMeshPayload', handle: h, payload: quadHandlePayload(7) }),
    /exceeds buffer/,
  );
});

void test('replaceMeshPayload releases the borrow on success (borrow → copy → release)', () => {
  const source = new MapBufferSource();
  source.set(7, quadHandleBytes());
  const r = new ThreeRenderer({ meshBufferSource: source });
  const h = renderHandle(1);
  r.applyDiff({ op: 'create', handle: h, parent: null, node: meshNode() });
  r.applyDiff({ op: 'replaceMeshPayload', handle: h, payload: quadHandlePayload(7) });
  assert.deepEqual(source.borrowed, [7]);
  assert.deepEqual(source.released, [7]);
  assert.equal(source.outstanding, 0, 'no borrow is retained past the upload');
});

// ── Handle-backed static mesh ASSETS (#2428) ──────────────────────────────────

/** A `mesh/crate` static mesh asset whose payload is addressed by a buffer handle. */
function handleCrateAsset(buffer: number): StaticMeshAsset {
  return { ...crateAsset(), payload: { ...quadHandlePayload(buffer), provenance: 'staticAsset' } };
}

void test('defineStaticMesh consumes a handle-backed payload and releases the borrow', () => {
  const source = new MapBufferSource();
  source.set(7, quadHandleBytes());
  const r = new ThreeRenderer({ meshBufferSource: source });

  r.applyDiff({ op: 'defineStaticMesh', asset: handleCrateAsset(7) });
  r.applyDiff({
    op: 'createStaticMeshInstance',
    handle: renderHandle(1),
    parent: null,
    instance: crateInstance(),
  });

  // Borrow was released; nothing retained.
  assert.deepEqual(source.released, [7]);
  assert.equal(source.outstanding, 0);

  // The handle-backed asset produced the same geometry as the inline path.
  const inline = new ThreeRenderer();
  inline.applyDiff({ op: 'defineStaticMesh', asset: crateAsset() });
  inline.applyDiff({
    op: 'createStaticMeshInstance',
    handle: renderHandle(1),
    parent: null,
    instance: crateInstance(),
  });
  const handleGeo = (r.objectFor(renderHandle(1)) as THREE.Mesh).geometry;
  const inlineGeo = (inline.objectFor(renderHandle(1)) as THREE.Mesh).geometry;
  assert.deepEqual(
    Array.from(handleGeo.getAttribute('position').array),
    Array.from(inlineGeo.getAttribute('position').array),
  );
  assert.deepEqual(Array.from(handleGeo.getIndex()!.array), Array.from(inlineGeo.getIndex()!.array));
});

void test('defineStaticMesh with a handle payload but no provider fails closed', () => {
  const r = new ThreeRenderer(); // no buffer source
  assert.throws(
    () => r.applyDiff({ op: 'defineStaticMesh', asset: handleCrateAsset(7) }),
    /defineStaticMesh: handle-source payload needs a runtime buffer provider/,
  );
  // The asset was not defined (no empty geometry left behind).
  assert.throws(
    () =>
      r.applyDiff({
        op: 'createStaticMeshInstance',
        handle: renderHandle(1),
        parent: null,
        instance: crateInstance(),
      }),
    /undefined static mesh asset/,
  );
});

void test('defineStaticMesh with an unknown handle fails closed without leaking a borrow', () => {
  const source = new MapBufferSource(); // buffer 7 never set
  const r = new ThreeRenderer({ meshBufferSource: source });
  assert.throws(
    () => r.applyDiff({ op: 'defineStaticMesh', asset: handleCrateAsset(7) }),
    /defineStaticMesh: buffer 7 unavailable \[unknown_handle\]/,
  );
  assert.equal(source.outstanding, 0, 'getBuffer threw, so no borrow to release');
  assert.deepEqual(source.released, []);
});

void test('defineStaticMesh releases the borrow even when the copy fails (too small)', () => {
  const source = new MapBufferSource();
  source.set(7, quadHandleBytes().slice(0, 64)); // truncated
  const r = new ThreeRenderer({ meshBufferSource: source });
  assert.throws(
    () => r.applyDiff({ op: 'defineStaticMesh', asset: handleCrateAsset(7) }),
    /defineStaticMesh: .* exceeds buffer/,
  );
  // Borrow acquired then released on the failure path — no leak.
  assert.deepEqual(source.borrowed, [7]);
  assert.deepEqual(source.released, [7]);
  assert.equal(source.outstanding, 0);
});

void test('a release failure on the success path is classified, not swallowed', () => {
  const source = new MapBufferSource();
  source.set(7, quadHandleBytes());
  source.failReleaseOf(7);
  const r = new ThreeRenderer({ meshBufferSource: source });
  assert.throws(
    () => r.applyDiff({ op: 'defineStaticMesh', asset: handleCrateAsset(7) }),
    /defineStaticMesh: buffer 7 release failed \[unknown_handle\]/,
  );
});

// ── static mesh assets + instances (render-asset-04 / #2327) ──────────────────

import type {
  StaticMeshAsset,
  StaticMeshInstanceDescriptor,
  SpriteInstanceDescriptor,
} from '@asha/contracts';

function crateAsset(): StaticMeshAsset {
  return {
    asset: 'mesh/crate',
    payload: { ...quadPayload(), provenance: 'staticAsset' },
    materialSlots: [{ slot: 1, material: 'material/wood' }, { slot: 2, material: 'material/iron' }],
    collision: { kind: 'aabbFallback' },
  };
}

function crateInstance(
  asset = 'mesh/crate',
  overrides: StaticMeshInstanceDescriptor['materialOverrides'] = [],
): StaticMeshInstanceDescriptor {
  return {
    asset,
    transform: { translation: [0, 0, 0], rotation: [0, 0, 0, 1], scale: [1, 1, 1] },
    materialOverrides: overrides,
    metadata: { source: null, tags: [], label: asset },
  };
}

void test('two instances share one BufferGeometry and the asset is reference-counted', () => {
  const r = new ThreeRenderer();
  r.applyDiff({ op: 'defineStaticMesh', asset: crateAsset() });
  r.applyDiff({ op: 'createStaticMeshInstance', handle: renderHandle(1), parent: null, instance: crateInstance() });
  r.applyDiff({ op: 'createStaticMeshInstance', handle: renderHandle(2), parent: null, instance: crateInstance() });

  const a = r.objectFor(renderHandle(1)) as THREE.Mesh;
  const b = r.objectFor(renderHandle(2)) as THREE.Mesh;
  assert.equal(a.geometry, b.geometry, 'instances must share one geometry');
  assert.equal(r.instanceCountFor('mesh/crate'), 2);
});

void test('destroying one instance does not dispose geometry still used by another', () => {
  const r = new ThreeRenderer();
  r.applyDiff({ op: 'defineStaticMesh', asset: crateAsset() });
  r.applyDiff({ op: 'createStaticMeshInstance', handle: renderHandle(1), parent: null, instance: crateInstance() });
  r.applyDiff({ op: 'createStaticMeshInstance', handle: renderHandle(2), parent: null, instance: crateInstance() });

  const shared = (r.objectFor(renderHandle(1)) as THREE.Mesh).geometry;
  let disposed = false;
  shared.addEventListener('dispose', () => { disposed = true; });

  r.applyDiff({ op: 'destroy', handle: renderHandle(1) });
  assert.equal(disposed, false, 'shared geometry must survive while an instance remains');
  assert.equal(r.instanceCountFor('mesh/crate'), 1);

  r.applyDiff({ op: 'destroy', handle: renderHandle(2) });
  assert.ok(disposed, 'shared geometry is disposed when the last instance is gone');
  assert.equal(r.instanceCountFor('mesh/crate'), 0);
});

void test('per-instance material overrides apply only to that instance', () => {
  const r = new ThreeRenderer();
  r.registerSlotColor(1, 0, 0, 1); // base slot 1 → blue
  r.applyDiff({ op: 'defineStaticMesh', asset: crateAsset() });
  r.applyDiff({ op: 'createStaticMeshInstance', handle: renderHandle(1), parent: null, instance: crateInstance() });
  r.applyDiff({
    op: 'createStaticMeshInstance',
    handle: renderHandle(2),
    parent: null,
    instance: crateInstance('mesh/crate', [{ slot: 1, material: 'material/wood-red' }]),
  });

  const base = (r.objectFor(renderHandle(1)) as THREE.Mesh).material as THREE.MeshBasicMaterial[];
  const overridden = (r.objectFor(renderHandle(2)) as THREE.Mesh).material as THREE.MeshBasicMaterial[];
  // Slot-2 material is shared (identical object); slot-1 override is a distinct material instance.
  assert.equal(base[1], overridden[1], 'non-overridden slot material is shared');
  assert.notEqual(base[0], overridden[0], 'overridden slot gets its own material');
});

// ── catalog material descriptors (material-wiring super, #2373) ────────────────

import type { RenderMaterialDescriptor } from '@asha/contracts';

function woodMaterial(): RenderMaterialDescriptor {
  return {
    id: 'material/wood',
    color: [0.6, 0.4, 0.2, 1],
    texture: null,
    roughness: 1,
    emissive: 0,
    uvStrategy: 'flat',
  };
}

void test('defineMaterial maps a static-mesh slot to its catalog colour, not a placeholder', () => {
  const r = new ThreeRenderer();
  r.applyDiff({ op: 'defineMaterial', material: woodMaterial() });
  assert.deepEqual(r.materialDescriptor('material/wood')?.color, [0.6, 0.4, 0.2, 1]);

  // Define a single-slot mesh bound to material/wood, then instance it.
  r.applyDiff({
    op: 'defineStaticMesh',
    asset: {
      asset: 'mesh/plank',
      payload: { ...quadPayload(), provenance: 'staticAsset' },
      materialSlots: [{ slot: 0, material: 'material/wood' }],
      collision: { kind: 'visualOnly' },
    },
  });
  r.applyDiff({
    op: 'createStaticMeshInstance',
    handle: renderHandle(1),
    parent: null,
    instance: crateInstance('mesh/plank'),
  });

  const mat = (r.objectFor(renderHandle(1)) as THREE.Mesh).material as THREE.MeshBasicMaterial;
  // The catalog wood colour (0.6,0.4,0.2), not the deterministic per-slot hue.
  assert.ok(Math.abs(mat.color.r - 0.6) < 1e-6 && Math.abs(mat.color.b - 0.2) < 1e-6);
  assert.equal(r.fallbackMaterialCount, 0, 'a defined material is not a fallback');
});

void test('a slot with no catalog descriptor falls back deterministically and is counted', () => {
  const r = new ThreeRenderer();
  r.applyDiff({ op: 'defineStaticMesh', asset: crateAsset() }); // two slots, no descriptors
  r.applyDiff({
    op: 'createStaticMeshInstance',
    handle: renderHandle(1),
    parent: null,
    instance: crateInstance(),
  });
  assert.equal(r.fallbackMaterialCount, 2, 'both unresolved slots count as fallbacks');
});

void test('two voxel materials project to distinct catalog render descriptors (#2375)', () => {
  // The fixture is generated by render-bridge's project_voxel_materials from a
  // VoxelMaterialTable + catalog (compact u16 ids → catalog material assets).
  const fixture: unknown = JSON.parse(
    readFileSync(
      resolve(import.meta.dirname, '../../../../harness/fixtures/render-diffs/voxel-materials.json'),
      'utf8',
    ),
  );
  const r = new ThreeRenderer();
  r.applyEncodedFrame(fixture);
  const stone = r.materialDescriptor('material/stone');
  const dirt = r.materialDescriptor('material/dirt');
  assert.ok(stone && dirt, 'both voxel materials register a descriptor');
  assert.notDeepEqual(stone!.color, dirt!.color, 'distinct catalog styles');
  // Visual projection only — the descriptor has no collision field.
  assert.ok(!('structuralClass' in stone!) && !('collidable' in stone!));
});

// ── material update lifecycle + fallback diagnostics (#2376) ───────────────────

void test('redefining a material live-replaces instance materials and disposes the old', () => {
  const r = new ThreeRenderer();
  r.applyDiff({ op: 'defineMaterial', material: woodMaterial() });
  r.applyDiff({
    op: 'defineStaticMesh',
    asset: {
      asset: 'mesh/plank',
      payload: { ...quadPayload(), provenance: 'staticAsset' },
      materialSlots: [{ slot: 0, material: 'material/wood' }],
      collision: { kind: 'visualOnly' },
    },
  });
  r.applyDiff({
    op: 'createStaticMeshInstance',
    handle: renderHandle(1),
    parent: null,
    instance: crateInstance('mesh/plank'),
  });

  const before = (r.objectFor(renderHandle(1)) as THREE.Mesh).material as THREE.MeshBasicMaterial;
  assert.ok(Math.abs(before.color.r - 0.6) < 1e-6);
  let disposed = false;
  before.addEventListener('dispose', () => {
    disposed = true;
  });

  // A visual-only redefine (new colour) applies live, deterministically.
  r.applyDiff({
    op: 'defineMaterial',
    material: { ...woodMaterial(), color: [0.1, 0.8, 0.2, 1] },
  });
  const after = (r.objectFor(renderHandle(1)) as THREE.Mesh).material as THREE.MeshBasicMaterial;
  assert.ok(Math.abs(after.color.g - 0.8) < 1e-6, 'rendered colour updated live');
  assert.ok(disposed, 'the old material was disposed (leak-safe)');
});

void test('fallback material use is visible in diagnostics with the material id', () => {
  const r = new ThreeRenderer();
  // No defineMaterial for the crate's slots → both fall back.
  r.applyDiff({ op: 'defineStaticMesh', asset: crateAsset() });
  r.applyDiff({
    op: 'createStaticMeshInstance',
    handle: renderHandle(1),
    parent: null,
    instance: crateInstance(),
  });
  assert.deepEqual(r.fallbackMaterials(), ['material/iron', 'material/wood']);
  assert.equal(r.fallbackMaterialCount, 2);
});

// ── textures + sprite atlases (material-wiring super, #2374) ───────────────────

import type { SpriteAtlasDescriptor, TextureDescriptor } from '@asha/contracts';

function sparkTexture(): TextureDescriptor {
  return {
    id: 'texture/spark',
    width: 64,
    height: 32,
    filter: 'nearest',
    wrap: 'clamp',
    contentHash: null,
    version: 1,
  };
}

function sparkAtlas(): SpriteAtlasDescriptor {
  return {
    id: 'sprite/spark-sheet',
    texture: 'texture/spark',
    frames: [
      { frame: 0, uvMin: [0, 0], uvMax: [0.5, 1] },
      { frame: 3, uvMin: [0.5, 0], uvMax: [1, 1] },
    ],
  };
}

function atlasSprite(frame = 0): SpriteInstanceDescriptor {
  return {
    asset: 'sprite/spark-sheet',
    frame,
    pivot: [0.5, 0.5],
    size: [1, 1],
    sizeMode: 'world',
    billboard: 'spherical',
    tint: [1, 1, 1, 1],
    renderOrder: 0,
    depth: 'default',
    shading: 'unlit',
    transform: { translation: [0, 0, 0], rotation: [0, 0, 0, 1], scale: [1, 1, 1] },
    attachment: { sourceEntity: null, sourceSceneNode: 10, attachmentPoint: null },
    metadata: { source: null, tags: [], label: 'spark' },
  };
}

function spriteUv(r: ThreeRenderer, handle: number): number[] {
  return (r.objectFor(renderHandle(handle))!.userData['uv'] as number[]).map((x) => Number(x.toFixed(4)));
}

void test('a sprite frame maps to its atlas UV sub-rectangle deterministically', () => {
  const r = new ThreeRenderer();
  r.applyDiff({ op: 'defineTexture', texture: sparkTexture() });
  r.applyDiff({ op: 'defineSpriteAtlas', atlas: sparkAtlas() });
  assert.equal(r.textureDescriptor('texture/spark')?.width, 64);
  assert.equal(r.spriteAtlas('sprite/spark-sheet')?.frames.length, 2);

  r.applyDiff({ op: 'createSprite', handle: renderHandle(1), parent: null, sprite: atlasSprite(0) });
  assert.deepEqual(spriteUv(r, 1), [0, 0, 0.5, 1], 'frame 0 → left half');

  // Advancing the frame re-resolves the UV rect deterministically.
  r.applyDiff({ op: 'updateSprite', handle: renderHandle(1), frame: 3, tint: null, renderOrder: null, visible: null });
  assert.deepEqual(spriteUv(r, 1), [0.5, 0, 1, 1], 'frame 3 → right half');
  assert.equal(r.spriteFallbackCount, 0, 'known frames are not fallbacks');
});

void test('a sprite frame with no atlas frame falls back to full UVs and is counted', () => {
  const r = new ThreeRenderer();
  r.applyDiff({ op: 'defineTexture', texture: sparkTexture() });
  r.applyDiff({ op: 'defineSpriteAtlas', atlas: sparkAtlas() });
  r.applyDiff({ op: 'createSprite', handle: renderHandle(1), parent: null, sprite: atlasSprite(9) });
  assert.deepEqual(spriteUv(r, 1), [0, 0, 1, 1], 'unknown frame → full UVs');
  assert.equal(r.spriteFallbackCount, 1);
});

void test('instance of an undefined asset, and redefine while in use, are classified errors', () => {
  const r = new ThreeRenderer();
  assert.throws(
    () => r.applyDiff({ op: 'createStaticMeshInstance', handle: renderHandle(1), parent: null, instance: crateInstance() }),
    RenderApplyError,
  );
  r.applyDiff({ op: 'defineStaticMesh', asset: crateAsset() });
  r.applyDiff({ op: 'createStaticMeshInstance', handle: renderHandle(1), parent: null, instance: crateInstance() });
  assert.throws(() => r.applyDiff({ op: 'defineStaticMesh', asset: crateAsset() }), RenderApplyError);
});

// ── sprites / billboards + picking (render-asset-05/06 / #2328-2329) ──────────

function sparkSprite(over: Partial<SpriteInstanceDescriptor> = {}): SpriteInstanceDescriptor {
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
    attachment: { sourceEntity: null, sourceSceneNode: null, attachmentPoint: null },
    metadata: { source: null, tags: [], label: 'spark' },
    ...over,
  };
}

void test('createSprite builds a plane geometry (not THREE.Sprite) with render order + depth policy', () => {
  const r = new ThreeRenderer();
  r.applyDiff({ op: 'createSprite', handle: renderHandle(1), parent: null, sprite: sparkSprite({ renderOrder: 7, depth: 'depthTestOff' }) });
  const mesh = r.objectFor(renderHandle(1)) as THREE.Mesh;
  assert.ok(mesh instanceof THREE.Mesh, 'sprite uses a Mesh + PlaneGeometry, not THREE.Sprite');
  assert.ok(mesh.geometry instanceof THREE.PlaneGeometry);
  assert.equal(mesh.renderOrder, 7);
  assert.equal((mesh.material as THREE.MeshBasicMaterial).depthTest, false);
});

void test('sprite frame/tint updates are deterministic and projection-driven', () => {
  const r = new ThreeRenderer();
  r.applyDiff({ op: 'createSprite', handle: renderHandle(1), parent: null, sprite: sparkSprite() });
  r.applyDiff({ op: 'updateSprite', handle: renderHandle(1), frame: 4, tint: [1, 0, 0, 1], renderOrder: 2, visible: false });

  const mesh = r.objectFor(renderHandle(1)) as THREE.Mesh;
  assert.equal(mesh.userData['frame'], 4);
  assert.equal(mesh.renderOrder, 2);
  assert.equal(mesh.visible, false);
  const c = (mesh.material as THREE.MeshBasicMaterial).color;
  assert.deepEqual([c.r, c.g, c.b], [1, 0, 0]);
});

void test('reserved lit/shadow shading is accepted (renderer does not force unlit-only)', () => {
  const r = new ThreeRenderer();
  assert.doesNotThrow(() =>
    r.applyDiff({ op: 'createSprite', handle: renderHandle(1), parent: null, sprite: sparkSprite({ shading: 'lit' }) }),
  );
});

void test('pickSprite traces to source entity / scene node / asset, never a render handle as authority', () => {
  const r = new ThreeRenderer();
  r.applyDiff({
    op: 'createSprite',
    handle: renderHandle(5),
    parent: null,
    sprite: sparkSprite({ attachment: { sourceEntity: entityId(42), sourceSceneNode: 9, attachmentPoint: 'muzzle' } }),
  });
  const hit = r.pickSprite(renderHandle(5));
  assert.ok(hit);
  assert.equal(hit!.handle, renderHandle(5));
  assert.equal(hit!.sourceEntity, 42);
  assert.equal(hit!.sourceSceneNode, 9);
  assert.equal(hit!.asset, 'sprite/spark');
  assert.equal(hit!.attachmentPoint, 'muzzle');
  // A non-sprite handle yields no pick hit.
  assert.equal(r.pickSprite(renderHandle(99)), undefined);
});

// ── Large-payload lifecycle conformance + resource-leak (#2383) ──────────────

/** Generate a non-trivially large triangle-strip mesh's inline streams. */
function bigMeshStreams(vertexCount: number): {
  positions: number[];
  normals: number[];
  indices: number[];
} {
  const positions: number[] = [];
  const normals: number[] = [];
  for (let i = 0; i < vertexCount; i++) {
    positions.push(i, i * 0.5, 0);
    normals.push(0, 0, 1);
  }
  const indices: number[] = [];
  for (let i = 0; i + 2 < vertexCount; i++) {
    indices.push(i, i + 1, i + 2);
  }
  return { positions, normals, indices };
}

/** Pack inline streams into one `[positions|normals|indices]` little-endian blob. */
function packStreams(streams: {
  positions: number[];
  normals: number[];
  indices: number[];
}): Uint8Array {
  const { positions, normals, indices } = streams;
  const bytes = new Uint8Array((positions.length + normals.length + indices.length) * 4);
  const dv = new DataView(bytes.buffer);
  let offset = 0;
  for (const v of positions) {
    dv.setFloat32(offset, v, true);
    offset += 4;
  }
  for (const v of normals) {
    dv.setFloat32(offset, v, true);
    offset += 4;
  }
  for (const v of indices) {
    dv.setUint32(offset, v, true);
    offset += 4;
  }
  return bytes;
}

/** A handle-backed payload sized for arbitrary vertex/index counts. */
function bigMeshPayload(buffer: number, vertexCount: number, indexCount: number): MeshPayloadDescriptor {
  return {
    layout: {
      vertexCount,
      indexCount,
      indexWidth: 'u32',
      attributes: [
        { name: 'position', components: 3, kind: 'f32' },
        { name: 'normal', components: 3, kind: 'f32' },
      ],
    },
    groups: [{ materialSlot: 1, start: 0, count: indexCount }],
    bounds: { min: [0, 0, 0], max: [vertexCount, vertexCount, 0] },
    source: {
      kind: 'handle',
      buffer,
      positionsByteOffset: 0,
      normalsByteOffset: vertexCount * 3 * 4,
      indicesByteOffset: vertexCount * 3 * 4 * 2,
    },
    provenance: 'voxelChunk',
  };
}

void test('large handle-backed payload uploads with the declared counts', () => {
  const vertexCount = 4096;
  const streams = bigMeshStreams(vertexCount);
  const source = new MapBufferSource();
  source.set(10, packStreams(streams));
  const r = new ThreeRenderer({ meshBufferSource: source });
  const h = renderHandle(1);
  r.applyDiff({ op: 'create', handle: h, parent: null, node: meshNode() });
  r.applyDiff({
    op: 'replaceMeshPayload',
    handle: h,
    payload: bigMeshPayload(10, vertexCount, streams.indices.length),
  });
  const geo = (r.objectFor(h) as THREE.Mesh).geometry;
  assert.equal(geo.getAttribute('position').count, vertexCount);
  assert.equal(geo.getIndex()!.count, streams.indices.length);
});

void test('create/replace/destroy/invalidate cycle leaves no leaked geometry and stable diagnostics', () => {
  const source = new MapBufferSource();
  source.set(1, quadHandleBytes());
  source.set(2, quadHandleBytes());
  source.set(3, quadHandleBytes());
  const r = new ThreeRenderer({ meshBufferSource: source });

  // Upload three handle-backed meshes.
  const handles = [renderHandle(1), renderHandle(2), renderHandle(3)];
  for (const h of handles) {
    r.applyDiff({ op: 'create', handle: h, parent: null, node: meshNode() });
    r.applyDiff({ op: 'replaceMeshPayload', handle: h, payload: quadHandlePayload((h as number)) });
  }
  for (const h of handles) {
    assert.ok(r.has(h));
  }

  // Replace one: its previous uploaded geometry must be disposed (no leak).
  const replaced = (r.objectFor(handles[0]!) as THREE.Mesh).geometry;
  let replacedDisposed = false;
  replaced.addEventListener('dispose', () => {
    replacedDisposed = true;
  });
  r.applyDiff({ op: 'replaceMeshPayload', handle: handles[0]!, payload: quadHandlePayload(1) });
  assert.ok(replacedDisposed, 'replaced geometry should be disposed');

  // Destroy one: handle freed and its geometry disposed.
  const destroyed = (r.objectFor(handles[1]!) as THREE.Mesh).geometry;
  let destroyedDisposed = false;
  destroyed.addEventListener('dispose', () => {
    destroyedDisposed = true;
  });
  r.applyDiff({ op: 'destroy', handle: handles[1]! });
  assert.ok(!r.has(handles[1]!));
  assert.ok(destroyedDisposed, 'destroyed geometry should be disposed');

  // Invalidate the third buffer in the provider: a re-upload referencing it fails
  // closed with a stable, source-linked diagnostic, and the node keeps its prior
  // geometry (no partial mutation).
  const survivor = (r.objectFor(handles[2]!) as THREE.Mesh).geometry;
  source.expire(3);
  assert.throws(
    () => r.applyDiff({ op: 'replaceMeshPayload', handle: handles[2]!, payload: quadHandlePayload(3) }),
    /buffer 3 unavailable \[buffer_expired\]/,
  );
  assert.equal((r.objectFor(handles[2]!) as THREE.Mesh).geometry, survivor, 'failed upload must not swap geometry');

  // Final state: handles 1 and 3 survive, handle 2 destroyed.
  assert.ok(r.has(handles[0]!));
  assert.ok(!r.has(handles[1]!));
  assert.ok(r.has(handles[2]!));
});
