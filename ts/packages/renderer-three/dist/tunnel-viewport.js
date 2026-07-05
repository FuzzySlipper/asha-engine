import { renderHandle, } from '@asha/contracts';
export const FIRST_PERSON_TUNNEL_VIEWPORT_FIXTURE_NAME = 'generated-tunnel-first-person-viewport';
const IDENTITY_ROTATION = [0, 0, 0, 1];
const DEFAULT_TUNNEL_VIEWPORT_MATERIALS = {
    wall: [0.42, 0.46, 0.5, 1],
    floor: [0.25, 0.32, 0.29, 1],
    accent: [0.95, 0.62, 0.18, 1],
    playerMarker: [0.18, 0.68, 0.92, 1],
    exitMarker: [0.72, 0.5, 0.94, 1],
};
export function createGeneratedTunnelViewportFrame(tunnel, materials = {}) {
    const palette = {
        ...DEFAULT_TUNNEL_VIEWPORT_MATERIALS,
        ...materials,
    };
    const [width, height, length] = tunnel.volume.tunnelDims;
    const center = [width / 2, height / 2, length / 2];
    return {
        ops: [
            material('material/generated-tunnel-wall', palette.wall),
            material('material/generated-tunnel-floor', palette.floor),
            material('material/generated-tunnel-accent', palette.accent),
            material('material/generated-tunnel-player-marker', palette.playerMarker),
            material('material/generated-tunnel-exit-marker', palette.exitMarker),
            { op: 'defineStaticMesh', asset: panelAsset('mesh/generated-tunnel-floor', 'material/generated-tunnel-floor') },
            { op: 'defineStaticMesh', asset: panelAsset('mesh/generated-tunnel-wall', 'material/generated-tunnel-wall') },
            { op: 'defineStaticMesh', asset: panelAsset('mesh/generated-tunnel-accent', 'material/generated-tunnel-accent') },
            {
                op: 'defineStaticMesh',
                asset: panelAsset('mesh/generated-tunnel-player-marker', 'material/generated-tunnel-player-marker'),
            },
            {
                op: 'defineStaticMesh',
                asset: panelAsset('mesh/generated-tunnel-exit-marker', 'material/generated-tunnel-exit-marker'),
            },
            instance(100, 'mesh/generated-tunnel-floor', 'generated-tunnel-floor', [center[0], 0, center[2]], [
                width,
                1,
                length,
            ]),
            instance(101, 'mesh/generated-tunnel-wall', 'generated-tunnel-ceiling', [center[0], height, center[2]], [
                width,
                1,
                length,
            ]),
            instance(102, 'mesh/generated-tunnel-wall', 'generated-tunnel-wall-west', [0, center[1], center[2]], [
                1,
                height,
                length,
            ]),
            instance(103, 'mesh/generated-tunnel-wall', 'generated-tunnel-wall-east', [width, center[1], center[2]], [
                1,
                height,
                length,
            ]),
            instance(104, 'mesh/generated-tunnel-accent', 'generated-tunnel-entrance-cap', [center[0], center[1], 0], [
                width,
                height,
                1,
            ]),
            instance(105, 'mesh/generated-tunnel-accent', 'generated-tunnel-exit-cap', [center[0], center[1], length], [
                width,
                height,
                1,
            ]),
            ...tunnel.spawnMarkers.map((marker, index) => instance(120 + index, marker.kind === 'player' ? 'mesh/generated-tunnel-player-marker' : 'mesh/generated-tunnel-exit-marker', `generated-tunnel-spawn-${marker.id}`, marker.world, [0.35, 0.35, 0.35])),
        ],
    };
}
export function summarizeFirstPersonTunnelViewport(input) {
    const frameHash = viewportStableHash(frameHashRecord(input.frame));
    const structuralHash = viewportStableHash({
        frameHash,
        snapshot: input.structuralSnapshot ?? null,
    });
    return {
        kind: 'first_person_tunnel_viewport.v0',
        fixture: FIRST_PERSON_TUNNEL_VIEWPORT_FIXTURE_NAME,
        presetId: input.tunnel.generator.presetId,
        seed: input.tunnel.generator.seed,
        camera: {
            camera: input.camera.camera,
            tick: input.camera.tick,
            position: input.camera.pose.position,
            yawDegrees: input.camera.pose.yawDegrees,
            pitchDegrees: input.camera.pose.pitchDegrees,
            projectionHash: input.camera.projectionHash,
            viewport: {
                width: input.camera.viewport.width,
                height: input.camera.viewport.height,
            },
        },
        tunnel: {
            dims: input.tunnel.volume.tunnelDims,
            solidVoxels: input.tunnel.volume.solidVoxels,
            spawnMarkers: input.tunnel.spawnMarkers.map((marker) => marker.id),
            materialRoles: input.tunnel.materials.map((entry) => `${entry.role}:${entry.material}`),
        },
        debug: {
            generatorHash: input.tunnel.generator.generationHash,
            outputHash: input.tunnel.generator.outputHash,
            renderProjectionHash: input.tunnel.renderProjection.hash,
            collisionProjectionHash: input.tunnel.collisionProjection.hash,
            replayHash: input.tunnel.replayHash,
            collision: input.collision ?? null,
        },
        scene: {
            frameHash,
            structuralHash,
            opCount: input.frame.ops.length,
            instanceCount: input.frame.ops.filter((op) => op.op === 'createStaticMeshInstance').length,
        },
        nonClaims: [
            'not_runtime_authority',
            'not_collision_authority',
            'not_local_generation',
            'not_pixel_golden',
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
function frameHashRecord(frame) {
    return {
        opCount: frame.ops.length,
        materialIds: frame.ops
            .filter((op) => op.op === 'defineMaterial')
            .map((op) => op.material.id),
        instanceLabels: frame.ops
            .filter((op) => op.op === 'createStaticMeshInstance')
            .map((op) => op.instance.metadata.label ?? ''),
    };
}
function viewportStableHash(value) {
    return `fnv1a64:${viewportFnv1a64(viewportStableStringify(value))}`;
}
function viewportStableStringify(value) {
    if (value === undefined) {
        return 'undefined';
    }
    if (value === null || typeof value !== 'object') {
        return JSON.stringify(value);
    }
    if (Array.isArray(value)) {
        const entries = value;
        return `[${entries.map((entry) => viewportStableStringify(entry)).join(',')}]`;
    }
    const record = value;
    return `{${Object.keys(record)
        .sort()
        .map((key) => `${JSON.stringify(key)}:${viewportStableStringify(record[key])}`)
        .join(',')}}`;
}
function viewportFnv1a64(text) {
    let hash = 0xcbf29ce484222325n;
    const prime = 0x100000001b3n;
    const mask = 0xffffffffffffffffn;
    for (let index = 0; index < text.length; index += 1) {
        hash ^= BigInt(text.charCodeAt(index));
        hash = (hash * prime) & mask;
    }
    return hash.toString(16).padStart(16, '0');
}
//# sourceMappingURL=tunnel-viewport.js.map