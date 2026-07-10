import {
  renderHandle,
  type CameraProjectionSnapshot,
  type CollisionAxis,
  type Geometry,
  type MeshPayloadDescriptor,
  type RenderDiff,
  type RenderFrameDiff,
  type RenderMaterialDescriptor,
  type RenderNode,
  type StaticMeshAsset,
  type StaticMeshInstanceDescriptor,
  type Transform,
} from '@asha/contracts';

export const FIRST_PERSON_TUNNEL_VIEWPORT_FIXTURE_NAME = 'generated-tunnel-first-person-viewport';

export type TunnelViewportVec3 = readonly [number, number, number];
export type TunnelViewportColor = readonly [number, number, number, number];
export type TunnelViewportMaterialRole = 'wall' | 'floor' | 'accent' | 'playerMarker' | 'exitMarker';

export interface GeneratedTunnelFrameReadout {
  readonly generator: {
    readonly presetId: string;
    readonly seed: number;
    readonly generationHash: string;
    readonly outputHash: string;
  };
  readonly volume: {
    readonly tunnelDims: readonly [number, number, number];
    readonly solidVoxels: number;
  };
  readonly spawnMarkers: readonly {
    readonly id: string;
    readonly kind: 'player' | 'exit' | string;
    readonly world: TunnelViewportVec3;
  }[];
  readonly materials: readonly {
    readonly role: string;
    readonly material: string | number;
  }[];
  readonly renderProjection: {
    readonly hash: string;
  };
  readonly collisionProjection: {
    readonly hash: string;
  };
  readonly runtimeFrame: {
    readonly worldOffset: TunnelViewportVec3;
    readonly playableMin: TunnelViewportVec3;
    readonly playableMax: TunnelViewportVec3;
  };
  readonly replayHash: string;
}

export interface TunnelViewportMaterialPalette {
  readonly wall: TunnelViewportColor;
  readonly floor: TunnelViewportColor;
  readonly accent: TunnelViewportColor;
  readonly playerMarker: TunnelViewportColor;
  readonly exitMarker: TunnelViewportColor;
}

export interface FirstPersonTunnelViewportCollisionDebug {
  readonly collided: boolean;
  readonly blockedAxes: readonly CollisionAxis[];
  readonly collisionSourceHash: string;
  readonly collisionProjectionHash: string;
  readonly movementHash: string;
}

export interface FirstPersonTunnelViewportInput {
  readonly tunnel: GeneratedTunnelFrameReadout;
  readonly camera: CameraProjectionSnapshot;
  readonly materials?: Partial<TunnelViewportMaterialPalette>;
  readonly collision?: FirstPersonTunnelViewportCollisionDebug | null;
}

export interface GeneratedTunnelRoomFrameTarget {
  readonly label?: string;
  readonly position: TunnelViewportVec3;
  readonly scale?: TunnelViewportVec3;
}

export interface GeneratedTunnelRoomFrameInput {
  readonly enemy?: GeneratedTunnelRoomFrameTarget | null;
  readonly materials?: Partial<TunnelViewportMaterialPalette>;
  readonly tunnel: GeneratedTunnelFrameReadout;
}

export interface FirstPersonTunnelViewportSummary {
  readonly kind: 'first_person_tunnel_viewport.v0';
  readonly fixture: typeof FIRST_PERSON_TUNNEL_VIEWPORT_FIXTURE_NAME;
  readonly presetId: GeneratedTunnelFrameReadout['generator']['presetId'];
  readonly seed: GeneratedTunnelFrameReadout['generator']['seed'];
  readonly camera: {
    readonly camera: CameraProjectionSnapshot['camera'];
    readonly tick: number;
    readonly position: TunnelViewportVec3;
    readonly yawDegrees: number;
    readonly pitchDegrees: number;
    readonly projectionHash: string;
    readonly viewport: {
      readonly width: number;
      readonly height: number;
    };
  };
  readonly tunnel: {
    readonly dims: GeneratedTunnelFrameReadout['volume']['tunnelDims'];
    readonly solidVoxels: number;
    readonly spawnMarkers: readonly string[];
    readonly materialRoles: readonly string[];
  };
  readonly debug: {
    readonly generatorHash: string;
    readonly outputHash: string;
    readonly renderProjectionHash: string;
    readonly collisionProjectionHash: string;
    readonly replayHash: string;
    readonly collision: FirstPersonTunnelViewportCollisionDebug | null;
  };
  readonly scene: {
    readonly frameHash: string;
    readonly structuralHash: string;
    readonly opCount: number;
    readonly instanceCount: number;
  };
  readonly nonClaims: readonly [
    'not_runtime_authority',
    'not_collision_authority',
    'not_local_generation',
    'not_pixel_golden',
  ];
}

