export type GeneratedTunnelPresetId = 'tiny-enclosed';

export interface GeneratedTunnelReadoutRequest {
  readonly presetId?: GeneratedTunnelPresetId;
  readonly seed?: number;
}

export interface GeneratedTunnelGeneratorSummary {
  readonly generatorId: 'asha.tunnel.enclosed.v2';
  readonly generatorVersion: 2;
  readonly presetId: GeneratedTunnelPresetId;
  readonly seed: 17;
  readonly configHash: 'e1d156c6b55137a7';
  readonly outputHash: '1471496d88d70647';
  readonly generationHash: 'fnv1a64:0821a0c2aea17dff';
}

export interface GeneratedTunnelVolumeSummary {
  readonly grid: 0;
  readonly voxelSize: 1;
  readonly chunkDims: readonly [8, 6, 12];
  readonly tunnelDims: readonly [5, 4, 9];
  readonly solidVoxels: 282;
  readonly collisionAabbCount: 282;
}

export interface GeneratedTunnelCorridorSummary {
  readonly count: 1;
  readonly width: 5;
  readonly height: 4;
  readonly length: 9;
}

export interface GeneratedTunnelRoomSummary {
  readonly count: 0;
}

export interface GeneratedTunnelSpawnMarkerSummary {
  readonly id: 'player_start' | 'exit_hint';
  readonly kind: 'player' | 'navigation';
  readonly voxel: readonly [number, number, number];
  readonly world: readonly [number, number, number];
  readonly yawDegrees: number;
}

export interface GeneratedTunnelMaterialSummary {
  readonly role: 'wall' | 'floor' | 'accent';
  readonly material: 1 | 2 | 3;
}

export interface GeneratedTunnelProjectionAvailability {
  readonly available: true;
  readonly hash: string;
}

export interface GeneratedTunnelRuntimeFrameSummary {
  readonly worldOffset: readonly [number, number, number];
  readonly playableMin: readonly [number, number, number];
  readonly playableMax: readonly [number, number, number];
}

export interface GeneratedTunnelReadout {
  readonly status: 'available';
  readonly generator: GeneratedTunnelGeneratorSummary;
  readonly volume: GeneratedTunnelVolumeSummary;
  readonly rooms: GeneratedTunnelRoomSummary;
  readonly corridors: GeneratedTunnelCorridorSummary;
  readonly spawnMarkers: readonly GeneratedTunnelSpawnMarkerSummary[];
  readonly materials: readonly GeneratedTunnelMaterialSummary[];
  readonly renderProjection: GeneratedTunnelProjectionAvailability;
  readonly collisionProjection: GeneratedTunnelProjectionAvailability;
  readonly runtimeFrame: GeneratedTunnelRuntimeFrameSummary;
  readonly replayHash: 'fnv1a64:0821a0c2aea17dff';
  readonly fixture: 'harness/fixtures/generated-levels/tiny-tunnel.snapshot.txt';
}

export type GeneratedTunnelOperation = 'regenerate' | 'apply_to_runtime_world';

export interface GeneratedTunnelOperationRequest {
  readonly operation: GeneratedTunnelOperation;
  readonly presetId?: GeneratedTunnelPresetId;
  readonly seed?: number;
}

export interface GeneratedTunnelUnsupportedOperationReceipt {
  readonly operation: GeneratedTunnelOperation;
  readonly status: 'unsupported';
  readonly reason: 'generated_tunnel_operation_not_wired';
  readonly detail: string;
}

export interface GeneratedTunnelAppliedOperationReceipt {
  readonly operation: 'apply_to_runtime_world';
  readonly status: 'applied';
  readonly presetId: GeneratedTunnelPresetId;
  readonly seed: number;
  readonly grid: number;
  readonly configHash: string;
  readonly outputHash: string;
  readonly collisionSourceHash: string;
  readonly collisionProjectionHash: string;
  readonly runtimeFrame: GeneratedTunnelRuntimeFrameSummary;
}

export type GeneratedTunnelOperationReceipt =
  | GeneratedTunnelUnsupportedOperationReceipt
  | GeneratedTunnelAppliedOperationReceipt;

export const TINY_GENERATED_TUNNEL_READOUT: GeneratedTunnelReadout = {
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
