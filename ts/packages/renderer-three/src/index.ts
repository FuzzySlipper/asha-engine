// @asha/renderer-three — a minimal retained-mode renderer shell.
//
// It applies create/update/destroy render diffs (generated `@asha/contracts`
// types) to a placeholder Three.js scene through a handle registry. It is a
// thin *projector consumer*: it never reads authority state, never validates,
// and imports no policy/core packages. Building the scene graph needs no GL
// context (only pixel rendering does), so this is testable headlessly; a real
// WebGL/offscreen renderer for screenshots is layered on in a later task.

import * as THREE from 'three';
import { decodeRenderFrameDiff } from '@asha/runtime-bridge';
import type {
  Geometry,
  Material,
  MeshCollisionPolicy,
  MeshMaterialSlot,
  MeshPayloadDescriptor,
  RenderDiff,
  RenderFrameDiff,
  RenderHandle,
  RenderLayer,
  RenderMetadata,
  RenderNode,
  SpriteInstanceDescriptor,
  SpritePickHit,
  StaticMeshAsset,
  Transform,
} from '@asha/contracts';

/** Raised when a diff cannot be applied (duplicate, unknown, or stale handle). */
export class RenderApplyError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'RenderApplyError';
  }
}

type NodeKind = 'primitive' | 'staticMesh' | 'sprite';

interface NodeEntry {
  readonly object: THREE.Object3D;
  readonly kind: NodeKind;
  /** Primitive shape, for `kind === 'primitive'`. */
  readonly shape: Geometry['shape'];
  /** Source asset id, for static mesh instances and sprites. */
  readonly asset?: string;
  /** Whether destroying this node may dispose its geometry (false = shared). */
  readonly ownsGeometry: boolean;
  /** The full sprite descriptor, for `kind === 'sprite'` (frame/tint/pick). */
  sprite?: SpriteInstanceDescriptor;
}

/** A defined static mesh asset: one shared geometry + materials, reference-counted. */
interface StaticMeshDef {
  readonly geometry: THREE.BufferGeometry;
  readonly materials: THREE.Material[];
  /** material slot index → position in `materials`. */
  readonly slotIndex: Map<number, number>;
  readonly materialSlots: readonly MeshMaterialSlot[];
  readonly collision: MeshCollisionPolicy;
  refCount: number;
}

/**
 * A retained Three.js scene driven entirely by render diffs.
 *
 * Nodes are addressed by `RenderHandle`; the registry maps each handle to a
 * Three.js `Object3D`. Scene and debug layers are separate groups so overlays
 * can be toggled independently.
 */
export class ThreeRenderer {
  readonly scene = new THREE.Scene();
  readonly #sceneGroup = new THREE.Group();
  readonly #debugGroup = new THREE.Group();
  readonly #handles = new Map<RenderHandle, NodeEntry>();
  /** Defined static mesh assets, keyed by asset id (shared geometry lifecycle). */
  readonly #staticMeshes = new Map<string, StaticMeshDef>();
  /** Per-material-slot colours for the initial flat/debug material strategy. */
  readonly #slotColors = new Map<number, THREE.Color>();

  constructor() {
    this.#sceneGroup.name = 'scene';
    this.#debugGroup.name = 'debug';
    this.scene.add(this.#sceneGroup, this.#debugGroup);
  }

  #layerGroup(layer: RenderLayer): THREE.Group {
    return layer === 'debug' ? this.#debugGroup : this.#sceneGroup;
  }

  /** Apply a whole frame of diffs in order. */
  applyFrame(frame: RenderFrameDiff): void {
    for (const op of frame.ops) {
      this.applyDiff(op);
    }
  }

  /** Decode a raw payload through `@asha/runtime-bridge` and apply it. */
  applyEncodedFrame(payload: unknown): void {
    this.applyFrame(decodeRenderFrameDiff(payload));
  }

  /** Apply a single diff. Throws `RenderApplyError` on a bad handle. */
  applyDiff(diff: RenderDiff): void {
    switch (diff.op) {
      case 'create':
        this.#create(diff);
        break;
      case 'update':
        this.#update(diff);
        break;
      case 'destroy':
        this.#destroy(diff);
        break;
      case 'replaceMeshPayload':
        this.#replaceMeshPayload(diff);
        break;
      case 'defineStaticMesh':
        this.#defineStaticMesh(diff.asset);
        break;
      case 'createStaticMeshInstance':
        this.#createStaticMeshInstance(diff);
        break;
      case 'createSprite':
        this.#createSprite(diff);
        break;
      case 'updateSprite':
        this.#updateSprite(diff);
        break;
    }
  }

