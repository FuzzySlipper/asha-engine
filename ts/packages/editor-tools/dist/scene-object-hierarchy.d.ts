import type { AssetReference, BootstrapRecord, FlatSceneDocument, SceneNodeId, SceneNodeKind, SceneTransform } from '@asha/contracts';
import { type SceneEditProposal } from './scene-authoring.js';
export type SceneObjectId = `scene-node:${number}`;
export type SceneObjectKind = SceneNodeKind['kind'];
export type SceneObjectDiagnosticCode = 'missing_object' | 'missing_parent' | 'stale_scene_object_snapshot' | 'scene_object_cycle' | 'self_parent' | 'invalid_name';
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
export type SceneObjectProposalResult = {
    readonly ok: true;
    readonly proposal: SceneEditProposal;
} | {
    readonly ok: false;
    readonly diagnostics: readonly SceneObjectDiagnostic[];
};
export interface SceneObjectLink {
    readonly sceneNodeId: SceneNodeId;
    readonly renderableId: string;
}
export declare function sceneObjectIdForNode(sceneNodeId: SceneNodeId): SceneObjectId;
export declare function buildSceneObjectSnapshot(options: {
    readonly document: FlatSceneDocument;
    readonly bootstrap?: BootstrapRecord | null;
    readonly renderableLinks?: readonly SceneObjectLink[];
}): SceneObjectSnapshot;
export declare function proposeRenameSceneObject(options: {
    readonly snapshot: SceneObjectSnapshot;
    readonly objectId: SceneObjectId;
    readonly displayName: string;
}): SceneObjectProposalResult;
export declare function proposeReparentSceneObject(options: {
    readonly snapshot: SceneObjectSnapshot;
    readonly objectId: SceneObjectId;
    readonly parentObjectId: SceneObjectId | null;
    readonly childOrder?: number;
}): SceneObjectProposalResult;
//# sourceMappingURL=scene-object-hierarchy.d.ts.map