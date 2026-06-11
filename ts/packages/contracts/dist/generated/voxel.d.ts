export interface VoxelCoord {
    readonly x: number;
    readonly y: number;
    readonly z: number;
}
export interface ChunkCoord {
    readonly x: number;
    readonly y: number;
    readonly z: number;
}
export type VoxelValue = {
    readonly kind: 'empty';
} | {
    readonly kind: 'solid';
    readonly material: number;
};
export type VoxelCommand = {
    readonly op: 'setVoxel';
    readonly grid: number;
    readonly coord: VoxelCoord;
    readonly value: VoxelValue;
} | {
    readonly op: 'fillRegion';
    readonly grid: number;
    readonly min: VoxelCoord;
    readonly max: VoxelCoord;
    readonly value: VoxelValue;
} | {
    readonly op: 'generateChunk';
    readonly grid: number;
    readonly chunk: ChunkCoord;
    readonly seed: number;
    readonly generatorVersion: number;
};
export type VoxelEditEvent = {
    readonly event: 'voxelSet';
    readonly grid: number;
    readonly coord: VoxelCoord;
    readonly value: VoxelValue;
} | {
    readonly event: 'voxelRegionFilled';
    readonly grid: number;
    readonly min: VoxelCoord;
    readonly max: VoxelCoord;
    readonly value: VoxelValue;
} | {
    readonly event: 'chunkGenerated';
    readonly grid: number;
    readonly chunk: ChunkCoord;
    readonly seed: number;
    readonly generatorVersion: number;
    readonly hash: number;
};
export type VoxelEditRejection = {
    readonly reason: 'unknownMaterial';
    readonly material: number;
} | {
    readonly reason: 'emptyRegion';
    readonly min: VoxelCoord;
    readonly max: VoxelCoord;
} | {
    readonly reason: 'chunkNotResident';
    readonly chunk: ChunkCoord;
} | {
    readonly reason: 'generationDivergence';
    readonly chunk: ChunkCoord;
    readonly expected: number;
    readonly actual: number;
};
//# sourceMappingURL=voxel.d.ts.map