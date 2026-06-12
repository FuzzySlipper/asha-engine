// @asha/runtime-bridge / render-decode — decodes retained render-diff payloads
// into generated contract types for renderer consumption (ADR 0006).
//
// Moved here from the former `@asha/wasm-bridge`: decoding a render-diff payload
// into validated `@asha/contracts` `RenderFrameDiff` values is transport-neutral
// and belongs behind the runtime facade (it backs `readRenderDiffs`), reused by
// the native and WASM-replay paths alike. It applies nothing to a scene and
// imports no policy, renderer, UI, or Electron package. `FrameMemory` is the
// borrowed-bytes lifetime primitive for large payloads (cf. the facade
// `getBuffer`/`releaseBuffer` buffer handles).

import {
  renderHandle,
  entityId,
  tagId,
  type RenderFrameDiff,
  type RenderDiff,
  type RenderNode,
  type Geometry,
  type Material,
  type Transform,
  type RenderMetadata,
  type RenderLayer,
  type RenderHandle,
  type MeshAttribute,
  type MeshBufferLayout,
  type MeshGroupDescriptor,
  type MeshBoundsDescriptor,
  type MeshPayloadSource,
  type MeshPayloadDescriptor,
  type MeshProvenance,
  type MeshMaterialSlot,
  type MeshCollisionPolicy,
  type StaticMeshAsset,
  type StaticMeshInstanceDescriptor,
  type SpriteSizeMode,
  type BillboardMode,
  type SpriteDepthPolicy,
  type SpriteShading,
  type SpriteAttachment,
  type SpriteInstanceDescriptor,
} from '@asha/contracts';

/** Raised when a payload does not match the render-diff contract shape. */
export class RenderDecodeError extends Error {
  constructor(message: string, readonly path: string) {
    super(`render decode error at ${path}: ${message}`);
    this.name = 'RenderDecodeError';
  }
}

// ── Primitive validators ──────────────────────────────────────────────────────

function asObject(v: unknown, path: string): Record<string, unknown> {
  if (typeof v !== 'object' || v === null || Array.isArray(v)) {
    throw new RenderDecodeError('expected an object', path);
  }
  return v as Record<string, unknown>;
}

function asNumber(v: unknown, path: string): number {
  if (typeof v !== 'number' || !Number.isFinite(v)) {
    throw new RenderDecodeError('expected a finite number', path);
  }
  return v;
}

function asBoolean(v: unknown, path: string): boolean {
  if (typeof v !== 'boolean') {
    throw new RenderDecodeError('expected a boolean', path);
  }
  return v;
}

function asArray(v: unknown, path: string): readonly unknown[] {
  if (!Array.isArray(v)) {
    throw new RenderDecodeError('expected an array', path);
  }
  return v;
}

function asNumberArray(v: unknown, len: number, path: string): number[] {
  const arr = asArray(v, path);
  if (arr.length !== len) {
    throw new RenderDecodeError(`expected ${len} numbers, got ${arr.length}`, path);
  }
  return arr.map((x, i) => asNumber(x, `${path}[${i}]`));
}

function tuple3(v: unknown, path: string): [number, number, number] {
  const [a, b, c] = asNumberArray(v, 3, path);
  return [a!, b!, c!];
}

function tuple4(v: unknown, path: string): [number, number, number, number] {
  const [a, b, c, d] = asNumberArray(v, 4, path);
  return [a!, b!, c!, d!];
}

function nullable<T>(v: unknown, decode: (v: unknown) => T): T | null {
  return v === null ? null : decode(v);
}

// ── Component validators ──────────────────────────────────────────────────────

function decodeHandle(v: unknown, path: string): RenderHandle {
  return renderHandle(asNumber(v, path));
}

function decodeTransform(v: unknown, path: string): Transform {
  const o = asObject(v, path);
  return {
    translation: tuple3(o.translation, `${path}.translation`),
    rotation: tuple4(o.rotation, `${path}.rotation`),
    scale: tuple3(o.scale, `${path}.scale`),
  };
}