  /**
   * Register the flat colour used for a material slot (the initial flat/debug
   * material strategy — ADR 0007). Unregistered slots fall back to a deterministic
   * per-slot colour, so a payload always maps to *some* visible material.
   */
  registerSlotColor(slot: number, r: number, g: number, b: number): void {
    this.#slotColors.set(slot, new THREE.Color(r, g, b));
  }

  #slotColor(slot: number): THREE.Color {
    const registered = this.#slotColors.get(slot);
    if (registered) {
      return registered.clone();
    }
    // Deterministic fallback hue per slot (golden angle), so missing slots are
    // visible and stable rather than silently skipped.
    const hue = (slot * 0.61803398875) % 1;
    return new THREE.Color().setHSL(hue, 0.7, 0.5);
  }

  /** Whether a handle is currently live in the scene. */
  has(handle: RenderHandle): boolean {
    return this.#handles.has(handle);
  }

  /** Number of live scene handles. */
  get handleCount(): number {
    return this.#handles.size;
  }

  /** The Three.js object for a handle, for inspection/tests. */
  objectFor(handle: RenderHandle): THREE.Object3D | undefined {
    return this.#handles.get(handle)?.object;
  }

  /**
   * A deterministic textual snapshot of the rendered scene — one line per live
   * handle (sorted), capturing layer, shape, transform, visibility, and colour.
   *
   * This is the "render artifact" the golden check diffs. It is a structural
   * snapshot rather than a pixel screenshot: GPU pixel output is
   * non-deterministic across drivers and headless GL is a heavy native
   * dependency, whereas this is exact, reviewable, and needs no GL context.
   */
  snapshot(): string {
    const entries = [...this.#handles.entries()].sort((a, b) => a[0] - b[0]);
    if (entries.length === 0) {
      return '(empty scene)\n';
    }
    return entries.map(([handle, entry]) => snapshotLine(handle, entry)).join('\n') + '\n';
  }

  #create(diff: Extract<RenderDiff, { op: 'create' }>): void {
    if (this.#handles.has(diff.handle)) {
      throw new RenderApplyError(`create: handle ${diff.handle} already exists`);
    }
    const object = buildObject(diff.node);
    const parent =
      diff.parent === null
        ? this.#layerGroup(diff.node.layer)
        : this.#require(diff.parent, 'create.parent').object;
    parent.add(object);
    this.#handles.set(diff.handle, {
      object,
      kind: 'primitive',
      shape: diff.node.geometry.shape,
      ownsGeometry: true,
    });
  }

  #update(diff: Extract<RenderDiff, { op: 'update' }>): void {
    const entry = this.#require(diff.handle, 'update');
    if (diff.transform) {
      applyTransform(entry.object, diff.transform);
    }
    if (diff.material) {
      applyMaterial(entry, diff.material);
    }
    if (diff.visible !== null) {
      entry.object.visible = diff.visible;
    }
    if (diff.metadata) {
      applyMetadata(entry.object, diff.metadata);
    }
  }

  #destroy(diff: Extract<RenderDiff, { op: 'destroy' }>): void {
    const entry = this.#require(diff.handle, 'destroy');
    entry.object.parent?.remove(entry.object);
    if (entry.kind === 'staticMesh' && entry.asset !== undefined) {
      // Shared geometry: dispose only this instance's override materials, then
      // release the asset reference. The asset's geometry is disposed only when
      // its last instance is gone (reference-safe — never while another shares it).
      disposeInstanceOverrides(entry.object);
      this.#releaseStaticMesh(entry.asset);
    } else {
      disposeObject(entry.object);
    }
    this.#handles.delete(diff.handle);
  }

  // ── Static mesh assets + instances (render-asset-04) ────────────────────────

  /**
   * Define (or redefine) a static mesh asset's shared geometry + slot materials.
   * Idempotent per asset id: a redefine while instances exist is rejected (it
   * would orphan shared geometry); a redefine of an unused asset replaces it.
   */
  #defineStaticMesh(asset: StaticMeshAsset): void {
    const existing = this.#staticMeshes.get(asset.asset);
    if (existing) {
      if (existing.refCount > 0) {
        throw new RenderApplyError(
          `defineStaticMesh: asset ${asset.asset} is in use by ${existing.refCount} instance(s)`,
        );
      }
      existing.geometry.dispose();
      existing.materials.forEach((m) => m.dispose());
    }
    if (asset.payload.source.kind !== 'inline') {
      throw new RenderApplyError(
        `defineStaticMesh: handle-source payloads need a runtime buffer provider (not wired yet)`,
      );
    }
    const geometry = buildMeshGeometry(asset.payload);
    const slotIndex = new Map<number, number>();
    const materials = asset.materialSlots.map((s, i) => {
      slotIndex.set(s.slot, i);
      return this.#materialFor(s);
    });
    this.#staticMeshes.set(asset.asset, {
      geometry,
      materials,
      slotIndex,
      materialSlots: asset.materialSlots,
      collision: asset.collision,
      refCount: 0,
    });
  }

  #createStaticMeshInstance(diff: Extract<RenderDiff, { op: 'createStaticMeshInstance' }>): void {
    if (this.#handles.has(diff.handle)) {
      throw new RenderApplyError(`createStaticMeshInstance: handle ${diff.handle} already exists`);
    }
    const def = this.#staticMeshes.get(diff.instance.asset);
    if (!def) {
      throw new RenderApplyError(
        `createStaticMeshInstance: undefined static mesh asset ${diff.instance.asset}`,
      );
    }
    // Materials default to the asset's; per-instance overrides clone-replace just
    // the named slots, so two instances of one asset can differ in material while
    // sharing one BufferGeometry.
    const materials = def.materials.slice();
    const ownMaterials: THREE.Material[] = [];
    for (const ov of diff.instance.materialOverrides) {
      const idx = def.slotIndex.get(ov.slot);
      if (idx === undefined) {
        throw new RenderApplyError(
          `createStaticMeshInstance: override for unbound slot ${ov.slot} on ${diff.instance.asset}`,
        );
      }
      const m = this.#materialFor(ov);
      materials[idx] = m;
      ownMaterials.push(m);
    }
    const mesh = new THREE.Mesh(def.geometry, materials.length === 1 ? materials[0]! : materials);
    // Instance-owned override materials (disposed on destroy; shared ones aren't).
    mesh.userData.ownMaterials = ownMaterials;
    applyTransform(mesh, diff.instance.transform);
    applyMetadata(mesh, diff.instance.metadata);

    const parent =
      diff.parent === null ? this.#sceneGroup : this.#require(diff.parent, 'createStaticMeshInstance.parent').object;
    parent.add(mesh);
    def.refCount += 1;
    this.#handles.set(diff.handle, {
      object: mesh,
      kind: 'staticMesh',
      shape: 'quad',
      asset: diff.instance.asset,
      ownsGeometry: false,
    });
  }

  #releaseStaticMesh(asset: string): void {
    const def = this.#staticMeshes.get(asset);
    if (!def) {
      return;
    }
    def.refCount -= 1;
    if (def.refCount <= 0) {
      def.geometry.dispose();
      def.materials.forEach((m) => m.dispose());
      this.#staticMeshes.delete(asset);
    }
  }

  /** How many live instances reference a defined static mesh asset (0 if undefined). */
  instanceCountFor(asset: string): number {
    return this.#staticMeshes.get(asset)?.refCount ?? 0;
  }

  #materialFor(slot: MeshMaterialSlot): THREE.MeshBasicMaterial {
    // The render border maps a material asset id → an appearance. Until catalog
    // RenderMaterial wiring lands, slots map to a deterministic per-slot colour
    // (registerSlotColor) so the mesh always shows *some* stable material.
    return new THREE.MeshBasicMaterial({ color: this.#slotColor(slot.slot) });
  }

  // ── Sprites / billboards (render-asset-05/06) ───────────────────────────────

  #createSprite(diff: Extract<RenderDiff, { op: 'createSprite' }>): void {
    if (this.#handles.has(diff.handle)) {
      throw new RenderApplyError(`createSprite: handle ${diff.handle} already exists`);
    }
    const s = diff.sprite;
    // Plane BufferGeometry (NOT THREE.Sprite) so the node fits the retained handle
    // lifecycle and future batching. Pivot shifts the plane so the anchor sits at
    // the node origin.
    const geometry = new THREE.PlaneGeometry(s.size[0], s.size[1]);
    geometry.translate((0.5 - s.pivot[0]) * s.size[0], (0.5 - s.pivot[1]) * s.size[1], 0);
    const material = new THREE.MeshBasicMaterial({
      color: new THREE.Color(s.tint[0], s.tint[1], s.tint[2]),
      opacity: s.tint[3],
      transparent: s.tint[3] < 1,
      depthTest: s.depth !== 'depthTestOff',
      depthWrite: s.depth === 'default',
    });
    const mesh = new THREE.Mesh(geometry, material);
    mesh.renderOrder = s.renderOrder;
    applyTransform(mesh, s.transform);
    applyMetadata(mesh, s.metadata);
    mesh.userData.frame = s.frame;
    mesh.userData.billboard = s.billboard;

    const parent =
      diff.parent === null ? this.#sceneGroup : this.#require(diff.parent, 'createSprite.parent').object;
    parent.add(mesh);
    this.#handles.set(diff.handle, {
      object: mesh,
      kind: 'sprite',
      shape: 'quad',
      asset: s.asset,
      ownsGeometry: true,
      sprite: s,
    });
  }

  /**
   * Deterministic, projection-driven sprite update. Frame/tint/order/visibility
   * come from an authority tick — never renderer wall-clock animation — so the
   * same diff sequence always produces the same scene.
   */
  #updateSprite(diff: Extract<RenderDiff, { op: 'updateSprite' }>): void {
    const entry = this.#require(diff.handle, 'updateSprite');
    if (entry.kind !== 'sprite' || !entry.sprite) {
      throw new RenderApplyError(`updateSprite: handle ${diff.handle} is not a sprite`);
    }
    const mesh = entry.object as THREE.Mesh;
    const material = mesh.material as THREE.MeshBasicMaterial;
    if (diff.frame !== null) {
      entry.sprite = { ...entry.sprite, frame: diff.frame };
      mesh.userData.frame = diff.frame;
    }
    if (diff.tint !== null) {
      entry.sprite = { ...entry.sprite, tint: diff.tint };
      material.color.setRGB(diff.tint[0], diff.tint[1], diff.tint[2]);
      material.opacity = diff.tint[3];
      material.transparent = diff.tint[3] < 1;
    }
    if (diff.renderOrder !== null) {
      entry.sprite = { ...entry.sprite, renderOrder: diff.renderOrder };
      mesh.renderOrder = diff.renderOrder;
    }
    if (diff.visible !== null) {
      mesh.visible = diff.visible;
    }
  }

  /**
   * Resolve a renderer-side sprite pick to an authority-facing trace: render
   * handle + source entity/scene-node ids + asset ref + attachment point. The
   * renderer decides no gameplay action — authority revalidates and acts.
   */
  pickSprite(handle: RenderHandle): SpritePickHit | undefined {
    const entry = this.#handles.get(handle);
    if (!entry || entry.kind !== 'sprite' || !entry.sprite) {
      return undefined;
    }
    const a = entry.sprite.attachment;
    return {
      handle,
      sourceEntity: a.sourceEntity,
      sourceSceneNode: a.sourceSceneNode,
      asset: entry.sprite.asset,
      attachmentPoint: a.attachmentPoint,
    };
  }

  /**
   * Replace a node's geometry with an uploaded voxel mesh payload. Uploads the
   * descriptor's attribute/index streams directly into a `BufferGeometry` (typed-
   * array views only — no per-frame transcoding) and maps material slots to flat
   * materials. The old geometry + materials are disposed.
   */
  #replaceMeshPayload(diff: Extract<RenderDiff, { op: 'replaceMeshPayload' }>): void {
    const entry = this.#require(diff.handle, 'replaceMeshPayload');
    const object = entry.object;
    if (!(object instanceof THREE.Mesh)) {
      throw new RenderApplyError(`replaceMeshPayload: handle ${diff.handle} is not a mesh`);
    }
    const geometry = buildMeshGeometry(diff.payload);
    const materials = diff.payload.groups.map((g) => {
      const m = new THREE.MeshBasicMaterial({ color: this.#slotColor(g.materialSlot) });
      return m;
    });

    const oldGeometry = object.geometry;
    const oldMaterial = object.material;
    object.geometry = geometry;
    // A multi-group geometry uses an array of materials indexed by group order.
    object.material = materials.length === 1 ? materials[0]! : materials;

    oldGeometry.dispose();
    if (Array.isArray(oldMaterial)) {
      oldMaterial.forEach((m) => m.dispose());
    } else {
      oldMaterial.dispose();
    }
  }

  #require(handle: RenderHandle, ctx: string): NodeEntry {
    const entry = this.#handles.get(handle);
    if (entry === undefined) {
      throw new RenderApplyError(`${ctx}: unknown handle ${handle}`);
    }
    return entry;
  }
}

