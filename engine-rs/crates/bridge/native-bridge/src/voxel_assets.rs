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

fn validate_palette_update_request_size(request_json: &str) -> napi::Result<()> {
    if request_json.len() as u64 > VOXEL_PALETTE_UPDATE_MAX_REQUEST_BYTES {
        return Err(to_napi(RuntimeBridgeError::new(
            RuntimeBridgeErrorKind::InvalidInput,
            format!(
                "voxel volume asset palette update request has {} bytes; hard limit is {VOXEL_PALETTE_UPDATE_MAX_REQUEST_BYTES}",
                request_json.len()
            ),
        )));
    }
    Ok(())
}

fn validate_mesh_import_request_size(request_bytes: usize) -> napi::Result<()> {
    if request_bytes as u64 > VOXEL_CONVERSION_MESH_IMPORT_MAX_REQUEST_BYTES {
        return Err(to_napi(RuntimeBridgeError::new(
            RuntimeBridgeErrorKind::InvalidInput,
            format!(
                "voxel conversion mesh import request has {request_bytes} bytes; hard limit is {VOXEL_CONVERSION_MESH_IMPORT_MAX_REQUEST_BYTES}"
            ),
        )));
    }
    Ok(())
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
    validate_palette_update_request_size(&request_json)?;
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
pub fn initialize_voxel_volume_authoring(
    handle: i64,
    request_json: String,
) -> napi::Result<String> {
    let request = parse_request::<VoxelVolumeAuthoringInitializeRequest>(
        &request_json,
        "authoring initialize",
    )?;
    with_bridge(handle, |bridge| {
        let receipt = bridge
            .initialize_voxel_volume_authoring(request)
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

#[napi]
pub fn unload_voxel_volume_asset(handle: i64, request_json: String) -> napi::Result<String> {
    let request = parse_request::<VoxelVolumeAssetUnloadRequest>(&request_json, "unload")?;
    with_bridge(handle, |bridge| {
        let receipt = bridge.unload_voxel_volume_asset(request).map_err(to_napi)?;
        voxel_conversion_json(&receipt)
    })
}

#[napi]
pub fn import_voxel_conversion_mesh_source(
    handle: i64,
    request_json: String,
) -> napi::Result<String> {
    validate_mesh_import_request_size(request_json.len())?;
    let request =
        parse_request::<VoxelConversionMeshSourceImportRequest>(&request_json, "mesh import")?;
    with_bridge(handle, |bridge| {
        let receipt = bridge
            .import_voxel_conversion_mesh_source(request)
            .map_err(to_napi)?;
        voxel_conversion_json(&receipt)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palette_update_json_is_bounded_before_deserialization() {
        let at_limit = "x".repeat(VOXEL_PALETTE_UPDATE_MAX_REQUEST_BYTES as usize);
        assert!(validate_palette_update_request_size(&at_limit).is_ok());

        let over_limit = "x".repeat(VOXEL_PALETTE_UPDATE_MAX_REQUEST_BYTES as usize + 1);
        let error = validate_palette_update_request_size(&over_limit).unwrap_err();
        assert!(error.reason.contains("hard limit"));
    }

    #[test]
    fn mesh_import_json_is_bounded_before_deserialization() {
        assert!(validate_mesh_import_request_size(
            VOXEL_CONVERSION_MESH_IMPORT_MAX_REQUEST_BYTES as usize
        )
        .is_ok());

        let error = validate_mesh_import_request_size(
            VOXEL_CONVERSION_MESH_IMPORT_MAX_REQUEST_BYTES as usize + 1,
        )
        .unwrap_err();
        assert!(error.reason.contains("hard limit"));
    }
}
