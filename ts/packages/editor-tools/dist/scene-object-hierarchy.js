// @asha/editor-tools — canonical scene-object hierarchy read model.
//
// This module projects generated FlatSceneDocument contracts into stable editor
// object ids and builds proposal-only rename/reparent commands. It never submits
// or validates authority; Rust/runtime validation remains the decider.
import { proposeReparent, proposeSetMetadata } from './scene-authoring.js';
export function sceneObjectIdForNode(sceneNodeId) {
    return `scene-node:${sceneNodeId}`;
}
function stableJson(value) {
    if (value === null || typeof value !== 'object') {
        return JSON.stringify(value);
    }
    if (Array.isArray(value)) {
        return `[${value.map(item => stableJson(item)).join(',')}]`;
    }
    return `{${Object.entries(value)
        .sort(([left], [right]) => left.localeCompare(right))
        .map(([key, item]) => `${JSON.stringify(key)}:${stableJson(item)}`)
        .join(',')}}`;
}
function fnv1aHash(prefix, value) {
    const text = stableJson(value);
    let hash = 0x811c9dc5;
    for (let index = 0; index < text.length; index += 1) {
        hash ^= text.charCodeAt(index);
        hash = Math.imul(hash, 0x01000193) >>> 0;
    }
    return `${prefix}-${hash.toString(16).padStart(8, '0')}`;
}
function assetForKind(kind) {
    switch (kind.kind) {
        case 'emptyGroup':
            return null;
        case 'staticMesh':
        case 'sprite':
        case 'voxelVolume':
            return kind.asset;
    }
}
function defaultDisplayName(node) {
    return node.label ?? `${node.kind.kind} ${node.id}`;
}
function sceneObjectRecord(doc, node, traceByNode, renderableByNode) {
    const nodeId = node.id;
    return {
        objectId: sceneObjectIdForNode(node.id),
        sceneNodeId: node.id,
        runtimeEntityId: traceByNode.get(nodeId) ?? null,
        parentObjectId: node.parent === null ? null : sceneObjectIdForNode(node.parent),
        childOrder: node.childOrder,
        displayName: defaultDisplayName(node),
        kind: node.kind.kind,
        tags: [...node.tags],
        transform: node.transform,
        asset: assetForKind(node.kind),
        editability: {
            selectable: true,
            rename: true,
            reparent: true,
            transform: true,
        },
        provenance: {
            source: 'flat_scene_document',
            sceneId: doc.id,
            renderableId: renderableByNode.get(nodeId) ?? null,
        },
    };
}
function hierarchyDiagnostics(objects) {
    const diagnostics = [];
    const objectIds = new Set(objects.map(object => object.objectId));
    const parentByObject = new Map(objects.map(object => [object.objectId, object.parentObjectId]));
    for (const object of objects) {
        if (object.parentObjectId !== null && !objectIds.has(object.parentObjectId)) {
            diagnostics.push({
                code: 'missing_parent',
                message: `${object.objectId} references missing parent ${object.parentObjectId}.`,
                objectId: object.objectId,
            });
        }
        const seen = new Set();
        let current = object.objectId;
        while (current !== null) {
            if (seen.has(current)) {
                diagnostics.push({
                    code: 'scene_object_cycle',
                    message: `Scene object hierarchy contains a parent cycle at ${current}.`,
                    objectId: object.objectId,
                });
                break;
            }
            seen.add(current);
            current = parentByObject.get(current) ?? null;
        }
    }
    return diagnostics;
}
export function buildSceneObjectSnapshot(options) {
    const traceByNode = new Map((options.bootstrap?.sourceTrace ?? []).map(trace => [
        trace.sceneNodeId,
        trace.runtimeEntityId,
    ]));
    const renderableByNode = new Map((options.renderableLinks ?? []).map(link => [link.sceneNodeId, link.renderableId]));
    const objects = options.document.nodes
        .map(node => sceneObjectRecord(options.document, node, traceByNode, renderableByNode))
        .sort((left, right) => (left.parentObjectId ?? '').localeCompare(right.parentObjectId ?? '')
        || left.childOrder - right.childOrder
        || left.sceneNodeId - right.sceneNodeId);
    return {
        schemaVersion: 1,
        snapshotVersion: 'scene-object-snapshot.v0',
        sceneId: options.document.id,
        sceneHash: fnv1aHash('scene-object', {
            id: options.document.id,
            nodes: options.document.nodes,
            trace: options.bootstrap?.sourceTrace ?? [],
            renderableLinks: options.renderableLinks ?? [],
        }),
        objects,
        diagnostics: hierarchyDiagnostics(objects),
        nonClaims: [
            'proposal_only',
            'not_authority_validation',
            'not_runtime_bridge_execution',
            'not_private_ui_state',
        ],
    };
}
function objectForId(snapshot, objectId) {
    return snapshot.objects.find(object => object.objectId === objectId) ?? null;
}
function staleSnapshotDiagnostic(objectId) {
    return {
        code: 'stale_scene_object_snapshot',
        message: 'Scene object snapshot already has diagnostics; refresh authority/readback before proposing edits.',
        objectId,
    };
}
export function proposeRenameSceneObject(options) {
    if (options.snapshot.diagnostics.length > 0) {
        return { ok: false, diagnostics: [staleSnapshotDiagnostic(options.objectId)] };
    }
    const object = objectForId(options.snapshot, options.objectId);
    if (object === null) {
        return {
            ok: false,
            diagnostics: [{
                    code: 'missing_object',
                    message: `Cannot rename missing scene object ${options.objectId}.`,
                    objectId: options.objectId,
                }],
        };
    }
    const displayName = options.displayName.trim();
    if (displayName.length === 0) {
        return {
            ok: false,
            diagnostics: [{
                    code: 'invalid_name',
                    message: 'Scene object display name must not be empty.',
                    objectId: options.objectId,
                }],
        };
    }
    return {
        ok: true,
        proposal: proposeSetMetadata(object.sceneNodeId, displayName, object.tags),
    };
}
function wouldCreateCycle(snapshot, objectId, parentObjectId) {
    const parentByObject = new Map(snapshot.objects.map(object => [object.objectId, object.parentObjectId]));
    let current = parentObjectId;
    while (current !== null) {
        if (current === objectId) {
            return true;
        }
        current = parentByObject.get(current) ?? null;
    }
    return false;
}
export function proposeReparentSceneObject(options) {
    if (options.snapshot.diagnostics.length > 0) {
        return { ok: false, diagnostics: [staleSnapshotDiagnostic(options.objectId)] };
    }
    const object = objectForId(options.snapshot, options.objectId);
    if (object === null) {
        return {
            ok: false,
            diagnostics: [{
                    code: 'missing_object',
                    message: `Cannot reparent missing scene object ${options.objectId}.`,
                    objectId: options.objectId,
                }],
        };
    }
    const parent = options.parentObjectId === null ? null : objectForId(options.snapshot, options.parentObjectId);
    if (options.parentObjectId === options.objectId) {
        return {
            ok: false,
            diagnostics: [{
                    code: 'self_parent',
                    message: 'A scene object cannot be parented under itself.',
                    objectId: options.objectId,
                }],
        };
    }
    if (options.parentObjectId !== null && parent === null) {
        return {
            ok: false,
            diagnostics: [{
                    code: 'missing_parent',
                    message: `Cannot reparent ${options.objectId} under missing parent ${options.parentObjectId}.`,
                    objectId: options.objectId,
                }],
        };
    }
    if (options.parentObjectId !== null && wouldCreateCycle(options.snapshot, options.objectId, options.parentObjectId)) {
        return {
            ok: false,
            diagnostics: [{
                    code: 'scene_object_cycle',
                    message: `Cannot reparent ${options.objectId} under its descendant ${options.parentObjectId}.`,
                    objectId: options.objectId,
                }],
        };
    }
    return {
        ok: true,
        proposal: proposeReparent(object.sceneNodeId, parent === null ? null : parent.sceneNodeId, options.childOrder ?? object.childOrder),
    };
}
//# sourceMappingURL=scene-object-hierarchy.js.map