export const TINY_GENERATED_TUNNEL_READOUT = {
    status: 'available',
    generator: {
        generatorId: 'asha.tunnel.enclosed.v1',
        generatorVersion: 1,
        presetId: 'tiny-enclosed',
        seed: 17,
        configHash: 'e1d156c6b55137a7',
        outputHash: 'a9b504096397f5b4',
        generationHash: 'fnv1a64:0821a0c2aea17dff',
    },
    volume: {
        grid: 0,
        voxelSize: 1,
        chunkDims: [8, 6, 12],
        tunnelDims: [5, 4, 9],
        solidVoxels: 138,
        collisionAabbCount: 138,
    },
    rooms: { count: 0 },
    corridors: { count: 1, width: 5, height: 4, length: 9 },
    spawnMarkers: [
        {
            id: 'player_start',
            kind: 'player',
            voxel: [1, 1, 1],
            world: [1.5, 1.5, 1.5],
            yawDegrees: 0,
        },
        {
            id: 'exit_hint',
            kind: 'navigation',
            voxel: [3, 1, 7],
            world: [3.5, 1.5, 7.5],
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
        hash: 'fnv1a64:b2312fbcfb060db3',
    },
    replayHash: 'fnv1a64:0821a0c2aea17dff',
    fixture: 'harness/fixtures/generated-levels/tiny-tunnel.snapshot.txt',
};
//# sourceMappingURL=generated-tunnel.js.map