// ── Snapshot lines (deterministic golden artifact) ────────────────────────────

function snapshotLine(handle: number, entry: NodeEntry): string {
  const o = entry.object;
  const head = `handle ${handle}  layer ${o.parent?.name ?? '?'}`;
  if (entry.kind === 'staticMesh') {
    return [
      head,
      `kind staticMesh`,
      `asset ${entry.asset}`,
      `pos ${fmtVec(o.position)}`,
      `scale ${fmtVec(o.scale)}`,
      `visible ${o.visible}`,
      `materials ${fmtMaterials(o)}`,
      `label ${JSON.stringify(o.name)}`,
    ].join('  ');
  }
  if (entry.kind === 'sprite' && entry.sprite) {
    const s = entry.sprite;
    const a = s.attachment;
    return [
      head,
      `kind sprite`,
      `asset ${s.asset}`,
      `frame ${s.frame}`,
      `pos ${fmtVec(o.position)}`,
      `size ${fmtNum(s.size[0])},${fmtNum(s.size[1])}`,
      `pivot ${fmtNum(s.pivot[0])},${fmtNum(s.pivot[1])}`,
      `billboard ${s.billboard}`,
      `tint ${s.tint.map(fmtNum).join(',')}`,
      `order ${o.renderOrder}`,
      `depth ${s.depth}`,
      `shading ${s.shading}`,
      `visible ${o.visible}`,
      `attach ${a.sourceEntity ?? '-'}/${a.sourceSceneNode ?? '-'}/${a.attachmentPoint ?? '-'}`,
      `label ${JSON.stringify(o.name)}`,
    ].join('  ');
  }
  return [
    head,
    `shape ${entry.shape}`,
    `pos ${fmtVec(o.position)}`,
    `scale ${fmtVec(o.scale)}`,
    `visible ${o.visible}`,
    `color ${fmtColor(o)}`,
    `label ${JSON.stringify(o.name)}`,
  ].join('  ');
}

