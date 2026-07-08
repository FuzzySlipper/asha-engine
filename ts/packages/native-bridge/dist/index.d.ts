import { REQUIRED_NATIVE_ADDON_EXPORTS } from './native-addon.js';
import type { NativeAddon } from './native-addon.js';
export type { GameRuleCatalog, GameRuleResolutionReceipt, GameRuleResolutionRequest, VoxelConversionApplyRequest, VoxelConversionEvidenceRef, VoxelConversionMeshAsset, VoxelConversionMeshAssetGroup, VoxelConversionMeshAssetRegistrationRequest, VoxelConversionPlan, VoxelConversionPlanRequest, VoxelConversionPreview, VoxelConversionPreviewRequest, VoxelConversionReceipt, VoxelConversionSourceRegistration, VoxelConversionSourceRegistrationRequest, VoxelModelInfoReadout, VoxelModelInfoRequest, VoxelVolumeAssetExportReceipt, VoxelVolumeAssetExportRequest, VoxelVolumeAssetLoadReceipt, VoxelVolumeAssetLoadRequest, VoxelVolumeAssetSaveReceipt, VoxelVolumeAssetSaveRequest, VoxelVolumeAssetStoredDiff, } from '@asha/contracts';
export { REQUIRED_NATIVE_ADDON_EXPORTS };
export type { NativeAddon };
/** Raised when the native addon cannot be loaded (missing build / ABI mismatch). */
export declare class NativeAddonUnavailable extends Error {
    constructor(message: string);
}
/**
 * Attempt to load the compiled addon. Returns a typed handle or throws a
 * classified {@link NativeAddonUnavailable} — never a raw module-resolution error,
 * so `@asha/runtime-bridge` can re-map it to a `native_unavailable` bridge error.
 *
 * Build the addon with `napi build --platform --release` in the native-bridge crate.
 */
export declare function loadNativeAddon(modulePath?: string): NativeAddon;
//# sourceMappingURL=index.d.ts.map