const IDENTITY_ROTATION = [0, 0, 0, 1] as const;

const DEFAULT_TUNNEL_VIEWPORT_MATERIALS: TunnelViewportMaterialPalette = {
  wall: [0.42, 0.46, 0.5, 1],
  floor: [0.25, 0.32, 0.29, 1],
  accent: [0.5, 0.55, 0.62, 1],
  playerMarker: [0.18, 0.68, 0.92, 1],
  exitMarker: [0.72, 0.5, 0.94, 1],
};

export function createGeneratedTunnelViewportFrame(
  tunnel: GeneratedTunnelFrameReadout,
  materials: Partial<TunnelViewportMaterialPalette> = {},
): RenderFrameDiff {
  const palette: TunnelViewportMaterialPalette = {
    ...DEFAULT_TUNNEL_VIEWPORT_MATERIALS,
    ...materials,
  };
  const { playableMin, playableMax, worldOffset } = tunnel.runtimeFrame;
  const width = playableMax[0] - playableMin[0];
  const height = playableMax[1] - playableMin[1];
  const length = playableMax[2] - playableMin[2];
  const center: TunnelViewportVec3 = [
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
      ...tunnel.spawnMarkers.map((marker, index) =>
        instance(
          120 + index,
          marker.kind === 'player' ? 'mesh/generated-tunnel-player-marker' : 'mesh/generated-tunnel-exit-marker',
          `generated-tunnel-spawn-${marker.id}`,
          [
            marker.world[0] + worldOffset[0],
            marker.world[1] + worldOffset[1],
            marker.world[2] + worldOffset[2],
          ],
          [0.35, 0.35, 0.35],
        ),
      ),
    ],
  };
}

export function createGeneratedTunnelRoomFrame(input: GeneratedTunnelRoomFrameInput): RenderFrameDiff {
  const base = createGeneratedTunnelViewportFrame(input.tunnel, input.materials);
  const enemy = input.enemy ?? {
    label: 'generated-tunnel-enemy',
    position: [0, 1.1, -1.35] as const,
    scale: [0.7, 1.8, 0.7] as const,
  };
  return {
    ops: [
      ...base.ops,
      ...generatedTunnelRoomDepthCueOps(),
      {
        op: 'create',
        handle: renderHandle(4103901),
        parent: null,
        node: primitiveNode(
          enemy.label ?? 'generated-tunnel-enemy',
          'cube',
          enemy.position,
          enemy.scale ?? [0.7, 1.8, 0.7],
          [0.92, 0.22, 0.18, 1],
        ),
      },
      {
        op: 'create',
        handle: renderHandle(4103902),
        parent: null,
        node: primitiveNode(
          'generated-tunnel-centerline',
          'cube',
          [0, 0.02, -0.4],
          [0.28, 0.04, 4.8],
          [0.94, 0.62, 0.2, 1],
        ),
      },
    ],
  };
}

