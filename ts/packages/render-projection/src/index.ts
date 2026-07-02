// @asha/render-projection - renderer-neutral retained render-diff application.
//
// This package applies generated render diffs to a typed retained projection
// model. It owns no authority, imports no renderer implementation, and never
// touches raw runtime transports. Browser/Three/WebGPU bindings consume the
// returned neutral instructions or inspect the retained snapshot.

import type {
  Material,
  MeshPayloadDescriptor,
  MeshPickHit,
  RenderDiff,
  RenderFrameDiff,
  RenderHandle,
  RenderLayer,
  RenderMaterialDescriptor,
  RenderMetadata,
  RenderNode,
  SpriteAtlasDescriptor,
  SpriteInstanceDescriptor,
  SpritePickHit,
  StaticMeshAsset,
  StaticMeshInstanceDescriptor,
  TextureDescriptor,
  Transform,
} from '@asha/contracts';

/** Raised when a render diff cannot be applied to the retained projection. */
export class RenderProjectionError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'RenderProjectionError';
  }
}

export type RenderProjectionNodeKind = 'primitive' | 'staticMesh' | 'sprite';

export interface RenderProjectionNodeBase {
  readonly handle: RenderHandle;
  readonly parent: RenderHandle | null;
  readonly children: readonly RenderHandle[];
  readonly kind: RenderProjectionNodeKind;
  readonly layer: RenderLayer;
  readonly transform: Transform;
  readonly visible: boolean;
  readonly metadata: RenderMetadata;
  readonly material: Material | null;
  readonly meshPayload: MeshPayloadDescriptor | null;
}

export interface PrimitiveProjectionNode extends RenderProjectionNodeBase {
  readonly kind: 'primitive';
  readonly node: RenderNode;
}

export interface StaticMeshProjectionNode extends RenderProjectionNodeBase {
  readonly kind: 'staticMesh';
  readonly asset: string;
  readonly instance: StaticMeshInstanceDescriptor;
}

export interface SpriteProjectionNode extends RenderProjectionNodeBase {
  readonly kind: 'sprite';
  readonly sprite: SpriteInstanceDescriptor;
  readonly frameUv: readonly [number, number, number, number];
  readonly renderOrder: number;
}

export type RenderProjectionNode =
  | PrimitiveProjectionNode
  | StaticMeshProjectionNode
  | SpriteProjectionNode;

export type RenderProjectionInstruction =
  | { readonly op: 'defineMaterial'; readonly material: RenderMaterialDescriptor }
  | { readonly op: 'defineTexture'; readonly texture: TextureDescriptor }
  | { readonly op: 'defineSpriteAtlas'; readonly atlas: SpriteAtlasDescriptor }
  | { readonly op: 'defineStaticMesh'; readonly asset: StaticMeshAsset }
  | { readonly op: 'upsertNode'; readonly node: RenderProjectionNode }
  | { readonly op: 'removeNode'; readonly handle: RenderHandle };

export interface RenderProjectionSnapshot {
  readonly nodes: readonly RenderProjectionNode[];
  readonly materials: readonly RenderMaterialDescriptor[];
  readonly textures: readonly TextureDescriptor[];
  readonly spriteAtlases: readonly SpriteAtlasDescriptor[];
  readonly staticMeshes: readonly StaticMeshAsset[];
}

type NodeRecord = MutablePrimitiveNode | MutableStaticMeshNode | MutableSpriteNode;

interface MutableNodeBase {
  handle: RenderHandle;
  parent: RenderHandle | null;
  children: Set<RenderHandle>;
  kind: RenderProjectionNodeKind;
  layer: RenderLayer;
  transform: Transform;
  visible: boolean;
  metadata: RenderMetadata;
  material: Material | null;
  meshPayload: MeshPayloadDescriptor | null;
}

interface MutablePrimitiveNode extends MutableNodeBase {
  kind: 'primitive';
  node: RenderNode;
}

interface MutableStaticMeshNode extends MutableNodeBase {
  kind: 'staticMesh';
  asset: string;
  instance: StaticMeshInstanceDescriptor;
}

interface MutableSpriteNode extends MutableNodeBase {
  kind: 'sprite';
  sprite: SpriteInstanceDescriptor;
  frameUv: [number, number, number, number];
  renderOrder: number;
}

interface StaticMeshRecord {
  asset: StaticMeshAsset;
  refCount: number;
}