function decodeMaterial(v: unknown, path: string): Material {
  const o = asObject(v, path);
  return {
    color: tuple4(o.color, `${path}.color`),
    wireframe: asBoolean(o.wireframe, `${path}.wireframe`),
  };
}

function decodeGeometry(v: unknown, path: string): Geometry {
  const o = asObject(v, path);
  const shape = o.shape;
  switch (shape) {
    case 'cube':
    case 'sphere':
    case 'quad':
    case 'point':
      return { shape };
    case 'line':
      return {
        shape,
        a: tuple3(o.a, `${path}.a`),
        b: tuple3(o.b, `${path}.b`),
      };
    default:
      throw new RenderDecodeError(`unknown geometry shape ${JSON.stringify(shape)}`, `${path}.shape`);
  }
}

function decodeLayer(v: unknown, path: string): RenderLayer {
  if (v === 'scene' || v === 'debug') {
    return v;
  }
  throw new RenderDecodeError(`unknown layer ${JSON.stringify(v)}`, path);
}

function decodeMetadata(v: unknown, path: string): RenderMetadata {
  const o = asObject(v, path);
  return {
    source: nullable(o.source, (s) => entityId(asNumber(s, `${path}.source`))),
    tags: asArray(o.tags, `${path}.tags`).map((t, i) => tagId(asNumber(t, `${path}.tags[${i}]`))),
    label: nullable(o.label, (l) => {
      if (typeof l !== 'string') {
        throw new RenderDecodeError('expected a string', `${path}.label`);
      }
      return l;
    }),
  };
}

function decodeNode(v: unknown, path: string): RenderNode {
  const o = asObject(v, path);
  return {
    geometry: decodeGeometry(o.geometry, `${path}.geometry`),
    material: decodeMaterial(o.material, `${path}.material`),
    transform: decodeTransform(o.transform, `${path}.transform`),
    visible: asBoolean(o.visible, `${path}.visible`),
    layer: decodeLayer(o.layer, `${path}.layer`),
    metadata: decodeMetadata(o.metadata, `${path}.metadata`),
  };
}

// ── Mesh payload validators (ADR 0007) ────────────────────────────────────────

function asU32(v: unknown, path: string): number {
  const n = asNumber(v, path);
  if (!Number.isInteger(n) || n < 0) {
    throw new RenderDecodeError('expected a non-negative integer', path);
  }
  return n;
}

function decodeMeshAttribute(v: unknown, path: string): MeshAttribute {
  const o = asObject(v, path);
  const name = o.name;
  if (name !== 'position' && name !== 'normal' && name !== 'uv' && name !== 'color') {
    throw new RenderDecodeError(`unknown mesh attribute name ${JSON.stringify(name)}`, `${path}.name`);
  }
  if (o.kind !== 'f32') {
    throw new RenderDecodeError(`unknown mesh attribute kind ${JSON.stringify(o.kind)}`, `${path}.kind`);
  }
  return { name, components: asU32(o.components, `${path}.components`), kind: 'f32' };
}

function decodeMeshLayout(v: unknown, path: string): MeshBufferLayout {
  const o = asObject(v, path);
  if (o.indexWidth !== 'u32') {
    throw new RenderDecodeError(`unknown index width ${JSON.stringify(o.indexWidth)}`, `${path}.indexWidth`);
  }
  return {
    vertexCount: asU32(o.vertexCount, `${path}.vertexCount`),
    indexCount: asU32(o.indexCount, `${path}.indexCount`),
    indexWidth: 'u32',
    attributes: asArray(o.attributes, `${path}.attributes`).map((a, i) =>
      decodeMeshAttribute(a, `${path}.attributes[${i}]`),
    ),
  };
}

function decodeMeshGroup(v: unknown, path: string): MeshGroupDescriptor {
  const o = asObject(v, path);
  return {
    materialSlot: asU32(o.materialSlot, `${path}.materialSlot`),
    start: asU32(o.start, `${path}.start`),
    count: asU32(o.count, `${path}.count`),
  };
}

