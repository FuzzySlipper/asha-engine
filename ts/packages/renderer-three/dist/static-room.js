import { renderHandle, } from '@asha/contracts';
export const STATIC_ROOM_FIXTURE_NAME = 'static-room';
const IDENTITY_ROTATION = [0, 0, 0, 1];
const IDENTITY_SCALE = [1, 1, 1];
export function createStaticRoomRenderFrame() {
    return {
        ops: [
            material('material/room-floor', [0.44, 0.48, 0.46, 1]),
            material('material/room-wall', [0.68, 0.73, 0.76, 1]),
            material('material/room-ceiling', [0.86, 0.88, 0.84, 1]),
            material('material/room-marker', [0.92, 0.42, 0.18, 1]),
            { op: 'defineStaticMesh', asset: panelAsset('mesh/room-floor', 'material/room-floor') },
            { op: 'defineStaticMesh', asset: panelAsset('mesh/room-wall', 'material/room-wall') },
            { op: 'defineStaticMesh', asset: panelAsset('mesh/room-ceiling', 'material/room-ceiling') },
            { op: 'defineStaticMesh', asset: panelAsset('mesh/room-marker', 'material/room-marker') },
            instance(1, 'mesh/room-floor', 'room-floor', [0, -1, 0], [8, 1, 8]),
            instance(2, 'mesh/room-wall', 'room-wall-north', [0, 1, -4], [8, 4, 1]),
            instance(3, 'mesh/room-wall', 'room-wall-south', [0, 1, 4], [8, 4, 1]),
            instance(4, 'mesh/room-wall', 'room-wall-west', [-4, 1, 0], [1, 4, 8]),
            instance(5, 'mesh/room-wall', 'room-wall-east', [4, 1, 0], [1, 4, 8]),
            instance(6, 'mesh/room-ceiling', 'room-ceiling', [0, 3, 0], [8, 1, 8]),
            instance(7, 'mesh/room-marker', 'room-origin-marker', [0, -0.48, 0], [0.5, 0.04, 0.5]),
        ],
    };
}
function material(id, color) {
    return {
        op: 'defineMaterial',
        material: {
            id,
            color,
            texture: null,
            roughness: 1,
            emissive: 0,
            uvStrategy: 'flat',
        },
    };
}
function panelAsset(asset, materialId) {
    return {
        asset,
        payload: quadPayload(),
        materialSlots: [{ slot: 0, material: materialId }],
        collision: { kind: 'aabbFallback' },
    };
}
function quadPayload() {
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
        groups: [{ materialSlot: 0, start: 0, count: 6 }],
        bounds: { min: [-0.5, -0.5, 0], max: [0.5, 0.5, 0] },
        source: {
            kind: 'inline',
            positions: [-0.5, -0.5, 0, 0.5, -0.5, 0, 0.5, 0.5, 0, -0.5, 0.5, 0],
            normals: [0, 0, 1, 0, 0, 1, 0, 0, 1, 0, 0, 1],
            indices: [0, 1, 2, 0, 2, 3],
        },
        provenance: 'generated',
    };
}
function instance(handle, asset, label, translation, scale) {
    return {
        op: 'createStaticMeshInstance',
        handle: renderHandle(handle),
        parent: null,
        instance: {
            asset,
            transform: transform(translation, scale),
            materialOverrides: [],
            metadata: { source: null, tags: [], label },
        },
    };
}
function transform(translation, scale) {
    return {
        translation,
        rotation: IDENTITY_ROTATION,
        scale,
    };
}
//# sourceMappingURL=static-room.js.map