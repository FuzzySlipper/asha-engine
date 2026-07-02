// @asha/render-projection - renderer-neutral retained render-diff application.
//
// This package applies generated render diffs to a typed retained projection
// model. It owns no authority, imports no renderer implementation, and never
// touches raw runtime transports. Browser/Three/WebGPU bindings consume the
// returned neutral instructions or inspect the retained snapshot.
/** Raised when a render diff cannot be applied to the retained projection. */
export class RenderProjectionError extends Error {
    constructor(message) {
        super(message);
        this.name = 'RenderProjectionError';
    }
}
/** A retained renderer-neutral projection driven only by render diffs. */
export class RenderProjection {
    #nodes = new Map();
    #materials = new Map();
    #textures = new Map();
    #spriteAtlases = new Map();
    #staticMeshes = new Map();
    /** Apply a frame in authored order and return renderer-neutral instructions. */
    applyFrame(frame) {
        const instructions = [];
        for (const diff of frame.ops) {
            instructions.push(...this.applyDiff(diff));
        }
        return instructions;
    }
    /** Apply one diff. Throws `RenderProjectionError` on stale handles or malformed payloads. */
    applyDiff(diff) {
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
                const unknown = diff;
                throw new RenderProjectionError(`unsupported render diff op ${JSON.stringify(unknown.op)}`);
            }
        }
    }
    has(handle) {
        return this.#nodes.has(handle);
    }
    get handleCount() {
        return this.#nodes.size;
    }
    node(handle) {
        const record = this.#nodes.get(handle);
        return record === undefined ? undefined : snapshotNode(record);
    }
    materialDescriptor(id) {
        return clone(this.#materials.get(id));
    }
    textureDescriptor(id) {
        return clone(this.#textures.get(id));
    }
    spriteAtlas(id) {
        return clone(this.#spriteAtlases.get(id));
    }
    staticMesh(asset) {
        return clone(this.#staticMeshes.get(asset)?.asset);
    }
    staticMeshRefCount(asset) {
        return this.#staticMeshes.get(asset)?.refCount ?? 0;
    }
    snapshot() {
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
    pickMesh(handle) {
        const payload = this.#nodes.get(handle)?.meshPayload;
        if (payload === undefined || payload === null) {
            return undefined;
        }
        return { handle, provenance: payload.provenance };
    }
    pickSprite(handle) {
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
    #create(diff) {
        this.#ensureFree(diff.handle, 'create');
        const parent = this.#parentHandle(diff.parent, 'create.parent');
        const node = clone(diff.node);
        const record = {
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
    #update(diff) {
        const record = this.#require(diff.handle, 'update');
        if (diff.transform !== null) {
            record.transform = clone(diff.transform);
            if (record.kind === 'primitive') {
                record.node = { ...record.node, transform: clone(diff.transform) };
            }
            else if (record.kind === 'staticMesh') {
                record.instance = { ...record.instance, transform: clone(diff.transform) };
            }
            else {
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
            }
            else if (record.kind === 'staticMesh') {
                record.instance = { ...record.instance, metadata: clone(diff.metadata) };
            }
            else {
                record.sprite = { ...record.sprite, metadata: clone(diff.metadata) };
            }
        }
        return { op: 'upsertNode', node: snapshotNode(record) };
    }
    #destroy(handle) {
        const record = this.#require(handle, 'destroy');
        const instructions = [];
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
    #replaceMeshPayload(diff) {
        const record = this.#require(diff.handle, 'replaceMeshPayload');
        if (record.kind === 'sprite') {
            throw new RenderProjectionError(`replaceMeshPayload: handle ${diff.handle} is a sprite`);
        }
        validateMeshPayload(diff.payload, 'replaceMeshPayload.payload');
        record.meshPayload = clone(diff.payload);
        return { op: 'upsertNode', node: snapshotNode(record) };
    }
    #defineMaterial(material) {
        this.#materials.set(material.id, clone(material));
        return { op: 'defineMaterial', material: clone(material) };
    }
    #defineTexture(texture) {
        this.#textures.set(texture.id, clone(texture));
        return { op: 'defineTexture', texture: clone(texture) };
    }
    #defineSpriteAtlas(atlas) {
        this.#spriteAtlases.set(atlas.id, clone(atlas));
        return { op: 'defineSpriteAtlas', atlas: clone(atlas) };
    }
    #defineStaticMesh(asset) {
        validateMeshPayload(asset.payload, `defineStaticMesh(${asset.asset}).payload`);
        const existing = this.#staticMeshes.get(asset.asset);
        if (existing !== undefined && existing.refCount > 0) {
            throw new RenderProjectionError(`defineStaticMesh: asset ${asset.asset} is in use by ${existing.refCount} instance(s)`);
        }
        this.#staticMeshes.set(asset.asset, { asset: clone(asset), refCount: 0 });
        return { op: 'defineStaticMesh', asset: clone(asset) };
    }
    #createStaticMeshInstance(diff) {
        this.#ensureFree(diff.handle, 'createStaticMeshInstance');
        const asset = this.#staticMeshes.get(diff.instance.asset);
        if (asset === undefined) {
            throw new RenderProjectionError(`createStaticMeshInstance: undefined static mesh asset ${diff.instance.asset}`);
        }
        const parent = this.#parentHandle(diff.parent, 'createStaticMeshInstance.parent');
        const instance = clone(diff.instance);
        const record = {
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
    #createSprite(diff) {
        this.#ensureFree(diff.handle, 'createSprite');
        const parent = this.#parentHandle(diff.parent, 'createSprite.parent');
        const sprite = clone(diff.sprite);
        const record = {
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
    #updateSprite(diff) {
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
    #resolveSpriteUv(asset, frame) {
        const atlas = this.#spriteAtlases.get(asset);
        const rect = atlas?.frames.find((candidate) => candidate.frame === frame);
        if (rect === undefined) {
            return [0, 0, 1, 1];
        }
        return [rect.uvMin[0], rect.uvMin[1], rect.uvMax[0], rect.uvMax[1]];
    }
    #insert(record) {
        this.#nodes.set(record.handle, record);
        if (record.parent !== null) {
            this.#require(record.parent, 'insert.parent').children.add(record.handle);
        }
    }
    #ensureFree(handle, ctx) {
        if (this.#nodes.has(handle)) {
            throw new RenderProjectionError(`${ctx}: handle ${handle} already exists`);
        }
    }
    #parentHandle(parent, ctx) {
        if (parent !== null) {
            this.#require(parent, ctx);
        }
        return parent;
    }
    #require(handle, ctx) {
        const record = this.#nodes.get(handle);
        if (record === undefined) {
            throw new RenderProjectionError(`${ctx}: unknown handle ${handle}`);
        }
        return record;
    }
}
function snapshotNode(record) {
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
function validateMeshPayload(payload, ctx) {
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
                throw new RenderProjectionError(`${ctx}.source.indices[${i}] ${value} is out of range for ${vertexCount} vertices`);
            }
        });
    }
    else {
        requireNonNegativeInteger(payload.source.buffer, `${ctx}.source.buffer`);
        requireNonNegativeInteger(payload.source.positionsByteOffset, `${ctx}.source.positionsByteOffset`);
        requireNonNegativeInteger(payload.source.normalsByteOffset, `${ctx}.source.normalsByteOffset`);
        requireNonNegativeInteger(payload.source.indicesByteOffset, `${ctx}.source.indicesByteOffset`);
    }
    for (let i = 0; i < payload.groups.length; i += 1) {
        const group = payload.groups[i];
        const start = requireNonNegativeInteger(group.start, `${ctx}.groups[${i}].start`);
        const count = requireNonNegativeInteger(group.count, `${ctx}.groups[${i}].count`);
        requireNonNegativeInteger(group.materialSlot, `${ctx}.groups[${i}].materialSlot`);
        if (start + count > indexCount) {
            throw new RenderProjectionError(`${ctx}.groups[${i}] window [${start}, ${start + count}) exceeds indexCount ${indexCount}`);
        }
    }
}
function attributeComponents(payload, name, ctx) {
    const attribute = payload.layout.attributes.find((candidate) => candidate.name === name);
    if (attribute === undefined) {
        throw new RenderProjectionError(`${ctx}.layout.attributes missing ${name}`);
    }
    return requireNonNegativeInteger(attribute.components, `${ctx}.layout.attributes.${name}.components`);
}
function requireLength(values, expected, ctx) {
    if (values.length !== expected) {
        throw new RenderProjectionError(`${ctx} expected length ${expected}, got ${values.length}`);
    }
}
function requireNonNegativeInteger(value, ctx) {
    if (!Number.isInteger(value) || value < 0) {
        throw new RenderProjectionError(`${ctx} must be a non-negative integer`);
    }
    return value;
}
function sortedHandles(map) {
    return [...map.keys()].sort(numberCompare);
}
function sortedValues(map) {
    return [...map.values()].map((value) => clone(value)).sort((a, b) => a.id.localeCompare(b.id));
}
function numberCompare(a, b) {
    return a - b;
}
function clone(value) {
    if (value === undefined) {
        return value;
    }
    return JSON.parse(JSON.stringify(value));
}
//# sourceMappingURL=index.js.map