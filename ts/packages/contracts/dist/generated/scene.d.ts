import type { EntityId } from './ids.js';
export type SceneId = number & {
    readonly __brand: 'SceneId';
};
export declare const sceneId: (raw: number) => SceneId;
export type WorldId = number & {
    readonly __brand: 'WorldId';
};
export declare const worldId: (raw: number) => WorldId;
export type SceneNodeId = number & {
    readonly __brand: 'SceneNodeId';
};
export declare const sceneNodeId: (raw: number) => SceneNodeId;
export type SceneNodeKindTag = 'emptyGroup' | 'staticMesh' | 'sprite' | 'voxelVolume';
export type SceneValidationCode = 'duplicate-node-id' | 'unknown-parent' | 'cycle' | 'invalid-transform' | 'asset-kind-mismatch';
export type SceneObjectCommandRejectionCode = 'stale-scene-object-snapshot' | 'invalid-scene-before-command' | 'invalid-scene-after-command' | 'missing-scene-object' | 'duplicate-scene-object' | 'missing-scene-object-parent' | 'scene-object-self-parent' | 'blank-scene-object-label';
export type AssetVersionReq = {
    readonly req: 'any';
} | {
    readonly req: 'exact';
    readonly value: number;
} | {
    readonly req: 'atLeast';
    readonly value: number;
};
export interface AssetReference {
    readonly id: string;
    readonly version: AssetVersionReq;
    readonly hash: string | null;
}
export interface SceneTransform {
    readonly translation: readonly [number, number, number];
    readonly rotation: readonly [number, number, number, number];
    readonly scale: readonly [number, number, number];
}
export type SceneNodeKind = {
    readonly kind: 'emptyGroup';
} | {
    readonly kind: 'staticMesh';
    readonly asset: AssetReference;
} | {
    readonly kind: 'sprite';
    readonly asset: AssetReference;
} | {
    readonly kind: 'voxelVolume';
    readonly asset: AssetReference;
};
export interface SceneNodeRecord {
    readonly id: SceneNodeId;
    readonly parent: SceneNodeId | null;
    readonly childOrder: number;
    readonly label: string | null;
    readonly tags: readonly string[];
    readonly transform: SceneTransform;
    readonly kind: SceneNodeKind;
}
export interface SceneMetadata {
    readonly name: string | null;
    readonly authoringFormatVersion: number;
}
export interface FlatSceneDocument {
    readonly schemaVersion: number;
    readonly id: SceneId;
    readonly metadata: SceneMetadata;
    readonly dependencies: readonly AssetReference[];
    readonly nodes: readonly SceneNodeRecord[];
}
export interface SceneValidationError {
    readonly code: SceneValidationCode;
    readonly node: SceneNodeId | null;
    readonly parent: SceneNodeId | null;
    readonly expectedKind: string | null;
    readonly actualKind: string | null;
    readonly transformReason: string | null;
    readonly cyclePath: readonly SceneNodeId[];
}
export interface SceneValidationReport {
    readonly errors: readonly SceneValidationError[];
}
export interface SceneObjectRecord {
    readonly id: SceneNodeId;
    readonly parent: SceneNodeId | null;
    readonly childOrder: number;
    readonly label: string | null;
    readonly kind: SceneNodeKindTag;
    readonly hasRenderableAsset: boolean;
}
export interface SceneObjectSnapshot {
    readonly documentHash: number;
    readonly objects: readonly SceneObjectRecord[];
}
export type SceneObjectCommand = {
    readonly kind: 'create';
    readonly record: SceneNodeRecord;
} | {
    readonly kind: 'delete';
    readonly id: SceneNodeId;
} | {
    readonly kind: 'rename';
    readonly id: SceneNodeId;
    readonly label: string | null;
} | {
    readonly kind: 'reparent';
    readonly id: SceneNodeId;
    readonly parent: SceneNodeId | null;
    readonly childOrder: number;
} | {
    readonly kind: 'select';
    readonly id: SceneNodeId | null;
};
export interface SceneObjectCommandRejection {
    readonly code: SceneObjectCommandRejectionCode;
    readonly id: SceneNodeId | null;
    readonly parent: SceneNodeId | null;
    readonly expectedHash: number | null;
    readonly actualHash: number | null;
    readonly validationErrors: readonly SceneValidationError[];
}
export interface SceneObjectCommandOutcome {
    readonly document: FlatSceneDocument;
    readonly snapshot: SceneObjectSnapshot;
    readonly selected: SceneNodeId | null;
}
export interface SceneObjectCommandRequest {
    readonly expectedDocumentHash: number;
    readonly command: SceneObjectCommand;
}
export interface SceneObjectCommandResult {
    readonly accepted: boolean;
    readonly outcome: SceneObjectCommandOutcome | null;
    readonly rejection: SceneObjectCommandRejection | null;
}
export interface SceneSourceTrace {
    readonly sceneNodeId: SceneNodeId;
    readonly runtimeEntityId: EntityId;
}
export interface BootstrapRecord {
    readonly sceneId: SceneId;
    readonly worldId: WorldId;
    readonly schemaVersion: number;
    readonly nodeCount: number;
    readonly entityCount: number;
    readonly worldHash: number;
    readonly sourceTrace: readonly SceneSourceTrace[];
}
//# sourceMappingURL=scene.d.ts.map