use super::*;

#[test]
fn reference_glb_import_registers_converts_and_returns_bounded_evidence() {
    let source_bytes = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../../harness/fixtures/voxel-conversion/kenney-wall-a.glb"
    ));
    let mut bridge = init_bridge();
    let imported = bridge
        .import_voxel_conversion_mesh_source(VoxelConversionMeshSourceImportRequest {
            source_asset_id: "mesh/kenney-wall-a".to_string(),
            asset_version: 1,
            source_path: "assets/reference/kenney-wall-a.glb".to_string(),
            format: VoxelConversionMeshSourceFormat::Glb,
            source_bytes: source_bytes.to_vec(),
            mesh_primitive: None,
        })
        .unwrap();
    assert!(imported.imported);
    assert_eq!(imported.source_byte_count, 3_352);
    assert_eq!(
        imported.source.source_hash,
        "sha256:6fceda24c30d2c22694f232f03fe2115fb1a462046fbbf719a90eea10dc9af00"
    );
    assert_eq!(imported.vertex_count, 48);
    assert_eq!(imported.triangle_count, 12);
    assert_eq!(imported.groups.len(), 2);
    assert_eq!(imported.material_slots.len(), 2);
    assert_eq!(
        imported.source_bounds,
        Some(VoxelConversionSourceBounds {
            min: [-0.5, 0.0, -0.5],
            max: [0.5, 1.0, 0.5],
        })
    );
    assert_eq!(imported.evidence.len(), 1);
    assert!(imported.mesh_asset.is_some());

    let mut plan_request = project_voxel_conversion_request(7);
    plan_request.source = imported.source.clone();
    plan_request.settings.resolution = [8, 8, 8];
    plan_request.settings.voxel_size = 0.25;
    plan_request.settings.max_output_voxels = 512;
    plan_request.settings.material_map.entries = imported
        .material_slots
        .iter()
        .enumerate()
        .map(
            |(index, slot)| protocol_voxel_conversion::VoxelConversionMaterialMapEntry {
                source_material_slot: slot.source_material_slot,
                source_material_id: slot.source_material_id.clone(),
                voxel_material: u16::try_from(index + 1).unwrap(),
            },
        )
        .collect();
    let plan = bridge.plan_voxel_conversion(plan_request).unwrap();
    assert!(plan.diagnostics.is_empty(), "{:?}", plan.diagnostics);
    let preview = bridge
        .preview_voxel_conversion(VoxelConversionPreviewRequest {
            plan_id: plan.plan_id.clone(),
            expected_plan_hash: plan.plan_hash.clone(),
        })
        .unwrap();
    assert!(preview.diagnostics.is_empty(), "{:?}", preview.diagnostics);
    assert!(preview.output_voxel_count > 0);
    let applied = bridge
        .apply_voxel_conversion(VoxelConversionApplyRequest {
            plan_id: plan.plan_id,
            expected_plan_hash: plan.plan_hash,
            expected_preview_hash: Some(preview.output_hash),
        })
        .unwrap();
    assert!(applied.applied, "{:?}", applied.diagnostics);
    assert!(applied.output_voxel_count > 0);

    let metadata = bridge
        .read_voxel_conversion_source_metadata(VoxelConversionSourceMetadataRequest {
            source: imported.source,
        })
        .unwrap();
    assert!(metadata.registered);
    assert_eq!(metadata.vertex_count, 48);
    assert_eq!(metadata.triangle_count, 12);
    assert_eq!(metadata.groups.len(), 2);
    assert_eq!(metadata.evidence.len(), 1);
    let model = bridge
        .read_voxel_model_info(VoxelModelInfoRequest {
            grid: 7,
            volume_asset_id: Some("voxel/generated".to_string()),
            include_material_counts: true,
        })
        .unwrap();
    assert!(model.resident);
    assert_eq!(model.voxel_count, applied.output_voxel_count);
}

#[test]
fn mesh_import_preflight_rejects_before_hash_or_authority_mutation() {
    let source_bytes = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../../harness/fixtures/voxel-conversion/kenney-wall-a.glb"
    ));
    let mut bridge = init_bridge();
    let existing_plan = bridge
        .plan_voxel_conversion(project_voxel_conversion_request(7))
        .unwrap();
    let source_count = bridge.voxel.voxel_conversion_sources.len();
    let metadata_count = bridge.voxel.voxel_conversion_source_metadata.len();

    let rejected = bridge
        .import_voxel_conversion_mesh_source(VoxelConversionMeshSourceImportRequest {
            source_asset_id: "a"
                .repeat(VOXEL_CONVERSION_MESH_IMPORT_MAX_ASSET_ID_BYTES as usize + 1),
            asset_version: 1,
            source_path: "assets/reference/kenney-wall-a.glb".to_string(),
            format: VoxelConversionMeshSourceFormat::Glb,
            source_bytes: source_bytes.to_vec(),
            mesh_primitive: None,
        })
        .unwrap();

    assert!(!rejected.imported);
    assert_eq!(rejected.source.source_hash, "sha256:not-computed");
    assert_eq!(
        rejected.diagnostics[0].code,
        VoxelConversionDiagnosticCode::OutputLimitExceeded
    );
    assert_eq!(bridge.voxel.voxel_conversion_sources.len(), source_count);
    assert_eq!(
        bridge.voxel.voxel_conversion_source_metadata.len(),
        metadata_count
    );
    assert_eq!(
        bridge
            .voxel
            .voxel_conversion_plan
            .as_ref()
            .map(|plan| &plan.plan.plan_id),
        Some(&existing_plan.plan_id)
    );
}
