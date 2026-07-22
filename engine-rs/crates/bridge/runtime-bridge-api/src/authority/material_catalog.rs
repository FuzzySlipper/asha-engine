use super::*;

pub(super) fn project_voxel_material_frame(
    documents: &[ProjectContentDocumentDto],
    palette: &[VoxelAssetMaterialBinding],
) -> BridgeResult<RenderFrameDiff> {
    let mut catalog_entries = Vec::new();
    for catalog in documents.iter().filter_map(|document| match document {
        ProjectContentDocumentDto::AssetCatalog { catalog, .. } => Some(catalog),
        _ => None,
    }) {
        let catalog =
            svc_project_content::compile_stored_asset_catalog(catalog).map_err(|error| {
                RuntimeBridgeError::new(
                    RuntimeBridgeErrorKind::InvalidInput,
                    format!("stored material catalog could not be projected: {error}"),
                )
            })?;
        catalog_entries.extend(catalog.entries);
    }
    let catalog = Catalog::from_entries(catalog_entries);
    let pairs = palette
        .iter()
        .map(|binding| {
            AssetId::parse(&binding.material_asset_id)
                .map(|asset| (VoxelMaterialId::new(binding.voxel_material), asset))
                .map_err(|error| {
                    RuntimeBridgeError::new(
                        RuntimeBridgeErrorKind::InvalidInput,
                        format!("invalid voxel material asset reference: {error}"),
                    )
                })
        })
        .collect::<BridgeResult<Vec<_>>>()?;
    let table = core_catalog::VoxelMaterialTable::from_pairs(pairs);
    let used = palette
        .iter()
        .map(|binding| VoxelMaterialId::new(binding.voxel_material))
        .collect::<Vec<_>>();
    let (ops, fallbacks) =
        render_bridge::presentation::project_voxel_materials(&table, &catalog, &used);
    if !fallbacks.is_empty() {
        return Err(RuntimeBridgeError::new(
            RuntimeBridgeErrorKind::InvalidInput,
            format!(
                "stored voxel palette has unresolved material slots: {:?}",
                fallbacks
                    .iter()
                    .map(|material| material.raw())
                    .collect::<Vec<_>>()
            ),
        ));
    }
    Ok(RenderFrameDiff { ops })
}

impl EngineBridge {
    pub(super) fn queue_current_project_voxel_materials(&mut self) -> BridgeResult<()> {
        let Some(documents) = self
            .workspace_authoring
            .as_ref()
            .and_then(|authority| authority.project_content_current.as_ref())
            .map(|content| content.result().documents.clone())
        else {
            return Ok(());
        };
        let Some(palette) = self
            .voxel
            .active_voxel_model
            .as_ref()
            .and_then(|key| self.voxel.voxel_model_infos.get(key))
            .map(|info| info.material_palette.clone())
        else {
            return Ok(());
        };
        let frame = project_voxel_material_frame(&documents, &palette)?;
        self.projection.pending_voxel_frame.ops.extend(frame.ops);
        Ok(())
    }
}