function fmtMaterials(object: THREE.Object3D): string {
  const material = (object as THREE.Mesh).material;
  const list = Array.isArray(material) ? material : [material];
  return (
    '[' +
    list
      .map((m) => {
        const c = (m as THREE.MeshBasicMaterial).color;
        return c ? `${fmtNum(c.r)},${fmtNum(c.g)},${fmtNum(c.b)}` : 'none';
      })
      .join(' ') +
    ']'
  );
}

/** Dispose just an instance's *override* materials, leaving shared ones alone. */
function disposeInstanceOverrides(object: THREE.Object3D): void {
  const own = object.userData.ownMaterials as THREE.Material[] | undefined;
  own?.forEach((m) => m.dispose());
}

// ── Builders (contract → Three.js) ────────────────────────────────────────────

function buildObject(node: RenderNode): THREE.Object3D {
  const material = buildMaterial(node.geometry.shape, node.material);
  let object: THREE.Object3D;
  switch (node.geometry.shape) {
    case 'cube':
      object = new THREE.Mesh(new THREE.BoxGeometry(1, 1, 1), material);
      break;
    case 'sphere':
      object = new THREE.Mesh(new THREE.SphereGeometry(0.5, 8, 8), material);
      break;
    case 'quad':
      object = new THREE.Mesh(new THREE.PlaneGeometry(1, 1), material);
      break;
    case 'point':
      object = new THREE.Points(pointGeometry(), material);
      break;
    case 'line':
      object = new THREE.LineSegments(
        lineGeometry(node.geometry.a, node.geometry.b),
        material,
      );
      break;
    default: {
      const exhaustive: never = node.geometry;
      throw new RenderApplyError(`unhandled geometry ${JSON.stringify(exhaustive)}`);
    }
  }
  applyTransform(object, node.transform);
  object.visible = node.visible;
  applyMetadata(object, node.metadata);
  return object;
}

