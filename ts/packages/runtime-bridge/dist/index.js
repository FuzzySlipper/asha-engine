// @asha/runtime-bridge — the public, transport-agnostic runtime facade (ADR 0006).
//
// App / UI / renderer / devtools couple ONLY to this package for runtime. They do
// not know whether the implementation is napi-rs (`@asha/native-bridge`), a mock,
// or the WASM replay path. The facade exports generated-compatible contract types
// and explicit buffer-handle APIs — never raw addon exports, WASM memory, or JSON
// escape hatches.
//
// The public facade is hand-written for readability but MUST satisfy the
// manifest-derived conformance test (see conformance.test.ts).
import { loadNativeAddon, NativeAddonUnavailable } from '@asha/native-bridge';
import { MANIFEST_OPERATIONS } from './generated/operations.js';
export { MANIFEST_OPERATIONS } from './generated/operations.js';
// Render-diff decode (moved from the former @asha/wasm-bridge). Transport-neutral
// payload → contract types; backs `readRenderDiffs`. See render-decode.ts.
export { decodeRenderDiff, decodeRenderFrameDiff, RenderDecodeError, RenderDiffStream, FrameMemory, } from './render-decode.js';
export const frameCursor = (frame) => frame;
/** Typed, classified error for every facade operation. No JSON error blobs. */
export class RuntimeBridgeError extends Error {
    kind;
    constructor(kind, message) {
        super(`runtime bridge error [${kind}]: ${message}`);
        this.kind = kind;
        this.name = 'RuntimeBridgeError';
    }
}
function nonNegativeSafeInteger(value, field) {
    if (!Number.isSafeInteger(value) || value < 0) {
        throw new RuntimeBridgeError('invalid_input', `${field} must be a non-negative safe integer`);
    }
    return value;
}
function u32(value, field) {
    nonNegativeSafeInteger(value, field);
    if (value > 0xffffffff) {
        throw new RuntimeBridgeError('invalid_input', `${field} must fit in u32`);
    }
    return value;
}
function finite(value, field) {
    if (!Number.isFinite(value)) {
        throw new RuntimeBridgeError('invalid_input', `${field} must be finite`);
    }
    return value;
}
function validateViewport(viewport) {
    if (!Number.isInteger(viewport.width) || viewport.width <= 0) {
        throw new RuntimeBridgeError('invalid_input', 'viewport width must be a positive integer');
    }
    if (!Number.isInteger(viewport.height) || viewport.height <= 0) {
        throw new RuntimeBridgeError('invalid_input', 'viewport height must be a positive integer');
    }
}
function validateProjection(projection) {
    finite(projection.fovYDegrees, 'fovYDegrees');
    finite(projection.near, 'near');
    finite(projection.far, 'far');
    if (projection.fovYDegrees <= 0 || projection.fovYDegrees >= 180) {
        throw new RuntimeBridgeError('invalid_input', 'fovYDegrees must be in (0, 180)');
    }
    if (projection.near <= 0 || projection.far <= projection.near) {
        throw new RuntimeBridgeError('invalid_input', 'projection near/far must satisfy 0 < near < far');
    }
}
function f32(value) {
    return Math.fround(value);
}
function basisFromPose(pose) {
    const yaw = f32((pose.yawDegrees * Math.PI) / 180);
    const pitch = f32((pose.pitchDegrees * Math.PI) / 180);
    const cp = f32(Math.cos(pitch));
    const sp = f32(Math.sin(pitch));
    const sy = f32(Math.sin(yaw));
    const cy = f32(Math.cos(yaw));
    return {
        forward: [f32(sy * cp), sp, f32(-cy * cp)],
        right: [cy, 0, sy],
        up: [f32(-sy * sp), cp, f32(cy * sp)],
    };
}
function matrixKey(values) {
    return values.map((value) => value.toFixed(3)).join(',');
}
function fnv1a64(text) {
    let hash = 0xcbf29ce484222325n;
    const prime = 0x100000001b3n;
    const mask = 0xffffffffffffffffn;
    for (let i = 0; i < text.length; i += 1) {
        hash ^= BigInt(text.charCodeAt(i));
        hash = (hash * prime) & mask;
    }
    return hash.toString(16).padStart(16, '0');
}
function multiplyMatrix4(a, b) {
    const out = new Array(16).fill(0);
    for (let col = 0; col < 4; col += 1) {
        for (let row = 0; row < 4; row += 1) {
            let sum = 0;
            for (let k = 0; k < 4; k += 1) {
                sum = f32(sum + f32((a[k * 4 + row] ?? 0) * (b[col * 4 + k] ?? 0)));
            }
            out[col * 4 + row] = sum;
        }
    }
    return out;
}
function viewMatrixFromSnapshot(snapshot) {
    const { right, up, forward } = snapshot.basis;
    const position = snapshot.pose.position;
    const dotRight = f32(f32(right[0] * position[0]) + f32(right[1] * position[1]) + f32(right[2] * position[2]));
    const dotUp = f32(f32(up[0] * position[0]) + f32(up[1] * position[1]) + f32(up[2] * position[2]));
    const dotForward = f32(f32(forward[0] * position[0]) + f32(forward[1] * position[1]) + f32(forward[2] * position[2]));
    return [
        right[0],
        up[0],
        -forward[0],
        0,
        right[1],
        up[1],
        -forward[1],
        0,
        right[2],
        up[2],
        -forward[2],
        0,
        -dotRight,
        -dotUp,
        dotForward,
        1,
    ];
}
function projectionMatrixFromSnapshot(snapshot, viewport) {
    const aspect = f32(viewport.width / viewport.height);
    const f = f32(1 / Math.tan(f32((snapshot.projection.fovYDegrees * Math.PI) / 360)));
    const near = snapshot.projection.near;
    const far = snapshot.projection.far;
    return [
        f32(f / aspect),
        0,
        0,
        0,
        0,
        f,
        0,
        0,
        0,
        0,
        f32((far + near) / (near - far)),
        -1,
        0,
        0,
        f32((2 * far * near) / (near - far)),
        0,
    ];
}
function materialDescriptor(id, material) {
    return {
        id,
        color: [material.render.color.r, material.render.color.g, material.render.color.b, material.render.color.a],
        texture: material.render.texture?.id ?? null,
        roughness: material.render.roughness,
        emissive: material.render.emissive,
        uvStrategy: material.render.uvStrategy,
    };
}
function projectionSnapshot(snapshot, viewport = snapshot.viewport) {
    const viewMatrix = viewMatrixFromSnapshot(snapshot);
    const projectionMatrix = projectionMatrixFromSnapshot(snapshot, viewport);
    const viewProjectionMatrix = multiplyMatrix4(projectionMatrix, viewMatrix);
    const projectionHash = `fnv1a64:${fnv1a64(matrixKey([
        ...viewMatrix,
        ...projectionMatrix,
        ...viewProjectionMatrix,
    ]))}`;
    return {
        ...snapshot,
        viewport,
        viewMatrix,
        projectionMatrix,
        viewProjectionMatrix,
        projectionHash,
    };
}
function cloneFlatSceneDocument(document) {
    return JSON.parse(JSON.stringify(document));
}
function initialMockSceneDocument() {
    return {
        schemaVersion: 1,
        id: 1,
        metadata: { name: 'Mock scene', authoringFormatVersion: 1 },
        dependencies: [],
        nodes: [
            {
                id: 1,
                parent: null,
                childOrder: 0,
                label: 'Root',
                tags: [],
                transform: { translation: [0, 0, 0], rotation: [0, 0, 0, 1], scale: [1, 1, 1] },
                kind: { kind: 'emptyGroup' },
            },
            {
                id: 2,
                parent: 1,
                childOrder: 0,
                label: 'Preview cube',
                tags: [],
                transform: { translation: [0, 0, 0], rotation: [0, 0, 0, 1], scale: [1, 1, 1] },
                kind: {
                    kind: 'staticMesh',
                    asset: { id: 'static-mesh:preview/cube', version: { req: 'any' }, hash: null },
                },
            },
        ],
    };
}
function sceneDocumentHash(document) {
    const hex = fnv1a64(JSON.stringify({
        ...document,
        nodes: [...document.nodes].sort((a, b) => a.id - b.id),
    }));
    return Number.parseInt(hex.slice(0, 13), 16);
}
function sceneObjectSnapshotFromDocument(document) {
    return {
        documentHash: sceneDocumentHash(document),
        objects: [...document.nodes]
            .sort((a, b) => a.id - b.id)
            .map((node) => ({
            id: node.id,
            parent: node.parent,
            childOrder: node.childOrder,
            label: node.label,
            kind: node.kind.kind,
            hasRenderableAsset: node.kind.kind !== 'emptyGroup',
        })),
    };
}
function nodeIndex(document, id) {
    return document.nodes.findIndex((node) => node.id === id);
}
function commandRejection(code, id, parent = null, expectedHash = null, actualHash = null) {
    return {
        accepted: false,
        outcome: null,
        rejection: { code, id, parent, expectedHash, actualHash, validationErrors: [] },
    };
}
function descendantIds(document, root) {
    const doomed = new Set([root]);
    let changed = true;
    while (changed) {
        changed = false;
        for (const node of document.nodes) {
            if (node.parent !== null && doomed.has(node.parent) && !doomed.has(node.id)) {
                doomed.add(node.id);
                changed = true;
            }
        }
    }
    return doomed;
}
function createsCycle(document, id, parent) {
    let current = parent;
    while (current !== null) {
        if (current === id)
            return true;
        const parentNode = document.nodes.find((node) => node.id === current);
        current = parentNode?.parent ?? null;
    }
    return false;
}
function applyMockSceneObjectCommand(document, request) {
    const actualHash = sceneDocumentHash(document);
    if (request.expectedDocumentHash !== actualHash) {
        return commandRejection('stale-scene-object-snapshot', null, null, request.expectedDocumentHash, actualHash);
    }
    let next = cloneFlatSceneDocument(document);
    let selected = null;
    switch (request.command.kind) {
        case 'create': {
            if (nodeIndex(next, request.command.record.id) !== -1) {
                return commandRejection('duplicate-scene-object', request.command.record.id);
            }
            if (request.command.record.parent !== null && nodeIndex(next, request.command.record.parent) === -1) {
                return commandRejection('missing-scene-object-parent', request.command.record.id, request.command.record.parent);
            }
            next = { ...next, nodes: [...next.nodes, request.command.record] };
            break;
        }
        case 'delete': {
            if (nodeIndex(next, request.command.id) === -1) {
                return commandRejection('missing-scene-object', request.command.id);
            }
            const doomed = descendantIds(next, request.command.id);
            next = { ...next, nodes: next.nodes.filter((node) => !doomed.has(node.id)) };
            break;
        }
        case 'rename': {
            if (request.command.label !== null && request.command.label.trim() === '') {
                return commandRejection('blank-scene-object-label', request.command.id);
            }
            const id = request.command.id;
            const label = request.command.label;
            const index = nodeIndex(next, id);
            if (index === -1)
                return commandRejection('missing-scene-object', id);
            const node = next.nodes[index];
            next = { ...next, nodes: next.nodes.map((existing) => existing.id === id ? { ...node, label } : existing) };
            selected = id;
            break;
        }
        case 'reparent': {
            const id = request.command.id;
            const parent = request.command.parent;
            const childOrder = request.command.childOrder;
            const index = nodeIndex(next, id);
            if (index === -1)
                return commandRejection('missing-scene-object', id);
            if (parent === id)
                return commandRejection('scene-object-self-parent', id);
            if (parent !== null && nodeIndex(next, parent) === -1) {
                return commandRejection('missing-scene-object-parent', id, parent);
            }
            if (createsCycle(next, id, parent)) {
                return commandRejection('invalid-scene-after-command', id, parent);
            }
            const node = next.nodes[index];
            next = { ...next, nodes: next.nodes.map((existing) => existing.id === id ? { ...node, parent, childOrder } : existing) };
            selected = id;
            break;
        }
        case 'select': {
            if (request.command.id !== null && nodeIndex(next, request.command.id) === -1) {
                return commandRejection('missing-scene-object', request.command.id);
            }
            selected = request.command.id;
            break;
        }
    }
    const snapshot = sceneObjectSnapshotFromDocument(next);
    return {
        accepted: true,
        outcome: { document: next, snapshot, selected },
        rejection: null,
    };
}
export class MockRuntimeBridge {
    #engine = null;
    #buffer = new Uint8Array();
    #replaySteps = 0;
    #loadedWorld = null;
    #sceneDocument = initialMockSceneDocument();
    #nextCamera = 1;
    #cameras = new Map();
    initializeEngine(config) {
        if (!Number.isInteger(config.seed) || config.seed < 0) {
            throw new RuntimeBridgeError('invalid_input', `seed must be a non-negative integer`);
        }
        const handle = config.seed;
        this.#engine = handle;
        // Deterministic: little-endian seed bytes, mirroring ReferenceBridge.
        const bytes = new Uint8Array(8);
        new DataView(bytes.buffer).setBigUint64(0, BigInt(config.seed), true);
        this.#buffer = bytes;
        return handle;
    }
    stepSimulation(input) {
        if (this.#engine === null) {
            throw new RuntimeBridgeError('not_initialized', 'step before initializeEngine');
        }
        const tick = nonNegativeSafeInteger(input.tick, 'tick');
        return { tick, diffCount: tick % 4 };
    }
    submitCommands(batch) {
        if (this.#engine === null) {
            throw new RuntimeBridgeError('not_initialized', 'submitCommands before initializeEngine');
        }
        const rejections = [];
        for (const command of batch.commands) {
            const value = command.op === 'setVoxel' || command.op === 'fillRegion' ? command.value : null;
            if (value?.kind === 'solid' && (value.material < 1 || value.material > 16)) {
                rejections.push({ reason: 'unknownMaterial', material: value.material });
            }
        }
        return {
            accepted: batch.commands.length - rejections.length,
            rejected: rejections.length,
            rejections,
        };
    }
    pickVoxel(ray) {
        if (this.#engine === null) {
            throw new RuntimeBridgeError('not_initialized', 'pickVoxel before initializeEngine');
        }
        // The mock hosts no authority voxel geometry (Rust `svc-collision` owns the
        // raycast on the native path), so a pick always classifies as a miss. It still
        // fail-closes on the transport precondition (init) and validates the ray shape.
        if (ray.direction.every((c) => c === 0)) {
            throw new RuntimeBridgeError('invalid_input', 'pick ray direction must be non-zero');
        }
        return { outcome: 'miss', rejection: { reason: 'noHit' } };
    }
    applyCollisionConstrainedCameraInput(input) {
        if (this.#engine === null) {
            throw new RuntimeBridgeError('not_initialized', 'applyCollisionConstrainedCameraInput before initializeEngine');
        }
        if (input.grid !== 1) {
            throw new RuntimeBridgeError('invalid_input', 'collision camera input targets an unknown grid');
        }
        const before = this.#cameras.get(input.camera);
        if (before === undefined) {
            throw new RuntimeBridgeError('unknown_handle', 'unknown camera handle');
        }
        for (const [idx, halfExtent] of input.shape.halfExtents.entries()) {
            finite(halfExtent, `shape.halfExtents[${idx}]`);
            if (halfExtent <= 0) {
                throw new RuntimeBridgeError('invalid_input', 'collision shape halfExtents must be positive');
            }
        }
        if (input.policy.mode !== 'axis_separable_slide' || input.policy.maxIterations < 1 || input.policy.maxIterations > 3) {
            throw new RuntimeBridgeError('invalid_input', 'only axis_separable_slide with maxIterations in 1..=3 is supported');
        }
        const distance = input.input.dtSeconds * input.input.moveSpeedUnitsPerSecond;
        const attemptedPose = {
            position: [
                f32(before.pose.position[0] + before.basis.forward[0] * input.input.moveForward * distance + before.basis.right[0] * input.input.moveRight * distance + before.basis.up[0] * input.input.moveUp * distance),
                f32(before.pose.position[1] + before.basis.forward[1] * input.input.moveForward * distance + before.basis.right[1] * input.input.moveRight * distance + before.basis.up[1] * input.input.moveUp * distance),
                f32(before.pose.position[2] + before.basis.forward[2] * input.input.moveForward * distance + before.basis.right[2] * input.input.moveRight * distance + before.basis.up[2] * input.input.moveUp * distance),
            ],
            yawDegrees: before.pose.yawDegrees + input.input.yawDeltaDegrees,
            pitchDegrees: Math.max(-89, Math.min(89, before.pose.pitchDegrees + input.input.pitchDeltaDegrees)),
        };
        const attempted = { ...before, tick: input.tick, pose: attemptedPose, basis: basisFromPose(attemptedPose) };
        const after = attempted;
        this.#cameras.set(input.camera, after);
        return {
            camera: input.camera,
            tick: input.tick,
            before,
            attempted,
            after,
            collision: {
                grid: input.grid,
                shape: input.shape,
                policy: input.policy,
                collided: false,
                blockedAxes: [],
                correction: [0, 0, 0],
                queriedAabb: {
                    min: [
                        after.pose.position[0] - input.shape.halfExtents[0],
                        after.pose.position[1] - input.shape.halfExtents[1],
                        after.pose.position[2] - input.shape.halfExtents[2],
                    ],
                    max: [
                        after.pose.position[0] + input.shape.halfExtents[0],
                        after.pose.position[1] + input.shape.halfExtents[1],
                        after.pose.position[2] + input.shape.halfExtents[2],
                    ],
                },
                worldHash: 'mock-voxel-world',
                collisionProjectionHash: 'fnv1a64:mock-collision-projection',
            },
            movementHash: `fnv1a64:${fnv1a64(`${input.camera}|${input.tick}|${JSON.stringify(before.pose)}|${JSON.stringify(after.pose)}`)}`,
        };
    }
    selectVoxel(request) {
        if (this.#engine === null) {
            throw new RuntimeBridgeError('not_initialized', 'selectVoxel before initializeEngine');
        }
        const camera = this.#cameras.get(request.camera);
        if (camera === undefined) {
            throw new RuntimeBridgeError('unknown_handle', 'unknown camera handle');
        }
        if (request.grid !== 1) {
            throw new RuntimeBridgeError('invalid_input', 'selectVoxel request targets an unknown grid');
        }
        finite(request.maxDistance, 'maxDistance');
        if (request.maxDistance <= 0) {
            throw new RuntimeBridgeError('invalid_input', 'maxDistance must be positive');
        }
        const viewport = request.viewport ?? camera.viewport;
        validateViewport(viewport);
        const sx = request.screenPoint.space === 'pixel' ? request.screenPoint.x / viewport.width : request.screenPoint.x;
        const sy = request.screenPoint.space === 'pixel' ? request.screenPoint.y / viewport.height : request.screenPoint.y;
        if (!Number.isFinite(sx) || !Number.isFinite(sy) || sx < 0 || sx > 1 || sy < 0 || sy > 1) {
            throw new RuntimeBridgeError('invalid_input', 'screen point must be inside the viewport');
        }
        const ndcX = sx * 2 - 1;
        const ndcY = 1 - sy * 2;
        const tanY = Math.tan((camera.projection.fovYDegrees * Math.PI) / 360);
        const tanX = tanY * (viewport.width / viewport.height);
        const raw = [
            camera.basis.forward[0] + camera.basis.right[0] * ndcX * tanX + camera.basis.up[0] * ndcY * tanY,
            camera.basis.forward[1] + camera.basis.right[1] * ndcX * tanX + camera.basis.up[1] * ndcY * tanY,
            camera.basis.forward[2] + camera.basis.right[2] * ndcX * tanX + camera.basis.up[2] * ndcY * tanY,
        ];
        const len = Math.hypot(raw[0], raw[1], raw[2]);
        if (!Number.isFinite(len) || len <= 0) {
            throw new RuntimeBridgeError('invalid_input', 'derived pick ray direction is invalid');
        }
        const origin = [camera.pose.position[0], camera.pose.position[1], camera.pose.position[2]];
        const direction = [raw[0] / len, raw[1] / len, raw[2] / len];
        const pickRay = {
            camera: request.camera,
            tick: camera.tick,
            grid: request.grid,
            screenPoint: request.screenPoint,
            origin,
            direction,
            maxDistance: request.maxDistance,
            cameraProjectionHash: projectionSnapshot(camera, viewport).projectionHash,
            rayHash: `fnv1a64:${fnv1a64(`${request.camera}|${request.grid}|${origin.join(',')}|${direction.join(',')}|${request.maxDistance}`)}`,
        };
        // Mock fixture mirrors the canonical launch world enough for downstream
        // conformance: a flat solid terrain slab covering x/y [-16,16) at z=[0,1).
        let selectedVoxel = null;
        let selectedFace = null;
        let editAnchor = null;
        if (direction[2] < 0) {
            const t = (1 - origin[2]) / direction[2];
            const x = origin[0] + direction[0] * t;
            const y = origin[1] + direction[1] * t;
            if (t >= 0 && t <= request.maxDistance && x >= -16 && x < 16 && y >= 0 && y < 16) {
                selectedVoxel = { x: Math.floor(x), y: Math.floor(y), z: 0 };
                selectedFace = 'posZ';
                editAnchor = { x: selectedVoxel.x, y: selectedVoxel.y, z: 1 };
            }
        }
        const outcome = selectedVoxel === null ? 'miss' : 'hit';
        return {
            pickRay,
            outcome,
            selectedVoxel,
            selectedFace,
            editAnchor,
            selectionHash: `fnv1a64:${fnv1a64(`${pickRay.rayHash}|${outcome}|${JSON.stringify(selectedVoxel)}|${JSON.stringify(editAnchor)}`)}`,
        };
    }
    readVoxelMeshEvidence(request) {
        if (this.#engine === null) {
            throw new RuntimeBridgeError('not_initialized', 'readVoxelMeshEvidence before initializeEngine');
        }
        if (request.grid !== 1) {
            throw new RuntimeBridgeError('invalid_input', 'readVoxelMeshEvidence request targets an unknown grid');
        }
        const chunks = request.chunks.length === 0 ? [{ x: 0, y: 0, z: 0 }] : request.chunks;
        return {
            grid: request.grid,
            fixtureId: 'basic-voxel-landscape-interaction',
            worldHash: 'mock-voxel-world',
            meshingStrategy: 'visible-face',
            chunks: chunks.map((coord) => ({
                coord,
                resident: coord.x === 0 && coord.y === 0 && coord.z === 0,
                visible: coord.x === 0 && coord.y === 0 && coord.z === 0,
                contentHash: coord.x === 0 && coord.y === 0 && coord.z === 0 ? 'mock-content' : null,
                meshHash: coord.x === 0 && coord.y === 0 && coord.z === 0 ? 'fnv1a64:mock-mesh' : null,
                stats: coord.x === 0 && coord.y === 0 && coord.z === 0
                    ? { vertices: 48, indices: 72, quads: 12, facesEmitted: 12, facesCulled: 12 }
                    : null,
                bounds: coord.x === 0 && coord.y === 0 && coord.z === 0 ? { min: [0, 0, 0], max: [2, 2, 1] } : null,
                materialSlots: coord.x === 0 && coord.y === 0 && coord.z === 0 ? [1] : [],
            })),
            diagnostics: [],
        };
    }
    readModelMaterialPreview(request) {
        if (this.#engine === null) {
            throw new RuntimeBridgeError('not_initialized', 'readModelMaterialPreview before initializeEngine');
        }
        const entry = request.catalogEntry;
        if (entry.kind !== 'material' || entry.material === null) {
            throw new RuntimeBridgeError('invalid_input', `catalog entry '${entry.id}' is not a material`);
        }
        if (!request.meshAsset.materialSlots.some((slot) => slot.material === entry.id)) {
            throw new RuntimeBridgeError('invalid_input', `mesh asset '${request.meshAsset.asset}' does not reference material '${entry.id}'`);
        }
        return {
            catalogEntry: entry,
            material: entry.material,
            meshAsset: request.meshAsset,
            previewDiff: {
                ops: [
                    { op: 'defineMaterial', material: materialDescriptor(entry.id, entry.material) },
                    { op: 'defineStaticMesh', asset: request.meshAsset },
                    {
                        op: 'createStaticMeshInstance',
                        handle: request.instanceHandle,
                        parent: null,
                        instance: {
                            asset: request.meshAsset.asset,
                            transform: { translation: [0, 0, 0], rotation: [0, 0, 0, 1], scale: [1, 1, 1] },
                            materialOverrides: [],
                            metadata: { source: null, tags: [], label: `Preview ${request.meshAsset.asset}` },
                        },
                    },
                ],
            },
            rendererClassification: 'reference_preview',
            diagnostics: ['native runtime readback for model/material preview may fail closed until wired'],
        };
    }
    readSceneObjectSnapshot() {
        if (this.#engine === null) {
            throw new RuntimeBridgeError('not_initialized', 'readSceneObjectSnapshot before initializeEngine');
        }
        return sceneObjectSnapshotFromDocument(this.#sceneDocument);
    }
    applySceneObjectCommand(request) {
        if (this.#engine === null) {
            throw new RuntimeBridgeError('not_initialized', 'applySceneObjectCommand before initializeEngine');
        }
        const result = applyMockSceneObjectCommand(this.#sceneDocument, request);
        if (result.outcome !== null) {
            this.#sceneDocument = result.outcome.document;
        }
        return result;
    }
    readRenderDiffs(cursor) {
        if (this.#engine === null) {
            throw new RuntimeBridgeError('not_initialized', 'readRenderDiffs before initializeEngine');
        }
        if (!Number.isInteger(cursor) || cursor < 0) {
            throw new RuntimeBridgeError('invalid_input', `frame cursor must be a non-negative integer`);
        }
        return { ops: [] };
    }
    createCamera(request) {
        if (this.#engine === null) {
            throw new RuntimeBridgeError('not_initialized', 'createCamera before initializeEngine');
        }
        validateProjection(request.projection);
        validateViewport(request.viewport);
        for (const [index, value] of request.initialPose.position.entries()) {
            finite(value, `initialPose.position[${index}]`);
        }
        finite(request.initialPose.yawDegrees, 'initialPose.yawDegrees');
        finite(request.initialPose.pitchDegrees, 'initialPose.pitchDegrees');
        const camera = this.#nextCamera++;
        const snapshot = {
            camera,
            tick: 0,
            pose: request.initialPose,
            basis: basisFromPose(request.initialPose),
            projection: request.projection,
            viewport: request.viewport,
        };
        this.#cameras.set(camera, snapshot);
        return snapshot;
    }
    applyFirstPersonCameraInput(envelope) {
        if (this.#engine === null) {
            throw new RuntimeBridgeError('not_initialized', 'applyFirstPersonCameraInput before initializeEngine');
        }
        const prior = this.#cameras.get(envelope.camera);
        if (!prior) {
            throw new RuntimeBridgeError('unknown_handle', `no camera for handle ${envelope.camera}`);
        }
        const i = envelope.input;
        finite(i.moveForward, 'moveForward');
        finite(i.moveRight, 'moveRight');
        finite(i.moveUp, 'moveUp');
        finite(i.yawDeltaDegrees, 'yawDeltaDegrees');
        finite(i.pitchDeltaDegrees, 'pitchDeltaDegrees');
        finite(i.dtSeconds, 'dtSeconds');
        finite(i.moveSpeedUnitsPerSecond, 'moveSpeedUnitsPerSecond');
        if (i.dtSeconds < 0 || i.moveSpeedUnitsPerSecond < 0) {
            throw new RuntimeBridgeError('invalid_input', 'dtSeconds and moveSpeedUnitsPerSecond must be non-negative');
        }
        const basis = prior.basis;
        const distance = f32(i.dtSeconds * i.moveSpeedUnitsPerSecond);
        const position = [
            f32(prior.pose.position[0] +
                f32(f32(basis.forward[0] * i.moveForward) +
                    f32(basis.right[0] * i.moveRight) +
                    f32(basis.up[0] * i.moveUp)) *
                    distance),
            f32(prior.pose.position[1] +
                f32(f32(basis.forward[1] * i.moveForward) +
                    f32(basis.right[1] * i.moveRight) +
                    f32(basis.up[1] * i.moveUp)) *
                    distance),
            f32(prior.pose.position[2] +
                f32(f32(basis.forward[2] * i.moveForward) +
                    f32(basis.right[2] * i.moveRight) +
                    f32(basis.up[2] * i.moveUp)) *
                    distance),
        ];
        const pitchDegrees = Math.max(-89, Math.min(89, f32(prior.pose.pitchDegrees + i.pitchDeltaDegrees)));
        const pose = {
            position,
            yawDegrees: f32(prior.pose.yawDegrees + i.yawDeltaDegrees),
            pitchDegrees,
        };
        const snapshot = {
            ...prior,
            tick: envelope.tick,
            pose,
            basis: basisFromPose(pose),
        };
        this.#cameras.set(envelope.camera, snapshot);
        return snapshot;
    }
    readCameraProjection(request) {
        if (this.#engine === null) {
            throw new RuntimeBridgeError('not_initialized', 'readCameraProjection before initializeEngine');
        }
        const snapshot = this.#cameras.get(request.camera);
        if (!snapshot) {
            throw new RuntimeBridgeError('unknown_handle', `no camera for handle ${request.camera}`);
        }
        if (request.viewport !== null)
            validateViewport(request.viewport);
        return projectionSnapshot(snapshot, request.viewport ?? snapshot.viewport);
    }
    getBuffer(handle) {
        if (handle !== 0) {
            throw new RuntimeBridgeError('unknown_handle', `no buffer for handle ${handle}`);
        }
        return { handle, bytes: this.#buffer };
    }
    releaseBuffer(handle) {
        if (handle !== 0) {
            throw new RuntimeBridgeError('unknown_handle', `no buffer for handle ${handle}`);
        }
        this.#buffer = new Uint8Array();
    }
    loadWorldBundle(request) {
        const bundleSchemaVersion = u32(request.bundleSchemaVersion, 'bundleSchemaVersion');
        const protocolVersion = u32(request.protocolVersion, 'protocolVersion');
        const sceneId = nonNegativeSafeInteger(request.sceneId, 'sceneId');
        // Fail closed on a newer bundle; the prior loaded world is left untouched
        // (we only set #loadedWorld on success — the staged commit/swap).
        if (bundleSchemaVersion > 1 || protocolVersion > 1) {
            throw new RuntimeBridgeError('invalid_input', `unsupported bundle schema ${bundleSchemaVersion} / protocol ${protocolVersion}`);
        }
        this.#loadedWorld = sceneId;
        return { loadedWorld: sceneId, fatalCount: 0, totalCount: 0, blocksLoad: false };
    }
    saveCurrentWorld() {
        if (this.#loadedWorld === null) {
            throw new RuntimeBridgeError('not_initialized', 'saveCurrentWorld with no world loaded');
        }
        return { artifactsWritten: 3, compactedEdits: 0, retainedEdits: 0 };
    }
    getCompositionStatus() {
        return { loadedWorld: this.#loadedWorld, fatalCount: 0, totalCount: 0, blocksLoad: false };
    }
    unloadWorld() {
        this.#loadedWorld = null;
    }
    loadReplayFixture(fixture) {
        this.#replaySteps = fixture.steps;
        return 0;
    }
    runReplayStep(session) {
        const step = this.#replaySteps;
        this.#replaySteps = Math.max(0, this.#replaySteps - 1);
        return { step, hash: `mock-${session}-${step}`, diverged: false };
    }
}
/** Construct the default mock bridge. */
export function createMockRuntimeBridge() {
    return new MockRuntimeBridge();
}
function requireNonEmpty(value, field) {
    if (value.trim().length === 0) {
        throw new RuntimeBridgeError('invalid_input', `${field} must be a non-empty string`);
    }
    return value;
}
function referenceNonClaims() {
    return ['not_native_runtime', 'not_hardware_gpu', 'not_performance_evidence', 'not_publish_artifact', 'not_wasm_authority'];
}
function referenceRuntimeProfile(config) {
    return {
        profileId: 'reference.launcher.v1',
        runtimeMode: 'reference',
        launcherName: 'reference-game-runtime-launcher',
        bridgeCompatibility: config.compatibility,
        nonClaims: referenceNonClaims(),
    };
}
function projectionSummary(config, status, sequenceId, acceptedCommandCount) {
    const loadedWorld = status.loadedWorld;
    const worldHash = `reference-world:${config.gameId}:${loadedWorld ?? 'none'}:accepted:${acceptedCommandCount}`;
    const authorityHash = `reference-authority:${config.workspaceId}:${loadedWorld ?? 'none'}:accepted:${acceptedCommandCount}`;
    return {
        sequenceId,
        worldHash,
        authorityHash,
        loadedWorld,
        fatalCount: status.fatalCount,
        totalDiagnosticCount: status.totalCount,
        evidenceRefs: [{ kind: 'projection', id: `projection:${sequenceId}`, sequenceId }],
    };
}
class ReferenceGameRuntimeSession {
    bridge;
    config;
    identity;
    launch;
    #sequenceId = 0;
    #acceptedCommandCount = 0;
    #rejectedCommandCount = 0;
    #shutdown = false;
    constructor(bridge, config, runtimeProfile, initialStatus) {
        this.bridge = bridge;
        this.config = config;
        const startedAtIso = config.startedAtIso ?? new Date(0).toISOString();
        this.identity = {
            gameId: config.gameId,
            workspaceId: config.workspaceId,
            runtimeMode: 'reference',
            runtimeEntry: config.runtimeEntry,
            startedAtIso,
            compatibility: config.compatibility,
            nonClaims: runtimeProfile.nonClaims,
        };
        const projection = projectionSummary(config, initialStatus, this.#sequenceId, this.#acceptedCommandCount);
        this.launch = {
            status: 'launched',
            identity: this.identity,
            runtimeProfile,
            resourceProfile: config.resourceProfile,
            projection,
            diagnostics: [],
            evidenceRefs: projection.evidenceRefs,
        };
    }
    async pullProjection() {
        this.#assertOpen();
        return projectionSummary(this.config, this.bridge.getCompositionStatus(), this.#sequenceId, this.#acceptedCommandCount);
    }
    async pullRenderDiff(cursor = frameCursor(0)) {
        this.#assertOpen();
        const frame = this.bridge.readRenderDiffs(cursor);
        return {
            sequenceId: this.#sequenceId,
            cursor,
            frame,
            evidenceRefs: [{ kind: 'render_diff', id: `render-diff:${cursor}`, sequenceId: this.#sequenceId }],
        };
    }
    async pullTelemetry() {
        this.#assertOpen();
        return {
            sequenceId: this.#sequenceId,
            runtimeMode: 'reference',
            acceptedCommandCount: this.#acceptedCommandCount,
            rejectedCommandCount: this.#rejectedCommandCount,
            diagnostics: [],
            evidenceRefs: [{ kind: 'telemetry', id: `telemetry:${this.#sequenceId}`, sequenceId: this.#sequenceId }],
        };
    }
    async proposeCommands(batch) {
        this.#assertOpen();
        const before = await this.pullProjection();
        this.#sequenceId += 1;
        try {
            const result = this.bridge.submitCommands(batch);
            this.#acceptedCommandCount += result.accepted;
            this.#rejectedCommandCount += result.rejected;
            const after = await this.pullProjection();
            return {
                sequenceId: this.#sequenceId,
                status: result.rejected > 0 ? 'rejected' : 'accepted',
                batch,
                result,
                authorityHashBefore: before.authorityHash,
                authorityHashAfter: after.authorityHash,
                diagnostics: result.rejections.map((rejection, index) => ({
                    code: 'command_rejected',
                    severity: 'warning',
                    message: `command ${index} rejected: ${rejection.reason}`,
                })),
                evidenceRefs: [{ kind: 'replay', id: `command:${this.#sequenceId}`, sequenceId: this.#sequenceId }],
            };
        }
        catch (cause) {
            const error = cause instanceof RuntimeBridgeError ? cause : new RuntimeBridgeError('internal', String(cause));
            const after = await this.pullProjection();
            return {
                sequenceId: this.#sequenceId,
                status: 'failed',
                batch,
                result: null,
                authorityHashBefore: before.authorityHash,
                authorityHashAfter: after.authorityHash,
                diagnostics: [{ code: 'internal', severity: 'error', message: error.message }],
                evidenceRefs: [{ kind: 'diagnostic', id: `command-failed:${this.#sequenceId}`, sequenceId: this.#sequenceId }],
            };
        }
    }
    async exportReplay(request) {
        this.#assertOpen();
        requireNonEmpty(request.replayId, 'replayId');
        const projection = await this.pullProjection();
        return {
            replayId: request.replayId,
            sequenceId: this.#sequenceId,
            authorityHash: projection.authorityHash,
            evidenceRefs: [{ kind: 'replay', id: request.replayId, sequenceId: this.#sequenceId }],
        };
    }
    async exportEvidence(request) {
        this.#assertOpen();
        requireNonEmpty(request.evidenceId, 'evidenceId');
        const projection = await this.pullProjection();
        return {
            evidenceId: request.evidenceId,
            sequenceId: this.#sequenceId,
            projection,
            nonClaims: this.identity.nonClaims,
            evidenceRefs: [{ kind: 'evidence_export', id: request.evidenceId, sequenceId: this.#sequenceId }],
        };
    }
    async shutdown() {
        if (this.#shutdown)
            return;
        this.bridge.unloadWorld();
        this.#shutdown = true;
    }
    #assertOpen() {
        if (this.#shutdown) {
            throw new RuntimeBridgeError('not_initialized', 'game runtime session has been shut down');
        }
    }
}
export class ReferenceGameRuntimeLauncher {
    mode = 'reference';
    async launch(config) {
        requireNonEmpty(config.gameId, 'gameId');
        requireNonEmpty(config.workspaceId, 'workspaceId');
        requireNonEmpty(config.runtimeEntry, 'runtimeEntry');
        requireNonEmpty(config.compatibility.contractsPackageVersion, 'compatibility.contractsPackageVersion');
        requireNonEmpty(config.compatibility.runtimeBridgePackageVersion, 'compatibility.runtimeBridgePackageVersion');
        if (config.runtimeEntry !== config.resourceProfile.runtimeEntry) {
            throw new RuntimeBridgeError('invalid_input', 'runtimeEntry must match resourceProfile.runtimeEntry');
        }
        const bridge = createMockRuntimeBridge();
        bridge.initializeEngine({ seed: config.world.sceneId });
        const status = bridge.loadWorldBundle(config.world);
        if (status.blocksLoad || status.loadedWorld === null) {
            throw new RuntimeBridgeError('invalid_input', 'world bundle failed to load for reference launcher');
        }
        return new ReferenceGameRuntimeSession(bridge, config, referenceRuntimeProfile(config), status);
    }
}
export function createReferenceGameRuntimeLauncher() {
    return new ReferenceGameRuntimeLauncher();
}
// ── Native implementation factory ─────────────────────────────────────────────
// The ONLY place that touches `@asha/native-bridge`. Wraps the addon's wired
// exports and re-classifies load failures into the bridge error taxonomy.
//
// Fail-closed by construction: `NativeRuntimeBridge` implements `RuntimeBridge`
// directly — it does NOT extend `MockRuntimeBridge`, so an unwired operation can
// never silently inherit mock/reference behaviour. Every stable + quarantined
// operation is either routed to a real `#[napi]` export (and listed in
// NATIVE_WIRED_OPERATIONS) or throws a classified `operation_unimplemented`.
// `native-fail-closed.test.ts` enforces that this stays true for every manifest op.
/**
 * Manifest names of operations whose native (`#[napi]`) implementation is actually
 * wired. Everything else on {@link NativeRuntimeBridge} fail-closes with
 * `operation_unimplemented`. Adding a name here is the explicit signal that a
 * native implementation landed; the native conformance test keeps this set and the
 * routed methods in lockstep with the bridge manifest.
 */
export const NATIVE_WIRED_OPERATIONS = new Set([
    'initialize_engine',
    'load_world_bundle',
    'submit_commands',
    'step_simulation',
    'read_render_diffs',
    'save_current_world',
    'get_composition_status',
]);
function nativeUnimplemented(manifestName) {
    return new RuntimeBridgeError('operation_unimplemented', `native bridge operation '${manifestName}' is not wired; the native facade is ` +
        `fail-closed (no mock fallback). Wire its #[napi] export and add it to ` +
        `NATIVE_WIRED_OPERATIONS.`);
}
const RUST_ERROR_KIND = {
    NotInitialized: 'not_initialized',
    InvalidInput: 'invalid_input',
    UnknownHandle: 'unknown_handle',
    BufferExpired: 'buffer_expired',
    Internal: 'internal',
};
function classifyNativeAddonError(cause) {
    if (cause instanceof RuntimeBridgeError)
        return cause;
    const message = cause instanceof Error ? cause.message : String(cause);
    const match = /^(\w+):\s*(.*)$/u.exec(message);
    if (match?.[1]) {
        const kind = RUST_ERROR_KIND[match[1]];
        if (kind)
            return new RuntimeBridgeError(kind, match[2] || message);
    }
    return new RuntimeBridgeError('internal', message);
}
function callNative(body) {
    try {
        return body();
    }
    catch (cause) {
        throw classifyNativeAddonError(cause);
    }
}
export class NativeRuntimeBridge {
    #addon;
    #seed = 0;
    #initialized = false;
    #engineHandle = null;
    constructor(addon) {
        this.#addon = addon;
    }
    // ── Wired native operations ───────────────────────────────────────────────
    initializeEngine(config) {
        if (!Number.isInteger(config.seed) || config.seed < 0) {
            throw new RuntimeBridgeError('invalid_input', `seed must be a non-negative integer`);
        }
        this.#seed = config.seed;
        const handle = this.#addon.initializeEngine(config.seed);
        this.#engineHandle = handle;
        this.#initialized = true;
        return handle;
    }
    #requireHandle(operation) {
        if (!this.#initialized || this.#engineHandle === null) {
            throw new RuntimeBridgeError('not_initialized', `${operation} before initializeEngine`);
        }
        return this.#engineHandle;
    }
    loadWorldBundle(request) {
        const handle = this.#requireHandle('loadWorldBundle');
        const bundleSchemaVersion = u32(request.bundleSchemaVersion, 'bundleSchemaVersion');
        const protocolVersion = u32(request.protocolVersion, 'protocolVersion');
        const sceneId = nonNegativeSafeInteger(request.sceneId, 'sceneId');
        return callNative(() => this.#addon.loadWorldBundle(handle, bundleSchemaVersion, protocolVersion, sceneId));
    }
    submitCommands(batch) {
        const handle = this.#requireHandle('submitCommands');
        return callNative(() => this.#addon.submitCommands(handle, JSON.stringify(batch.commands)));
    }
    stepSimulation(input) {
        const handle = this.#requireHandle('stepSimulation');
        const tick = nonNegativeSafeInteger(input.tick, 'tick');
        const diffCount = callNative(() => this.#addon.stepSimulation(handle, tick));
        return { tick, diffCount };
    }
    readModelMaterialPreview(_request) {
        throw nativeUnimplemented('read_model_material_preview');
    }
    readSceneObjectSnapshot() {
        throw nativeUnimplemented('read_scene_object_snapshot');
    }
    applySceneObjectCommand() {
        throw nativeUnimplemented('apply_scene_object_command');
    }
    readRenderDiffs(cursor) {
        const handle = this.#requireHandle('readRenderDiffs');
        const frame = nonNegativeSafeInteger(cursor, 'frame cursor');
        return callNative(() => this.#addon.readRenderDiffs(handle, frame));
    }
    saveCurrentWorld() {
        const handle = this.#requireHandle('saveCurrentWorld');
        return callNative(() => this.#addon.saveCurrentWorld(handle));
    }
    getCompositionStatus() {
        const handle = this.#requireHandle('getCompositionStatus');
        return callNative(() => this.#addon.getCompositionStatus(handle));
    }
    // ── Unwired operations: fail-closed, never mock-backed ─────────────────────
    // Replace each body with its real native call (and add the manifest name to
    // NATIVE_WIRED_OPERATIONS) when the codegen emitter wires the `#[napi]` export.
    pickVoxel() {
        throw nativeUnimplemented('pick_voxel');
    }
    applyCollisionConstrainedCameraInput() {
        throw nativeUnimplemented('apply_collision_constrained_camera_input');
    }
    selectVoxel() {
        throw nativeUnimplemented('select_voxel');
    }
    readVoxelMeshEvidence() {
        throw nativeUnimplemented('read_voxel_mesh_evidence');
    }
    createCamera() {
        throw nativeUnimplemented('create_camera');
    }
    applyFirstPersonCameraInput() {
        throw nativeUnimplemented('apply_first_person_camera_input');
    }
    readCameraProjection() {
        throw nativeUnimplemented('read_camera_projection');
    }
    getBuffer() {
        throw nativeUnimplemented('get_buffer');
    }
    releaseBuffer() {
        throw nativeUnimplemented('release_buffer');
    }
    unloadWorld() {
        throw nativeUnimplemented('unload_world');
    }
    loadReplayFixture() {
        throw nativeUnimplemented('load_replay_fixture');
    }
    runReplayStep() {
        throw nativeUnimplemented('run_replay_step');
    }
}
/**
 * Construct the native (napi-rs) bridge. Throws a classified
 * {@link RuntimeBridgeError} of kind `native_unavailable` if the addon is not built
 * — callers can fall back to the mock for tests/dev.
 */
export function createNativeRuntimeBridge(modulePath) {
    try {
        const addon = modulePath ? loadNativeAddon(modulePath) : loadNativeAddon();
        return new NativeRuntimeBridge(addon);
    }
    catch (cause) {
        if (cause instanceof NativeAddonUnavailable) {
            throw new RuntimeBridgeError('native_unavailable', cause.message);
        }
        throw cause;
    }
}
/** Operation count for quick sanity in consumers/tests. */
export const STABLE_OPERATION_COUNT = MANIFEST_OPERATIONS.filter((o) => o.surface === 'stable').length;
//# sourceMappingURL=index.js.map