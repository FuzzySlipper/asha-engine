// @asha/editor-tools — canonical scene-object hierarchy read model.
//
// This module projects generated FlatSceneDocument contracts into stable editor
// object ids and builds proposal-only rename/reparent commands. It never submits
// or validates authority; Rust/runtime validation remains the decider.

import type {
  AssetReference,
  BootstrapRecord,
  FlatSceneDocument,
  SceneNodeId,
  SceneNodeKind,
  SceneNodeRecord,
  SceneTransform,
} from '@asha/contracts';
import { proposeReparent, proposeSetMetadata, type SceneEditProposal } from './scene-authoring.js';

export type SceneObjectId = `scene-node:${number}`;
export type SceneObjectKind = SceneNodeKind['kind'];
export type SceneObjectDiagnosticCode =
  | 'missing_object'
  | 'missing_parent'
  | 'stale_scene_object_snapshot'
  | 'scene_object_cycle'
  | 'self_parent'
  | 'invalid_name';

export interface SceneObjectDiagnostic {
  readonly code: SceneObjectDiagnosticCode;
  readonly message: string;
  readonly objectId: SceneObjectId | null;
}

export interface SceneObjectEditability {
  readonly selectable: boolean;
  readonly rename: boolean;
  readonly reparent: boolean;
  readonly transform: boolean;
}

export interface SceneObjectRecord {
  readonly objectId: SceneObjectId;
  readonly sceneNodeId: SceneNodeId;
  readonly runtimeEntityId: BootstrapRecord['sourceTrace'][number]['runtimeEntityId'] | null;
  readonly parentObjectId: SceneObjectId | null;
  readonly childOrder: number;
  readonly displayName: string;
  readonly kind: SceneObjectKind;
  readonly tags: readonly string[];
  readonly transform: SceneTransform;
  readonly asset: AssetReference | null;
  readonly editability: SceneObjectEditability;
  readonly provenance: {
    readonly source: 'flat_scene_document';
    readonly sceneId: FlatSceneDocument['id'];
    readonly renderableId: string | null;
  };
}

export interface SceneObjectSnapshot {
  readonly schemaVersion: 1;
  readonly snapshotVersion: 'scene-object-snapshot.v0';
  readonly sceneId: FlatSceneDocument['id'];
  readonly sceneHash: string;
  readonly objects: readonly SceneObjectRecord[];
  readonly diagnostics: readonly SceneObjectDiagnostic[];
  readonly nonClaims: readonly string[];
}

export type SceneObjectProposalResult =
  | { readonly ok: true; readonly proposal: SceneEditProposal }
  | { readonly ok: false; readonly diagnostics: readonly SceneObjectDiagnostic[] };

export interface SceneObjectLink {
  readonly sceneNodeId: SceneNodeId;
  readonly renderableId: string;
}

export function sceneObjectIdForNode(sceneNodeId: SceneNodeId): SceneObjectId {
  return `scene-node:${sceneNodeId as number}`;
}

function stableJson(value: unknown): string {
  if (value === null || typeof value !== 'object') {
    return JSON.stringify(value);
  }
  if (Array.isArray(value)) {
    return `[${value.map(item => stableJson(item)).join(',')}]`;
  }
  return `{${Object.entries(value)
    .sort(([left], [right]) => left.localeCompare(right))
    .map(([key, item]) => `${JSON.stringify(key)}:${stableJson(item)}`)
    .join(',')}}`;
}

function fnv1aHash(prefix: string, value: unknown): string {
  const text = stableJson(value);
  let hash = 0x811c9dc5;
  for (let index = 0; index < text.length; index += 1) {
    hash ^= text.charCodeAt(index);
    hash = Math.imul(hash, 0x01000193) >>> 0;
  }
  return `${prefix}-${hash.toString(16).padStart(8, '0')}`;
}

function assetForKind(kind: SceneNodeKind): AssetReference | null {
  switch (kind.kind) {
    case 'emptyGroup':
      return null;
    case 'staticMesh':
    case 'sprite':
    case 'voxelVolume':
      return kind.asset;
  }
}

