import type {
  FlatSceneDocument,
  MaterialProjection,
  ModelMaterialPreviewRequest,
  ModelMaterialPreviewSnapshot,
  RenderFrameDiff,
  SceneNodeId,
  SceneNodeRecord,
  SceneObjectCommandRejection,
  SceneObjectCommandRequest,
  SceneObjectCommandResult,
  SceneObjectSnapshot,
} from '@asha/contracts';

import { RuntimeBridgeError } from './bridge.js';
import { fnv1a64 } from './mock-primitives.js';

function materialDescriptor(
  id: string,
  material: MaterialProjection,
): Extract<RenderFrameDiff['ops'][number], { readonly op: 'defineMaterial' }>['material'] {
  return {
    schemaVersion: 2,
    id,
    color: [material.render.color.r, material.render.color.g, material.render.color.b, material.render.color.a],
    texture: material.render.texture?.id ?? null,
    roughness: material.render.roughness,
    textureTint: [material.render.textureTint.r, material.render.textureTint.g, material.render.textureTint.b, material.render.textureTint.a],
    emissionColor: [material.render.emissionColor.r, material.render.emissionColor.g, material.render.emissionColor.b],
    emissionIntensity: material.render.emissive,
    uvStrategy: material.render.uvStrategy,
  };
}

export function mockModelMaterialPreview(
  request: ModelMaterialPreviewRequest,
): ModelMaterialPreviewSnapshot {
  const entry = request.catalogEntry;
  if (entry.kind !== 'material' || entry.material === null) {
    throw new RuntimeBridgeError('invalid_input', `catalog entry '${entry.id}' is not a material`);
  }
  if (!request.meshAsset.materialSlots.some((slot) => slot.material === entry.id)) {
    throw new RuntimeBridgeError(
      'invalid_input',
      `mesh asset '${request.meshAsset.asset}' does not reference material '${entry.id}'`,
    );
  }
  return {
    catalogEntry: entry,
    material: entry.material,
    meshAsset: request.meshAsset,
    previewDiff: {
      ops: [
        { op: 'defineMaterial', material: materialDescriptor(entry.id, entry.material) },
        { op: 'defineStaticMesh', asset: request.meshAsset },
        {
          op: 'createStaticMeshInstance',
          handle: request.instanceHandle,
          parent: null,
          instance: {
            asset: request.meshAsset.asset,
            transform: { translation: [0, 0, 0], rotation: [0, 0, 0, 1], scale: [1, 1, 1] },
            materialOverrides: [],
            metadata: { source: null, tags: [], label: `Preview ${request.meshAsset.asset}` },
          },
        },
      ],
    },
    rendererClassification: 'reference_preview',
    diagnostics: ['native runtime readback for model/material preview may fail closed until wired'],
  };
}

function cloneFlatSceneDocument(document: FlatSceneDocument): FlatSceneDocument {
  return JSON.parse(JSON.stringify(document)) as FlatSceneDocument;
}

export function initialMockSceneDocument(): FlatSceneDocument {
  return {
    schemaVersion: 1,
    id: 1 as FlatSceneDocument['id'],
    metadata: { name: 'Mock scene', authoringFormatVersion: 1 },
    dependencies: [],
    nodes: [
      {
        id: 1 as SceneNodeId,
        parent: null,
        childOrder: 0,
        label: 'Root',
        tags: [],
        transform: { translation: [0, 0, 0], rotation: [0, 0, 0, 1], scale: [1, 1, 1] },
        kind: { kind: 'emptyGroup' },
      },
      {
        id: 2 as SceneNodeId,
        parent: 1 as SceneNodeId,
        childOrder: 0,
        label: 'Preview cube',
        tags: [],
        transform: { translation: [0, 0, 0], rotation: [0, 0, 0, 1], scale: [1, 1, 1] },
        kind: {
          kind: 'staticMesh',
          asset: { id: 'static-mesh:preview/cube', version: { req: 'any' }, hash: null },
        },
      },
    ],
  };
}

function sceneDocumentHash(document: FlatSceneDocument): number {
  const hex = fnv1a64(JSON.stringify({
    ...document,
    nodes: [...document.nodes].sort((a, b) => a.id - b.id),
  }));
  return Number.parseInt(hex.slice(0, 13), 16);
}