function decodeMeshBounds(v: unknown, path: string): MeshBoundsDescriptor {
  const o = asObject(v, path);
  return { min: tuple3(o.min, `${path}.min`), max: tuple3(o.max, `${path}.max`) };
}

function decodeMeshSource(v: unknown, path: string): MeshPayloadSource {
  const o = asObject(v, path);
  switch (o.kind) {
    case 'inline':
      return {
        kind: 'inline',
        positions: asArray(o.positions, `${path}.positions`).map((x, i) => asNumber(x, `${path}.positions[${i}]`)),
        normals: asArray(o.normals, `${path}.normals`).map((x, i) => asNumber(x, `${path}.normals[${i}]`)),
        indices: asArray(o.indices, `${path}.indices`).map((x, i) => asU32(x, `${path}.indices[${i}]`)),
      };
    case 'handle':
      return {
        kind: 'handle',
        buffer: asU32(o.buffer, `${path}.buffer`),
        positionsByteOffset: asU32(o.positionsByteOffset, `${path}.positionsByteOffset`),
        normalsByteOffset: asU32(o.normalsByteOffset, `${path}.normalsByteOffset`),
        indicesByteOffset: asU32(o.indicesByteOffset, `${path}.indicesByteOffset`),
      };
    default:
      throw new RenderDecodeError(`unknown mesh payload source ${JSON.stringify(o.kind)}`, `${path}.kind`);
  }
}

function decodeMeshProvenance(v: unknown, path: string): MeshProvenance {
  if (v === 'voxelChunk' || v === 'staticAsset' || v === 'generated' || v === 'debug') {
    return v;
  }
  throw new RenderDecodeError(`unknown mesh provenance ${JSON.stringify(v)}`, path);
}

/** Decode and structurally validate a mesh payload descriptor. */
export function decodeMeshPayloadDescriptor(v: unknown, path = '$'): MeshPayloadDescriptor {
  const o = asObject(v, path);
  const layout = decodeMeshLayout(o.layout, `${path}.layout`);
  const groups = asArray(o.groups, `${path}.groups`).map((g, i) => decodeMeshGroup(g, `${path}.groups[${i}]`));
  const bounds = decodeMeshBounds(o.bounds, `${path}.bounds`);
  const source = decodeMeshSource(o.source, `${path}.source`);
  const provenance = decodeMeshProvenance(o.provenance, `${path}.provenance`);

  // Cross-field checks mirroring protocol-render's MeshDescriptorError.
  if (source.kind === 'inline') {
    const expectV = layout.vertexCount * 3;
    if (source.positions.length !== expectV) {
      throw new RenderDecodeError(`positions length ${source.positions.length}, expected ${expectV}`, `${path}.source.positions`);
    }
    if (source.normals.length !== expectV) {
      throw new RenderDecodeError(`normals length ${source.normals.length}, expected ${expectV}`, `${path}.source.normals`);
    }
    if (source.indices.length !== layout.indexCount) {
      throw new RenderDecodeError(`indices length ${source.indices.length}, expected ${layout.indexCount}`, `${path}.source.indices`);
    }
    for (let i = 0; i < source.indices.length; i++) {
      if ((source.indices[i] as number) >= layout.vertexCount) {
        throw new RenderDecodeError(`index ${source.indices[i]} out of range for ${layout.vertexCount} vertices`, `${path}.source.indices[${i}]`);
      }
    }
  }
  const covered = groups.reduce((a, g) => a + g.count, 0);
  if (covered !== layout.indexCount) {
    throw new RenderDecodeError(`groups cover ${covered} indices, expected ${layout.indexCount}`, `${path}.groups`);
  }
  return { layout, groups, bounds, source, provenance };
}

// ── Static mesh + sprite validators (render-asset-04/05/06) ────────────────────

