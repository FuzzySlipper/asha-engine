export function voxelPaletteUpdateRequest(asset) {
    return {
        asset,
        materialPalette: asset.materialPalette,
        targetProjectBundle: 'asha-demo',
        targetAssetPath: 'assets/voxels/native-export.avxl.json',
        expectedCanonicalJsonHash: asset.contentHashes.canonicalJson,
        expectedVoxelDataHash: asset.contentHashes.voxelData,
        maxMaterialBindings: 16,
    };
}
export function createVoxelPaletteUpdateHandler(calls) {
    return (_handle, requestJson) => {
        calls.push(`voxelVolumeAssetPaletteUpdate:${requestJson}`);
        const request = JSON.parse(requestJson);
        const asset = { ...request.asset, materialPalette: request.materialPalette };
        return JSON.stringify({
            request,
            updated: true,
            diff: {
                projectBundle: request.targetProjectBundle,
                assetId: asset.assetId,
                assetPath: request.targetAssetPath,
                operation: 'replace_palette',
                previousCanonicalJsonHash: request.asset.contentHashes.canonicalJson,
                nextCanonicalJsonHash: 'fnv1a64:0000000000000115',
                voxelDataHash: asset.contentHashes.voxelData,
                previousMaterialCount: request.asset.materialPalette.length,
                nextMaterialCount: request.materialPalette.length,
            },
            asset,
            canonicalJson: `${JSON.stringify(asset)}\n`,
            canonicalJsonHash: 'fnv1a64:0000000000000115',
            voxelDataHash: asset.contentHashes.voxelData,
            diagnostics: [],
        });
    };
}
//# sourceMappingURL=native-voxel-palette.test-fixture.js.map