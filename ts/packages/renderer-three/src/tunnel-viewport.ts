import {
  renderHandle,
  type CameraProjectionSnapshot,
  type CollisionAxis,
  type MeshPayloadDescriptor,
  type RenderFrameDiff,
  type RenderMaterialDescriptor,
  type StaticMeshAsset,
  type StaticMeshInstanceDescriptor,
  type Transform,
} from '@asha/contracts';
import type { GeneratedTunnelReadout } from '@asha/runtime-bridge';

export const FIRST_PERSON_TUNNEL_VIEWPORT_FIXTURE_NAME = 'generated-tunnel-first-person-viewport';

export type TunnelViewportVec3 = readonly [number, number, number];
export type TunnelViewportColor = readonly [number, number, number, number];
export type TunnelViewportMaterialRole = 'wall' | 'floor' | 'accent' | 'playerMarker' | 'exitMarker';

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
  readonly worldHash: string;
  readonly collisionProjectionHash: string;
  readonly movementHash: string;
}

export interface FirstPersonTunnelViewportInput {
  readonly tunnel: GeneratedTunnelReadout;
  readonly camera: CameraProjectionSnapshot;
  readonly materials?: Partial<TunnelViewportMaterialPalette>;
  readonly collision?: FirstPersonTunnelViewportCollisionDebug | null;
}

export interface FirstPersonTunnelViewportSummary {
  readonly kind: 'first_person_tunnel_viewport.v0';
  readonly fixture: typeof FIRST_PERSON_TUNNEL_VIEWPORT_FIXTURE_NAME;
  readonly presetId: GeneratedTunnelReadout['generator']['presetId'];
  readonly seed: GeneratedTunnelReadout['generator']['seed'];
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
    readonly dims: GeneratedTunnelReadout['volume']['tunnelDims'];
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
  tunnel: GeneratedTunnelReadout,
  materials: Partial<TunnelViewportMaterialPalette> = {},
): RenderFrameDiff {
  const palette: TunnelViewportMaterialPalette = {
    ...DEFAULT_TUNNEL_VIEWPORT_MATERIALS,
    ...materials,
  };
  const [width, height, length] = tunnel.volume.tunnelDims;
  const center: TunnelViewportVec3 = [width / 2, height / 2, length / 2];

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
      instance(100, 'mesh/generated-tunnel-floor', 'generated-tunnel-floor', [center[0], -0.05, center[2]], [
        width,
        0.1,
        length,
      ]),
      instance(101, 'mesh/generated-tunnel-wall', 'generated-tunnel-ceiling', [center[0], height + 0.05, center[2]], [
        width,
        0.1,
        length,
      ]),
      instance(102, 'mesh/generated-tunnel-wall', 'generated-tunnel-wall-west', [-0.05, center[1], center[2]], [
        0.1,
        height,
        length,
      ]),
      instance(103, 'mesh/generated-tunnel-wall', 'generated-tunnel-wall-east', [width + 0.05, center[1], center[2]], [
        0.1,
        height,
        length,
      ]),
      instance(104, 'mesh/generated-tunnel-accent', 'generated-tunnel-entrance-cap', [center[0], center[1], -0.05], [
        width,
        height,
        0.1,
      ]),
      instance(105, 'mesh/generated-tunnel-accent', 'generated-tunnel-exit-cap', [center[0], center[1], length + 0.05], [
        width,
        height,
        0.1,
      ]),
      ...tunnel.spawnMarkers.map((marker, index) =>
        instance(
          120 + index,
          marker.kind === 'player' ? 'mesh/generated-tunnel-player-marker' : 'mesh/generated-tunnel-exit-marker',
          `generated-tunnel-spawn-${marker.id}`,
          marker.world,
          [0.35, 0.35, 0.35],
        ),
      ),
    ],
  };
}

export function summarizeFirstPersonTunnelViewport(input: {
  readonly tunnel: GeneratedTunnelReadout;
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
