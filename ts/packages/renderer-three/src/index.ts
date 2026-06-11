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
  RenderDiff,
  RenderFrameDiff,
  RenderHandle,
  RenderLayer,
  RenderMetadata,
  RenderNode,
  Transform,
} from '@asha/contracts';

/** Raised when a diff cannot be applied (duplicate, unknown, or stale handle). */
export class RenderApplyError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'RenderApplyError';
  }
}

interface NodeEntry {
  readonly object: THREE.Object3D;
  readonly shape: Geometry['shape'];
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
    }
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
    return (
      entries
        .map(([handle, entry]) => {
          const o = entry.object;
          return [
            `handle ${handle}`,
            `layer ${o.parent?.name ?? '?'}`,
            `shape ${entry.shape}`,
            `pos ${fmtVec(o.position)}`,
            `scale ${fmtVec(o.scale)}`,
            `visible ${o.visible}`,
            `color ${fmtColor(o)}`,
            `label ${JSON.stringify(o.name)}`,
          ].join('  ');
        })
        .join('\n') + '\n'
    );
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
    this.#handles.set(diff.handle, { object, shape: diff.node.geometry.shape });
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
    disposeObject(entry.object);
    this.#handles.delete(diff.handle);
  }

  #require(handle: RenderHandle, ctx: string): NodeEntry {
    const entry = this.#handles.get(handle);
    if (entry === undefined) {
      throw new RenderApplyError(`${ctx}: unknown handle ${handle}`);
    }
    return entry;
  }
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
