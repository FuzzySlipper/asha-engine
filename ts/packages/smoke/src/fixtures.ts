// Deterministic abstract fixtures for the smoke harness. No product nouns; these
// exercise the real facade load path and the real renderer upload path.

import type {
  CommandEnvelope,
  MeshPayloadDescriptor,
  PickRay,
  RenderFrameDiff,
  RenderNode,
  VoxelCommand,
} from '@asha/contracts';
import { entityId, renderHandle } from '@asha/contracts';
import type { CommandBatch, ProjectBundleLoadRequest } from '@asha/runtime-bridge';

/** The abstract fixture ProjectBundle the smoke harness loads through the facade. */
export const FIXTURE_PROJECT_BUNDLE: ProjectBundleLoadRequest = {
  bundleSchemaVersion: 1,
  protocolVersion: 1,
  sceneId: 1001,
};

/** A deterministic FNV-1a hash over the fixture ProjectBundle definition (stable evidence). */
export function fixtureProjectBundleHash(request: ProjectBundleLoadRequest): string {
  let hash = 0xcbf29ce484222325n;
  const prime = 0x100000001b3n;
  const mask = (1n << 64n) - 1n;
  const writeU32 = (value: number): void => {
    for (let i = 0; i < 4; i++) {
      hash ^= BigInt((value >>> (i * 8)) & 0xff);
      hash = (hash * prime) & mask;
    }
  };
  writeU32(request.bundleSchemaVersion);
  writeU32(request.protocolVersion);
  writeU32(request.sceneId);
  return hash.toString(16).padStart(16, '0');
}

/**
 * Deterministic, contract-shaped command envelopes (generated `@asha/contracts`
 * types). The smoke edit stage submits these instead of an ad-hoc `{ kind:
 * 'smoke-edit' }` literal, so the edit it proposes is a real authority command.
 */
export function fixtureCommandEnvelopes(): readonly CommandEnvelope[] {
  return [
    {
      kind: 'system',
      command: { domain: 'entity', command: { kind: 'create', id: entityId(1) } },
    },
  ];
}

/**
 * A deterministic generated `VoxelCommand` the smoke edit stage submits: set the
 * origin voxel solid with launch material 1. It lands in the reference bridge's
 * resident origin chunk (0,0,0) with an in-catalog material, so Rust authority
 * accepts it on the native path.
 */
export function fixtureVoxelCommand(): VoxelCommand {
  return { op: 'setVoxel', grid: 1, coord: { x: 0, y: 0, z: 0 }, value: { kind: 'solid', material: 1 } };
}

/**
 * The fixture edit batch for the facade. `submitCommands` now carries the generated
 * `VoxelCommand` union (manifest `protocol_voxel::CommandBatch`) straight to Rust
 * authority — no `{ kind: 'smoke-edit' }` placeholder command tunnel.
 */
export function fixtureCommandBatch(): CommandBatch {
  return { commands: [fixtureVoxelCommand()] };
}

/**
 * A deterministic pick ray for the picking stage: cast from outside the launch grid
 * toward the origin voxel along +X. Against the reference facade (no authority
 * geometry) it classifies as a miss; against a wired native facade it can hit the
 * origin voxel. Either way the picking PATH is exercised and a classified result is
 * returned — never a swallowed error.
 */
export function fixturePickRay(): PickRay {
  return { grid: 1, origin: [-5, 0.5, 0.5], direction: [1, 0, 0], maxDistance: 100 };
}

/**
 * A deterministic render-update frame for the post-edit render stage: nudge the
 * fixture node's transform. This proves the renderer applies a retained-mode UPDATE
 * (leak-safe, no destroy+create) reflecting a committed edit, without adding a node.
 */
export function fixtureEditUpdateFrame(): RenderFrameDiff {
  return {
    ops: [
      {
        op: 'update',
        handle: renderHandle(1),
        transform: { translation: [0, 1, 0], rotation: [0, 0, 0, 1], scale: [1, 1, 1] },
        material: null,
        visible: null,
        metadata: null,
      },
    ],
  };
}

/** A minimal mesh node to host the fixture geometry. */
function meshNode(): RenderNode {
  return {
    geometry: { shape: 'cube' },
    material: { color: [1, 1, 1, 1], wireframe: false },
    transform: { translation: [0, 0, 0], rotation: [0, 0, 0, 1], scale: [1, 1, 1] },
    visible: true,
    layer: 'scene',
    metadata: { source: null, tags: [], label: 'smoke-fixture' },
  };
}

/** A small inline quad payload (2 triangles, two material-slot groups). */
function quadPayload(): MeshPayloadDescriptor {
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
    groups: [
      { materialSlot: 1, start: 0, count: 3 },
      { materialSlot: 2, start: 3, count: 3 },
    ],
    bounds: { min: [0, 0, 0], max: [1, 1, 0] },
    source: {
      kind: 'inline',
      positions: [0, 0, 0, 1, 0, 0, 1, 1, 0, 0, 1, 0],
      normals: [0, 0, 1, 0, 0, 1, 0, 0, 1, 0, 0, 1],
      indices: [0, 1, 2, 0, 2, 3],
    },
    provenance: 'generated',
  };
}

/**
 * A deterministic fixture render frame: create one mesh node, then upload the quad
 * payload. Drives the renderer through its real create→replaceMeshPayload path.
 */
export function fixtureRenderFrame(): RenderFrameDiff {
  const handle = renderHandle(1);
  return {
    ops: [
      { op: 'create', handle, parent: null, node: meshNode() },
      { op: 'replaceMeshPayload', handle, payload: quadPayload() },
    ],
  };
}