export function summarizeFirstPersonTunnelViewport(input: {
  readonly tunnel: GeneratedTunnelFrameReadout;
  readonly camera: CameraProjectionSnapshot;
  readonly frame: RenderFrameDiff;
  readonly structuralSnapshot?: string;
  readonly collision?: FirstPersonTunnelViewportCollisionDebug | null;
}): FirstPersonTunnelViewportSummary {
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

function material(
  id: string,
  color: TunnelViewportColor,
): { readonly op: 'defineMaterial'; readonly material: RenderMaterialDescriptor } {
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

function cuboidAsset(asset: string, materialId: string): StaticMeshAsset {
  return {
    asset,
    payload: cuboidPayload(),
    materialSlots: [{ slot: 0, material: materialId }],
    collision: { kind: 'aabbFallback' },
  };
}

function cuboidPayload(): MeshPayloadDescriptor {
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

function instance(
  handle: number,
  asset: string,
  label: string,
  translation: TunnelViewportVec3,
  scale: TunnelViewportVec3,
): {
  readonly op: 'createStaticMeshInstance';
  readonly handle: ReturnType<typeof renderHandle>;
  readonly parent: null;
  readonly instance: StaticMeshInstanceDescriptor;
} {
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

function transform(translation: TunnelViewportVec3, scale: TunnelViewportVec3): Transform {
  return {
    translation,
    rotation: IDENTITY_ROTATION,
    scale,
  };
}

function generatedTunnelRoomDepthCueOps(): RenderDiff[] {
  const wallRibColor = [0.28, 0.32, 0.36, 1] as const;
  const coverColor = [0.34, 0.38, 0.34, 1] as const;
  const ceilingColor = [0.38, 0.42, 0.47, 1] as const;
  const ribZ = [-3.55, -2.25, -0.95, 0.35] as const;
  const ops: RenderDiff[] = [];
  ribZ.forEach((z, index) => {
    ops.push(
      {
        op: 'create',
        handle: renderHandle(4103910 + index * 2),
        parent: null,
        node: primitiveNode(
          `generated-tunnel-wall-rib-west-${index + 1}`,
          'cube',
          [-2.42, 1.45, z],
          [0.18, 2.9, 0.18],
          wallRibColor,
        ),
      },
      {
        op: 'create',
        handle: renderHandle(4103911 + index * 2),
        parent: null,
        node: primitiveNode(
          `generated-tunnel-wall-rib-east-${index + 1}`,
          'cube',
          [2.42, 1.45, z],
          [0.18, 2.9, 0.18],
          wallRibColor,
        ),
      },
    );
  });
  return [
    ...ops,
    {
      op: 'create',
      handle: renderHandle(4103920),
      parent: null,
      node: primitiveNode(
        'generated-tunnel-low-cover-west',
        'cube',
        [-1.25, 0.24, -1.65],
        [0.72, 0.48, 0.7],
        coverColor,
      ),
    },
    {
      op: 'create',
      handle: renderHandle(4103921),
      parent: null,
      node: primitiveNode(
        'generated-tunnel-low-cover-east',
        'cube',
        [1.25, 0.24, -3.05],
        [0.72, 0.48, 0.7],
        coverColor,
      ),
    },
    {
      op: 'create',
      handle: renderHandle(4103922),
      parent: null,
      node: primitiveNode(
        'generated-tunnel-ceiling-crossbeam',
        'cube',
        [0, 3.08, -2.55],
        [4.75, 0.2, 0.24],
        ceilingColor,
      ),
    },
  ];
}

function primitiveNode(
  label: string,
  shape: Exclude<Geometry['shape'], 'line'>,
  translation: TunnelViewportVec3,
  scale: TunnelViewportVec3,
  color: TunnelViewportColor,
): RenderNode {
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

type ViewportHashPrimitive = string | number | boolean | null;
type ViewportHashValue = ViewportHashPrimitive | readonly ViewportHashValue[] | ViewportHashRecord;
interface ViewportHashRecord {
  readonly [key: string]: ViewportHashValue | undefined;
}

function frameHashRecord(frame: RenderFrameDiff): ViewportHashRecord {
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

function viewportStableHash(value: ViewportHashValue | undefined): string {
  return `fnv1a64:${viewportFnv1a64(viewportStableStringify(value))}`;
}

function viewportStableStringify(value: ViewportHashValue | undefined): string {
  if (value === undefined) {
    return 'undefined';
  }
  if (value === null || typeof value !== 'object') {
    return JSON.stringify(value);
  }
  if (Array.isArray(value)) {
    const entries = value as readonly ViewportHashValue[];
    return `[${entries.map((entry) => viewportStableStringify(entry)).join(',')}]`;
  }
  const record = value as ViewportHashRecord;
  return `{${Object.keys(record)
    .sort()
    .map((key) => `${JSON.stringify(key)}:${viewportStableStringify(record[key])}`)
    .join(',')}}`;
}

function viewportFnv1a64(text: string): string {
  let hash = 0xcbf29ce484222325n;
  const prime = 0x100000001b3n;
  const mask = 0xffffffffffffffffn;
  for (let index = 0; index < text.length; index += 1) {
    hash ^= BigInt(text.charCodeAt(index));
    hash = (hash * prime) & mask;
  }
  return hash.toString(16).padStart(16, '0');
}
