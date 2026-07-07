import type { DiagnosticSeverity } from './diagnostics.js';
import type { VoxelCoord } from './voxel.js';
export type VoxelConversionMode = 'surface' | 'solid';
export type VoxelConversionFitPolicy = 'contain' | 'cover' | 'stretch';
export type VoxelConversionOriginPolicy = 'source_origin' | 'target_min' | 'centered';
export type VoxelConversionDiagnosticCode = 'voxel_conversion_unavailable' | 'operation_unimplemented' | 'unsupported_source_asset' | 'source_hash_mismatch' | 'invalid_material_map' | 'output_limit_exceeded' | 'non_manifold_or_ambiguous_solid' | 'stale_authority_snapshot' | 'conversion_replay_mismatch';
export type VoxelConversionEvidenceKind = 'plan' | 'preview' | 'apply_receipt' | 'diagnostics' | 'source_snapshot' | 'output_snapshot';
export interface VoxelConversionSourceRef {
    readonly assetId: string;
    readonly assetKind: string;
    readonly assetVersion: number;
    readonly sourceHash: string;
    readonly meshPrimitive: string | null;
}
export interface VoxelConversionTargetRef {
    readonly grid: number;
    readonly volumeAssetId: string | null;
    readonly origin: VoxelCoord;
}
export interface VoxelConversionBounds {
    readonly min: VoxelCoord;
    readonly max: VoxelCoord;
}
export interface VoxelConversionMaterialMapEntry {
    readonly sourceMaterialSlot: number;
    readonly sourceMaterialId: string | null;
    readonly voxelMaterial: number;
}
export interface VoxelConversionMaterialMap {
    readonly entries: readonly VoxelConversionMaterialMapEntry[];
    readonly defaultVoxelMaterial: number | null;
}
export interface VoxelConversionSettings {
    readonly mode: VoxelConversionMode;
    readonly fitPolicy: VoxelConversionFitPolicy;
    readonly originPolicy: VoxelConversionOriginPolicy;
    readonly resolution: readonly [number, number, number];
    readonly voxelSize: number;
    readonly maxOutputVoxels: number;
    readonly transform: readonly [number, number, number, number, number, number, number, number, number, number, number, number, number, number, number, number];
    readonly materialMap: VoxelConversionMaterialMap;
}
export interface VoxelConversionPlanRequest {
    readonly source: VoxelConversionSourceRef;
    readonly target: VoxelConversionTargetRef;
    readonly settings: VoxelConversionSettings;
}
export interface VoxelConversionDiagnostic {
    readonly code: VoxelConversionDiagnosticCode;
    readonly severity: DiagnosticSeverity;
    readonly reference: string;
    readonly message: string;
}
export interface VoxelConversionEvidenceRef {
    readonly kind: VoxelConversionEvidenceKind;
    readonly uri: string;
    readonly contentHash: string;
}
export interface VoxelConversionPlan {
    readonly planId: string;
    readonly source: VoxelConversionSourceRef;
    readonly target: VoxelConversionTargetRef;
    readonly settings: VoxelConversionSettings;
    readonly authorityVersion: string;
    readonly expectedSourceHash: string;
    readonly settingsHash: string;
    readonly planHash: string;
    readonly estimatedOutputVoxels: number;
    readonly estimatedBounds: VoxelConversionBounds | null;
    readonly diagnostics: readonly VoxelConversionDiagnostic[];
    readonly evidence: readonly VoxelConversionEvidenceRef[];
}
export interface VoxelConversionPreviewRequest {
    readonly planId: string;
    readonly expectedPlanHash: string;
}
export interface VoxelConversionPreviewVoxel {
    readonly coord: VoxelCoord;
    readonly material: number;
}
export interface VoxelConversionPreview {
    readonly planId: string;
    readonly outputHash: string;
    readonly outputVoxelCount: number;
    readonly outputBounds: VoxelConversionBounds | null;
    readonly sampleVoxels: readonly VoxelConversionPreviewVoxel[];
    readonly diagnostics: readonly VoxelConversionDiagnostic[];
    readonly evidence: readonly VoxelConversionEvidenceRef[];
}
export interface VoxelConversionApplyRequest {
    readonly planId: string;
    readonly expectedPlanHash: string;
    readonly expectedPreviewHash: string | null;
}
export interface VoxelConversionReceipt {
    readonly planId: string;
    readonly applied: boolean;
    readonly outputHash: string | null;
    readonly outputVoxelCount: number;
    readonly outputBounds: VoxelConversionBounds | null;
    readonly diagnostics: readonly VoxelConversionDiagnostic[];
    readonly evidence: readonly VoxelConversionEvidenceRef[];
}
//# sourceMappingURL=voxelConversion.d.ts.map