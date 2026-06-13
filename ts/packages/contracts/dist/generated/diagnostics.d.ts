export type DiagnosticSeverity = 'info' | 'warning' | 'error' | 'fatal';
export type DiagnosticScope = 'scene' | 'assetCatalog' | 'worldBundle' | 'renderProjection' | 'rendererResources' | 'worldComposition';
export type DiagnosticCode = 'duplicateSceneId' | 'invalidSceneParent' | 'sceneParentCycle' | 'invalidSceneTransform' | 'sceneAssetMissing' | 'sceneAssetWrongKind' | 'duplicateAssetId' | 'catalogStructuralError' | 'missingAsset' | 'staleAsset' | 'wrongKindAssetRef' | 'assetCycle' | 'manifestProtocolMismatch' | 'corruptBundleArtifact' | 'missingCacheWarning' | 'generatorMismatch' | 'fallbackUsed' | 'missingSourceTrace' | 'rendererResourceSummary' | 'suspectedResourceLeak' | 'loadStageFailed' | 'finalConsistencyMismatch' | 'roundTripMismatch';
export type RemedyAction = 'inspect' | 'provideAsset' | 'fixReference' | 'breakCycle' | 'regenerate' | 'restoreArtifact' | 'refreshCache' | 'acceptFallback';
export interface SuggestedRemedy {
    readonly action: RemedyAction;
    readonly detail: string;
}
export interface DiagnosticSourceRef {
    readonly sceneNodeId: number | null;
    readonly runtimeEntityId: number | null;
    readonly assetId: string | null;
    readonly chunkCoord: readonly [number, number, number] | null;
    readonly renderHandle: number | null;
    readonly bundlePath: string | null;
}
export interface DiagnosticReport {
    readonly scope: DiagnosticScope;
    readonly severity: DiagnosticSeverity;
    readonly code: DiagnosticCode;
    readonly reference: string;
    readonly source: DiagnosticSourceRef;
    readonly message: string;
    readonly remedy: SuggestedRemedy | null;
}
export interface DiagnosticReportSet {
    readonly reports: readonly DiagnosticReport[];
}
export interface SourceTrace {
    readonly renderHandle: number;
    readonly sceneNodeId: number | null;
    readonly runtimeEntityId: number | null;
    readonly assetId: string | null;
    readonly assetResolved: boolean;
}
export interface RendererResourceReport {
    readonly liveHandles: number;
    readonly geometries: number;
    readonly materials: number;
    readonly spriteInstances: number;
    readonly spritesUpdatedLastTick: number;
    readonly resourcesCreated: number;
    readonly resourcesDisposed: number;
    readonly fallbackMaterials: number;
}
//# sourceMappingURL=diagnostics.d.ts.map