function decodeMaterialSlot(v: unknown, path: string): MeshMaterialSlot {
  const o = asObject(v, path);
  if (typeof o.material !== 'string') {
    throw new RenderDecodeError('expected a string material id', `${path}.material`);
  }
  return { slot: asU32(o.slot, `${path}.slot`), material: o.material };
}

function decodeCollisionPolicy(v: unknown, path: string): MeshCollisionPolicy {
  const o = asObject(v, path);
  switch (o.kind) {
    case 'visualOnly':
      return { kind: 'visualOnly' };
    case 'aabbFallback':
      return { kind: 'aabbFallback' };
    case 'proxy':
      if (typeof o.proxyAsset !== 'string' || o.proxyAsset.length === 0) {
        throw new RenderDecodeError('proxy policy needs a non-empty proxyAsset', `${path}.proxyAsset`);
      }
      return { kind: 'proxy', proxyAsset: o.proxyAsset };
    default:
      throw new RenderDecodeError(`unknown collision policy ${JSON.stringify(o.kind)}`, `${path}.kind`);
  }
}

/** Decode a static mesh asset, validating slot uniqueness and group bindings. */
export function decodeStaticMeshAsset(v: unknown, path = '$'): StaticMeshAsset {
  const o = asObject(v, path);
  if (typeof o.asset !== 'string' || o.asset.length === 0) {
    throw new RenderDecodeError('expected a non-empty asset id', `${path}.asset`);
  }
  const payload = decodeMeshPayloadDescriptor(o.payload, `${path}.payload`);
  const materialSlots = asArray(o.materialSlots, `${path}.materialSlots`).map((s, i) =>
    decodeMaterialSlot(s, `${path}.materialSlots[${i}]`),
  );
  const seen = new Set<number>();
  for (const s of materialSlots) {
    if (seen.has(s.slot)) {
      throw new RenderDecodeError(`duplicate material slot ${s.slot}`, `${path}.materialSlots`);
    }
    seen.add(s.slot);
  }
  for (const g of payload.groups) {
    if (!seen.has(g.materialSlot)) {
      throw new RenderDecodeError(`mesh group references unbound slot ${g.materialSlot}`, `${path}.materialSlots`);
    }
  }
  return { asset: o.asset, payload, materialSlots, collision: decodeCollisionPolicy(o.collision, `${path}.collision`) };
}

function decodeStaticMeshInstance(v: unknown, path: string): StaticMeshInstanceDescriptor {
  const o = asObject(v, path);
  if (typeof o.asset !== 'string' || o.asset.length === 0) {
    throw new RenderDecodeError('expected a non-empty asset id', `${path}.asset`);
  }
  return {
    asset: o.asset,
    transform: decodeTransform(o.transform, `${path}.transform`),
    materialOverrides: asArray(o.materialOverrides, `${path}.materialOverrides`).map((s, i) =>
      decodeMaterialSlot(s, `${path}.materialOverrides[${i}]`),
    ),
    metadata: decodeMetadata(o.metadata, `${path}.metadata`),
  };
}

function decodeSizeMode(v: unknown, path: string): SpriteSizeMode {
  if (v === 'world' || v === 'pixel') return v;
  throw new RenderDecodeError(`unknown sprite size mode ${JSON.stringify(v)}`, path);
}

function decodeBillboard(v: unknown, path: string): BillboardMode {
  if (v === 'none' || v === 'spherical' || v === 'cylindrical') return v;
  throw new RenderDecodeError(`unknown billboard mode ${JSON.stringify(v)}`, path);
}

function decodeDepthPolicy(v: unknown, path: string): SpriteDepthPolicy {
  if (v === 'default' || v === 'depthTestOff' || v === 'depthWriteOff') return v;
  throw new RenderDecodeError(`unknown sprite depth policy ${JSON.stringify(v)}`, path);
}

function decodeShading(v: unknown, path: string): SpriteShading {
  if (v === 'unlit' || v === 'lit' || v === 'shadowed' || v === 'custom') return v;
  throw new RenderDecodeError(`unknown sprite shading ${JSON.stringify(v)}`, path);
}