export function sceneObjectSnapshotFromDocument(document: FlatSceneDocument): SceneObjectSnapshot {
  return {
    documentHash: sceneDocumentHash(document),
    objects: [...document.nodes]
      .sort((a, b) => a.id - b.id)
      .map((node) => ({
        id: node.id,
        parent: node.parent,
        childOrder: node.childOrder,
        label: node.label,
        kind: node.kind.kind,
        hasRenderableAsset: node.kind.kind !== 'emptyGroup',
      })),
  };
}

function nodeIndex(document: FlatSceneDocument, id: SceneNodeId): number {
  return document.nodes.findIndex((node) => node.id === id);
}

function commandRejection(
  code: SceneObjectCommandRejection['code'],
  id: SceneNodeId | null,
  parent: SceneNodeId | null = null,
  expectedHash: number | null = null,
  actualHash: number | null = null,
): SceneObjectCommandResult {
  return {
    accepted: false,
    outcome: null,
    rejection: { code, id, parent, expectedHash, actualHash, validationErrors: [] },
  };
}

function descendantIds(document: FlatSceneDocument, root: SceneNodeId): Set<SceneNodeId> {
  const doomed = new Set<SceneNodeId>([root]);
  let changed = true;
  while (changed) {
    changed = false;
    for (const node of document.nodes) {
      if (node.parent !== null && doomed.has(node.parent) && !doomed.has(node.id)) {
        doomed.add(node.id);
        changed = true;
      }
    }
  }
  return doomed;
}

function createsCycle(document: FlatSceneDocument, id: SceneNodeId, parent: SceneNodeId | null): boolean {
  let current = parent;
  while (current !== null) {
    if (current === id) return true;
    current = document.nodes.find((node) => node.id === current)?.parent ?? null;
  }
  return false;
}

export function applyMockSceneObjectCommand(
  document: FlatSceneDocument,
  request: SceneObjectCommandRequest,
): SceneObjectCommandResult {
  const actualHash = sceneDocumentHash(document);
  if (request.expectedDocumentHash !== actualHash) {
    return commandRejection('stale-scene-object-snapshot', null, null, request.expectedDocumentHash, actualHash);
  }
  let next = cloneFlatSceneDocument(document);
  let selected: SceneNodeId | null = null;

  switch (request.command.kind) {
    case 'create': {
      if (nodeIndex(next, request.command.record.id) !== -1) {
        return commandRejection('duplicate-scene-object', request.command.record.id);
      }
      if (request.command.record.parent !== null && nodeIndex(next, request.command.record.parent) === -1) {
        return commandRejection('missing-scene-object-parent', request.command.record.id, request.command.record.parent);
      }
      next = { ...next, nodes: [...next.nodes, request.command.record] };
      break;
    }
    case 'delete': {
      if (nodeIndex(next, request.command.id) === -1) {
        return commandRejection('missing-scene-object', request.command.id);
      }
      const doomed = descendantIds(next, request.command.id);
      next = { ...next, nodes: next.nodes.filter((node) => !doomed.has(node.id)) };
      break;
    }
    case 'rename': {
      if (request.command.label !== null && request.command.label.trim() === '') {
        return commandRejection('blank-scene-object-label', request.command.id);
      }
      const { id, label } = request.command;
      const index = nodeIndex(next, id);
      if (index === -1) return commandRejection('missing-scene-object', id);
      const node = next.nodes[index] as SceneNodeRecord;
      next = { ...next, nodes: next.nodes.map((existing) => existing.id === id ? { ...node, label } : existing) };
      selected = id;
      break;
    }
    case 'reparent': {
      const { id, parent, childOrder } = request.command;
      const index = nodeIndex(next, id);
      if (index === -1) return commandRejection('missing-scene-object', id);
      if (parent === id) return commandRejection('scene-object-self-parent', id);
      if (parent !== null && nodeIndex(next, parent) === -1) {
        return commandRejection('missing-scene-object-parent', id, parent);
      }
      if (createsCycle(next, id, parent)) return commandRejection('invalid-scene-after-command', id, parent);
      const node = next.nodes[index] as SceneNodeRecord;
      next = {
        ...next,
        nodes: next.nodes.map((existing) => existing.id === id ? { ...node, parent, childOrder } : existing),
      };
      selected = id;
      break;
    }
    case 'select':
      if (request.command.id !== null && nodeIndex(next, request.command.id) === -1) {
        return commandRejection('missing-scene-object', request.command.id);
      }
      selected = request.command.id;
      break;
  }

  const snapshot = sceneObjectSnapshotFromDocument(next);
  return {
    accepted: true,
    outcome: { document: next, snapshot, selected },
    rejection: null,
  };
}
