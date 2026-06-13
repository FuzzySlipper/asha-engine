import type { AssetReference } from './scene.js';
export type AssetKind = 'material' | 'mesh' | 'sprite' | 'sprite-sheet' | 'texture' | 'voxel-volume' | 'voxel-object' | 'script' | 'scene';
export type StructuralClass = 'decorative' | 'solid' | 'structural';
export type UvStrategy = 'flat' | 'planar' | 'atlas';
export type CatalogValidationCode = 'duplicate-asset-id' | 'material-payload-missing' | 'material-payload-on-non-material' | 'wrong-kind-reference' | 'unknown-dependency' | 'dependency-cycle' | 'empty-source-path';
export type LockIssueCode = 'missing' | 'wrong-kind' | 'stale-version' | 'stale-hash' | 'dependency-drift' | 'new-in-catalog';
export type FallbackContext = 'debugOverlay' | 'cosmeticSurface' | 'collisionCritical' | 'backgroundDecoration';
export type FallbackVisual = 'magentaSquare' | 'greyMaterial';
export interface Rgba {
    readonly r: number;
    readonly g: number;
    readonly b: number;
    readonly a: number;
}
export interface RenderMaterial {
    readonly color: Rgba;
    readonly texture: AssetReference | null;
    readonly roughness: number;
    readonly emissive: number;
    readonly uvStrategy: UvStrategy;
}
export interface CollisionMaterial {
    readonly solid: boolean;
    readonly collidable: boolean;
    readonly occludes: boolean;
    readonly structuralClass: StructuralClass;
}
export interface MaterialProjection {
    readonly render: RenderMaterial;
    readonly collision: CollisionMaterial;
}
export interface CatalogEntry {
    readonly id: string;
    readonly kind: AssetKind;
    readonly version: number;
    readonly hash: string | null;
    readonly sourcePath: string | null;
    readonly label: string | null;
    readonly dependencies: readonly AssetReference[];
    readonly material: MaterialProjection | null;
}
export interface Catalog {
    readonly entries: readonly CatalogEntry[];
}
export interface CatalogValidationError {
    readonly code: CatalogValidationCode;
    readonly id: string | null;
    readonly kind: AssetKind | null;
    readonly from: string | null;
    readonly slot: string | null;
    readonly expected: AssetKind | null;
    readonly actual: AssetKind | null;
    readonly reference: string | null;
    readonly dependency: string | null;
    readonly cyclePath: readonly string[];
}
export interface CatalogValidationReport {
    readonly errors: readonly CatalogValidationError[];
}
export interface AssetLockEntry {
    readonly id: string;
    readonly kind: AssetKind;
    readonly version: number;
    readonly hash: string | null;
    readonly dependencies: readonly string[];
}
export interface AssetLock {
    readonly entries: readonly AssetLockEntry[];
}
export interface LockFinding {
    readonly id: string;
    readonly code: LockIssueCode;
    readonly lockedKind: AssetKind | null;
    readonly currentKind: AssetKind | null;
    readonly lockedVersion: number | null;
    readonly currentVersion: number | null;
    readonly lockedHash: string | null;
    readonly currentHash: string | null;
    readonly addedDependencies: readonly string[];
    readonly removedDependencies: readonly string[];
}
export interface LockValidationReport {
    readonly findings: readonly LockFinding[];
}
export type FallbackDecision = {
    readonly outcome: 'useFallback';
    readonly reason: string;
    readonly visual: FallbackVisual;
} | {
    readonly outcome: 'failClosed';
    readonly reason: string;
} | {
    readonly outcome: 'skip';
    readonly reason: string;
};
//# sourceMappingURL=assets.d.ts.map