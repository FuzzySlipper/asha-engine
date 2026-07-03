// @asha/renderer-three — a minimal retained-mode renderer shell.
//
// It applies create/update/destroy render diffs (generated `@asha/contracts`
// types) to a placeholder Three.js scene through a handle registry. It is a
// thin *projector consumer*: it never reads authority state, never validates,
// and imports no policy/core packages. Building the scene graph needs no GL
// context (only pixel rendering does), so this is testable headlessly; a real
// WebGL/offscreen renderer for screenshots is layered on in a later task.
import * as THREE from 'three';
import { RenderProjection } from '@asha/render-projection';
import { decodeRenderFrameDiff, RuntimeBridgeError, } from '@asha/runtime-bridge';
export * from './static-room.js';
/** Raised when a diff cannot be applied (duplicate, unknown, or stale handle). */
export class RenderApplyError extends Error {
    constructor(message) {
        super(message);
        this.name = 'RenderApplyError';
    }
}
/**
 * A retained Three.js scene driven entirely by render diffs.
 *
 * Nodes are addressed by `RenderHandle`; the registry maps each handle to a
 * Three.js `Object3D`. Scene and debug layers are separate groups so overlays
 * can be toggled independently.
 */
export class ThreeRenderer {
    scene = new THREE.Scene();
    #sceneGroup = new THREE.Group();
    #debugGroup = new THREE.Group();
    #handles = new Map();
    /** Defined static mesh assets, keyed by asset id (shared geometry lifecycle). */
    #staticMeshes = new Map();
    /** Per-material-slot colours for the initial flat/debug material strategy. */
    #slotColors = new Map();
    /** Catalog material descriptors, keyed by material asset id (#2373). */
    #materials = new Map();
    /** How many times a slot fell back to a placeholder (no catalog descriptor). */
    #fallbackMaterialCount = 0;
    /** Catalog material ids that fell back to a placeholder (fallback diagnostic). */
    #fallbackMaterials = new Set();
    /** Texture descriptors, keyed by texture asset id (#2374). */
    #textures = new Map();
    /** Sprite atlas descriptors, keyed by sprite-sheet asset id (#2374). */
    #atlases = new Map();
    /** How many times a sprite frame fell back to full UVs (no atlas/frame). */
    #spriteFallbackCount = 0;
    /**
     * Optional runtime buffer source for handle-backed mesh payloads. When absent,
     * handle sources fail closed (the inline fixture path still works for goldens).
     */
    #meshBufferSource;
    constructor(options = {}) {
        this.#meshBufferSource = options.meshBufferSource;
        this.#sceneGroup.name = 'scene';
        this.#debugGroup.name = 'debug';
        this.scene.add(this.#sceneGroup, this.#debugGroup);
    }
    #layerGroup(layer) {
        return layer === 'debug' ? this.#debugGroup : this.#sceneGroup;
    }
    /** Apply a whole frame of diffs in order. */
    applyFrame(frame) {
        for (const op of frame.ops) {
            this.applyDiff(op);
        }
    }
    /** Decode a raw payload through `@asha/runtime-bridge` and apply it. */
    applyEncodedFrame(payload) {
        this.applyFrame(decodeRenderFrameDiff(payload));
    }
    /** Apply a single diff. Throws `RenderApplyError` on a bad handle. */
    applyDiff(diff) {
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
            case 'defineMaterial':
                this.#defineMaterial(diff.material);
                break;
            case 'defineTexture':
                this.#textures.set(diff.texture.id, diff.texture);
                break;
            case 'defineSpriteAtlas':
                this.#atlases.set(diff.atlas.id, diff.atlas);
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
    registerSlotColor(slot, r, g, b) {
        this.#slotColors.set(slot, new THREE.Color(r, g, b));
    }
    #slotColor(slot) {
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
    has(handle) {
        return this.#handles.has(handle);
    }
    /** Number of live scene handles. */
    get handleCount() {
        return this.#handles.size;
    }
    /** The Three.js object for a handle, for inspection/tests. */
    objectFor(handle) {
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
    snapshot() {
        const entries = [...this.#handles.entries()].sort((a, b) => a[0] - b[0]);
        if (entries.length === 0) {
            return '(empty scene)\n';
        }
        return entries.map(([handle, entry]) => snapshotLine(handle, entry)).join('\n') + '\n';
    }
    #create(diff) {
        if (this.#handles.has(diff.handle)) {
            throw new RenderApplyError(`create: handle ${diff.handle} already exists`);
        }
        const object = buildObject(diff.node);
        const parent = diff.parent === null
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
    #update(diff) {
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
    #destroy(diff) {
        const entry = this.#require(diff.handle, 'destroy');
        entry.object.parent?.remove(entry.object);
        if (entry.kind === 'staticMesh' && entry.asset !== undefined) {
            // Shared geometry: dispose only this instance's override materials, then
            // release the asset reference. The asset's geometry is disposed only when
            // its last instance is gone (reference-safe — never while another shares it).
            disposeInstanceOverrides(entry.object);
            this.#releaseStaticMesh(entry.asset);
        }
        else {
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
    #defineStaticMesh(asset) {
        const existing = this.#staticMeshes.get(asset.asset);
        if (existing) {
            if (existing.refCount > 0) {
                throw new RenderApplyError(`defineStaticMesh: asset ${asset.asset} is in use by ${existing.refCount} instance(s)`);
            }
            existing.geometry.dispose();
            existing.materials.forEach((m) => m.dispose());
        }
        // Inline and handle-backed payloads both upload here (#2428): a handle-backed
        // static mesh asset borrows the bridge buffer, copies its bytes out, and
        // releases the borrow. A missing provider / unknown / stale / too-small buffer
        // fails closed below — never silently producing empty geometry.
        const geometry = buildMeshGeometry(asset.payload, this.#meshBufferSource, 'defineStaticMesh');
        const slotIndex = new Map();
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
    #createStaticMeshInstance(diff) {
        if (this.#handles.has(diff.handle)) {
            throw new RenderApplyError(`createStaticMeshInstance: handle ${diff.handle} already exists`);
        }
        const def = this.#staticMeshes.get(diff.instance.asset);
        if (!def) {
            throw new RenderApplyError(`createStaticMeshInstance: undefined static mesh asset ${diff.instance.asset}`);
        }
        // Materials default to the asset's; per-instance overrides clone-replace just
        // the named slots, so two instances of one asset can differ in material while
        // sharing one BufferGeometry.
        const materials = def.materials.slice();
        // Catalog material id behind each material-array entry (for live redefine).
        const materialIds = def.materialSlots.map((s) => s.material);
        const ownMaterials = [];
        for (const ov of diff.instance.materialOverrides) {
            const idx = def.slotIndex.get(ov.slot);
            if (idx === undefined) {
                throw new RenderApplyError(`createStaticMeshInstance: override for unbound slot ${ov.slot} on ${diff.instance.asset}`);
            }
            const m = this.#materialFor(ov);
            materials[idx] = m;
            materialIds[idx] = ov.material;
            ownMaterials.push(m);
        }
        const mesh = new THREE.Mesh(def.geometry, materials.length === 1 ? materials[0] : materials);
        // Instance-owned override materials (disposed on destroy; shared ones aren't).
        mesh.userData.ownMaterials = ownMaterials;
        applyTransform(mesh, diff.instance.transform);
        applyMetadata(mesh, diff.instance.metadata);
        const parent = diff.parent === null ? this.#sceneGroup : this.#require(diff.parent, 'createStaticMeshInstance.parent').object;
        parent.add(mesh);
        def.refCount += 1;
        this.#handles.set(diff.handle, {
            object: mesh,
            kind: 'staticMesh',
            shape: 'quad',
            asset: diff.instance.asset,
            ownsGeometry: false,
            materialIds,
        });
    }
    #releaseStaticMesh(asset) {
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
    instanceCountFor(asset) {
        return this.#staticMeshes.get(asset)?.refCount ?? 0;
    }
    /**
     * Register (or replace) a catalog material descriptor by id (#2373). The
     * renderer resolves a static-mesh slot or sprite ref to this descriptor so a
     * mesh renders its real catalog colour/texture instead of a placeholder hue.
     * Authority/collision flags never reach here — the descriptor is the disjoint
     * visual projection (boundary 18).
     */
    /**
     * Register (or replace) a catalog material descriptor by id (#2373/#2376). A
     * *redefine* of an already-registered id is a live visual-only update: every
     * static-mesh material bound to that id is rebuilt from the new descriptor and
     * the old material disposed (leak-safe), so a visual edit changes the rendered
     * output deterministically without a destroy+create. Authority-impacting changes
     * are NOT applied here — the catalog change-impact path (Rust
     * `material_change_impact`) classifies those as requires-reload before they ever
     * reach the renderer.
     */
    #defineMaterial(material) {
        const isRedefine = this.#materials.has(material.id);
        this.#materials.set(material.id, material);
        if (isRedefine) {
            this.#replaceLiveMaterial(material.id);
        }
    }
    /** Rebuild every live static-mesh material bound to `id`, disposing the old. */
    #replaceLiveMaterial(id) {
        for (const entry of this.#handles.values()) {
            if (entry.kind !== 'staticMesh' || !entry.materialIds) {
                continue;
            }
            const mesh = entry.object;
            const arr = Array.isArray(mesh.material) ? mesh.material : [mesh.material];
            let changed = false;
            for (let i = 0; i < entry.materialIds.length; i += 1) {
                if (entry.materialIds[i] !== id) {
                    continue;
                }
                const replacement = this.#materialFor({ slot: i, material: id });
                arr[i]?.dispose();
                arr[i] = replacement;
                changed = true;
            }
            if (changed) {
                mesh.material = arr.length === 1 ? arr[0] : arr;
            }
        }
    }
    /** A registered catalog material descriptor by id, for inspection/tests. */
    materialDescriptor(id) {
        return this.#materials.get(id);
    }
    /** Total placeholder-fallback material resolutions so far (fallback diagnostic). */
    get fallbackMaterialCount() {
        return this.#fallbackMaterialCount;
    }
    /** Catalog material ids that resolved to a placeholder fallback (no descriptor). */
    fallbackMaterials() {
        return [...this.#fallbackMaterials].sort();
    }
    #materialFor(slot) {
        // Resolve the slot's catalog material id → registered RenderMaterialDescriptor
        // (defineMaterial). A descriptor drives the real catalog colour; a missing one
        // falls back deterministically to the per-slot hue and is recorded (id + count)
        // so the gap is an observable diagnostic rather than silent (#2373/#2376).
        const descriptor = this.#materials.get(slot.material);
        if (descriptor) {
            const [r, g, b] = descriptor.color;
            return new THREE.MeshBasicMaterial({ color: new THREE.Color(r, g, b) });
        }
        this.#fallbackMaterialCount += 1;
        this.#fallbackMaterials.add(slot.material);
        return new THREE.MeshBasicMaterial({ color: this.#slotColor(slot.slot) });
    }
    /** A registered texture descriptor by id, for inspection/tests. */
    textureDescriptor(id) {
        return this.#textures.get(id);
    }
    /** A registered sprite atlas by id, for inspection/tests. */
    spriteAtlas(id) {
        return this.#atlases.get(id);
    }
    /** Total sprite-frame fallbacks (no atlas / unknown frame) so far. */
    get spriteFallbackCount() {
        return this.#spriteFallbackCount;
    }
    /**
     * Resolve a sprite asset + frame to its atlas UV sub-rectangle and write it into
     * the plane geometry's `uv` attribute (#2374). A missing atlas or unknown frame
     * falls back deterministically to full `[0,1]` UVs and is counted, so the gap is
     * observable rather than a silent wrong-frame. Returns the resolved rect
     * `[u0,v0,u1,v1]` (or the full-UV fallback) for the snapshot.
     */
    #applySpriteUv(geometry, asset, frame) {
        const atlas = this.#atlases.get(asset);
        const rect = atlas?.frames.find((f) => f.frame === frame);
        if (!rect) {
            if (atlas !== undefined || this.#textures.size > 0 || frame !== 0) {
                this.#spriteFallbackCount += 1;
            }
            return [0, 0, 1, 1];
        }
        const [u0, v0] = rect.uvMin;
        const [u1, v1] = rect.uvMax;
        // PlaneGeometry vertex order: top-left, top-right, bottom-left, bottom-right.
        const uv = geometry.getAttribute('uv');
        uv.setXY(0, u0, v1);
        uv.setXY(1, u1, v1);
        uv.setXY(2, u0, v0);
        uv.setXY(3, u1, v0);
        uv.needsUpdate = true;
        return [u0, v0, u1, v1];
    }
    // ── Sprites / billboards (render-asset-05/06) ───────────────────────────────
    #createSprite(diff) {
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
        mesh.userData.uv = this.#applySpriteUv(geometry, s.asset, s.frame);
        const parent = diff.parent === null ? this.#sceneGroup : this.#require(diff.parent, 'createSprite.parent').object;
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
    #updateSprite(diff) {
        const entry = this.#require(diff.handle, 'updateSprite');
        if (entry.kind !== 'sprite' || !entry.sprite) {
            throw new RenderApplyError(`updateSprite: handle ${diff.handle} is not a sprite`);
        }
        const mesh = entry.object;
        const material = mesh.material;
        if (diff.frame !== null) {
            entry.sprite = { ...entry.sprite, frame: diff.frame };
            mesh.userData.frame = diff.frame;
            // Re-resolve the atlas UV rect for the new frame (deterministic, no anim).
            mesh.userData.uv = this.#applySpriteUv(mesh.geometry, entry.sprite.asset, diff.frame);
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
    pickSprite(handle) {
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
    #replaceMeshPayload(diff) {
        const entry = this.#require(diff.handle, 'replaceMeshPayload');
        const object = entry.object;
        if (!(object instanceof THREE.Mesh)) {
            throw new RenderApplyError(`replaceMeshPayload: handle ${diff.handle} is not a mesh`);
        }
        const geometry = buildMeshGeometry(diff.payload, this.#meshBufferSource, 'replaceMeshPayload');
        const materials = diff.payload.groups.map((g) => {
            const m = new THREE.MeshBasicMaterial({ color: this.#slotColor(g.materialSlot) });
            return m;
        });
        const oldGeometry = object.geometry;
        const oldMaterial = object.material;
        object.geometry = geometry;
        // A multi-group geometry uses an array of materials indexed by group order.
        object.material = materials.length === 1 ? materials[0] : materials;
        oldGeometry.dispose();
        if (Array.isArray(oldMaterial)) {
            oldMaterial.forEach((m) => m.dispose());
        }
        else {
            oldMaterial.dispose();
        }
        // Remember the authority source that produced this mesh so a pick can trace the
        // handle back to it (#2437). The renderer holds the provenance, never the coords.
        entry.meshProvenance = diff.payload.provenance;
    }
    /**
     * Resolve a renderer-side mesh pick to an authority source trace: the render handle
     * + the provenance of the uploaded mesh. Only a **hint** — authority picking
     * (`pickVoxel`) revalidates before any selection/edit acts on it. Returns
     * `undefined` for a handle with no uploaded mesh, or a stale/destroyed/unknown
     * handle (fail closed — the renderer never invents a source for missing metadata).
     */
    pickMesh(handle) {
        const entry = this.#handles.get(handle);
        if (!entry || entry.meshProvenance === undefined) {
            return undefined;
        }
        return { handle, provenance: entry.meshProvenance };
    }
    #require(handle, ctx) {
        const entry = this.#handles.get(handle);
        if (entry === undefined) {
            throw new RenderApplyError(`${ctx}: unknown handle ${handle}`);
        }
        return entry;
    }
}
/**
 * Apply a render frame through the renderer-neutral projection and then the
 * retained Three.js renderer. This is the package-root bridge used by demo
 * proofs: no authority state, no raw transport, no arbitrary JSON tunnel.
 */
export function renderProjectedFrame(frame, renderer = new ThreeRenderer()) {
    const projection = new RenderProjection();
    projection.applyFrame(frame);
    renderer.applyFrame(frame);
    return {
        projection,
        renderer,
        structuralSnapshot: renderer.snapshot(),
    };
}
// ── Snapshot lines (deterministic golden artifact) ────────────────────────────
function snapshotLine(handle, entry) {
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
            `uv ${(o.userData.uv ?? [0, 0, 1, 1]).map(fmtNum).join(',')}`,
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
function fmtMaterials(object) {
    const material = object.material;
    const list = Array.isArray(material) ? material : [material];
    return ('[' +
        list
            .map((m) => {
            const c = m.color;
            return c ? `${fmtNum(c.r)},${fmtNum(c.g)},${fmtNum(c.b)}` : 'none';
        })
            .join(' ') +
        ']');
}
/** Dispose just an instance's *override* materials, leaving shared ones alone. */
function disposeInstanceOverrides(object) {
    const own = object.userData.ownMaterials;
    own?.forEach((m) => m.dispose());
}
// ── Builders (contract → Three.js) ────────────────────────────────────────────
function buildObject(node) {
    const material = buildMaterial(node.geometry.shape, node.material);
    let object;
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
            object = new THREE.LineSegments(lineGeometry(node.geometry.a, node.geometry.b), material);
            break;
        default: {
            const exhaustive = node.geometry;
            throw new RenderApplyError(`unhandled geometry ${JSON.stringify(exhaustive)}`);
        }
    }
    applyTransform(object, node.transform);
    object.visible = node.visible;
    applyMetadata(object, node.metadata);
    return object;
}
function buildMaterial(shape, material) {
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
 * wrap the contract number arrays as typed arrays directly; handle sources resolve
 * the bridge-owned bytes through the optional {@link MeshBufferSource} and slice the
 * attribute/index streams out by byte offset. A handle source with no provider, an
 * unknown/stale handle, or a buffer too small for the declared layout fails closed
 * with a classified `RenderApplyError` — never a silent empty mesh.
 */
function buildMeshGeometry(payload, bufferSource, ctx) {
    const streams = payload.source.kind === 'inline'
        ? inlineStreams(payload.source)
        : handleStreams(payload, payload.source, bufferSource, ctx);
    const positionComponents = attributeComponents(payload, 'position');
    const normalComponents = attributeComponents(payload, 'normal');
    const geometry = new THREE.BufferGeometry();
    geometry.setAttribute('position', new THREE.BufferAttribute(streams.positions, positionComponents));
    geometry.setAttribute('normal', new THREE.BufferAttribute(streams.normals, normalComponents));
    geometry.setIndex(new THREE.BufferAttribute(streams.indices, 1));
    // One draw group per material slot (BufferGeometry.addGroup(start, count, index)).
    payload.groups.forEach((g, i) => geometry.addGroup(g.start, g.count, i));
    geometry.boundingBox = new THREE.Box3(new THREE.Vector3(payload.bounds.min[0], payload.bounds.min[1], payload.bounds.min[2]), new THREE.Vector3(payload.bounds.max[0], payload.bounds.max[1], payload.bounds.max[2]));
    return geometry;
}
/** Wrap inline contract number arrays as typed arrays (the golden-fixture path). */
function inlineStreams(source) {
    return {
        positions: new Float32Array(source.positions),
        normals: new Float32Array(source.normals),
        indices: new Uint32Array(source.indices),
    };
}
/**
 * Resolve a handle-backed payload's bytes under the **borrow → copy → release**
 * contract (#2428): borrow the buffer, copy every declared stream out immediately
 * (so the borrow is never retained), then release the borrow. The borrow is
 * released on both the success and the failure path; a missing provider, an
 * unknown/stale/expired handle, an out-of-bounds window, or an out-of-range index
 * all fail closed with a classified `RenderApplyError` — never empty geometry.
 */
function handleStreams(payload, source, bufferSource, ctx) {
    if (bufferSource === undefined) {
        throw new RenderApplyError(`${ctx}: handle-source payload needs a runtime buffer provider (buffer ${source.buffer})`);
    }
    const handle = source.buffer;
    let view;
    try {
        view = bufferSource.getBuffer(handle);
    }
    catch (cause) {
        // No borrow was acquired, so nothing to release. Classify and fail closed.
        throw classifyBufferError(cause, source.buffer, ctx, 'unavailable');
    }
    // Borrow acquired — copy out, then release exactly once on every exit path.
    let streams;
    try {
        streams = copyHandleStreams(view, payload, source, ctx);
    }
    catch (cause) {
        releaseBorrowBestEffort(bufferSource, handle); // failure path: never mask the cause
        throw cause;
    }
    // Success path: release and surface a classified error if release itself fails.
    releaseBorrow(bufferSource, handle, source.buffer, ctx);
    return streams;
}
/** Copy + validate the three streams out of a borrowed view (no borrow retained). */
function copyHandleStreams(view, payload, source, ctx) {
    const { vertexCount, indexCount } = payload.layout;
    const positionComponents = attributeComponents(payload, 'position');
    const normalComponents = attributeComponents(payload, 'normal');
    const positions = sliceFloat32(view, source.positionsByteOffset, vertexCount * positionComponents, 'positions', source.buffer, ctx);
    const normals = sliceFloat32(view, source.normalsByteOffset, vertexCount * normalComponents, 'normals', source.buffer, ctx);
    const indices = sliceUint32(view, source.indicesByteOffset, indexCount, source.buffer, ctx);
    for (let i = 0; i < indices.length; i++) {
        if (indices[i] >= vertexCount) {
            throw new RenderApplyError(`${ctx}: index ${indices[i]} out of range for ${vertexCount} vertices (buffer ${source.buffer})`);
        }
    }
    return { positions, normals, indices };
}
/** Map a classified bridge error to a renderer-boundary `RenderApplyError`. */
function classifyBufferError(cause, buffer, ctx, what) {
    if (cause instanceof RuntimeBridgeError) {
        return new RenderApplyError(`${ctx}: buffer ${buffer} ${what} [${cause.kind}]: ${cause.message}`);
    }
    return cause;
}
/** Release a borrow on the success path; a release failure is classified, not hidden. */
function releaseBorrow(bufferSource, handle, buffer, ctx) {
    try {
        bufferSource.releaseBuffer(handle);
    }
    catch (cause) {
        throw classifyBufferError(cause, buffer, ctx, 'release failed');
    }
}
/** Release a borrow on a failure path; swallow release errors so the original
 *  failure (the reason we are unwinding) is the one the caller sees. */
function releaseBorrowBestEffort(bufferSource, handle) {
    try {
        bufferSource.releaseBuffer(handle);
    }
    catch {
        // best-effort: the copy/validation error already in flight is the primary one
    }
}
/** Components-per-vertex for a declared attribute (defaults to 3 if unspecified). */
function attributeComponents(payload, name) {
    const attribute = payload.layout.attributes.find((a) => a.name === name);
    return attribute?.components ?? 3;
}
/** Copy `count` f32s out of a borrowed buffer at `byteOffset`, failing closed if out of bounds. */
function sliceFloat32(view, byteOffset, count, label, buffer, ctx) {
    const byteLength = count * Float32Array.BYTES_PER_ELEMENT;
    const bytes = requireBytes(view, byteOffset, byteLength, label, buffer, ctx);
    return new Float32Array(bytes.buffer, bytes.byteOffset, count);
}
/** Copy `count` u32s out of a borrowed buffer at `byteOffset`, failing closed if out of bounds. */
function sliceUint32(view, byteOffset, count, buffer, ctx) {
    const byteLength = count * Uint32Array.BYTES_PER_ELEMENT;
    const bytes = requireBytes(view, byteOffset, byteLength, 'indices', buffer, ctx);
    return new Uint32Array(bytes.buffer, bytes.byteOffset, count);
}
/**
 * Copy a `[byteOffset, byteOffset+byteLength)` window out of the borrowed view into
 * a fresh, alignment-safe buffer. Throws a classified `RenderApplyError` if the
 * window does not fit — a stale/wrong-layout handle must not read past its bytes.
 */
function requireBytes(view, byteOffset, byteLength, label, buffer, ctx) {
    if (byteOffset < 0 || byteOffset + byteLength > view.bytes.length) {
        throw new RenderApplyError(`${ctx}: ${label} window [${byteOffset}, ${byteOffset + byteLength}) ` +
            `exceeds buffer ${buffer} length ${view.bytes.length}`);
    }
    // slice() returns a fresh ArrayBuffer at offset 0 — a copy-out that drops the
    // borrow and guarantees 4-byte alignment for the typed-array views above.
    return view.bytes.slice(byteOffset, byteOffset + byteLength);
}
function pointGeometry() {
    const geometry = new THREE.BufferGeometry();
    geometry.setAttribute('position', new THREE.Float32BufferAttribute([0, 0, 0], 3));
    return geometry;
}
function lineGeometry(a, b) {
    const geometry = new THREE.BufferGeometry();
    geometry.setAttribute('position', new THREE.Float32BufferAttribute([a[0], a[1], a[2], b[0], b[1], b[2]], 3));
    return geometry;
}
function fmtNum(x) {
    // Round to tame float noise; String(-0) is "0", keeping snapshots stable.
    return String(Number(x.toFixed(4)));
}
function fmtVec(v) {
    return `${fmtNum(v.x)},${fmtNum(v.y)},${fmtNum(v.z)}`;
}
function fmtColor(object) {
    const material = object.material;
    const single = Array.isArray(material) ? material[0] : material;
    const color = single?.color;
    return color ? `${fmtNum(color.r)},${fmtNum(color.g)},${fmtNum(color.b)}` : 'none';
}
function applyTransform(object, t) {
    object.position.set(t.translation[0], t.translation[1], t.translation[2]);
    object.quaternion.set(t.rotation[0], t.rotation[1], t.rotation[2], t.rotation[3]);
    object.scale.set(t.scale[0], t.scale[1], t.scale[2]);
}
function applyMetadata(object, metadata) {
    object.name = metadata.label ?? '';
    object.userData = { source: metadata.source, tags: metadata.tags };
}
function applyMaterial(entry, material) {
    const object = entry.object;
    const previous = object.material;
    object.material = buildMaterial(entry.shape, material);
    if (Array.isArray(previous)) {
        previous.forEach((m) => m.dispose());
    }
    else {
        previous.dispose();
    }
}
function disposeObject(object) {
    const disposable = object;
    disposable.geometry?.dispose();
    if (Array.isArray(disposable.material)) {
        disposable.material.forEach((m) => m.dispose());
    }
    else {
        disposable.material?.dispose();
    }
}
//# sourceMappingURL=index.js.map