function tuple2(v: unknown, path: string): [number, number] {
  const [a, b] = asNumberArray(v, 2, path);
  return [a!, b!];
}

function decodeSpriteAttachment(v: unknown, path: string): SpriteAttachment {
  const o = asObject(v, path);
  return {
    sourceEntity: nullable(o.sourceEntity, (s) => entityId(asNumber(s, `${path}.sourceEntity`))),
    sourceSceneNode: nullable(o.sourceSceneNode, (s) => asU32(s, `${path}.sourceSceneNode`)),
    attachmentPoint: nullable(o.attachmentPoint, (s) => {
      if (typeof s !== 'string') throw new RenderDecodeError('expected a string', `${path}.attachmentPoint`);
      return s;
    }),
  };
}

/** Decode and validate a sprite instance descriptor. */
export function decodeSpriteInstance(v: unknown, path = '$'): SpriteInstanceDescriptor {
  const o = asObject(v, path);
  if (typeof o.asset !== 'string' || o.asset.length === 0) {
    throw new RenderDecodeError('expected a non-empty asset id', `${path}.asset`);
  }
  const pivot = tuple2(o.pivot, `${path}.pivot`);
  if (!(pivot[0] >= 0 && pivot[0] <= 1 && pivot[1] >= 0 && pivot[1] <= 1)) {
    throw new RenderDecodeError(`pivot ${JSON.stringify(pivot)} outside 0..=1`, `${path}.pivot`);
  }
  const size = tuple2(o.size, `${path}.size`);
  if (size[0] <= 0 || size[1] <= 0) {
    throw new RenderDecodeError(`size ${JSON.stringify(size)} must be positive`, `${path}.size`);
  }
  return {
    asset: o.asset,
    frame: asU32(o.frame, `${path}.frame`),
    pivot,
    size,
    sizeMode: decodeSizeMode(o.sizeMode, `${path}.sizeMode`),
    billboard: decodeBillboard(o.billboard, `${path}.billboard`),
    tint: tuple4(o.tint, `${path}.tint`),
    renderOrder: asNumber(o.renderOrder, `${path}.renderOrder`),
    depth: decodeDepthPolicy(o.depth, `${path}.depth`),
    shading: decodeShading(o.shading, `${path}.shading`),
    transform: decodeTransform(o.transform, `${path}.transform`),
    attachment: decodeSpriteAttachment(o.attachment, `${path}.attachment`),
    metadata: decodeMetadata(o.metadata, `${path}.metadata`),
  };
}

// ── Diff validators ───────────────────────────────────────────────────────────