function defaultDisplayName(node: SceneNodeRecord): string {
  return node.label ?? `${node.kind.kind} ${node.id as number}`;
}

function sceneObjectRecord(
  doc: FlatSceneDocument,
  node: SceneNodeRecord,
  traceByNode: ReadonlyMap<number, BootstrapRecord['sourceTrace'][number]['runtimeEntityId']>,
  renderableByNode: ReadonlyMap<number, string>,
): SceneObjectRecord {
  const nodeId = node.id as number;
  return {
    objectId: sceneObjectIdForNode(node.id),
    sceneNodeId: node.id,
    runtimeEntityId: traceByNode.get(nodeId) ?? null,
    parentObjectId: node.parent === null ? null : sceneObjectIdForNode(node.parent),
    childOrder: node.childOrder,
    displayName: defaultDisplayName(node),
    kind: node.kind.kind,
    tags: [...node.tags],
    transform: node.transform,
    asset: assetForKind(node.kind),
    editability: {
      selectable: true,
      rename: true,
      reparent: true,
      transform: true,
    },
    provenance: {
      source: 'flat_scene_document',
      sceneId: doc.id,
      renderableId: renderableByNode.get(nodeId) ?? null,
    },
  };
}

function hierarchyDiagnostics(objects: readonly SceneObjectRecord[]): readonly SceneObjectDiagnostic[] {
  const diagnostics: SceneObjectDiagnostic[] = [];
  const objectIds = new Set(objects.map(object => object.objectId));
  const parentByObject = new Map(objects.map(object => [object.objectId, object.parentObjectId]));

  for (const object of objects) {
    if (object.parentObjectId !== null && !objectIds.has(object.parentObjectId)) {
      diagnostics.push({
        code: 'missing_parent',
        message: `${object.objectId} references missing parent ${object.parentObjectId}.`,
        objectId: object.objectId,
      });
    }

    const seen = new Set<SceneObjectId>();
    let current: SceneObjectId | null = object.objectId;
    while (current !== null) {
      if (seen.has(current)) {
        diagnostics.push({
          code: 'scene_object_cycle',
          message: `Scene object hierarchy contains a parent cycle at ${current}.`,
          objectId: object.objectId,
        });
        break;
      }
      seen.add(current);
      current = parentByObject.get(current) ?? null;
    }
  }

  return diagnostics;
}

export function buildSceneObjectSnapshot(options: {
  readonly document: FlatSceneDocument;
  readonly bootstrap?: BootstrapRecord | null;
  readonly renderableLinks?: readonly SceneObjectLink[];
}): SceneObjectSnapshot {
  const traceByNode = new Map(
    (options.bootstrap?.sourceTrace ?? []).map(trace => [
      trace.sceneNodeId as number,
      trace.runtimeEntityId,
    ]),
  );
  const renderableByNode = new Map(
    (options.renderableLinks ?? []).map(link => [link.sceneNodeId as number, link.renderableId]),
  );
  const objects = options.document.nodes
    .map(node => sceneObjectRecord(options.document, node, traceByNode, renderableByNode))
    .sort((left, right) =>
      (left.parentObjectId ?? '').localeCompare(right.parentObjectId ?? '')
      || left.childOrder - right.childOrder
      || (left.sceneNodeId as number) - (right.sceneNodeId as number),
    );

  return {
    schemaVersion: 1,
    snapshotVersion: 'scene-object-snapshot.v0',
    sceneId: options.document.id,
    sceneHash: fnv1aHash('scene-object', {
      id: options.document.id,
      nodes: options.document.nodes,
      trace: options.bootstrap?.sourceTrace ?? [],
      renderableLinks: options.renderableLinks ?? [],
    }),
    objects,
    diagnostics: hierarchyDiagnostics(objects),
    nonClaims: [
      'proposal_only',
      'not_authority_validation',
      'not_runtime_bridge_execution',
      'not_private_ui_state',
    ],
  };
}

