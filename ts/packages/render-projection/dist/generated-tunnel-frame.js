import { renderHandle, } from '@asha/contracts';
export const FIRST_PERSON_TUNNEL_VIEWPORT_FIXTURE_NAME = 'generated-tunnel-first-person-viewport';
const IDENTITY_ROTATION = [0, 0, 0, 1];
const DEFAULT_TUNNEL_VIEWPORT_MATERIALS = {
    wall: [0.42, 0.46, 0.5, 1],
    floor: [0.25, 0.32, 0.29, 1],
    accent: [0.5, 0.55, 0.62, 1],
    playerMarker: [0.18, 0.68, 0.92, 1],
    exitMarker: [0.72, 0.5, 0.94, 1],
};
export function createGeneratedTunnelViewportFrame(tunnel, materials = {}) {
    const palette = {
        ...DEFAULT_TUNNEL_VIEWPORT_MATERIALS,
        ...materials,
    };
    const { playableMin, playableMax, worldOffset } = tunnel.runtimeFrame;
    const width = playableMax[0] - playableMin[0];
    const height = playableMax[1] - playableMin[1];
    const length = playableMax[2] - playableMin[2];
    const center = [
        (playableMin[0] + playableMax[0]) / 2,
        (playableMin[1] + playableMax[1]) / 2,
        (playableMin[2] + playableMax[2]) / 2,
    ];
    return {
        ops: [
            material('material/generated-tunnel-wall', palette.wall),
            material('material/generated-tunnel-floor', palette.floor),
            material('material/generated-tunnel-accent', palette.accent),
            material('material/generated-tunnel-player-marker', palette.playerMarker),
            material('material/generated-tunnel-exit-marker', palette.exitMarker),
            { op: 'defineStaticMesh', asset: cuboidAsset('mesh/generated-tunnel-floor', 'material/generated-tunnel-floor') },
            { op: 'defineStaticMesh', asset: cuboidAsset('mesh/generated-tunnel-wall', 'material/generated-tunnel-wall') },
            { op: 'defineStaticMesh', asset: cuboidAsset('mesh/generated-tunnel-accent', 'material/generated-tunnel-accent') },
            {
                op: 'defineStaticMesh',
                asset: cuboidAsset('mesh/generated-tunnel-player-marker', 'material/generated-tunnel-player-marker'),
            },
            {
                op: 'defineStaticMesh',
                asset: cuboidAsset('mesh/generated-tunnel-exit-marker', 'material/generated-tunnel-exit-marker'),
            },
            instance(100, 'mesh/generated-tunnel-floor', 'generated-tunnel-floor', [center[0], playableMin[1] - 0.05, center[2]], [
                width,
                0.1,
                length,
            ]),
            instance(101, 'mesh/generated-tunnel-wall', 'generated-tunnel-ceiling', [center[0], playableMax[1] + 0.05, center[2]], [
                width,
                0.1,
                length,
            ]),
            instance(102, 'mesh/generated-tunnel-wall', 'generated-tunnel-wall-west', [playableMin[0] - 0.05, center[1], center[2]], [
                0.1,
                height,
                length,
            ]),
            instance(103, 'mesh/generated-tunnel-wall', 'generated-tunnel-wall-east', [playableMax[0] + 0.05, center[1], center[2]], [
                0.1,
                height,
                length,
            ]),
            instance(104, 'mesh/generated-tunnel-accent', 'generated-tunnel-entrance-cap', [center[0], center[1], playableMin[2] - 0.05], [
                width,
                height,
                0.1,
            ]),
            instance(105, 'mesh/generated-tunnel-accent', 'generated-tunnel-exit-cap', [center[0], center[1], playableMax[2] + 0.05], [
                width,
                height,
                0.1,
            ]),
            ...tunnel.spawnMarkers.map((marker, index) => instance(120 + index, marker.kind === 'player' ? 'mesh/generated-tunnel-player-marker' : 'mesh/generated-tunnel-exit-marker', `generated-tunnel-spawn-${marker.id}`, [
                marker.world[0] + worldOffset[0],
                marker.world[1] + worldOffset[1],
                marker.world[2] + worldOffset[2],
            ], [0.35, 0.35, 0.35])),
        ],
    };
}
export function createGeneratedTunnelRoomFrame(input) {
    const base = createGeneratedTunnelViewportFrame(input.tunnel, input.materials);
    const enemy = input.enemy ?? {
        label: 'generated-tunnel-enemy',
        position: [0, 1.1, -1.35],
        scale: [0.7, 1.8, 0.7],
    };
    return {
        ops: [
            ...base.ops,
            ...generatedTunnelRoomDepthCueOps(),
            {
                op: 'create',
                handle: renderHandle(4103901),
                parent: null,
                node: primitiveNode(enemy.label ?? 'generated-tunnel-enemy', 'cube', enemy.position, enemy.scale ?? [0.7, 1.8, 0.7], [0.92, 0.22, 0.18, 1]),
            },
            {
                op: 'create',
                handle: renderHandle(4103902),
                parent: null,
                node: primitiveNode('generated-tunnel-centerline', 'cube', [0, 0.02, -0.4], [0.28, 0.04, 4.8], [0.94, 0.62, 0.2, 1]),
            },
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
            schemaVersion: 2,
            id,
            color,
            texture: null,
            roughness: 1,
            textureTint: [1, 1, 1, 1],
            emissionColor: [0, 0, 0],
            emissionIntensity: 0,
            uvStrategy: 'flat',
        },
    };
}
function cuboidAsset(asset, materialId) {
    return {
        asset,
        payload: cuboidPayload(),
        materialSlots: [{ slot: 0, material: materialId }],
        collision: { kind: 'aabbFallback' },
    };
}
function cuboidPayload() {
    return {
        layout: {
            vertexCount: 24,
            indexCount: 36,
            indexWidth: 'u32',
            attributes: [
                { name: 'position', components: 3, kind: 'f32' },
                { name: 'normal', components: 3, kind: 'f32' },
            ],
        },
        groups: [{ materialSlot: 0, start: 0, count: 36 }],
        bounds: { min: [-0.5, -0.5, -0.5], max: [0.5, 0.5, 0.5] },
        source: {
            kind: 'inline',
            positions: [
                -0.5, -0.5, 0.5, 0.5, -0.5, 0.5, 0.5, 0.5, 0.5, -0.5, 0.5, 0.5,
                0.5, -0.5, -0.5, -0.5, -0.5, -0.5, -0.5, 0.5, -0.5, 0.5, 0.5, -0.5,
                -0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, -0.5, -0.5, 0.5, -0.5,
                -0.5, -0.5, -0.5, 0.5, -0.5, -0.5, 0.5, -0.5, 0.5, -0.5, -0.5, 0.5,
                0.5, -0.5, 0.5, 0.5, -0.5, -0.5, 0.5, 0.5, -0.5, 0.5, 0.5, 0.5,
                -0.5, -0.5, -0.5, -0.5, -0.5, 0.5, -0.5, 0.5, 0.5, -0.5, 0.5, -0.5,
            ],
            normals: [
                0, 0, 1, 0, 0, 1, 0, 0, 1, 0, 0, 1,
                0, 0, -1, 0, 0, -1, 0, 0, -1, 0, 0, -1,
                0, 1, 0, 0, 1, 0, 0, 1, 0, 0, 1, 0,
                0, -1, 0, 0, -1, 0, 0, -1, 0, 0, -1, 0,
                1, 0, 0, 1, 0, 0, 1, 0, 0, 1, 0, 0,
                -1, 0, 0, -1, 0, 0, -1, 0, 0, -1, 0, 0,
            ],
            indices: [
                0, 1, 2, 0, 2, 3,
                4, 5, 6, 4, 6, 7,
                8, 9, 10, 8, 10, 11,
                12, 13, 14, 12, 14, 15,
                16, 17, 18, 16, 18, 19,
                20, 21, 22, 20, 22, 23,
            ],
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
function generatedTunnelRoomDepthCueOps() {
    const wallRibColor = [0.28, 0.32, 0.36, 1];
    const coverColor = [0.34, 0.38, 0.34, 1];
    const ceilingColor = [0.38, 0.42, 0.47, 1];
    const ribZ = [-3.55, -2.25, -0.95, 0.35];
    const ops = [];
    ribZ.forEach((z, index) => {
        ops.push({
            op: 'create',
            handle: renderHandle(4103910 + index * 2),
            parent: null,
            node: primitiveNode(`generated-tunnel-wall-rib-west-${index + 1}`, 'cube', [-2.42, 1.45, z], [0.18, 2.9, 0.18], wallRibColor),
        }, {
            op: 'create',
            handle: renderHandle(4103911 + index * 2),
            parent: null,
            node: primitiveNode(`generated-tunnel-wall-rib-east-${index + 1}`, 'cube', [2.42, 1.45, z], [0.18, 2.9, 0.18], wallRibColor),
        });
    });
    return [
        ...ops,
        {
            op: 'create',
            handle: renderHandle(4103920),
            parent: null,
            node: primitiveNode('generated-tunnel-low-cover-west', 'cube', [-1.25, 0.24, -1.65], [0.72, 0.48, 0.7], coverColor),
        },
        {
            op: 'create',
            handle: renderHandle(4103921),
            parent: null,
            node: primitiveNode('generated-tunnel-low-cover-east', 'cube', [1.25, 0.24, -3.05], [0.72, 0.48, 0.7], coverColor),
        },
        {
            op: 'create',
            handle: renderHandle(4103922),
            parent: null,
            node: primitiveNode('generated-tunnel-ceiling-crossbeam', 'cube', [0, 3.08, -2.55], [4.75, 0.2, 0.24], ceilingColor),
        },
    ];
}
function primitiveNode(label, shape, translation, scale, color) {
    return {
        geometry: { shape },
        material: { color, wireframe: false },
        transform: {
            translation,
            rotation: IDENTITY_ROTATION,
            scale,
        },
        visible: true,
        layer: 'scene',
        metadata: { source: null, tags: [], label },
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
//# sourceMappingURL=generated-tunnel-frame.js.map