/** A retained renderer-neutral projection driven only by render diffs. */
export class RenderProjection {
  readonly #nodes = new Map<RenderHandle, NodeRecord>();
  readonly #materials = new Map<string, RenderMaterialDescriptor>();
  readonly #textures = new Map<string, TextureDescriptor>();
  readonly #spriteAtlases = new Map<string, SpriteAtlasDescriptor>();
  readonly #staticMeshes = new Map<string, StaticMeshRecord>();

  /** Apply a frame in authored order and return renderer-neutral instructions. */
  applyFrame(frame: RenderFrameDiff): readonly RenderProjectionInstruction[] {
    const instructions: RenderProjectionInstruction[] = [];
    for (const diff of frame.ops) {
      instructions.push(...this.applyDiff(diff));
    }
    return instructions;
  }

  /** Apply one diff. Throws `RenderProjectionError` on stale handles or malformed payloads. */
  applyDiff(diff: RenderDiff): readonly RenderProjectionInstruction[] {
    switch (diff.op) {
      case 'create':
        return [this.#create(diff)];
      case 'update':
        return [this.#update(diff)];
      case 'destroy':
        return this.#destroy(diff.handle);
      case 'replaceMeshPayload':
        return [this.#replaceMeshPayload(diff)];
      case 'defineMaterial':
        return [this.#defineMaterial(diff.material)];
      case 'defineTexture':
        return [this.#defineTexture(diff.texture)];
      case 'defineSpriteAtlas':
        return [this.#defineSpriteAtlas(diff.atlas)];
      case 'defineStaticMesh':
        return [this.#defineStaticMesh(diff.asset)];
      case 'createStaticMeshInstance':
        return [this.#createStaticMeshInstance(diff)];
      case 'createSprite':
        return [this.#createSprite(diff)];
      case 'updateSprite':
        return [this.#updateSprite(diff)];
      default: {
        const unknown = diff as { readonly op?: unknown };
        throw new RenderProjectionError(`unsupported render diff op ${JSON.stringify(unknown.op)}`);
      }
    }
  }

  has(handle: RenderHandle): boolean {
    return this.#nodes.has(handle);
  }

  get handleCount(): number {
    return this.#nodes.size;
  }

  node(handle: RenderHandle): RenderProjectionNode | undefined {
    const record = this.#nodes.get(handle);
    return record === undefined ? undefined : snapshotNode(record);
  }

  materialDescriptor(id: string): RenderMaterialDescriptor | undefined {
    return clone(this.#materials.get(id));
  }

  textureDescriptor(id: string): TextureDescriptor | undefined {
    return clone(this.#textures.get(id));
  }

  spriteAtlas(id: string): SpriteAtlasDescriptor | undefined {
    return clone(this.#spriteAtlases.get(id));
  }

  staticMesh(asset: string): StaticMeshAsset | undefined {
    return clone(this.#staticMeshes.get(asset)?.asset);
  }

  staticMeshRefCount(asset: string): number {
    return this.#staticMeshes.get(asset)?.refCount ?? 0;
  }

  snapshot(): RenderProjectionSnapshot {
    return {
      nodes: sortedHandles(this.#nodes).map((handle) => snapshotNode(this.#require(handle, 'snapshot'))),
      materials: sortedValues(this.#materials),
      textures: sortedValues(this.#textures),
      spriteAtlases: sortedValues(this.#spriteAtlases),
      staticMeshes: [...this.#staticMeshes.values()]
        .map((record) => clone(record.asset))
        .sort((a, b) => a.asset.localeCompare(b.asset)),
    };
  }

  pickMesh(handle: RenderHandle): MeshPickHit | undefined {
    const payload = this.#nodes.get(handle)?.meshPayload;
    if (payload === undefined || payload === null) {
      return undefined;
    }
    return { handle, provenance: payload.provenance };
  }

  pickSprite(handle: RenderHandle): SpritePickHit | undefined {
    const record = this.#nodes.get(handle);
    if (record?.kind !== 'sprite') {
      return undefined;
    }
    const attachment = record.sprite.attachment;
    return {
      handle,
      sourceEntity: attachment.sourceEntity,
      sourceSceneNode: attachment.sourceSceneNode,
      asset: record.sprite.asset,
      attachmentPoint: attachment.attachmentPoint,
    };
  }

  #create(diff: Extract<RenderDiff, { op: 'create' }>): RenderProjectionInstruction {
    this.#ensureFree(diff.handle, 'create');
    const parent = this.#parentHandle(diff.parent, 'create.parent');
    const node = clone(diff.node);
    const record: MutablePrimitiveNode = {
      handle: diff.handle,
      parent,
      children: new Set(),
      kind: 'primitive',
      layer: parent === null ? node.layer : this.#require(parent, 'create.parent').layer,
      transform: clone(node.transform),
      visible: node.visible,
      metadata: clone(node.metadata),
      material: clone(node.material),
      meshPayload: null,
      node,
    };
    this.#insert(record);
    return { op: 'upsertNode', node: snapshotNode(record) };
  }

  #update(diff: Extract<RenderDiff, { op: 'update' }>): RenderProjectionInstruction {
    const record = this.#require(diff.handle, 'update');
    if (diff.transform !== null) {
      record.transform = clone(diff.transform);
      if (record.kind === 'primitive') {
        record.node = { ...record.node, transform: clone(diff.transform) };
      } else if (record.kind === 'staticMesh') {
        record.instance = { ...record.instance, transform: clone(diff.transform) };
      } else {
        record.sprite = { ...record.sprite, transform: clone(diff.transform) };
      }
    }
    if (diff.material !== null) {
      record.material = clone(diff.material);
      if (record.kind === 'primitive') {
        record.node = { ...record.node, material: clone(diff.material) };
      }
    }
    if (diff.visible !== null) {
      record.visible = diff.visible;
      if (record.kind === 'primitive') {
        record.node = { ...record.node, visible: diff.visible };
      }
    }
    if (diff.metadata !== null) {
      record.metadata = clone(diff.metadata);
      if (record.kind === 'primitive') {
        record.node = { ...record.node, metadata: clone(diff.metadata) };
      } else if (record.kind === 'staticMesh') {
        record.instance = { ...record.instance, metadata: clone(diff.metadata) };
      } else {
        record.sprite = { ...record.sprite, metadata: clone(diff.metadata) };
      }
    }
    return { op: 'upsertNode', node: snapshotNode(record) };
  }

  #destroy(handle: RenderHandle): readonly RenderProjectionInstruction[] {
    const record = this.#require(handle, 'destroy');
    const instructions: RenderProjectionInstruction[] = [];
    for (const child of [...record.children].sort(numberCompare)) {
      instructions.push(...this.#destroy(child));
    }
    this.#nodes.delete(handle);
    if (record.parent !== null) {
      this.#require(record.parent, 'destroy.parent').children.delete(handle);
    }
    if (record.kind === 'staticMesh') {
      const mesh = this.#staticMeshes.get(record.asset);
      if (mesh !== undefined) {
        mesh.refCount -= 1;
      }
    }
    instructions.push({ op: 'removeNode', handle });
    return instructions;
  }

  #replaceMeshPayload(
    diff: Extract<RenderDiff, { op: 'replaceMeshPayload' }>,
  ): RenderProjectionInstruction {
    const record = this.#require(diff.handle, 'replaceMeshPayload');
    if (record.kind === 'sprite') {
      throw new RenderProjectionError(`replaceMeshPayload: handle ${diff.handle} is a sprite`);
    }
    validateMeshPayload(diff.payload, 'replaceMeshPayload.payload');
    record.meshPayload = clone(diff.payload);
    return { op: 'upsertNode', node: snapshotNode(record) };
  }

  #defineMaterial(material: RenderMaterialDescriptor): RenderProjectionInstruction {
    this.#materials.set(material.id, clone(material));
    return { op: 'defineMaterial', material: clone(material) };
  }

  #defineTexture(texture: TextureDescriptor): RenderProjectionInstruction {
    this.#textures.set(texture.id, clone(texture));
    return { op: 'defineTexture', texture: clone(texture) };
  }

  #defineSpriteAtlas(atlas: SpriteAtlasDescriptor): RenderProjectionInstruction {
    this.#spriteAtlases.set(atlas.id, clone(atlas));
    return { op: 'defineSpriteAtlas', atlas: clone(atlas) };
  }

  #defineStaticMesh(asset: StaticMeshAsset): RenderProjectionInstruction {
    validateMeshPayload(asset.payload, `defineStaticMesh(${asset.asset}).payload`);
    const existing = this.#staticMeshes.get(asset.asset);
    if (existing !== undefined && existing.refCount > 0) {
      throw new RenderProjectionError(
        `defineStaticMesh: asset ${asset.asset} is in use by ${existing.refCount} instance(s)`,
      );
    }
    this.#staticMeshes.set(asset.asset, { asset: clone(asset), refCount: 0 });
    return { op: 'defineStaticMesh', asset: clone(asset) };
  }

  #createStaticMeshInstance(
    diff: Extract<RenderDiff, { op: 'createStaticMeshInstance' }>,
  ): RenderProjectionInstruction {
    this.#ensureFree(diff.handle, 'createStaticMeshInstance');
    const asset = this.#staticMeshes.get(diff.instance.asset);
    if (asset === undefined) {
      throw new RenderProjectionError(
        `createStaticMeshInstance: undefined static mesh asset ${diff.instance.asset}`,
      );
    }
    const parent = this.#parentHandle(diff.parent, 'createStaticMeshInstance.parent');
    const instance = clone(diff.instance);
    const record: MutableStaticMeshNode = {
      handle: diff.handle,
      parent,
      children: new Set(),
      kind: 'staticMesh',
      layer: parent === null ? 'scene' : this.#require(parent, 'createStaticMeshInstance.parent').layer,
      transform: clone(instance.transform),
      visible: true,
      metadata: clone(instance.metadata),
      material: null,
      meshPayload: clone(asset.asset.payload),
      asset: instance.asset,
      instance,
    };
    asset.refCount += 1;
    this.#insert(record);
    return { op: 'upsertNode', node: snapshotNode(record) };
  }

  #createSprite(diff: Extract<RenderDiff, { op: 'createSprite' }>): RenderProjectionInstruction {
    this.#ensureFree(diff.handle, 'createSprite');
    const parent = this.#parentHandle(diff.parent, 'createSprite.parent');
    const sprite = clone(diff.sprite);
    const record: MutableSpriteNode = {
      handle: diff.handle,
      parent,
      children: new Set(),
      kind: 'sprite',
      layer: parent === null ? 'scene' : this.#require(parent, 'createSprite.parent').layer,
      transform: clone(sprite.transform),
      visible: true,
      metadata: clone(sprite.metadata),
      material: null,
      meshPayload: null,
      sprite,
      frameUv: this.#resolveSpriteUv(sprite.asset, sprite.frame),
      renderOrder: sprite.renderOrder,
    };
    this.#insert(record);
    return { op: 'upsertNode', node: snapshotNode(record) };
  }

  #updateSprite(diff: Extract<RenderDiff, { op: 'updateSprite' }>): RenderProjectionInstruction {
    const record = this.#require(diff.handle, 'updateSprite');
    if (record.kind !== 'sprite') {
      throw new RenderProjectionError(`updateSprite: handle ${diff.handle} is not a sprite`);
    }
    if (diff.frame !== null) {
      record.sprite = { ...record.sprite, frame: diff.frame };
      record.frameUv = this.#resolveSpriteUv(record.sprite.asset, diff.frame);
    }
    if (diff.tint !== null) {
      record.sprite = { ...record.sprite, tint: clone(diff.tint) };
    }
    if (diff.renderOrder !== null) {
      record.sprite = { ...record.sprite, renderOrder: diff.renderOrder };
      record.renderOrder = diff.renderOrder;
    }
    if (diff.visible !== null) {
      record.visible = diff.visible;
    }
    return { op: 'upsertNode', node: snapshotNode(record) };
  }

  #resolveSpriteUv(asset: string, frame: number): [number, number, number, number] {
    const atlas = this.#spriteAtlases.get(asset);
    const rect = atlas?.frames.find((candidate) => candidate.frame === frame);
    if (rect === undefined) {
      return [0, 0, 1, 1];
    }
    return [rect.uvMin[0], rect.uvMin[1], rect.uvMax[0], rect.uvMax[1]];
  }

  #insert(record: NodeRecord): void {
    this.#nodes.set(record.handle, record);
    if (record.parent !== null) {
      this.#require(record.parent, 'insert.parent').children.add(record.handle);
    }
  }

  #ensureFree(handle: RenderHandle, ctx: string): void {
    if (this.#nodes.has(handle)) {
      throw new RenderProjectionError(`${ctx}: handle ${handle} already exists`);
    }
  }

  #parentHandle(parent: RenderHandle | null, ctx: string): RenderHandle | null {
    if (parent !== null) {
      this.#require(parent, ctx);
    }
    return parent;
  }

  #require(handle: RenderHandle, ctx: string): NodeRecord {
    const record = this.#nodes.get(handle);
    if (record === undefined) {
      throw new RenderProjectionError(`${ctx}: unknown handle ${handle}`);
    }
    return record;
  }
}

function snapshotNode(record: NodeRecord): RenderProjectionNode {
  const base = {
    handle: record.handle,
    parent: record.parent,
    children: [...record.children].sort(numberCompare),
    layer: record.layer,
    transform: clone(record.transform),
    visible: record.visible,
    metadata: clone(record.metadata),
    material: clone(record.material),
    meshPayload: clone(record.meshPayload),
  };
  if (record.kind === 'primitive') {
    return { ...base, kind: 'primitive', node: clone(record.node) };
  }
  if (record.kind === 'staticMesh') {
    return {
      ...base,
      kind: 'staticMesh',
      asset: record.asset,
      instance: clone(record.instance),
    };
  }
  return {
    ...base,
    kind: 'sprite',
    sprite: clone(record.sprite),
    frameUv: clone(record.frameUv),
    renderOrder: record.renderOrder,
  };
}

function validateMeshPayload(payload: MeshPayloadDescriptor, ctx: string): void {
  const vertexCount = requireNonNegativeInteger(payload.layout.vertexCount, `${ctx}.layout.vertexCount`);
  const indexCount = requireNonNegativeInteger(payload.layout.indexCount, `${ctx}.layout.indexCount`);
  const positionComponents = attributeComponents(payload, 'position', ctx);
  const normalComponents = attributeComponents(payload, 'normal', ctx);

  if (payload.source.kind === 'inline') {
    requireLength(payload.source.positions, vertexCount * positionComponents, `${ctx}.source.positions`);
    requireLength(payload.source.normals, vertexCount * normalComponents, `${ctx}.source.normals`);
    requireLength(payload.source.indices, indexCount, `${ctx}.source.indices`);
    payload.source.indices.forEach((index, i) => {
      const value = requireNonNegativeInteger(index, `${ctx}.source.indices[${i}]`);
      if (value >= vertexCount) {
        throw new RenderProjectionError(
          `${ctx}.source.indices[${i}] ${value} is out of range for ${vertexCount} vertices`,
        );
      }
    });
  } else {
    requireNonNegativeInteger(payload.source.buffer, `${ctx}.source.buffer`);
    requireNonNegativeInteger(payload.source.positionsByteOffset, `${ctx}.source.positionsByteOffset`);
    requireNonNegativeInteger(payload.source.normalsByteOffset, `${ctx}.source.normalsByteOffset`);
    requireNonNegativeInteger(payload.source.indicesByteOffset, `${ctx}.source.indicesByteOffset`);
  }

  for (let i = 0; i < payload.groups.length; i += 1) {
    const group = payload.groups[i]!;
    const start = requireNonNegativeInteger(group.start, `${ctx}.groups[${i}].start`);
    const count = requireNonNegativeInteger(group.count, `${ctx}.groups[${i}].count`);
    requireNonNegativeInteger(group.materialSlot, `${ctx}.groups[${i}].materialSlot`);
    if (start + count > indexCount) {
      throw new RenderProjectionError(
        `${ctx}.groups[${i}] window [${start}, ${start + count}) exceeds indexCount ${indexCount}`,
      );
    }
  }
}

function attributeComponents(
  payload: MeshPayloadDescriptor,
  name: 'position' | 'normal',
  ctx: string,
): number {
  const attribute = payload.layout.attributes.find((candidate) => candidate.name === name);
  if (attribute === undefined) {
    throw new RenderProjectionError(`${ctx}.layout.attributes missing ${name}`);
  }
  return requireNonNegativeInteger(attribute.components, `${ctx}.layout.attributes.${name}.components`);
}

function requireLength(values: readonly unknown[], expected: number, ctx: string): void {
  if (values.length !== expected) {
    throw new RenderProjectionError(`${ctx} expected length ${expected}, got ${values.length}`);
  }
}

function requireNonNegativeInteger(value: number, ctx: string): number {
  if (!Number.isInteger(value) || value < 0) {
    throw new RenderProjectionError(`${ctx} must be a non-negative integer`);
  }
  return value;
}

function sortedHandles(map: ReadonlyMap<RenderHandle, unknown>): RenderHandle[] {
  return [...map.keys()].sort(numberCompare);
}

function sortedValues<T extends { readonly id: string }>(map: ReadonlyMap<string, T>): T[] {
  return [...map.values()].map((value) => clone(value)).sort((a, b) => a.id.localeCompare(b.id));
}

function numberCompare(a: number, b: number): number {
  return a - b;
}

function clone<T>(value: T): T {
  if (value === undefined) {
    return value;
  }
  return JSON.parse(JSON.stringify(value)) as T;
}
