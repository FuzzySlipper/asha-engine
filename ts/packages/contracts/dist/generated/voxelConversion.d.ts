import type { DiagnosticSeverity } from './diagnostics.js';
import type { VoxelCoord } from './voxel.js';
export type VoxelConversionMode = 'surface' | 'solid';
export type VoxelConversionFitPolicy = 'contain' | 'cover' | 'stretch';
export type VoxelConversionOriginPolicy = 'source_origin' | 'target_min' | 'centered';
export type VoxelConversionDiagnosticCode = 'voxel_conversion_unavailable' | 'operation_unimplemented' | 'unsupported_source_asset' | 'source_hash_mismatch' | 'invalid_material_map' | 'missing_texture_source' | 'texture_hash_mismatch' | 'missing_uv_attribute' | 'unsupported_texture_format' | 'unsupported_sampling_policy' | 'invalid_texture_material_rule' | 'output_limit_exceeded' | 'non_manifold_or_ambiguous_solid' | 'stale_authority_snapshot' | 'conversion_replay_mismatch';
export type VoxelConversionEvidenceKind = 'plan' | 'preview' | 'apply_receipt' | 'diagnostics' | 'source_snapshot' | 'output_snapshot';
export interface VoxelConversionSourceRef {
    readonly assetId: string;
    readonly assetKind: string;
    readonly assetVersion: number;
    readonly sourceHash: string;
    readonly meshPrimitive: string | null;
}
export interface VoxelConversionSourceTriangle {
    readonly indices: readonly [number, number, number];
    readonly sourceMaterialSlot: number;
}
export interface VoxelConversionSourceMaterialSlot {
    readonly sourceMaterialSlot: number;
    readonly sourceMaterialId: string | null;
}
export interface VoxelConversionSourceRegistrationRequest {
    readonly source: VoxelConversionSourceRef;
    readonly positions: readonly (readonly [number, number, number])[];
    readonly triangles: readonly VoxelConversionSourceTriangle[];
    readonly materialSlots: readonly VoxelConversionSourceMaterialSlot[];
}
export interface VoxelConversionMeshAssetGroup {
    readonly materialSlot: number;
    readonly start: number;
    readonly count: number;
}
export interface VoxelConversionMeshAsset {
    readonly assetId: string;
    readonly sourcePath: string | null;
    readonly positions: readonly (readonly [number, number, number])[];
    readonly normals: readonly (readonly [number, number, number])[];
    readonly indices: readonly number[];
    readonly groups: readonly VoxelConversionMeshAssetGroup[];
    readonly materialSlots: readonly VoxelConversionSourceMaterialSlot[];
}
export interface VoxelConversionMeshAssetRegistrationRequest {
    readonly source: VoxelConversionSourceRef;
    readonly meshAsset: VoxelConversionMeshAsset;
}
export interface VoxelConversionSourceRegistration {
    readonly source: VoxelConversionSourceRef;
    readonly registered: boolean;
    readonly materialSlots: readonly VoxelConversionSourceMaterialSlot[];
    readonly diagnostics: readonly VoxelConversionDiagnostic[];
    readonly evidence: readonly VoxelConversionEvidenceRef[];
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
export interface VoxelConversionUvAttributeRef {
    readonly attributeName: string;
    readonly sourceHash: string;
}
export interface VoxelConversionTextureSourceRef {
    readonly textureAssetId: string;
    readonly assetVersion: number;
    readonly contentHash: string;
    readonly width: number;
    readonly height: number;
    readonly colorSpace: string;
    readonly channelLayout: string;
}
export interface VoxelConversionTextureSampleAsset {
    readonly texture: VoxelConversionTextureSourceRef;
    readonly texelMaterials: readonly number[];
}
export interface VoxelConversionTextureBinding {
    readonly sourceMaterialSlot: number;
    readonly texture: VoxelConversionTextureSourceRef;
    readonly uvAttribute: VoxelConversionUvAttributeRef;
    readonly sampleUv: readonly [number, number];
    readonly samplingPolicy: string;
    readonly wrapPolicy: string;
    readonly materialMode: string;
}
export interface VoxelConversionMaterialMap {
    readonly entries: readonly VoxelConversionMaterialMapEntry[];
    readonly textureAssets: readonly VoxelConversionTextureSampleAsset[];
    readonly textureBindings: readonly VoxelConversionTextureBinding[];
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
export interface VoxelModelInfoRequest {
    readonly grid: number;
    readonly volumeAssetId: string | null;
    readonly includeMaterialCounts: boolean;
}
export interface VoxelModelMaterialCount {
    readonly material: number;
    readonly voxelCount: number;
}
export interface VoxelModelInfoReadout {
    readonly request: VoxelModelInfoRequest;
    readonly resident: boolean;
    readonly modelId: string;
    readonly volumeAssetId: string | null;
    readonly grid: number;
    readonly bounds: VoxelConversionBounds | null;
    readonly voxelCount: number;
    readonly materialCounts: readonly VoxelModelMaterialCount[];
    readonly source: VoxelConversionSourceRef | null;
    readonly latestPlanId: string | null;
    readonly latestOutputHash: string | null;
    readonly sessionHash: string;
    readonly replayHash: string;
    readonly evidence: readonly VoxelConversionEvidenceRef[];
    readonly diagnostics: readonly VoxelConversionDiagnostic[];
}
//# sourceMappingURL=voxelConversion.d.ts.map