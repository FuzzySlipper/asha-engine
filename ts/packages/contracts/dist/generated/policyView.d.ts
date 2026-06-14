import type { EntityId, TagId } from './ids.js';
export interface PolicyTransform {
    readonly translation: readonly [number, number, number];
    readonly rotation: readonly [number, number, number, number];
    readonly scale: readonly [number, number, number];
}
export type PolicyEntityLifecycle = 'active' | 'disabled';
export type PolicyEntitySource = {
    readonly kind: 'sceneNode';
    readonly node: number;
} | {
    readonly kind: 'runtime';
} | {
    readonly kind: 'imported';
    readonly asset: string;
} | {
    readonly kind: 'policy';
};
export type PolicyAssetStatus = 'resolved' | 'missing' | 'stale';
export interface PolicyAssetView {
    readonly id: string;
    readonly kind: string;
    readonly status: PolicyAssetStatus;
}
export interface PolicyEntityView {
    readonly id: EntityId;
    readonly lifecycle: PolicyEntityLifecycle;
    readonly transform: PolicyTransform | null;
    readonly source: PolicyEntitySource;
    readonly labels: readonly TagId[];
    readonly spatial: boolean;
}
export interface PolicyWorldSummary {
    readonly tick: number;
    readonly activeEntities: number;
    readonly spatialEntities: number;
    readonly assetCount: number;
    readonly missingAssets: number;
}
export interface PolicyWorldView {
    readonly tick: number;
    readonly entities: readonly PolicyEntityView[];
    readonly assets: readonly PolicyAssetView[];
    readonly summary: PolicyWorldSummary;
}
export type PolicyWorldCommand = {
    readonly kind: 'requestSetTransform';
    readonly entity: EntityId;
    readonly transform: PolicyTransform;
} | {
    readonly kind: 'requestAddLabel';
    readonly entity: EntityId;
    readonly label: TagId;
} | {
    readonly kind: 'requestDisable';
    readonly entity: EntityId;
} | {
    readonly kind: 'noopMarker';
    readonly note: string;
};
export type PolicyWorldEvent = {
    readonly kind: 'transformSet';
    readonly entity: EntityId;
    readonly transform: PolicyTransform;
} | {
    readonly kind: 'labelAdded';
    readonly entity: EntityId;
    readonly label: TagId;
} | {
    readonly kind: 'disabled';
    readonly entity: EntityId;
} | {
    readonly kind: 'noopRecorded';
    readonly note: string;
};
export type PolicyWorldRejection = 'unknownEntity' | 'entityDisabled' | 'notSpatial' | 'immovable' | 'invalidTransform' | 'labelAlreadyPresent' | 'alreadyDisabled';
export type PolicyWorldOutcome = {
    readonly status: 'accepted';
    readonly event: PolicyWorldEvent;
} | {
    readonly status: 'rejected';
    readonly rejection: PolicyWorldRejection;
};
//# sourceMappingURL=policyView.d.ts.map