/** Decode a single render diff (`create` / `update` / `destroy` / `replaceMeshPayload`). */
export function decodeRenderDiff(v: unknown, path = '$'): RenderDiff {
  const o = asObject(v, path);
  switch (o.op) {
    case 'create':
      return {
        op: 'create',
        handle: decodeHandle(o.handle, `${path}.handle`),
        parent: nullable(o.parent, (p) => decodeHandle(p, `${path}.parent`)),
        node: decodeNode(o.node, `${path}.node`),
      };
    case 'update':
      return {
        op: 'update',
        handle: decodeHandle(o.handle, `${path}.handle`),
        transform: nullable(o.transform, (t) => decodeTransform(t, `${path}.transform`)),
        material: nullable(o.material, (m) => decodeMaterial(m, `${path}.material`)),
        visible: nullable(o.visible, (b) => asBoolean(b, `${path}.visible`)),
        metadata: nullable(o.metadata, (m) => decodeMetadata(m, `${path}.metadata`)),
      };
    case 'destroy':
      return {
        op: 'destroy',
        handle: decodeHandle(o.handle, `${path}.handle`),
      };
    case 'replaceMeshPayload':
      return {
        op: 'replaceMeshPayload',
        handle: decodeHandle(o.handle, `${path}.handle`),
        payload: decodeMeshPayloadDescriptor(o.payload, `${path}.payload`),
      };
    case 'defineStaticMesh':
      return {
        op: 'defineStaticMesh',
        asset: decodeStaticMeshAsset(o.asset, `${path}.asset`),
      };
    case 'createStaticMeshInstance':
      return {
        op: 'createStaticMeshInstance',
        handle: decodeHandle(o.handle, `${path}.handle`),
        parent: nullable(o.parent, (p) => decodeHandle(p, `${path}.parent`)),
        instance: decodeStaticMeshInstance(o.instance, `${path}.instance`),
      };
    case 'createSprite':
      return {
        op: 'createSprite',
        handle: decodeHandle(o.handle, `${path}.handle`),
        parent: nullable(o.parent, (p) => decodeHandle(p, `${path}.parent`)),
        sprite: decodeSpriteInstance(o.sprite, `${path}.sprite`),
      };
    case 'updateSprite':
      return {
        op: 'updateSprite',
        handle: decodeHandle(o.handle, `${path}.handle`),
        frame: nullable(o.frame, (f) => asU32(f, `${path}.frame`)),
        tint: nullable(o.tint, (t) => tuple4(t, `${path}.tint`)),
        renderOrder: nullable(o.renderOrder, (n) => asNumber(n, `${path}.renderOrder`)),
        visible: nullable(o.visible, (b) => asBoolean(b, `${path}.visible`)),
      };
    default:
      throw new RenderDecodeError(`unknown render diff op ${JSON.stringify(o.op)}`, `${path}.op`);
  }
}

/** Decode a whole frame of render diffs into the generated contract type. */
export function decodeRenderFrameDiff(v: unknown, path = '$'): RenderFrameDiff {
  const o = asObject(v, path);
  const ops = asArray(o.ops, `${path}.ops`).map((op, i) =>
    decodeRenderDiff(op, `${path}.ops[${i}]`),
  );
  return { ops };
}

// ── Stream access for renderer consumption ────────────────────────────────────

/**
 * A small FIFO of decoded render frames for a renderer to drain each tick.
 *
 * The renderer pulls validated, contract-shaped frames out of here; it never
 * touches the raw payload or any WASM memory directly.
 */
export class RenderDiffStream {
  #frames: RenderFrameDiff[] = [];

  /** Decode and enqueue a raw frame payload. Throws `RenderDecodeError` if malformed. */
  push(payload: unknown): void {
    this.#frames.push(decodeRenderFrameDiff(payload));
  }

  /** Remove and return all enqueued frames, in arrival order. */
  drain(): RenderFrameDiff[] {
    const frames = this.#frames;
    this.#frames = [];
    return frames;
  }

  /** How many decoded frames are waiting. */
  get pending(): number {
    return this.#frames.length;
  }
}

// ── Memory-view lifetime contract (placeholder for large payloads) ────────────

/**
 * A borrowed view over WASM-owned bytes for a single frame.
 *
 * This is a placeholder for future large render payloads (e.g. vertex/index
 * buffers) that will be passed by reference into WASM memory rather than copied
 * through JSON. LIFETIME: a `FrameMemory` view is valid only for the frame it
 * was issued for. When the frame is superseded the host calls `invalidate()`,
 * after which `bytes()` throws — consumers must copy out anything they need to
 * retain *before* the next frame. Policy packages never receive one.
 */
export class FrameMemory {
  #bytes: Uint8Array | null;

  constructor(bytes: Uint8Array) {
    this.#bytes = bytes;
  }

  /** The borrowed bytes. Throws `RenderDecodeError` if the view was invalidated. */
  bytes(): Uint8Array {
    if (this.#bytes === null) {
      throw new RenderDecodeError('frame memory view used after invalidation', '$');
    }
    return this.#bytes;
  }

  /** Whether this view is still usable. */
  get valid(): boolean {
    return this.#bytes !== null;
  }

  /** Drop the borrow; subsequent `bytes()` calls throw. */
  invalidate(): void {
    this.#bytes = null;
  }
}
