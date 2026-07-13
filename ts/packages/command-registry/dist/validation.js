const REQUIRED_METADATA_FIELDS = [
    'id',
    'version',
    'label',
    'summary',
    'category',
    'menuPath',
    'commandPalette',
    'inputSchema',
    'outputSchema',
    'operationClass',
    'agentExposure',
    'guiMirror',
    'undo',
    'retry',
    'idempotency',
    'artifacts',
    'stateImpact',
    'owningLane',
    'owningPackage',
    'runtimeRequirements',
    'compatibility',
];
function commandLabel(definition) {
    return definition.id ?? '<missing id>';
}
function hasOwn(definition, field) {
    return Object.prototype.hasOwnProperty.call(definition, field);
}
function mutatesOrWrites(impact) {
    return impact.authority === 'mutate' || impact.editor === 'mutate' || impact.render === 'capture' || impact.workspace === 'write';
}
function isNonEmptyString(value) {
    return typeof value === 'string' && value.trim().length > 0;
}
function arraysEqual(left, right) {
    if (left === undefined || right === undefined || left.length !== right.length) {
        return false;
    }
    return left.every((value, index) => value === right[index]);
}
function visitSchemaShape(commandId, fieldPath, shape, issues) {
    switch (shape.kind) {
        case 'empty':
        case 'contract':
        case 'literal':
        case 'scalar':
            return;
        case 'object':
            if (shape.allowExtraFields !== false) {
                issues.push({ commandId, field: fieldPath, message: 'object schemas must fail closed with allowExtraFields=false' });
            }
            for (const field of shape.fields) {
                visitSchemaShape(commandId, `${fieldPath}.${field.name}`, field.shape, issues);
            }
            return;
        case 'array':
            visitSchemaShape(commandId, `${fieldPath}[]`, shape.items, issues);
            return;
        case 'nullable':
            visitSchemaShape(commandId, `${fieldPath}?`, shape.inner, issues);
            return;
    }
}
function hasField(value, fieldName) {
    return Object.prototype.hasOwnProperty.call(value, fieldName);
}
function isPlainObject(value) {
    return typeof value === 'object' && value !== null && !Array.isArray(value);
}
function hasExactKeys(value, keys) {
    const actual = Object.keys(value);
    return actual.length === keys.length && keys.every((key) => hasField(value, key));
}
function isFiniteNumber(value) {
    return typeof value === 'number' && Number.isFinite(value);
}
function isInteger(value) {
    return typeof value === 'number' && Number.isInteger(value);
}
function isString(value) {
    return typeof value === 'string';
}
function isNumberTuple3(value) {
    return Array.isArray(value) && value.length === 3 && value.every(isFiniteNumber);
}
function isNumberTuple2(value) {
    return Array.isArray(value) && value.length === 2 && value.every(isFiniteNumber);
}
function isNumberTuple4(value) {
    return Array.isArray(value) && value.length === 4 && value.every(isFiniteNumber);
}
function isNumberTuple16(value) {
    return Array.isArray(value) && value.length === 16 && value.every(isFiniteNumber);
}
function isLiteral(value, allowed) {
    return typeof value === 'string' && allowed.includes(value);
}
function isVoxelCoord(value) {
    return isPlainObject(value) && hasExactKeys(value, ['x', 'y', 'z']) && isInteger(value.x) && isInteger(value.y) && isInteger(value.z);
}
function isVoxelValue(value) {
    if (!isPlainObject(value) || !hasField(value, 'kind')) {
        return false;
    }
    if (value.kind === 'empty') {
        return hasExactKeys(value, ['kind']);
    }
    return value.kind === 'solid' && hasExactKeys(value, ['kind', 'material']) && isInteger(value.material);
}
function isVoxelCommand(value) {
    if (!isPlainObject(value) || !hasField(value, 'op')) {
        return false;
    }
    if (value.op === 'setVoxel') {
        return hasExactKeys(value, ['op', 'grid', 'coord', 'value']) && isInteger(value.grid) && isVoxelCoord(value.coord) && isVoxelValue(value.value);
    }
    if (value.op === 'fillRegion') {
        return hasExactKeys(value, ['op', 'grid', 'min', 'max', 'value']) && isInteger(value.grid) && isVoxelCoord(value.min) && isVoxelCoord(value.max) && isVoxelValue(value.value);
    }
    return value.op === 'generateChunk' && hasExactKeys(value, ['op', 'grid', 'chunk', 'seed', 'generatorVersion']) && isInteger(value.grid) && isVoxelCoord(value.chunk) && isInteger(value.seed) && isInteger(value.generatorVersion);
}
function isViewport(value) {
    return value === null || (isPlainObject(value) && hasExactKeys(value, ['width', 'height']) && isFiniteNumber(value.width) && isFiniteNumber(value.height));
}
function isScreenPoint(value) {
    return isPlainObject(value) && hasExactKeys(value, ['x', 'y', 'space']) && isFiniteNumber(value.x) && isFiniteNumber(value.y) && isLiteral(value.space, ['normalized_0_1', 'pixel']);
}
function isScreenPointToPickRayRequest(value) {
    return isPlainObject(value) && hasExactKeys(value, ['camera', 'grid', 'viewport', 'screenPoint', 'maxDistance']) && isInteger(value.camera) && isInteger(value.grid) && isViewport(value.viewport) && isScreenPoint(value.screenPoint) && isFiniteNumber(value.maxDistance);
}
function isPickRaySnapshot(value) {
    return isPlainObject(value) && hasExactKeys(value, ['camera', 'tick', 'grid', 'screenPoint', 'origin', 'direction', 'maxDistance', 'cameraProjectionHash', 'rayHash']) && isInteger(value.camera) && isInteger(value.tick) && isInteger(value.grid) && isScreenPoint(value.screenPoint) && isNumberTuple3(value.origin) && isNumberTuple3(value.direction) && isFiniteNumber(value.maxDistance) && isString(value.cameraProjectionHash) && isString(value.rayHash);
}
function isVoxelSelectionSnapshot(value) {
    return isPlainObject(value) && hasExactKeys(value, ['pickRay', 'outcome', 'selectedVoxel', 'selectedFace', 'editAnchor', 'selectionHash']) && isPickRaySnapshot(value.pickRay) && isLiteral(value.outcome, ['hit', 'miss']) && (value.selectedVoxel === null || isVoxelCoord(value.selectedVoxel)) && (value.selectedFace === null || isLiteral(value.selectedFace, ['posX', 'negX', 'posY', 'negY', 'posZ', 'negZ'])) && (value.editAnchor === null || isVoxelCoord(value.editAnchor)) && isString(value.selectionHash);
}
function isAssetReference(value) {
    return isPlainObject(value) && hasExactKeys(value, ['id', 'kind']) && isString(value.id) && isLiteral(value.kind, ['material', 'mesh', 'sprite', 'sprite-sheet', 'texture', 'voxel-volume', 'voxel-object', 'script', 'scene']);
}
function isRgbaObject(value) {
    return isPlainObject(value) && hasExactKeys(value, ['r', 'g', 'b', 'a']) && isFiniteNumber(value.r) && isFiniteNumber(value.g) && isFiniteNumber(value.b) && isFiniteNumber(value.a);
}
function isMaterialProjection(value) {
    if (!isPlainObject(value) || !hasExactKeys(value, ['render', 'collision']))
        return false;
    const render = value.render;
    const collision = value.collision;
    return isPlainObject(render)
        && hasExactKeys(render, ['color', 'texture', 'roughness', 'textureTint', 'emissionColor', 'emissive', 'uvStrategy'])
        && isRgbaObject(render.color)
        && (render.texture === null || isAssetReference(render.texture))
        && isFiniteNumber(render.roughness)
        && isRgbaObject(render['textureTint'])
        && isRgbaObject(render['emissionColor'])
        && isFiniteNumber(render.emissive)
        && isLiteral(render.uvStrategy, ['flat', 'planar', 'atlas'])
        && isPlainObject(collision)
        && hasExactKeys(collision, ['solid', 'collidable', 'occludes', 'structuralClass'])
        && typeof collision.solid === 'boolean'
        && typeof collision.collidable === 'boolean'
        && typeof collision.occludes === 'boolean'
        && isLiteral(collision.structuralClass, ['decorative', 'solid', 'structural']);
}
function isCatalogEntry(value) {
    return isPlainObject(value)
        && hasExactKeys(value, ['id', 'kind', 'version', 'hash', 'sourcePath', 'label', 'dependencies', 'material'])
        && isString(value.id)
        && isLiteral(value.kind, ['material', 'mesh', 'sprite', 'sprite-sheet', 'texture', 'voxel-volume', 'voxel-object', 'script', 'scene'])
        && isInteger(value.version)
        && (value.hash === null || isString(value.hash))
        && (value.sourcePath === null || isString(value.sourcePath))
        && (value.label === null || isString(value.label))
        && Array.isArray(value.dependencies)
        && value.dependencies.every(isAssetReference)
        && (value.material === null || isMaterialProjection(value.material));
}
function isMeshAttribute(value) {
    return isPlainObject(value) && hasExactKeys(value, ['name', 'components', 'kind']) && isLiteral(value.name, ['position', 'normal', 'uv', 'color']) && isInteger(value.components) && isLiteral(value.kind, ['f32']);
}
function isStaticMeshAsset(value) {
    if (!isPlainObject(value) || !hasExactKeys(value, ['asset', 'payload', 'materialSlots', 'collision']) || !isString(value.asset))
        return false;
    const payload = value.payload;
    const collision = value.collision;
    const source = isPlainObject(payload) ? payload.source : null;
    return isPlainObject(payload)
        && hasExactKeys(payload, ['layout', 'groups', 'bounds', 'source', 'provenance'])
        && isPlainObject(payload.layout)
        && hasExactKeys(payload.layout, ['vertexCount', 'indexCount', 'indexWidth', 'attributes'])
        && isInteger(payload.layout.vertexCount)
        && isInteger(payload.layout.indexCount)
        && isLiteral(payload.layout.indexWidth, ['u32'])
        && Array.isArray(payload.layout.attributes)
        && payload.layout.attributes.every(isMeshAttribute)
        && Array.isArray(payload.groups)
        && payload.groups.every((group) => isPlainObject(group) && hasExactKeys(group, ['materialSlot', 'start', 'count']) && isInteger(group.materialSlot) && isInteger(group.start) && isInteger(group.count))
        && isPlainObject(payload.bounds)
        && hasExactKeys(payload.bounds, ['min', 'max'])
        && isNumberTuple3(payload.bounds.min)
        && isNumberTuple3(payload.bounds.max)
        && isPlainObject(source)
        && ((source.kind === 'inline' && hasExactKeys(source, ['kind', 'positions', 'normals', 'indices']) && Array.isArray(source.positions) && source.positions.every(isFiniteNumber) && Array.isArray(source.normals) && source.normals.every(isFiniteNumber) && Array.isArray(source.indices) && source.indices.every(isInteger))
            || (source.kind === 'handle' && hasExactKeys(source, ['kind', 'buffer', 'positionsByteOffset', 'normalsByteOffset', 'indicesByteOffset']) && isInteger(source.buffer) && isInteger(source.positionsByteOffset) && isInteger(source.normalsByteOffset) && isInteger(source.indicesByteOffset)))
        && isLiteral(payload.provenance, ['voxelChunk', 'staticAsset', 'generated', 'debug'])
        && Array.isArray(value.materialSlots)
        && value.materialSlots.every((slot) => isPlainObject(slot) && hasExactKeys(slot, ['slot', 'material']) && isInteger(slot.slot) && isString(slot.material))
        && isPlainObject(collision)
        && ((collision.kind === 'visualOnly' && hasExactKeys(collision, ['kind']))
            || (collision.kind === 'aabbFallback' && hasExactKeys(collision, ['kind']))
            || (collision.kind === 'proxy' && hasExactKeys(collision, ['kind', 'proxyAsset']) && isString(collision.proxyAsset)));
}
function isTransform(value) {
    return isPlainObject(value) && hasExactKeys(value, ['translation', 'rotation', 'scale']) && isNumberTuple3(value.translation) && isNumberTuple4(value.rotation) && isNumberTuple3(value.scale);
}
function isRenderMetadata(value) {
    return isPlainObject(value) && hasExactKeys(value, ['source', 'tags', 'label']) && (value.source === null || isInteger(value.source)) && Array.isArray(value.tags) && value.tags.every(isInteger) && (value.label === null || isString(value.label));
}
function isRenderMaterialDescriptor(value) {
    return isPlainObject(value)
        && hasExactKeys(value, ['schemaVersion', 'id', 'color', 'texture', 'roughness', 'textureTint', 'emissionColor', 'emissionIntensity', 'uvStrategy'])
        && value['schemaVersion'] === 2
        && isString(value.id)
        && isNumberTuple4(value.color)
        && (value.texture === null || isString(value.texture))
        && isFiniteNumber(value.roughness)
        && isNumberTuple4(value['textureTint'])
        && isNumberTuple3(value['emissionColor'])
        && isFiniteNumber(value['emissionIntensity'])
        && isLiteral(value.uvStrategy, ['flat', 'planar', 'atlas']);
}
function isMaterialInstanceParameters(value) {
    return isPlainObject(value)
        && hasExactKeys(value, ['textureTint', 'emissionColor', 'emissionIntensity'])
        && isNumberTuple4(value['textureTint'])
        && isNumberTuple3(value['emissionColor'])
        && isFiniteNumber(value['emissionIntensity']);
}
function isRenderFrameDiff(value) {
    return isPlainObject(value) && hasExactKeys(value, ['ops']) && Array.isArray(value.ops) && value.ops.every((op) => {
        if (!isPlainObject(op) || !hasField(op, 'op'))
            return false;
        if (op.op === 'defineMaterial')
            return hasExactKeys(op, ['op', 'material']) && isRenderMaterialDescriptor(op.material);
        if (op.op === 'setMaterialInstanceParameters') {
            return hasExactKeys(op, ['op', 'handle', 'slot', 'parameters'])
                && isInteger(op.handle)
                && isInteger(op.slot)
                && (op['parameters'] === null || isMaterialInstanceParameters(op['parameters']));
        }
        if (op.op === 'defineStaticMesh')
            return hasExactKeys(op, ['op', 'asset']) && isStaticMeshAsset(op.asset);
        if (op.op === 'createStaticMeshInstance') {
            const instance = op.instance;
            return hasExactKeys(op, ['op', 'handle', 'parent', 'instance'])
                && isInteger(op.handle)
                && (op.parent === null || isInteger(op.parent))
                && isPlainObject(instance)
                && hasExactKeys(instance, ['asset', 'transform', 'materialOverrides', 'metadata'])
                && isString(instance.asset)
                && isTransform(instance.transform)
                && Array.isArray(instance.materialOverrides)
                && instance.materialOverrides.every((slot) => isPlainObject(slot) && hasExactKeys(slot, ['slot', 'material']) && isInteger(slot.slot) && isString(slot.material))
                && isRenderMetadata(instance.metadata);
        }
        return false;
    });
}
function isAssetVersionReq(value) {
    if (!isPlainObject(value) || !hasField(value, 'req'))
        return false;
    if (value.req === 'any')
        return hasExactKeys(value, ['req']);
    return (value.req === 'exact' || value.req === 'atLeast') && hasExactKeys(value, ['req', 'value']) && isInteger(value.value);
}
function isSceneTransform(value) {
    return isPlainObject(value) && hasExactKeys(value, ['translation', 'rotation', 'scale']) && isNumberTuple3(value.translation) && isNumberTuple4(value.rotation) && isNumberTuple3(value.scale);
}
function isSceneNodeKind(value) {
    if (!isPlainObject(value) || !hasField(value, 'kind'))
        return false;
    if (value.kind === 'emptyGroup')
        return hasExactKeys(value, ['kind']);
    return isLiteral(value.kind, ['staticMesh', 'sprite', 'voxelVolume'])
        && hasExactKeys(value, ['kind', 'asset'])
        && isPlainObject(value.asset)
        && hasExactKeys(value.asset, ['id', 'version', 'hash'])
        && isString(value.asset.id)
        && isAssetVersionReq(value.asset.version)
        && (value.asset.hash === null || isString(value.asset.hash));
}
function isSceneNodeRecord(value) {
    return isPlainObject(value)
        && hasExactKeys(value, ['id', 'parent', 'childOrder', 'label', 'tags', 'transform', 'kind'])
        && isInteger(value.id)
        && (value.parent === null || isInteger(value.parent))
        && isInteger(value.childOrder)
        && (value.label === null || isString(value.label))
        && Array.isArray(value.tags)
        && value.tags.every(isString)
        && isSceneTransform(value.transform)
        && isSceneNodeKind(value.kind);
}
function isFlatSceneDocument(value) {
    return isPlainObject(value)
        && hasExactKeys(value, ['schemaVersion', 'id', 'metadata', 'dependencies', 'nodes'])
        && isInteger(value.schemaVersion)
        && isInteger(value.id)
        && isPlainObject(value.metadata)
        && hasExactKeys(value.metadata, ['name', 'authoringFormatVersion'])
        && (value.metadata.name === null || isString(value.metadata.name))
        && isInteger(value.metadata.authoringFormatVersion)
        && Array.isArray(value.dependencies)
        && value.dependencies.every((dep) => isPlainObject(dep) && hasExactKeys(dep, ['id', 'version', 'hash']) && isString(dep.id) && isAssetVersionReq(dep.version) && (dep.hash === null || isString(dep.hash)))
        && Array.isArray(value.nodes)
        && value.nodes.every(isSceneNodeRecord);
}
function isSceneValidationError(value) {
    return isPlainObject(value)
        && hasExactKeys(value, ['code', 'node', 'parent', 'expectedKind', 'actualKind', 'transformReason', 'cyclePath'])
        && isLiteral(value.code, ['duplicate-node-id', 'unknown-parent', 'cycle', 'invalid-transform', 'asset-kind-mismatch'])
        && (value.node === null || isInteger(value.node))
        && (value.parent === null || isInteger(value.parent))
        && (value.expectedKind === null || isString(value.expectedKind))
        && (value.actualKind === null || isString(value.actualKind))
        && (value.transformReason === null || isString(value.transformReason))
        && Array.isArray(value.cyclePath)
        && value.cyclePath.every(isInteger);
}
function isSceneObjectRecord(value) {
    return isPlainObject(value)
        && hasExactKeys(value, ['id', 'parent', 'childOrder', 'label', 'kind', 'hasRenderableAsset'])
        && isInteger(value.id)
        && (value.parent === null || isInteger(value.parent))
        && isInteger(value.childOrder)
        && (value.label === null || isString(value.label))
        && isLiteral(value.kind, ['emptyGroup', 'staticMesh', 'sprite', 'voxelVolume'])
        && typeof value.hasRenderableAsset === 'boolean';
}
function isSceneObjectSnapshot(value) {
    return isPlainObject(value)
        && hasExactKeys(value, ['documentHash', 'objects'])
        && isInteger(value.documentHash)
        && Array.isArray(value.objects)
        && value.objects.every(isSceneObjectRecord);
}
function isSceneObjectCommand(value) {
    if (!isPlainObject(value) || !hasField(value, 'kind'))
        return false;
    if (value.kind === 'create')
        return hasExactKeys(value, ['kind', 'record']) && isSceneNodeRecord(value.record);
    if (value.kind === 'delete')
        return hasExactKeys(value, ['kind', 'id']) && isInteger(value.id);
    if (value.kind === 'rename')
        return hasExactKeys(value, ['kind', 'id', 'label']) && isInteger(value.id) && (value.label === null || isString(value.label));
    if (value.kind === 'reparent')
        return hasExactKeys(value, ['kind', 'id', 'parent', 'childOrder']) && isInteger(value.id) && (value.parent === null || isInteger(value.parent)) && isInteger(value.childOrder);
    if (value.kind === 'translate')
        return hasExactKeys(value, ['kind', 'id', 'delta']) && isInteger(value.id) && isNumberTuple3(value.delta);
    if (value.kind === 'rotate')
        return hasExactKeys(value, ['kind', 'id', 'rotation']) && isInteger(value.id) && isNumberTuple4(value.rotation);
    return value.kind === 'select' && hasExactKeys(value, ['kind', 'id']) && (value.id === null || isInteger(value.id));
}
function isSceneObjectCommandRejection(value) {
    return isPlainObject(value)
        && hasExactKeys(value, ['code', 'id', 'parent', 'expectedHash', 'actualHash', 'validationErrors'])
        && isLiteral(value.code, ['stale-scene-object-snapshot', 'invalid-scene-before-command', 'invalid-scene-after-command', 'missing-scene-object', 'duplicate-scene-object', 'missing-scene-object-parent', 'scene-object-self-parent', 'blank-scene-object-label', 'invalid-scene-object-transform', 'readonly-scene-object-transform'])
        && (value.id === null || isInteger(value.id))
        && (value.parent === null || isInteger(value.parent))
        && (value.expectedHash === null || isInteger(value.expectedHash))
        && (value.actualHash === null || isInteger(value.actualHash))
        && Array.isArray(value.validationErrors)
        && value.validationErrors.every(isSceneValidationError);
}
function isSceneObjectCommandOutcome(value) {
    return isPlainObject(value)
        && hasExactKeys(value, ['document', 'snapshot', 'selected'])
        && isFlatSceneDocument(value.document)
        && isSceneObjectSnapshot(value.snapshot)
        && (value.selected === null || isInteger(value.selected));
}
function isSceneObjectCommandRequest(value) {
    return isPlainObject(value)
        && hasExactKeys(value, ['expectedDocumentHash', 'command'])
        && isInteger(value.expectedDocumentHash)
        && isSceneObjectCommand(value.command);
}
function isSceneObjectCommandResult(value) {
    return isPlainObject(value)
        && hasExactKeys(value, ['accepted', 'outcome', 'rejection'])
        && typeof value.accepted === 'boolean'
        && (value.outcome === null || isSceneObjectCommandOutcome(value.outcome))
        && (value.rejection === null || isSceneObjectCommandRejection(value.rejection));
}
function isVoxelConversionSourceRef(value) {
    return isPlainObject(value)
        && hasExactKeys(value, ['assetId', 'assetKind', 'assetVersion', 'sourceHash', 'meshPrimitive'])
        && isString(value.assetId)
        && isString(value.assetKind)
        && isInteger(value.assetVersion)
        && isString(value.sourceHash)
        && (value.meshPrimitive === null || isString(value.meshPrimitive));
}
function isVoxelConversionTargetRef(value) {
    return isPlainObject(value)
        && hasExactKeys(value, ['grid', 'volumeAssetId', 'origin'])
        && isInteger(value.grid)
        && (value.volumeAssetId === null || isString(value.volumeAssetId))
        && isVoxelCoord(value.origin);
}
function isVoxelConversionBounds(value) {
    return isPlainObject(value)
        && hasExactKeys(value, ['min', 'max'])
        && isVoxelCoord(value.min)
        && isVoxelCoord(value.max);
}
function isVoxelConversionMaterialMapEntry(value) {
    return isPlainObject(value)
        && hasExactKeys(value, ['sourceMaterialSlot', 'sourceMaterialId', 'voxelMaterial'])
        && isInteger(value.sourceMaterialSlot)
        && (value.sourceMaterialId === null || isString(value.sourceMaterialId))
        && isInteger(value.voxelMaterial);
}
function isVoxelConversionUvAttributeRef(value) {
    return isPlainObject(value)
        && hasExactKeys(value, ['attributeName', 'sourceHash'])
        && isString(value.attributeName)
        && isString(value.sourceHash);
}
function isVoxelConversionTextureSourceRef(value) {
    return isPlainObject(value)
        && hasExactKeys(value, ['textureAssetId', 'assetVersion', 'contentHash', 'width', 'height', 'colorSpace', 'channelLayout'])
        && isString(value.textureAssetId)
        && isInteger(value.assetVersion)
        && isString(value.contentHash)
        && isInteger(value.width)
        && isInteger(value.height)
        && isString(value.colorSpace)
        && isString(value.channelLayout);
}
function isVoxelConversionTextureSampleAsset(value) {
    return isPlainObject(value)
        && hasExactKeys(value, ['texture', 'texelMaterials'])
        && isVoxelConversionTextureSourceRef(value.texture)
        && Array.isArray(value.texelMaterials)
        && value.texelMaterials.every(isInteger);
}
function isVoxelConversionTextureBinding(value) {
    return isPlainObject(value)
        && hasExactKeys(value, ['sourceMaterialSlot', 'texture', 'uvAttribute', 'sampleUv', 'samplingPolicy', 'wrapPolicy', 'materialMode'])
        && isInteger(value.sourceMaterialSlot)
        && isVoxelConversionTextureSourceRef(value.texture)
        && isVoxelConversionUvAttributeRef(value.uvAttribute)
        && isNumberTuple2(value.sampleUv)
        && isString(value.samplingPolicy)
        && isString(value.wrapPolicy)
        && isString(value.materialMode);
}
function isVoxelConversionMaterialMap(value) {
    return isPlainObject(value)
        && hasExactKeys(value, ['entries', 'textureAssets', 'textureBindings', 'defaultVoxelMaterial'])
        && Array.isArray(value.entries)
        && value.entries.every(isVoxelConversionMaterialMapEntry)
        && Array.isArray(value.textureAssets)
        && value.textureAssets.every(isVoxelConversionTextureSampleAsset)
        && Array.isArray(value.textureBindings)
        && value.textureBindings.every(isVoxelConversionTextureBinding)
        && (value.defaultVoxelMaterial === null || isInteger(value.defaultVoxelMaterial));
}
function isVoxelConversionSettings(value) {
    return isPlainObject(value)
        && hasExactKeys(value, ['mode', 'fitPolicy', 'originPolicy', 'resolution', 'voxelSize', 'maxOutputVoxels', 'transform', 'materialMap'])
        && isLiteral(value.mode, ['surface', 'solid'])
        && isLiteral(value.fitPolicy, ['contain', 'cover', 'stretch'])
        && isLiteral(value.originPolicy, ['source_origin', 'target_min', 'centered'])
        && isNumberTuple3(value.resolution)
        && isFiniteNumber(value.voxelSize)
        && isInteger(value.maxOutputVoxels)
        && isNumberTuple16(value.transform)
        && isVoxelConversionMaterialMap(value.materialMap);
}
function isVoxelConversionPlanRequest(value) {
    return isPlainObject(value)
        && hasExactKeys(value, ['source', 'target', 'settings'])
        && isVoxelConversionSourceRef(value.source)
        && isVoxelConversionTargetRef(value.target)
        && isVoxelConversionSettings(value.settings);
}
function isVoxelConversionDiagnostic(value) {
    return isPlainObject(value)
        && hasExactKeys(value, ['code', 'severity', 'reference', 'message'])
        && isLiteral(value.code, [
            'voxel_conversion_unavailable',
            'operation_unimplemented',
            'unsupported_source_asset',
            'source_hash_mismatch',
            'invalid_material_map',
            'output_limit_exceeded',
            'non_manifold_or_ambiguous_solid',
            'stale_authority_snapshot',
            'conversion_replay_mismatch',
        ])
        && isLiteral(value.severity, ['info', 'warning', 'error', 'fatal'])
        && isString(value.reference)
        && isString(value.message);
}
function isVoxelConversionEvidenceRef(value) {
    return isPlainObject(value)
        && hasExactKeys(value, ['kind', 'uri', 'contentHash'])
        && isLiteral(value.kind, ['plan', 'preview', 'apply_receipt', 'diagnostics', 'source_snapshot', 'output_snapshot'])
        && isString(value.uri)
        && isString(value.contentHash);
}
function isVoxelConversionPlan(value) {
    return isPlainObject(value)
        && hasExactKeys(value, [
            'planId',
            'source',
            'target',
            'settings',
            'authorityVersion',
            'expectedSourceHash',
            'settingsHash',
            'planHash',
            'estimatedOutputVoxels',
            'estimatedBounds',
            'diagnostics',
            'evidence',
        ])
        && isString(value.planId)
        && isVoxelConversionSourceRef(value.source)
        && isVoxelConversionTargetRef(value.target)
        && isVoxelConversionSettings(value.settings)
        && isString(value.authorityVersion)
        && isString(value.expectedSourceHash)
        && isString(value.settingsHash)
        && isString(value.planHash)
        && isInteger(value.estimatedOutputVoxels)
        && (value.estimatedBounds === null || isVoxelConversionBounds(value.estimatedBounds))
        && Array.isArray(value.diagnostics)
        && value.diagnostics.every(isVoxelConversionDiagnostic)
        && Array.isArray(value.evidence)
        && value.evidence.every(isVoxelConversionEvidenceRef);
}
function isVoxelConversionPreviewRequest(value) {
    return isPlainObject(value)
        && hasExactKeys(value, ['planId', 'expectedPlanHash'])
        && isString(value.planId)
        && isString(value.expectedPlanHash);
}
function isVoxelConversionPreviewVoxel(value) {
    return isPlainObject(value)
        && hasExactKeys(value, ['coord', 'material'])
        && isVoxelCoord(value.coord)
        && isInteger(value.material);
}
function isVoxelConversionPreview(value) {
    return isPlainObject(value)
        && hasExactKeys(value, ['planId', 'outputHash', 'outputVoxelCount', 'outputBounds', 'sampleVoxels', 'diagnostics', 'evidence'])
        && isString(value.planId)
        && isString(value.outputHash)
        && isInteger(value.outputVoxelCount)
        && (value.outputBounds === null || isVoxelConversionBounds(value.outputBounds))
        && Array.isArray(value.sampleVoxels)
        && value.sampleVoxels.every(isVoxelConversionPreviewVoxel)
        && Array.isArray(value.diagnostics)
        && value.diagnostics.every(isVoxelConversionDiagnostic)
        && Array.isArray(value.evidence)
        && value.evidence.every(isVoxelConversionEvidenceRef);
}
function isVoxelConversionApplyRequest(value) {
    return isPlainObject(value)
        && hasExactKeys(value, ['planId', 'expectedPlanHash', 'expectedPreviewHash'])
        && isString(value.planId)
        && isString(value.expectedPlanHash)
        && (value.expectedPreviewHash === null || isString(value.expectedPreviewHash));
}
function isVoxelConversionReceipt(value) {
    return isPlainObject(value)
        && hasExactKeys(value, ['planId', 'applied', 'outputHash', 'outputVoxelCount', 'outputBounds', 'diagnostics', 'evidence'])
        && isString(value.planId)
        && typeof value.applied === 'boolean'
        && (value.outputHash === null || isString(value.outputHash))
        && isInteger(value.outputVoxelCount)
        && (value.outputBounds === null || isVoxelConversionBounds(value.outputBounds))
        && Array.isArray(value.diagnostics)
        && value.diagnostics.every(isVoxelConversionDiagnostic)
        && Array.isArray(value.evidence)
        && value.evidence.every(isVoxelConversionEvidenceRef);
}
function validateContractValue(value, exportName) {
    switch (exportName) {
        case 'ScreenPointToPickRayRequest':
            return isScreenPointToPickRayRequest(value);
        case 'VoxelCoord':
            return isVoxelCoord(value);
        case 'VoxelSelectionSnapshot':
            return isVoxelSelectionSnapshot(value);
        case 'VoxelCommand':
            return isVoxelCommand(value);
        case 'CatalogEntry':
            return isCatalogEntry(value);
        case 'MaterialProjection':
            return isMaterialProjection(value);
        case 'StaticMeshAsset':
            return isStaticMeshAsset(value);
        case 'RenderFrameDiff':
            return isRenderFrameDiff(value);
        case 'SceneObjectSnapshot':
            return isSceneObjectSnapshot(value);
        case 'SceneObjectCommandRequest':
            return isSceneObjectCommandRequest(value);
        case 'SceneObjectCommandResult':
            return isSceneObjectCommandResult(value);
        case 'VoxelConversionPlanRequest':
            return isVoxelConversionPlanRequest(value);
        case 'VoxelConversionPlan':
            return isVoxelConversionPlan(value);
        case 'VoxelConversionPreviewRequest':
            return isVoxelConversionPreviewRequest(value);
        case 'VoxelConversionPreview':
            return isVoxelConversionPreview(value);
        case 'VoxelConversionApplyRequest':
            return isVoxelConversionApplyRequest(value);
        case 'VoxelConversionReceipt':
            return isVoxelConversionReceipt(value);
        case 'VoxelConversionEvidenceRef':
            return isVoxelConversionEvidenceRef(value);
        default:
            return false;
    }
}
function validateValueAgainstShape(value, shape) {
    switch (shape.kind) {
        case 'empty':
            return isPlainObject(value) && Object.keys(value).length === 1 && value.kind === 'empty';
        case 'contract':
            return validateContractValue(value, shape.ref.exportName);
        case 'literal':
            return typeof value === 'string' && shape.values.includes(value);
        case 'nullable':
            return value === null || validateValueAgainstShape(value, shape.inner);
        case 'scalar':
            switch (shape.scalar) {
                case 'string':
                case 'state_hash':
                case 'artifact_ref':
                    return typeof value === 'string';
                case 'number':
                    return typeof value === 'number' && Number.isFinite(value);
                case 'integer':
                    return typeof value === 'number' && Number.isInteger(value);
                case 'boolean':
                    return typeof value === 'boolean';
                case 'null':
                    return value === null;
            }
        case 'array':
            return Array.isArray(value) && (shape.minItems === undefined || value.length >= shape.minItems) && value.every((item) => validateValueAgainstShape(item, shape.items));
        case 'object': {
            if (typeof value !== 'object' || value === null || Array.isArray(value)) {
                return false;
            }
            const keys = Object.keys(value);
            const allowed = new Set(shape.fields.map((field) => field.name));
            if (keys.some((key) => !allowed.has(key))) {
                return false;
            }
            for (const field of shape.fields) {
                if (!hasField(value, field.name)) {
                    if (field.required) {
                        return false;
                    }
                    continue;
                }
                if (!validateValueAgainstShape(value[field.name], field.shape)) {
                    return false;
                }
            }
            return true;
        }
    }
}
export function validateExampleAgainstSchema(commandId, field, value, schemaShape) {
    if (validateValueAgainstShape(value, schemaShape)) {
        return [];
    }
    return [{ commandId, field, message: `${field} does not match its declared schema` }];
}
export function validateCommandDefinition(definition) {
    const commandId = commandLabel(definition);
    const issues = [];
    for (const field of REQUIRED_METADATA_FIELDS) {
        if (!hasOwn(definition, field)) {
            issues.push({ commandId, field, message: 'missing required command metadata' });
        }
    }
    if (definition.id !== undefined && !/^[a-z][a-z0-9_]*(\.[a-z0-9_]+)+$/.test(definition.id)) {
        issues.push({ commandId, field: 'id', message: 'command id must be stable dotted lowercase' });
    }
    if (definition.version !== undefined && (!Number.isInteger(definition.version) || definition.version < 1)) {
        issues.push({ commandId, field: 'version', message: 'version must be a positive integer' });
    }
    if (definition.menuPath !== undefined && definition.menuPath.length === 0) {
        issues.push({ commandId, field: 'menuPath', message: 'menu path must be visible and non-empty' });
    }
    if (definition.artifacts !== undefined && definition.artifacts.length === 0) {
        issues.push({ commandId, field: 'artifacts', message: 'commands must declare artifacts, even when optional' });
    }
    if (definition.agentExposure !== undefined && definition.agentExposure.kind !== 'hidden') {
        if (!isNonEmptyString(definition.label)) {
            issues.push({ commandId, field: 'label', message: 'agent-exposed commands require a human-visible label' });
        }
        if (!isNonEmptyString(definition.summary)) {
            issues.push({ commandId, field: 'summary', message: 'agent-exposed commands require a human-visible summary' });
        }
        if (definition.operationClass === undefined) {
            issues.push({ commandId, field: 'operationClass', message: 'agent-exposed commands require an operation class' });
        }
        if (definition.owningLane === undefined) {
            issues.push({ commandId, field: 'owningLane', message: 'agent-exposed commands require owning lane metadata' });
        }
        if (definition.owningPackage === undefined) {
            issues.push({ commandId, field: 'owningPackage', message: 'agent-exposed commands require owning package metadata' });
        }
        if (definition.guiMirror?.required !== true) {
            issues.push({ commandId, field: 'guiMirror.required', message: 'agent-exposed commands require a GUI mirror' });
        }
        if (definition.guiMirror?.menuPath === undefined || definition.guiMirror.menuPath.length === 0) {
            issues.push({ commandId, field: 'guiMirror.menuPath', message: 'agent-exposed commands require GUI/menu path metadata' });
        }
        if (!arraysEqual(definition.guiMirror?.menuPath, definition.menuPath)) {
            issues.push({ commandId, field: 'guiMirror.menuPath', message: 'GUI mirror menu path must match command menu path' });
        }
        if (definition.guiMirror?.commandPaletteVisible !== true && definition.guiMirror?.panel === undefined) {
            issues.push({ commandId, field: 'guiMirror', message: 'agent-exposed commands require command-palette visibility or a panel route' });
        }
        if (!isNonEmptyString(definition.guiMirror?.argumentSummary)) {
            issues.push({ commandId, field: 'guiMirror.argumentSummary', message: 'agent-exposed commands require GUI argument summary metadata' });
        }
        if (!isNonEmptyString(definition.guiMirror?.resultSummary)) {
            issues.push({ commandId, field: 'guiMirror.resultSummary', message: 'agent-exposed commands require GUI result/output summary metadata' });
        }
        if (!isNonEmptyString(definition.guiMirror?.artifactSummary)) {
            issues.push({ commandId, field: 'guiMirror.artifactSummary', message: 'agent-exposed commands require GUI artifact summary metadata' });
        }
    }
    if (definition.agentExposure?.kind === 'read_only') {
        if (definition.operationClass !== undefined && definition.operationClass !== 'read_only') {
            issues.push({ commandId, field: 'agentExposure', message: 'read_only exposure is only valid for read_only operations' });
        }
        if (definition.stateImpact !== undefined && mutatesOrWrites(definition.stateImpact)) {
            issues.push({ commandId, field: 'agentExposure', message: 'read_only exposure is invalid for mutating/writing/capturing state impacts' });
        }
    }
    if (definition.inputSchema !== undefined) {
        visitSchemaShape(commandId, 'inputSchema.shape', definition.inputSchema.shape, issues);
    }
    if (definition.outputSchema !== undefined) {
        visitSchemaShape(commandId, 'outputSchema.shape', definition.outputSchema.shape, issues);
    }
    if (definition.inputSchema !== undefined && definition.typedInputExample !== undefined) {
        issues.push(...validateExampleAgainstSchema(commandId, 'typedInputExample', definition.typedInputExample, definition.inputSchema.shape));
    }
    if (definition.outputSchema !== undefined && definition.typedOutputExample !== undefined) {
        issues.push(...validateExampleAgainstSchema(commandId, 'typedOutputExample', definition.typedOutputExample, definition.outputSchema.shape));
    }
    return issues;
}
export function validateCommandManifest(manifest) {
    const issues = [];
    const seen = new Set();
    for (const definition of manifest) {
        issues.push(...validateCommandDefinition(definition));
        if (definition.id !== undefined) {
            if (seen.has(definition.id)) {
                issues.push({ commandId: definition.id, field: 'id', message: 'duplicate command id' });
            }
            seen.add(definition.id);
        }
    }
    return issues;
}
export function requireKnownCommand(id, manifest) {
    const found = manifest.find((command) => command.id === id);
    if (found === undefined) {
        throw new Error(`Unknown ASHA studio command id: ${id}`);
    }
    return found;
}
//# sourceMappingURL=validation.js.map