function objectForId(
  snapshot: SceneObjectSnapshot,
  objectId: SceneObjectId,
): SceneObjectRecord | null {
  return snapshot.objects.find(object => object.objectId === objectId) ?? null;
}

function staleSnapshotDiagnostic(objectId: SceneObjectId): SceneObjectDiagnostic {
  return {
    code: 'stale_scene_object_snapshot',
    message: 'Scene object snapshot already has diagnostics; refresh authority/readback before proposing edits.',
    objectId,
  };
}

export function proposeRenameSceneObject(options: {
  readonly snapshot: SceneObjectSnapshot;
  readonly objectId: SceneObjectId;
  readonly displayName: string;
}): SceneObjectProposalResult {
  if (options.snapshot.diagnostics.length > 0) {
    return { ok: false, diagnostics: [staleSnapshotDiagnostic(options.objectId)] };
  }
  const object = objectForId(options.snapshot, options.objectId);
  if (object === null) {
    return {
      ok: false,
      diagnostics: [{
        code: 'missing_object',
        message: `Cannot rename missing scene object ${options.objectId}.`,
        objectId: options.objectId,
      }],
    };
  }
  const displayName = options.displayName.trim();
  if (displayName.length === 0) {
    return {
      ok: false,
      diagnostics: [{
        code: 'invalid_name',
        message: 'Scene object display name must not be empty.',
        objectId: options.objectId,
      }],
    };
  }

  return {
    ok: true,
    proposal: proposeSetMetadata(object.sceneNodeId, displayName, object.tags),
  };
}

function wouldCreateCycle(
  snapshot: SceneObjectSnapshot,
  objectId: SceneObjectId,
  parentObjectId: SceneObjectId,
): boolean {
  const parentByObject = new Map(snapshot.objects.map(object => [object.objectId, object.parentObjectId]));
  let current: SceneObjectId | null = parentObjectId;
  while (current !== null) {
    if (current === objectId) {
      return true;
    }
    current = parentByObject.get(current) ?? null;
  }
  return false;
}

export function proposeReparentSceneObject(options: {
  readonly snapshot: SceneObjectSnapshot;
  readonly objectId: SceneObjectId;
  readonly parentObjectId: SceneObjectId | null;
  readonly childOrder?: number;
}): SceneObjectProposalResult {
  if (options.snapshot.diagnostics.length > 0) {
    return { ok: false, diagnostics: [staleSnapshotDiagnostic(options.objectId)] };
  }
  const object = objectForId(options.snapshot, options.objectId);
  if (object === null) {
    return {
      ok: false,
      diagnostics: [{
        code: 'missing_object',
        message: `Cannot reparent missing scene object ${options.objectId}.`,
        objectId: options.objectId,
      }],
    };
  }
  const parent =
    options.parentObjectId === null ? null : objectForId(options.snapshot, options.parentObjectId);
  if (options.parentObjectId === options.objectId) {
    return {
      ok: false,
      diagnostics: [{
        code: 'self_parent',
        message: 'A scene object cannot be parented under itself.',
        objectId: options.objectId,
      }],
    };
  }
  if (options.parentObjectId !== null && parent === null) {
    return {
      ok: false,
      diagnostics: [{
        code: 'missing_parent',
        message: `Cannot reparent ${options.objectId} under missing parent ${options.parentObjectId}.`,
        objectId: options.objectId,
      }],
    };
  }
  if (options.parentObjectId !== null && wouldCreateCycle(options.snapshot, options.objectId, options.parentObjectId)) {
    return {
      ok: false,
      diagnostics: [{
        code: 'scene_object_cycle',
        message: `Cannot reparent ${options.objectId} under its descendant ${options.parentObjectId}.`,
        objectId: options.objectId,
      }],
    };
  }

  return {
    ok: true,
    proposal: proposeReparent(
      object.sceneNodeId,
      parent === null ? null : parent.sceneNodeId,
      options.childOrder ?? object.childOrder,
    ),
  };
}
