use super::*;

fn parse_request<T>(request_json: &str, label: &str) -> napi::Result<T>
where
    T: serde::de::DeserializeOwned,
{
    serde_json::from_str(request_json).map_err(|err| {
        to_napi(RuntimeBridgeError::new(
            RuntimeBridgeErrorKind::InvalidInput,
            format!("invalid voxel volume asset {label} request JSON: {err}"),
        ))
    })
}

#[napi]
pub fn export_voxel_volume_asset(handle: i64, request_json: String) -> napi::Result<String> {
    let request = parse_request::<VoxelVolumeAssetExportRequest>(&request_json, "export")?;
    with_bridge(handle, |bridge| {
        let receipt = bridge.export_voxel_volume_asset(request).map_err(to_napi)?;
        voxel_conversion_json(&receipt)
    })
}

#[napi]
pub fn save_voxel_volume_asset(handle: i64, request_json: String) -> napi::Result<String> {
    let request = parse_request::<VoxelVolumeAssetSaveRequest>(&request_json, "save")?;
    with_bridge(handle, |bridge| {
        let receipt = bridge.save_voxel_volume_asset(request).map_err(to_napi)?;
        voxel_conversion_json(&receipt)
    })
}

#[napi]
pub fn update_voxel_volume_asset_palette(
    handle: i64,
    request_json: String,
) -> napi::Result<String> {
    let request =
        parse_request::<VoxelVolumeAssetPaletteUpdateRequest>(&request_json, "palette update")?;
    with_bridge(handle, |bridge| {
        let receipt = bridge
            .update_voxel_volume_asset_palette(request)
            .map_err(to_napi)?;
        voxel_conversion_json(&receipt)
    })
}

#[napi]
pub fn load_voxel_volume_asset(handle: i64, request_json: String) -> napi::Result<String> {
    let request = parse_request::<VoxelVolumeAssetLoadRequest>(&request_json, "load")?;
    with_bridge(handle, |bridge| {
        let receipt = bridge.load_voxel_volume_asset(request).map_err(to_napi)?;
        voxel_conversion_json(&receipt)
    })
}
