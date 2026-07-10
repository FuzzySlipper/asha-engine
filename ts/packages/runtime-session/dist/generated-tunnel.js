export const TINY_GENERATED_TUNNEL_READOUT = {
    status: 'available',
    generator: {
        generatorId: 'asha.tunnel.enclosed.v2',
        generatorVersion: 2,
        presetId: 'tiny-enclosed',
        seed: 17,
        configHash: 'e1d156c6b55137a7',
        outputHash: '1471496d88d70647',
        generationHash: 'fnv1a64:0821a0c2aea17dff',
    },
    volume: {
        grid: 0,
        voxelSize: 1,
        chunkDims: [8, 6, 12],
        tunnelDims: [5, 4, 9],
        solidVoxels: 282,
        collisionAabbCount: 282,
    },
    rooms: { count: 0 },
    corridors: { count: 1, width: 5, height: 4, length: 9 },
    spawnMarkers: [
        {
            id: 'player_start',
            kind: 'player',
            voxel: [2, 2, 2],
            world: [2.5, 2.5, 2.5],
            yawDegrees: 0,
        },
        {
            id: 'exit_hint',
            kind: 'navigation',
            voxel: [4, 2, 8],
            world: [4.5, 2.5, 8.5],
            yawDegrees: 180,
        },
    ],
    materials: [
        { role: 'wall', material: 1 },
        { role: 'floor', material: 2 },
        { role: 'accent', material: 3 },
    ],
    renderProjection: {
        available: true,
        hash: 'fnv1a64:21eb8696f6f3b5c4',
    },
    collisionProjection: {
        available: true,
        hash: 'fnv1a64:627389be013a3154',
    },
    runtimeFrame: {
        worldOffset: [-3.5, -1, -5.5],
        playableMin: [-2.5, 0, -4.5],
        playableMax: [2.5, 4, 4.5],
    },
    replayHash: 'fnv1a64:0821a0c2aea17dff',
    fixture: 'harness/fixtures/generated-levels/tiny-tunnel.snapshot.txt',
};
//# sourceMappingURL=generated-tunnel.js.map