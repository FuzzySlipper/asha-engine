import type { DiagnosticSeverity } from './diagnostics.js';
export declare const VOXEL_ANNOTATION_SCHEMA_VERSION = 1;
export declare const VOXEL_ANNOTATION_MEDIA_TYPE = "application/vnd.asha.voxel-annotation+json;version=1";
export declare const VOXEL_ANNOTATION_EXTENSION = "avann.json";
export type VoxelAnnotationKind = 'selection' | 'room' | 'portal' | 'spawn_area' | 'cover' | 'hazard' | 'nav_hint' | 'custom';
export type VoxelAnnotationProvenanceKind = 'authored' | 'imported_reference' | 'runtime_export' | 'generated';
export type VoxelAnnotationDiagnosticCode = 'unsupported_schema_version' | 'unsupported_media_type' | 'invalid_layer_id' | 'invalid_target_voxel_volume_asset_id' | 'target_voxel_hash_mismatch' | 'invalid_bounds' | 'invalid_region_id' | 'duplicate_region_id' | 'unknown_parent_region' | 'parent_cycle' | 'unsupported_annotation_kind' | 'invalid_sparse_run' | 'duplicate_cell' | 'region_out_of_bounds' | 'quota_exceeded' | 'stale_layer_hash' | 'layer_not_loaded' | 'query_out_of_bounds' | 'edit_conflict';
export type VoxelAnnotationEditOperation = 'upsert_region' | 'remove_region' | 'add_runs' | 'remove_runs' | 'replace_selection' | 'set_parent' | 'set_tags' | 'set_label' | 'set_kind';
export type VoxelAnnotationQueryMode = 'cell' | 'bounds' | 'region' | 'layer_summary';
export interface VoxelAnnotationCoord {
    readonly x: number;
    readonly y: number;
    readonly z: number;
}
export interface VoxelAnnotationBounds {
    readonly min: VoxelAnnotationCoord;
    readonly max: VoxelAnnotationCoord;
}
export interface VoxelAnnotationSparseRun {
    readonly start: VoxelAnnotationCoord;
    readonly length: number;
}
export interface VoxelAnnotationSelection {
    readonly sparseRuns: readonly VoxelAnnotationSparseRun[];
}
export interface VoxelAnnotationProvenanceRef {
    readonly kind: VoxelAnnotationProvenanceKind;
    readonly uri: string;
    readonly contentHash: string;
}
export interface VoxelAnnotationContentHashes {
    readonly canonicalJson: string;
    readonly membershipData: string;
}
export interface VoxelAnnotationDiagnostic {
    readonly code: VoxelAnnotationDiagnosticCode;
    readonly severity: DiagnosticSeverity;
    readonly reference: string;
    readonly message: string;
}
export interface VoxelAnnotationRegion {
    readonly regionId: string;
    readonly label: string;
    readonly kind: VoxelAnnotationKind;
    readonly tags: readonly string[];
    readonly parentRegionId: string | null;
    readonly bounds: VoxelAnnotationBounds;
    readonly selection: VoxelAnnotationSelection;
}
export interface VoxelAnnotationLayer {
    readonly layerId: string;
    readonly schemaVersion: number;
    readonly mediaType: string;
    readonly targetVoxelVolumeAssetId: string;
    readonly targetVoxelDataHash: string;
    readonly targetBounds: VoxelAnnotationBounds;
    readonly regions: readonly VoxelAnnotationRegion[];
    readonly provenance: readonly VoxelAnnotationProvenanceRef[];
    readonly contentHashes: VoxelAnnotationContentHashes;
    readonly validationDiagnostics: readonly VoxelAnnotationDiagnostic[];
}
export interface VoxelAnnotationLayerValidationRequest {
    readonly layer: VoxelAnnotationLayer;
    readonly expectedTargetVoxelVolumeAssetId: string | null;
    readonly expectedTargetVoxelDataHash: string | null;
    readonly maxRegions: number;
    readonly maxSparseRunsPerRegion: number;
    readonly maxTotalAssignedCells: number;
}
export interface VoxelAnnotationLayerValidationReport {
    readonly layerId: string;
    readonly valid: boolean;
    readonly canonicalJsonHash: string | null;
    readonly membershipDataHash: string | null;
    readonly regionCount: number;
    readonly sparseRunCount: number;
    readonly assignedCellCount: number;
    readonly diagnostics: readonly VoxelAnnotationDiagnostic[];
}
export interface VoxelAnnotationLayerLoadRequest {
    readonly layer: VoxelAnnotationLayer;
    readonly targetGrid: number;
    readonly replaceExisting: boolean;
    readonly expectedSessionHash: string | null;
}
export interface VoxelAnnotationLayerLoadReceipt {
    readonly requestLayerId: string;
    readonly loaded: boolean;
    readonly runtimeLayerId: string | null;
    readonly targetVoxelVolumeAssetId: string;
    readonly targetVoxelDataHash: string;
    readonly regionCount: number;
    readonly assignedCellCount: number;
    readonly layerHash: string | null;
    readonly sessionHash: string;
    readonly replayHash: string;
    readonly diagnostics: readonly VoxelAnnotationDiagnostic[];
}
export interface VoxelAnnotationQueryRequest {
    readonly runtimeLayerId: string | null;
    readonly layerId: string;
    readonly mode: VoxelAnnotationQueryMode;
    readonly cell: VoxelAnnotationCoord | null;
    readonly bounds: VoxelAnnotationBounds | null;
    readonly regionId: string | null;
    readonly maxRegions: number;
    readonly expectedLayerHash: string | null;
}
export interface VoxelAnnotationRegionReadout {
    readonly regionId: string;
    readonly label: string;
    readonly kind: VoxelAnnotationKind;
    readonly tags: readonly string[];
    readonly parentRegionId: string | null;
    readonly bounds: VoxelAnnotationBounds;
    readonly assignedCellCount: number;
}
export interface VoxelAnnotationQueryReadout {
    readonly request: VoxelAnnotationQueryRequest;
    readonly matchedRegions: readonly VoxelAnnotationRegionReadout[];
    readonly regionCount: number;
    readonly truncated: boolean;
    readonly layerHash: string | null;
    readonly diagnostics: readonly VoxelAnnotationDiagnostic[];
}
export interface VoxelAnnotationEditRequest {
    readonly runtimeLayerId: string | null;
    readonly layerId: string;
    readonly expectedLayerHash: string;
    readonly operation: VoxelAnnotationEditOperation;
    readonly regionId: string | null;
    readonly region: VoxelAnnotationRegion | null;
    readonly sparseRuns: readonly VoxelAnnotationSparseRun[];
    readonly tags: readonly string[];
    readonly label: string | null;
    readonly kind: VoxelAnnotationKind | null;
    readonly parentRegionId: string | null;
}
export interface VoxelAnnotationEditReceipt {
    readonly request: VoxelAnnotationEditRequest;
    readonly edited: boolean;
    readonly layerHashBefore: string;
    readonly layerHashAfter: string | null;
    readonly regionCount: number;
    readonly assignedCellCount: number;
    readonly diagnostics: readonly VoxelAnnotationDiagnostic[];
    readonly replayHash: string;
}
export interface VoxelAnnotationLayerExportRequest {
    readonly runtimeLayerId: string | null;
    readonly layerId: string;
    readonly expectedLayerHash: string;
    readonly includeDiagnostics: boolean;
}
export interface VoxelAnnotationLayerExportReceipt {
    readonly request: VoxelAnnotationLayerExportRequest;
    readonly exported: boolean;
    readonly layer: VoxelAnnotationLayer | null;
    readonly canonicalJson: string | null;
    readonly canonicalJsonHash: string | null;
    readonly membershipDataHash: string | null;
    readonly diagnostics: readonly VoxelAnnotationDiagnostic[];
}
//# sourceMappingURL=voxelAnnotation.d.ts.map