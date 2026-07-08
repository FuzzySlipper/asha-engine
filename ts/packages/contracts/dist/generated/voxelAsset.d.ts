import type { DiagnosticSeverity } from './diagnostics.js';
export declare const VOXEL_ASSET_SCHEMA_VERSION = 1;
export declare const VOXEL_ASSET_MEDIA_TYPE = "application/vnd.asha.voxel-volume+json;version=1";
export declare const VOXEL_ASSET_EXTENSION = "avxl.json";
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
    readonly materialAssetId: string;
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
//# sourceMappingURL=voxelAsset.d.ts.map