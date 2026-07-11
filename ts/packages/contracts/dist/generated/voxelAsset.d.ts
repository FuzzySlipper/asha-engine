import type { DiagnosticSeverity } from './diagnostics.js';
export declare const VOXEL_ASSET_SCHEMA_VERSION = 1;
export declare const VOXEL_ASSET_MEDIA_TYPE = "application/vnd.asha.voxel-volume+json;version=1";
export declare const VOXEL_ASSET_EXTENSION = "avxl.json";
export declare const VOXEL_PALETTE_UPDATE_MAX_REQUEST_BYTES = 8388608;
export declare const VOXEL_PALETTE_UPDATE_MAX_SPARSE_RUNS = 65536;
export declare const VOXEL_PALETTE_UPDATE_MAX_REPRESENTED_VOXELS = 1000000000;
export declare const VOXEL_PALETTE_UPDATE_MAX_MATERIAL_BINDINGS = 4096;
export declare const VOXEL_PALETTE_UPDATE_MAX_PROVENANCE_REFS = 4096;
export declare const VOXEL_PALETTE_UPDATE_MAX_EMBEDDED_DIAGNOSTICS = 1024;
export declare const VOXEL_PALETTE_UPDATE_MAX_STRING_BYTES = 4096;
export type VoxelAssetRepresentationKind = 'sparse_runs';
export type VoxelAssetProvenanceKind = 'authored' | 'converted' | 'runtime_export' | 'imported_reference';
export type VoxelAssetDiagnosticCode = 'unsupported_schema_version' | 'unsupported_media_type' | 'invalid_asset_id' | 'invalid_grid' | 'invalid_bounds' | 'unsupported_representation' | 'invalid_sparse_run' | 'duplicate_voxel' | 'duplicate_material_binding' | 'invalid_material_reference' | 'unknown_voxel_material' | 'content_hash_mismatch' | 'runtime_model_unavailable' | 'export_limit_exceeded' | 'stale_runtime_snapshot';
export interface VoxelAssetCoord {
    readonly x: number;
    readonly y: number;
    readonly z: number;
}
export interface VoxelAssetBounds {
    readonly min: VoxelAssetCoord;
    readonly max: VoxelAssetCoord;
}
export interface VoxelAssetGrid {
    readonly origin: readonly [number, number, number];
    readonly cellSize: number;
    readonly coordinateSystem: string;
}
export interface VoxelAssetMaterialBinding {
    readonly voxelMaterial: number;
    readonly paletteEntryId: string;
    readonly displayName: string | null;
    readonly materialAssetId: string;
    readonly materialCatalogBindingId: string | null;
}
export interface VoxelAssetSparseRun {
    readonly start: VoxelAssetCoord;
    readonly length: number;
    readonly material: number;
}
export interface VoxelAssetRepresentation {
    readonly kind: VoxelAssetRepresentationKind;
    readonly sparseRuns: readonly VoxelAssetSparseRun[];
}
export interface VoxelAssetProvenanceRef {
    readonly kind: VoxelAssetProvenanceKind;
    readonly uri: string;
    readonly contentHash: string;
}
export interface VoxelAssetAuthoringMetadata {
    readonly label: string | null;
    readonly createdBy: string | null;
    readonly sourceTool: string | null;
}
export interface VoxelAssetContentHashes {
    readonly canonicalJson: string;
    readonly voxelData: string;
}
export interface VoxelAssetDiagnostic {
    readonly code: VoxelAssetDiagnosticCode;
    readonly severity: DiagnosticSeverity;
    readonly reference: string;
    readonly message: string;
}
export interface VoxelAssetMaterialCount {
    readonly material: number;
    readonly voxelCount: number;
}
export interface VoxelVolumeAsset {
    readonly assetId: string;
    readonly schemaVersion: number;
    readonly mediaType: string;
    readonly grid: VoxelAssetGrid;
    readonly bounds: VoxelAssetBounds;
    readonly representation: VoxelAssetRepresentation;
    readonly materialPalette: readonly VoxelAssetMaterialBinding[];
    readonly provenance: readonly VoxelAssetProvenanceRef[];
    readonly authoring: VoxelAssetAuthoringMetadata;
    readonly validationDiagnostics: readonly VoxelAssetDiagnostic[];
    readonly contentHashes: VoxelAssetContentHashes;
}
export interface VoxelVolumeAssetExportRequest {
    readonly grid: number;
    readonly volumeAssetId: string | null;
    readonly targetAssetId: string;
    readonly label: string | null;
    readonly createdBy: string | null;
    readonly sourceTool: string | null;
    readonly maxSparseRuns: number;
    readonly expectedSessionHash: string | null;
}
export interface VoxelVolumeAssetExportReceipt {
    readonly request: VoxelVolumeAssetExportRequest;
    readonly exported: boolean;
    readonly asset: VoxelVolumeAsset | null;
    readonly canonicalJson: string | null;
    readonly canonicalJsonHash: string | null;
    readonly voxelDataHash: string | null;
    readonly diagnostics: readonly VoxelAssetDiagnostic[];
}
export interface VoxelVolumeAssetSaveRequest {
    readonly exportRequest: VoxelVolumeAssetExportRequest;
    readonly targetProjectBundle: string;
    readonly targetAssetPath: string;
    readonly representationKind: string;
    readonly expectedExistingCanonicalJsonHash: string | null;
    readonly expectedCanonicalJsonHash: string | null;
    readonly expectedVoxelDataHash: string | null;
}
export interface VoxelVolumeAssetStoredDiff {
    readonly projectBundle: string;
    readonly assetId: string;
    readonly assetPath: string;
    readonly operation: string;
    readonly previousCanonicalJsonHash: string | null;
    readonly nextCanonicalJsonHash: string;
    readonly nextVoxelDataHash: string;
    readonly representationKind: VoxelAssetRepresentationKind;
    readonly sparseRunCount: number;
    readonly voxelCount: number;
    readonly materialCount: number;
    readonly provenanceCount: number;
    readonly runtimeSessionHash: string;
}
export interface VoxelVolumeAssetSaveReceipt {
    readonly request: VoxelVolumeAssetSaveRequest;
    readonly saved: boolean;
    readonly diff: VoxelVolumeAssetStoredDiff | null;
    readonly asset: VoxelVolumeAsset | null;
    readonly canonicalJson: string | null;
    readonly canonicalJsonHash: string | null;
    readonly voxelDataHash: string | null;
    readonly diagnostics: readonly VoxelAssetDiagnostic[];
}
export interface VoxelVolumeAssetPaletteUpdateRequest {
    readonly asset: VoxelVolumeAsset;
    readonly materialPalette: readonly VoxelAssetMaterialBinding[];
    readonly targetProjectBundle: string;
    readonly targetAssetPath: string;
    readonly expectedCanonicalJsonHash: string;
    readonly expectedVoxelDataHash: string;
    readonly maxMaterialBindings: number;
}
export interface VoxelVolumeAssetPaletteStoredDiff {
    readonly projectBundle: string;
    readonly assetId: string;
    readonly assetPath: string;
    readonly operation: string;
    readonly previousCanonicalJsonHash: string;
    readonly nextCanonicalJsonHash: string;
    readonly voxelDataHash: string;
    readonly previousMaterialCount: number;
    readonly nextMaterialCount: number;
}
export interface VoxelVolumeAssetPaletteUpdateReceipt {
    readonly request: VoxelVolumeAssetPaletteUpdateRequest;
    readonly updated: boolean;
    readonly diff: VoxelVolumeAssetPaletteStoredDiff | null;
    readonly asset: VoxelVolumeAsset | null;
    readonly canonicalJson: string | null;
    readonly canonicalJsonHash: string | null;
    readonly voxelDataHash: string | null;
    readonly diagnostics: readonly VoxelAssetDiagnostic[];
}
export interface VoxelVolumeAuthoringInitializeRequest {
    readonly grid: number;
    readonly volumeAssetId: string | null;
    readonly seedChunk: VoxelAssetCoord;
    readonly materialPalette: readonly VoxelAssetMaterialBinding[];
    readonly authoring: VoxelAssetAuthoringMetadata;
    readonly maxMaterialBindings: number;
}
export interface VoxelVolumeAuthoringInitializeReceipt {
    readonly request: VoxelVolumeAuthoringInitializeRequest;
    readonly initialized: boolean;
    readonly modelId: string;
    readonly volumeAssetId: string | null;
    readonly grid: number;
    readonly sessionHash: string;
    readonly replayHash: string;
    readonly diagnostics: readonly VoxelAssetDiagnostic[];
}
export interface VoxelVolumeAssetLoadRequest {
    readonly asset: VoxelVolumeAsset;
    readonly targetGrid: number;
    readonly targetVolumeAssetId: string | null;
    readonly replaceExisting: boolean;
    readonly includeMaterialCounts: boolean;
}
export interface VoxelVolumeAssetLoadReceipt {
    readonly requestAssetId: string;
    readonly loaded: boolean;
    readonly modelId: string;
    readonly volumeAssetId: string | null;
    readonly grid: number;
    readonly bounds: VoxelAssetBounds | null;
    readonly voxelCount: number;
    readonly materialCounts: readonly VoxelAssetMaterialCount[];
    readonly provenance: readonly VoxelAssetProvenanceRef[];
    readonly canonicalJsonHash: string | null;
    readonly voxelDataHash: string | null;
    readonly sessionHash: string;
    readonly replayHash: string;
    readonly diagnostics: readonly VoxelAssetDiagnostic[];
}
export interface VoxelVolumeAssetUnloadRequest {
    readonly grid: number;
    readonly volumeAssetId: string | null;
    readonly expectedSessionHash: string;
}
export interface VoxelVolumeAssetUnloadReceipt {
    readonly request: VoxelVolumeAssetUnloadRequest;
    readonly unloaded: boolean;
    readonly modelId: string;
    readonly volumeAssetId: string | null;
    readonly grid: number;
    readonly removedVoxelCount: number;
    readonly sessionHash: string;
    readonly replayHash: string;
    readonly diagnostics: readonly VoxelAssetDiagnostic[];
}
//# sourceMappingURL=voxelAsset.d.ts.map