function buildMaterial(shape: Geometry['shape'], material: Material): THREE.Material {
  const color = new THREE.Color(material.color[0], material.color[1], material.color[2]);
  const opacity = material.color[3];
  const transparent = opacity < 1;
  switch (shape) {
    case 'point':
      return new THREE.PointsMaterial({ color, opacity, transparent, size: 0.1 });
    case 'line':
      return new THREE.LineBasicMaterial({ color, opacity, transparent });
    default:
      return new THREE.MeshBasicMaterial({
        color,
        opacity,
        transparent,
        wireframe: material.wireframe,
      });
  }
}

/**
 * Build a `THREE.BufferGeometry` from a mesh payload descriptor. Inline sources
 * wrap the contract number arrays as typed arrays directly; handle sources need a
 * runtime buffer provider (deferred — runtime-bridge wiring), so they are rejected
 * here with a classified error rather than silently producing an empty mesh.
 */
function buildMeshGeometry(payload: MeshPayloadDescriptor): THREE.BufferGeometry {
  if (payload.source.kind !== 'inline') {
    throw new RenderApplyError(
      'replaceMeshPayload: handle-source payloads need a runtime buffer provider (not wired yet)',
    );
  }
  const { positions, normals, indices } = payload.source;
  const geometry = new THREE.BufferGeometry();
  geometry.setAttribute('position', new THREE.BufferAttribute(new Float32Array(positions), 3));
  geometry.setAttribute('normal', new THREE.BufferAttribute(new Float32Array(normals), 3));
  geometry.setIndex(new THREE.BufferAttribute(new Uint32Array(indices), 1));
  // One draw group per material slot (BufferGeometry.addGroup(start, count, index)).
  payload.groups.forEach((g, i) => geometry.addGroup(g.start, g.count, i));
  geometry.boundingBox = new THREE.Box3(
    new THREE.Vector3(payload.bounds.min[0], payload.bounds.min[1], payload.bounds.min[2]),
    new THREE.Vector3(payload.bounds.max[0], payload.bounds.max[1], payload.bounds.max[2]),
  );
  return geometry;
}

function pointGeometry(): THREE.BufferGeometry {
  const geometry = new THREE.BufferGeometry();
  geometry.setAttribute('position', new THREE.Float32BufferAttribute([0, 0, 0], 3));
  return geometry;
}

function lineGeometry(
  a: readonly [number, number, number],
  b: readonly [number, number, number],
): THREE.BufferGeometry {
  const geometry = new THREE.BufferGeometry();
  geometry.setAttribute(
    'position',
    new THREE.Float32BufferAttribute([a[0], a[1], a[2], b[0], b[1], b[2]], 3),
  );
  return geometry;
}

function fmtNum(x: number): string {
  // Round to tame float noise; String(-0) is "0", keeping snapshots stable.
  return String(Number(x.toFixed(4)));
}

function fmtVec(v: THREE.Vector3): string {
  return `${fmtNum(v.x)},${fmtNum(v.y)},${fmtNum(v.z)}`;
}

function fmtColor(object: THREE.Object3D): string {
  const material = (object as THREE.Mesh).material;
  const single = Array.isArray(material) ? material[0] : material;
  const color = (single as THREE.MeshBasicMaterial | undefined)?.color;
  return color ? `${fmtNum(color.r)},${fmtNum(color.g)},${fmtNum(color.b)}` : 'none';
}

function applyTransform(object: THREE.Object3D, t: Transform): void {
  object.position.set(t.translation[0], t.translation[1], t.translation[2]);
  object.quaternion.set(t.rotation[0], t.rotation[1], t.rotation[2], t.rotation[3]);
  object.scale.set(t.scale[0], t.scale[1], t.scale[2]);
}

function applyMetadata(object: THREE.Object3D, metadata: RenderMetadata): void {
  object.name = metadata.label ?? '';
  object.userData = { source: metadata.source, tags: metadata.tags };
}

function applyMaterial(entry: NodeEntry, material: Material): void {
  const object = entry.object as THREE.Mesh | THREE.Points | THREE.LineSegments;
  const previous = object.material;
  object.material = buildMaterial(entry.shape, material);
  if (Array.isArray(previous)) {
    previous.forEach((m) => m.dispose());
  } else {
    previous.dispose();
  }
}

function disposeObject(object: THREE.Object3D): void {
  const disposable = object as Partial<{
    geometry: THREE.BufferGeometry;
    material: THREE.Material | THREE.Material[];
  }>;
  disposable.geometry?.dispose();
  if (Array.isArray(disposable.material)) {
    disposable.material.forEach((m) => m.dispose());
  } else {
    disposable.material?.dispose();
  }
}
