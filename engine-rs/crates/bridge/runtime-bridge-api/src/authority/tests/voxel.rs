use super::*;

#[test]
fn native_runtime_seeds_default_studio_and_authored_voxel_targets() {
    let bridge = init_bridge();
    for grid in [1, 2, 7] {
        assert!(bridge
            .voxel
            .voxel_conversion_targets
            .contains_key(&(grid, Some("voxel/generated".to_string()))));
    }
}

#[test]
fn initialize_voxel_volume_authoring_commits_a_real_runtime_model_atomically() {
    let mut bridge = init_bridge();
    let stored_fixture = hand_authored_voxel_volume_asset();
    let request = VoxelVolumeAuthoringInitializeRequest {
        grid: 1,
        volume_asset_id: Some("voxel-volume/conformance-blank".to_string()),
        seed_chunk: VoxelAssetCoord { x: 0, y: 0, z: 0 },
        material_palette: stored_fixture.material_palette,
        authoring: stored_fixture.authoring,
        max_material_bindings: 8,
    };

    let receipt = bridge
        .initialize_voxel_volume_authoring(request.clone())
        .unwrap();
    assert!(receipt.initialized, "{:?}", receipt.diagnostics);
    assert_eq!(receipt.request, request);
    assert_eq!(receipt.grid, 1);
    assert!(receipt.session_hash.starts_with("fnv1a64:"));
    assert!(receipt.replay_hash.starts_with("fnv1a64:"));

    let duplicate = bridge.initialize_voxel_volume_authoring(request).unwrap();
    assert!(!duplicate.initialized);
    assert_eq!(duplicate.diagnostics.len(), 1);
}

#[test]
fn voxel_conversion_plan_preview_apply_uses_rust_authority_and_commands() {
    let mut bridge = init_bridge();
    let request = project_voxel_conversion_request(7);
    let plan = bridge.plan_voxel_conversion(request).unwrap();
    assert_eq!(
        plan.authority_version,
        svc_voxel_conversion::AUTHORITY_VERSION
    );
    assert_eq!(plan.source.asset_id, "mesh/import-fixture-a");
    assert_eq!(plan.target.grid, 7);
    assert!(plan.diagnostics.is_empty());
    assert_eq!(plan.estimated_output_voxels, 3);

    let stale = bridge
        .preview_voxel_conversion(VoxelConversionPreviewRequest {
            plan_id: plan.plan_id.clone(),
            expected_plan_hash: "fnv1a64:stale".to_string(),
        })
        .unwrap();
    assert_eq!(
        stale.diagnostics[0].code,
        VoxelConversionDiagnosticCode::StaleAuthoritySnapshot
    );

    let preview = bridge
        .preview_voxel_conversion(VoxelConversionPreviewRequest {
            plan_id: plan.plan_id.clone(),
            expected_plan_hash: svc_voxel_conversion::plan_hash(&plan),
        })
        .unwrap();
    assert!(preview.diagnostics.is_empty());
    assert_eq!(preview.output_voxel_count, 3);

    let receipt = bridge
        .apply_voxel_conversion(VoxelConversionApplyRequest {
            plan_id: plan.plan_id.clone(),
            expected_plan_hash: svc_voxel_conversion::plan_hash(&plan),
            expected_preview_hash: Some(preview.output_hash.clone()),
        })
        .unwrap();
    assert!(receipt.applied);
    assert_eq!(receipt.output_voxel_count, 3);

    let world = bridge.voxel.voxel.as_ref().unwrap();
    assert_eq!(world.grid().id(), GridId::new(7));
    let chunk = world.get(ChunkCoord::new(0, 0, 0)).unwrap();
    assert_eq!(
        chunk.get(LocalVoxelCoord::new(0, 0, 0)),
        Some(VoxelValue::solid_raw(3)),
        "conversion output applied through voxel command authority"
    );

    let exported = bridge
        .export_voxel_conversion_evidence(
            plan.evidence
                .iter()
                .chain(preview.evidence.iter())
                .chain(receipt.evidence.iter())
                .cloned()
                .collect(),
        )
        .unwrap();
    assert_eq!(exported.len(), 3);

    let model_info = bridge
        .read_voxel_model_info(VoxelModelInfoRequest {
            grid: 7,
            volume_asset_id: Some("voxel/generated".to_string()),
            include_material_counts: true,
        })
        .unwrap();
    assert!(model_info.resident);
    assert_eq!(
        model_info.model_id,
        "voxel-model:grid:7:volume:voxel/generated"
    );
    assert_eq!(model_info.voxel_count, 3);
    assert_eq!(
        model_info.material_counts,
        vec![VoxelModelMaterialCount {
            material: 3,
            voxel_count: 3
        }]
    );
    assert_eq!(
        model_info.source.as_ref().unwrap().asset_id,
        "mesh/import-fixture-a"
    );
    assert_eq!(
        model_info.latest_plan_id.as_deref(),
        Some(plan.plan_id.as_str())
    );
    assert!(model_info.latest_output_hash.is_some());
    assert!(model_info.session_hash.starts_with("fnv1a64:"));
    assert!(model_info.replay_hash.starts_with("fnv1a64:"));
    assert!(model_info.diagnostics.is_empty());

    let compact_info = bridge
        .read_voxel_model_info(VoxelModelInfoRequest {
            grid: 7,
            volume_asset_id: Some("voxel/generated".to_string()),
            include_material_counts: false,
        })
        .unwrap();
    assert!(compact_info.material_counts.is_empty());

    let window = bridge
        .read_voxel_model_window(VoxelModelWindowRequest {
            grid: 7,
            volume_asset_id: Some("voxel/generated".to_string()),
            bounds: protocol_voxel_conversion::VoxelConversionBounds {
                min: protocol_voxel_conversion::VoxelConversionCoord { x: 0, y: 0, z: 0 },
                max: protocol_voxel_conversion::VoxelConversionCoord { x: 3, y: 3, z: 0 },
            },
            include_empty: false,
            material_filter: Vec::new(),
            max_samples: 16,
        })
        .unwrap();
    assert!(window.resident);
    assert_eq!(window.scanned_voxel_count, 16);
    assert_eq!(window.returned_sample_count, 3);
    assert_eq!(
        window
            .samples
            .iter()
            .map(|sample| sample.material)
            .collect::<Vec<_>>(),
        vec![Some(3), Some(3), Some(3)]
    );
    assert_eq!(window.model_bounds, model_info.bounds);
    assert!(window.session_hash.starts_with("fnv1a64:"));
    assert!(window.replay_hash.starts_with("fnv1a64:"));
    assert!(window.diagnostics.is_empty());

    let filtered_window = bridge
        .read_voxel_model_window(VoxelModelWindowRequest {
            grid: 7,
            volume_asset_id: Some("voxel/generated".to_string()),
            bounds: protocol_voxel_conversion::VoxelConversionBounds {
                min: protocol_voxel_conversion::VoxelConversionCoord { x: 0, y: 0, z: 0 },
                max: protocol_voxel_conversion::VoxelConversionCoord { x: 3, y: 3, z: 0 },
            },
            include_empty: true,
            material_filter: vec![99],
            max_samples: 16,
        })
        .unwrap();
    assert!(filtered_window.resident);
    assert_eq!(filtered_window.scanned_voxel_count, 16);
    assert_eq!(filtered_window.returned_sample_count, 0);
    assert!(filtered_window.samples.is_empty());

    let exported = bridge
        .export_voxel_volume_asset(VoxelVolumeAssetExportRequest {
            grid: 7,
            volume_asset_id: Some("voxel/generated".to_string()),
            target_asset_id: "voxel-volume/generated-crate".to_string(),
            label: Some("Generated crate".to_string()),
            created_by: Some("runtime-bridge-api-test".to_string()),
            source_tool: Some("svc-voxel-conversion".to_string()),
            max_sparse_runs: 16,
            expected_session_hash: Some(model_info.session_hash.clone()),
        })
        .unwrap();
    assert!(exported.exported);
    assert!(exported.diagnostics.is_empty());
    let asset = exported.asset.as_ref().expect("exported asset");
    assert_eq!(asset.asset_id, "voxel-volume/generated-crate");
    assert_eq!(
        asset.schema_version,
        protocol_voxel_asset::VOXEL_ASSET_SCHEMA_VERSION
    );
    assert_eq!(
        asset.media_type,
        protocol_voxel_asset::VOXEL_ASSET_MEDIA_TYPE
    );
    assert_eq!(
        asset.material_palette[0].material_asset_id,
        "material/surface-a"
    );
    assert_eq!(
        asset
            .representation
            .sparse_runs
            .iter()
            .map(|run| run.length as u64)
            .sum::<u64>(),
        3
    );
    assert_eq!(
        exported.canonical_json_hash.as_deref(),
        Some(asset.content_hashes.canonical_json.as_str())
    );
    assert_eq!(
        exported.voxel_data_hash.as_deref(),
        Some(asset.content_hashes.voxel_data.as_str())
    );
    let canonical_json = exported.canonical_json.as_ref().expect("canonical json");
    let decoded = svc_voxel_asset::decode_asset(canonical_json).expect("canonical asset decodes");
    assert_eq!(decoded, *asset);

    let save = bridge
        .save_voxel_volume_asset(VoxelVolumeAssetSaveRequest {
            export_request: VoxelVolumeAssetExportRequest {
                grid: 7,
                volume_asset_id: Some("voxel/generated".to_string()),
                target_asset_id: "voxel-volume/generated-crate".to_string(),
                label: Some("Generated crate".to_string()),
                created_by: Some("runtime-bridge-api-test".to_string()),
                source_tool: Some("svc-voxel-conversion".to_string()),
                max_sparse_runs: 16,
                expected_session_hash: Some(model_info.session_hash.clone()),
            },
            target_project_bundle: "asha-demo".to_string(),
            target_asset_path: "assets/voxels/generated-crate.avxl.json".to_string(),
            representation_kind: "sparse_runs".to_string(),
            expected_existing_canonical_json_hash: None,
            expected_canonical_json_hash: exported.canonical_json_hash.clone(),
            expected_voxel_data_hash: exported.voxel_data_hash.clone(),
        })
        .unwrap();
    assert!(save.saved);
    assert!(save.diagnostics.is_empty());
    assert_eq!(
        save.canonical_json_hash.as_deref(),
        exported.canonical_json_hash.as_deref()
    );
    assert_eq!(
        save.voxel_data_hash.as_deref(),
        exported.voxel_data_hash.as_deref()
    );
    let diff = save.diff.as_ref().expect("stored diff");
    assert_eq!(diff.project_bundle, "asha-demo");
    assert_eq!(diff.asset_id, "voxel-volume/generated-crate");
    assert_eq!(diff.asset_path, "assets/voxels/generated-crate.avxl.json");
    assert_eq!(diff.operation, "create");
    assert_eq!(
        diff.sparse_run_count,
        asset.representation.sparse_runs.len() as u64
    );
    assert_eq!(diff.voxel_count, 3);
    assert_eq!(diff.material_count, 1);
    assert_eq!(diff.runtime_session_hash, model_info.session_hash);
    assert_eq!(
        save.canonical_json.as_deref(),
        exported.canonical_json.as_deref()
    );

    let invalid_path_save = bridge
        .save_voxel_volume_asset(VoxelVolumeAssetSaveRequest {
            export_request: VoxelVolumeAssetExportRequest {
                grid: 7,
                volume_asset_id: Some("voxel/generated".to_string()),
                target_asset_id: "voxel-volume/generated-crate".to_string(),
                label: None,
                created_by: None,
                source_tool: None,
                max_sparse_runs: 16,
                expected_session_hash: None,
            },
            target_project_bundle: "asha-demo".to_string(),
            target_asset_path: "/tmp/generated-crate.avxl.json".to_string(),
            representation_kind: "sparse_runs".to_string(),
            expected_existing_canonical_json_hash: None,
            expected_canonical_json_hash: None,
            expected_voxel_data_hash: None,
        })
        .unwrap();
    assert!(!invalid_path_save.saved);
    assert_eq!(
        invalid_path_save.diagnostics[0].code,
        protocol_voxel_asset::VoxelAssetDiagnosticCode::InvalidAssetId
    );

    let unsupported_save = bridge
        .save_voxel_volume_asset(VoxelVolumeAssetSaveRequest {
            export_request: VoxelVolumeAssetExportRequest {
                grid: 7,
                volume_asset_id: Some("voxel/generated".to_string()),
                target_asset_id: "voxel-volume/generated-crate".to_string(),
                label: None,
                created_by: None,
                source_tool: None,
                max_sparse_runs: 16,
                expected_session_hash: None,
            },
            target_project_bundle: "asha-demo".to_string(),
            target_asset_path: "assets/voxels/generated-crate.avxl.json".to_string(),
            representation_kind: "dense_grid".to_string(),
            expected_existing_canonical_json_hash: None,
            expected_canonical_json_hash: None,
            expected_voxel_data_hash: None,
        })
        .unwrap();
    assert!(!unsupported_save.saved);
    assert_eq!(
        unsupported_save.diagnostics[0].code,
        protocol_voxel_asset::VoxelAssetDiagnosticCode::UnsupportedRepresentation
    );

    let hash_mismatch_save = bridge
        .save_voxel_volume_asset(VoxelVolumeAssetSaveRequest {
            export_request: VoxelVolumeAssetExportRequest {
                grid: 7,
                volume_asset_id: Some("voxel/generated".to_string()),
                target_asset_id: "voxel-volume/generated-crate".to_string(),
                label: None,
                created_by: None,
                source_tool: None,
                max_sparse_runs: 16,
                expected_session_hash: None,
            },
            target_project_bundle: "asha-demo".to_string(),
            target_asset_path: "assets/voxels/generated-crate.avxl.json".to_string(),
            representation_kind: "sparse_runs".to_string(),
            expected_existing_canonical_json_hash: Some("fnv1a64:previous".to_string()),
            expected_canonical_json_hash: Some("fnv1a64:wrong".to_string()),
            expected_voxel_data_hash: exported.voxel_data_hash.clone(),
        })
        .unwrap();
    assert!(!hash_mismatch_save.saved);
    assert_eq!(
        hash_mismatch_save.diagnostics[0].code,
        protocol_voxel_asset::VoxelAssetDiagnosticCode::ContentHashMismatch
    );

    let stale_export = bridge
        .export_voxel_volume_asset(VoxelVolumeAssetExportRequest {
            grid: 7,
            volume_asset_id: Some("voxel/generated".to_string()),
            target_asset_id: "voxel-volume/stale".to_string(),
            label: None,
            created_by: None,
            source_tool: None,
            max_sparse_runs: 16,
            expected_session_hash: Some("fnv1a64:stale".to_string()),
        })
        .unwrap();
    assert!(!stale_export.exported);
    assert_eq!(
        stale_export.diagnostics[0].code,
        protocol_voxel_asset::VoxelAssetDiagnosticCode::StaleRuntimeSnapshot
    );

    let stale_save = bridge
        .save_voxel_volume_asset(VoxelVolumeAssetSaveRequest {
            export_request: VoxelVolumeAssetExportRequest {
                grid: 7,
                volume_asset_id: Some("voxel/generated".to_string()),
                target_asset_id: "voxel-volume/stale-save".to_string(),
                label: None,
                created_by: None,
                source_tool: None,
                max_sparse_runs: 16,
                expected_session_hash: Some("fnv1a64:stale".to_string()),
            },
            target_project_bundle: "asha-demo".to_string(),
            target_asset_path: "assets/voxels/stale-save.avxl.json".to_string(),
            representation_kind: "sparse_runs".to_string(),
            expected_existing_canonical_json_hash: None,
            expected_canonical_json_hash: None,
            expected_voxel_data_hash: None,
        })
        .unwrap();
    assert!(!stale_save.saved);
    assert_eq!(
        stale_save.diagnostics[0].code,
        protocol_voxel_asset::VoxelAssetDiagnosticCode::StaleRuntimeSnapshot
    );

    let mut load_bridge = init_bridge();
    let load_receipt = load_bridge
        .load_voxel_volume_asset(VoxelVolumeAssetLoadRequest {
            asset: asset.clone(),
            target_grid: 7,
            target_volume_asset_id: Some("voxel/generated".to_string()),
            replace_existing: true,
            include_material_counts: true,
        })
        .unwrap();
    assert!(load_receipt.loaded);
    assert_eq!(load_receipt.request_asset_id, asset.asset_id);
    assert_eq!(load_receipt.voxel_count, 3);
    assert_eq!(
        load_receipt.material_counts,
        vec![VoxelAssetMaterialCount {
            material: 3,
            voxel_count: 3
        }]
    );
    let reloaded_info = load_bridge
        .read_voxel_model_info(VoxelModelInfoRequest {
            grid: 7,
            volume_asset_id: Some("voxel/generated".to_string()),
            include_material_counts: true,
        })
        .unwrap();
    assert!(reloaded_info.resident);
    assert_eq!(reloaded_info.voxel_count, 3);
    assert_eq!(
        reloaded_info.source.as_ref().unwrap().asset_id,
        "voxel-volume/generated-crate"
    );
}

#[test]
fn voxel_model_window_fails_closed_for_invalid_bounds_and_query_quota() {
    let asset = hand_authored_voxel_volume_asset();
    let mut bridge = init_bridge();
    let load_receipt = bridge
        .load_voxel_volume_asset(VoxelVolumeAssetLoadRequest {
            asset,
            target_grid: 7,
            target_volume_asset_id: Some("voxel/window-test".to_string()),
            replace_existing: true,
            include_material_counts: true,
        })
        .unwrap();
    assert!(load_receipt.loaded);

    let invalid_bounds = bridge
        .read_voxel_model_window(VoxelModelWindowRequest {
            grid: 7,
            volume_asset_id: Some("voxel/window-test".to_string()),
            bounds: protocol_voxel_conversion::VoxelConversionBounds {
                min: protocol_voxel_conversion::VoxelConversionCoord { x: 3, y: 0, z: 0 },
                max: protocol_voxel_conversion::VoxelConversionCoord { x: 1, y: 0, z: 0 },
            },
            include_empty: true,
            material_filter: Vec::new(),
            max_samples: 16,
        })
        .unwrap();
    assert!(invalid_bounds.resident);
    assert_eq!(invalid_bounds.scanned_voxel_count, 0);
    assert!(invalid_bounds.samples.is_empty());
    assert_eq!(
        invalid_bounds.diagnostics[0].code,
        VoxelConversionDiagnosticCode::InvalidQueryBounds
    );

    let over_quota = bridge
        .read_voxel_model_window(VoxelModelWindowRequest {
            grid: 7,
            volume_asset_id: Some("voxel/window-test".to_string()),
            bounds: protocol_voxel_conversion::VoxelConversionBounds {
                min: protocol_voxel_conversion::VoxelConversionCoord { x: 0, y: 0, z: 0 },
                max: protocol_voxel_conversion::VoxelConversionCoord { x: 4, y: 0, z: 0 },
            },
            include_empty: true,
            material_filter: Vec::new(),
            max_samples: 4,
        })
        .unwrap();
    assert!(over_quota.resident);
    assert_eq!(over_quota.scanned_voxel_count, 0);
    assert!(over_quota.samples.is_empty());
    assert_eq!(
        over_quota.diagnostics[0].code,
        VoxelConversionDiagnosticCode::QueryQuotaExceeded
    );

    let missing = bridge
        .read_voxel_model_window(VoxelModelWindowRequest {
            grid: 7,
            volume_asset_id: Some("voxel/missing".to_string()),
            bounds: protocol_voxel_conversion::VoxelConversionBounds {
                min: protocol_voxel_conversion::VoxelConversionCoord { x: 0, y: 0, z: 0 },
                max: protocol_voxel_conversion::VoxelConversionCoord { x: 0, y: 0, z: 0 },
            },
            include_empty: true,
            material_filter: Vec::new(),
            max_samples: 1,
        })
        .unwrap();
    assert!(!missing.resident);
    assert_eq!(
        missing.diagnostics[0].code,
        VoxelConversionDiagnosticCode::VoxelConversionUnavailable
    );
}

#[test]
fn voxel_volume_asset_load_accepts_hand_authored_asset_and_rejects_invalid_assets() {
    let asset = hand_authored_voxel_volume_asset();
    let mut bridge = init_bridge();
    let receipt = bridge
        .load_voxel_volume_asset(VoxelVolumeAssetLoadRequest {
            asset: asset.clone(),
            target_grid: 7,
            target_volume_asset_id: Some(asset.asset_id.clone()),
            replace_existing: true,
            include_material_counts: true,
        })
        .unwrap();
    assert!(receipt.loaded);
    assert_eq!(receipt.voxel_count, 2);
    assert_eq!(
        receipt.material_counts,
        vec![VoxelAssetMaterialCount {
            material: 1,
            voxel_count: 2
        }]
    );
    let info = bridge
        .read_voxel_model_info(VoxelModelInfoRequest {
            grid: 7,
            volume_asset_id: Some(asset.asset_id.clone()),
            include_material_counts: true,
        })
        .unwrap();
    assert!(info.resident);
    assert_eq!(info.voxel_count, 2);
    assert_eq!(
        info.source.as_ref().unwrap().source_hash,
        asset.content_hashes.voxel_data
    );

    let mut invalid_hash = asset.clone();
    invalid_hash.content_hashes.voxel_data = "fnv1a64:stale".to_string();
    let mut invalid_bridge = init_bridge();
    let rejected = invalid_bridge
        .load_voxel_volume_asset(VoxelVolumeAssetLoadRequest {
            asset: invalid_hash,
            target_grid: 7,
            target_volume_asset_id: Some("voxel/invalid".to_string()),
            replace_existing: true,
            include_material_counts: true,
        })
        .unwrap();
    assert!(!rejected.loaded);
    assert_eq!(
        rejected.diagnostics[0].code,
        protocol_voxel_asset::VoxelAssetDiagnosticCode::ContentHashMismatch
    );
    let missing = invalid_bridge
        .read_voxel_model_info(VoxelModelInfoRequest {
            grid: 7,
            volume_asset_id: Some("voxel/invalid".to_string()),
            include_material_counts: true,
        })
        .unwrap();
    assert!(!missing.resident);

    let mut invalid_material = asset;
    invalid_material.material_palette[0].material_asset_id = "texture/not-material".to_string();
    invalid_material = svc_voxel_asset::with_computed_hashes(&invalid_material);
    let rejected_material = invalid_bridge
        .load_voxel_volume_asset(VoxelVolumeAssetLoadRequest {
            asset: invalid_material,
            target_grid: 7,
            target_volume_asset_id: Some("voxel/invalid-material".to_string()),
            replace_existing: true,
            include_material_counts: true,
        })
        .unwrap();
    assert!(!rejected_material.loaded);
    assert_eq!(
        rejected_material.diagnostics[0].code,
        protocol_voxel_asset::VoxelAssetDiagnosticCode::InvalidMaterialReference
    );
}

#[test]
fn voxel_annotation_layer_runtime_bridge_validates_loads_queries_edits_and_exports() {
    let asset = hand_authored_voxel_volume_asset();
    let finalized_fixture = hand_authored_voxel_annotation_layer(&asset);
    let mut bridge = init_bridge();
    let volume_load = bridge
        .load_voxel_volume_asset(VoxelVolumeAssetLoadRequest {
            asset: asset.clone(),
            target_grid: 7,
            target_volume_asset_id: Some(asset.asset_id.clone()),
            replace_existing: true,
            include_material_counts: false,
        })
        .unwrap();
    assert!(volume_load.loaded);

    let validation = bridge
        .validate_voxel_annotation_layer(VoxelAnnotationLayerValidationRequest {
            input: VoxelAnnotationLayerValidationInput::Draft {
                draft: VoxelAnnotationLayerDraft {
                    layer_id: finalized_fixture.layer_id.clone(),
                    schema_version: finalized_fixture.schema_version,
                    media_type: finalized_fixture.media_type.clone(),
                    target_voxel_volume_asset_id: finalized_fixture
                        .target_voxel_volume_asset_id
                        .clone(),
                    target_voxel_data_hash: finalized_fixture.target_voxel_data_hash.clone(),
                    target_bounds: finalized_fixture.target_bounds,
                    regions: finalized_fixture.regions.clone(),
                    provenance: finalized_fixture.provenance.clone(),
                },
            },
            expected_target_voxel_volume_asset_id: Some(asset.asset_id.clone()),
            expected_target_voxel_data_hash: Some(asset.content_hashes.voxel_data.clone()),
            max_regions: 16,
            max_sparse_runs_per_region: 16,
            max_total_assigned_cells: 16,
        })
        .unwrap();
    assert!(validation.valid);
    assert_eq!(validation.assigned_cell_count, 2);
    let layer = validation.normalized_layer.expect("normalized layer");
    assert_eq!(
        validation.canonical_json_hash.as_deref(),
        Some(layer.content_hashes.canonical_json.as_str())
    );

    let stale_load = bridge
        .load_voxel_annotation_layer(VoxelAnnotationLayerLoadRequest {
            layer: layer.clone(),
            target_grid: 7,
            replace_existing: true,
            expected_session_hash: Some("fnv1a64:stale".to_string()),
        })
        .unwrap();
    assert!(!stale_load.loaded);
    assert_eq!(
        stale_load.diagnostics[0].code,
        protocol_voxel_annotation::VoxelAnnotationDiagnosticCode::TargetVoxelHashMismatch
    );

    let load = bridge
        .load_voxel_annotation_layer(VoxelAnnotationLayerLoadRequest {
            layer: layer.clone(),
            target_grid: 7,
            replace_existing: true,
            expected_session_hash: Some(volume_load.session_hash.clone()),
        })
        .unwrap();
    assert!(load.loaded);
    assert_eq!(load.region_count, 1);
    assert_eq!(load.assigned_cell_count, 2);
    assert_eq!(
        load.layer_hash.as_deref(),
        Some(layer.content_hashes.canonical_json.as_str())
    );
    let runtime_layer_id = load.runtime_layer_id.clone();

    let query = bridge
        .read_voxel_annotation_query(VoxelAnnotationQueryRequest {
            runtime_layer_id: runtime_layer_id.clone(),
            layer_id: layer.layer_id.clone(),
            mode: protocol_voxel_annotation::VoxelAnnotationQueryMode::Cell,
            cell: Some(protocol_voxel_annotation::VoxelAnnotationCoord { x: 1, y: 0, z: 0 }),
            bounds: None,
            region_id: None,
            max_regions: 4,
            expected_layer_hash: Some(layer.content_hashes.canonical_json.clone()),
        })
        .unwrap();
    assert!(query.diagnostics.is_empty());
    assert_eq!(query.matched_regions.len(), 1);
    assert_eq!(query.matched_regions[0].region_id, "region/entry-room");

    let stale_edit = bridge
        .apply_voxel_annotation_edit(VoxelAnnotationEditRequest {
            runtime_layer_id: runtime_layer_id.clone(),
            layer_id: layer.layer_id.clone(),
            expected_layer_hash: "fnv1a64:stale".to_string(),
            operation: protocol_voxel_annotation::VoxelAnnotationEditOperation::SetLabel,
            region_id: Some("region/entry-room".to_string()),
            region: None,
            sparse_runs: Vec::new(),
            tags: Vec::new(),
            label: Some("Edited room".to_string()),
            kind: None,
            parent_region_id: None,
        })
        .unwrap();
    assert!(!stale_edit.edited);
    assert_eq!(
        stale_edit.diagnostics[0].code,
        protocol_voxel_annotation::VoxelAnnotationDiagnosticCode::StaleLayerHash
    );

    let edit = bridge
        .apply_voxel_annotation_edit(VoxelAnnotationEditRequest {
            runtime_layer_id: runtime_layer_id.clone(),
            layer_id: layer.layer_id.clone(),
            expected_layer_hash: layer.content_hashes.canonical_json.clone(),
            operation: protocol_voxel_annotation::VoxelAnnotationEditOperation::SetLabel,
            region_id: Some("region/entry-room".to_string()),
            region: None,
            sparse_runs: Vec::new(),
            tags: Vec::new(),
            label: Some("Edited room".to_string()),
            kind: None,
            parent_region_id: None,
        })
        .unwrap();
    assert!(edit.edited);
    let edited_hash = edit.layer_hash_after.clone().expect("edited hash");
    assert_ne!(edited_hash, layer.content_hashes.canonical_json);
    assert!(edit.replay_hash.starts_with("fnv1a64:"));

    let export = bridge
        .export_voxel_annotation_layer(VoxelAnnotationLayerExportRequest {
            runtime_layer_id,
            layer_id: layer.layer_id,
            expected_layer_hash: edited_hash,
            include_diagnostics: true,
        })
        .unwrap();
    assert!(export.exported);
    assert!(export.canonical_json.is_some());
    assert_eq!(
        export.layer.as_ref().unwrap().regions[0].label,
        "Edited room"
    );
    assert!(export.diagnostics.is_empty());
}

fn stored_voxel_palette_update_request(
    asset: VoxelVolumeAsset,
) -> VoxelVolumeAssetPaletteUpdateRequest {
    VoxelVolumeAssetPaletteUpdateRequest {
        material_palette: asset.material_palette.clone(),
        expected_canonical_json_hash: asset.content_hashes.canonical_json.clone(),
        expected_voxel_data_hash: asset.content_hashes.voxel_data.clone(),
        asset: asset.clone(),
        target_project_bundle: "asha-studio-palette".to_string(),
        target_asset_path: "assets/voxels/hand-authored-room.avxl.json".to_string(),
        max_material_bindings: 16,
    }
}

#[test]
fn stored_voxel_palette_update_is_hash_guarded_validated_and_round_trips() {
    let asset = hand_authored_voxel_volume_asset();
    let mut request = stored_voxel_palette_update_request(asset.clone());
    request.material_palette[0].palette_entry_id = "voxel-material/polished-concrete".to_string();
    request.material_palette[0].display_name = Some("Polished concrete".to_string());
    request.material_palette[0].material_asset_id = "material/polished-concrete".to_string();
    request.material_palette[0].material_catalog_binding_id =
        Some("catalog-binding/polished-concrete".to_string());
    let bridge = init_bridge();
    assert!(bridge.voxel.voxel_model_infos.is_empty());
    let receipt = bridge
        .update_voxel_volume_asset_palette(request.clone())
        .unwrap();
    assert!(receipt.updated, "{:?}", receipt.diagnostics);
    assert!(bridge.voxel.voxel_model_infos.is_empty());
    assert_eq!(
        receipt.voxel_data_hash.as_deref(),
        Some(asset.content_hashes.voxel_data.as_str())
    );
    assert_ne!(
        receipt.canonical_json_hash.as_deref(),
        Some(asset.content_hashes.canonical_json.as_str())
    );
    let reopened = svc_voxel_asset::decode_asset(
        receipt
            .canonical_json
            .as_deref()
            .expect("canonical payload"),
    )
    .expect("updated stored asset reopens");
    assert_eq!(
        reopened.material_palette[0].display_name.as_deref(),
        Some("Polished concrete")
    );
    assert_eq!(
        reopened.material_palette[0].palette_entry_id,
        "voxel-material/polished-concrete"
    );
    assert_eq!(
        reopened.material_palette[0].material_asset_id,
        "material/polished-concrete"
    );
    assert_eq!(
        reopened.material_palette[0]
            .material_catalog_binding_id
            .as_deref(),
        Some("catalog-binding/polished-concrete")
    );

    let mut stale = request.clone();
    stale.expected_canonical_json_hash = "fnv1a64:stale".to_string();
    stale.expected_voxel_data_hash = "fnv1a64:stale".to_string();
    let stale_receipt = bridge.update_voxel_volume_asset_palette(stale).unwrap();
    assert!(!stale_receipt.updated);
    assert_eq!(
        stale_receipt.diagnostics[0].code,
        VoxelAssetDiagnosticCode::ContentHashMismatch
    );
    assert!(stale_receipt
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.reference == "expectedVoxelDataHash"));

    let mut duplicate = request;
    duplicate.material_palette[0].palette_entry_id = "bad palette".to_string();
    duplicate.material_palette[0].material_asset_id = "texture/not-material".to_string();
    duplicate.material_palette[0].material_catalog_binding_id =
        Some("bad catalog binding".to_string());
    duplicate
        .material_palette
        .push(duplicate.material_palette[0].clone());
    let duplicate_receipt = bridge.update_voxel_volume_asset_palette(duplicate).unwrap();
    assert!(!duplicate_receipt.updated);
    assert!(duplicate_receipt
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == VoxelAssetDiagnosticCode::DuplicateMaterialBinding));
    assert!(duplicate_receipt
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == VoxelAssetDiagnosticCode::InvalidMaterialReference));
}

#[test]
fn stored_voxel_palette_update_rejects_source_shape_quotas_before_validation() {
    let bridge = init_bridge();

    let mut represented = stored_voxel_palette_update_request(hand_authored_voxel_volume_asset());
    represented.asset.representation.sparse_runs[0].length =
        (VOXEL_PALETTE_UPDATE_MAX_REPRESENTED_VOXELS + 1) as u32;
    let represented_receipt = bridge
        .update_voxel_volume_asset_palette(represented)
        .unwrap();
    assert!(!represented_receipt.updated);
    assert_eq!(
        represented_receipt.diagnostics[0].reference,
        "asset.representation.representedVoxelCount"
    );

    let mut run_count = stored_voxel_palette_update_request(hand_authored_voxel_volume_asset());
    run_count.asset.representation.sparse_runs =
        vec![
            run_count.asset.representation.sparse_runs[0].clone();
            VOXEL_PALETTE_UPDATE_MAX_SPARSE_RUNS as usize + 1
        ];
    let run_count_receipt = bridge.update_voxel_volume_asset_palette(run_count).unwrap();
    assert!(!run_count_receipt.updated);
    assert_eq!(
        run_count_receipt.diagnostics[0].reference,
        "asset.representation.sparseRuns"
    );
}

#[test]
fn stored_voxel_palette_update_rejects_palette_and_string_quotas() {
    let bridge = init_bridge();
    let binding = hand_authored_voxel_volume_asset().material_palette[0].clone();

    let mut source_palette =
        stored_voxel_palette_update_request(hand_authored_voxel_volume_asset());
    source_palette.asset.material_palette =
        vec![binding.clone(); VOXEL_PALETTE_UPDATE_MAX_MATERIAL_BINDINGS as usize + 1];
    let source_receipt = bridge
        .update_voxel_volume_asset_palette(source_palette)
        .unwrap();
    assert!(!source_receipt.updated);
    assert_eq!(
        source_receipt.diagnostics[0].reference,
        "asset.materialPalette"
    );

    let mut replacement_palette =
        stored_voxel_palette_update_request(hand_authored_voxel_volume_asset());
    replacement_palette.max_material_bindings = VOXEL_PALETTE_UPDATE_MAX_MATERIAL_BINDINGS;
    replacement_palette.material_palette =
        vec![binding; VOXEL_PALETTE_UPDATE_MAX_MATERIAL_BINDINGS as usize + 1];
    let replacement_receipt = bridge
        .update_voxel_volume_asset_palette(replacement_palette)
        .unwrap();
    assert!(!replacement_receipt.updated);
    assert!(replacement_receipt
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.reference == "materialPalette"));

    let mut oversized_string =
        stored_voxel_palette_update_request(hand_authored_voxel_volume_asset());
    oversized_string.target_project_bundle =
        "x".repeat(VOXEL_PALETTE_UPDATE_MAX_STRING_BYTES as usize + 1);
    let string_receipt = bridge
        .update_voxel_volume_asset_palette(oversized_string)
        .unwrap();
    assert!(!string_receipt.updated);
    assert_eq!(
        string_receipt.diagnostics[0].reference,
        "targetProjectBundle"
    );
}

#[test]
fn stored_voxel_palette_update_rejects_aggregate_serialized_size() {
    let bridge = init_bridge();
    let mut request = stored_voxel_palette_update_request(hand_authored_voxel_volume_asset());
    request.max_material_bindings = VOXEL_PALETTE_UPDATE_MAX_MATERIAL_BINDINGS;
    let long_value = "x".repeat(VOXEL_PALETTE_UPDATE_MAX_STRING_BYTES as usize / 2);
    let mut binding = request.material_palette[0].clone();
    binding.palette_entry_id = long_value.clone();
    binding.display_name = Some(long_value.clone());
    binding.material_asset_id = long_value.clone();
    binding.material_catalog_binding_id = Some(long_value);
    request.material_palette = vec![binding; 1_024];

    let receipt = bridge.update_voxel_volume_asset_palette(request).unwrap();
    assert!(!receipt.updated);
    assert_eq!(receipt.diagnostics[0].reference, "request");
    assert_eq!(
        receipt.diagnostics[0].code,
        VoxelAssetDiagnosticCode::ExportLimitExceeded
    );
}

#[test]
fn voxel_volume_asset_save_rejects_missing_material_refs_without_storage_diff() {
    let mut bridge = init_bridge();
    let mut request = project_voxel_conversion_request(7);
    request.settings.material_map.entries[0].source_material_id = None;
    let plan = bridge.plan_voxel_conversion(request).unwrap();
    assert!(plan.diagnostics.is_empty());

    let preview = bridge
        .preview_voxel_conversion(VoxelConversionPreviewRequest {
            plan_id: plan.plan_id.clone(),
            expected_plan_hash: svc_voxel_conversion::plan_hash(&plan),
        })
        .unwrap();
    assert!(preview.diagnostics.is_empty());

    let receipt = bridge
        .apply_voxel_conversion(VoxelConversionApplyRequest {
            plan_id: plan.plan_id.clone(),
            expected_plan_hash: svc_voxel_conversion::plan_hash(&plan),
            expected_preview_hash: Some(preview.output_hash),
        })
        .unwrap();
    assert!(receipt.applied);

    let model_info = bridge
        .read_voxel_model_info(VoxelModelInfoRequest {
            grid: 7,
            volume_asset_id: Some("voxel/generated".to_string()),
            include_material_counts: true,
        })
        .unwrap();
    assert!(model_info.resident);

    let save = bridge
        .save_voxel_volume_asset(VoxelVolumeAssetSaveRequest {
            export_request: VoxelVolumeAssetExportRequest {
                grid: 7,
                volume_asset_id: Some("voxel/generated".to_string()),
                target_asset_id: "voxel-volume/missing-material".to_string(),
                label: None,
                created_by: None,
                source_tool: None,
                max_sparse_runs: 16,
                expected_session_hash: Some(model_info.session_hash),
            },
            target_project_bundle: "asha-demo".to_string(),
            target_asset_path: "assets/voxels/missing-material.avxl.json".to_string(),
            representation_kind: "sparse_runs".to_string(),
            expected_existing_canonical_json_hash: None,
            expected_canonical_json_hash: None,
            expected_voxel_data_hash: None,
        })
        .unwrap();
    assert!(!save.saved);
    assert!(save.diff.is_none());
    assert_eq!(
        save.diagnostics[0].code,
        protocol_voxel_asset::VoxelAssetDiagnosticCode::InvalidMaterialReference
    );
}

#[test]
fn voxel_conversion_registers_studio_static_mesh_source_before_plan() {
    let mut bridge = init_bridge();
    let registration_request = studio_registered_source_request();
    let registration = bridge
        .register_voxel_conversion_source(registration_request.clone())
        .unwrap();
    assert!(registration.registered);
    assert!(registration.diagnostics.is_empty());
    assert_eq!(
        registration.source.asset_id,
        "mesh/studio-registered-triangle"
    );
    assert_eq!(registration.source.asset_version, 3);
    assert_eq!(registration.material_slots[0].source_material_slot, 4);
    assert_eq!(
        registration.material_slots[0].source_material_id.as_deref(),
        Some("material/studio-copper")
    );
    assert_eq!(
        registration.evidence[0].kind,
        protocol_voxel_conversion::VoxelConversionEvidenceKind::SourceSnapshot
    );

    let plan = bridge
        .plan_voxel_conversion(registered_source_plan_request(&registration_request))
        .unwrap();
    assert!(plan.diagnostics.is_empty());
    assert_eq!(plan.source.asset_id, "mesh/studio-registered-triangle");
    assert_eq!(
        plan.expected_source_hash,
        "sha256:studio-registered-triangle"
    );
    assert_eq!(
        plan.settings.material_map.entries[0].source_material_slot,
        4
    );
}

#[test]
fn voxel_conversion_registers_project_mesh_asset_before_plan() {
    let mut bridge = init_bridge();
    let registration_request = project_mesh_asset_registration_request();
    let registration = bridge
        .register_voxel_conversion_mesh_asset(registration_request.clone())
        .unwrap();
    assert!(registration.registered);
    assert!(registration.diagnostics.is_empty());
    assert_eq!(registration.source.asset_id, "mesh/project-quad");
    assert_eq!(registration.source.asset_version, 5);
    assert_eq!(registration.material_slots[0].source_material_slot, 2);
    assert_eq!(
        registration.material_slots[0].source_material_id.as_deref(),
        Some("material/project-brick")
    );
    assert_eq!(
        registration.evidence[0].kind,
        protocol_voxel_conversion::VoxelConversionEvidenceKind::SourceSnapshot
    );

    let plan = bridge
        .plan_voxel_conversion(project_mesh_asset_plan_request(&registration_request))
        .unwrap();
    assert!(plan.diagnostics.is_empty());
    assert_eq!(plan.source.asset_id, "mesh/project-quad");
    assert_eq!(plan.expected_source_hash, "sha256:project-quad");
    assert_eq!(
        plan.settings.material_map.entries[0].source_material_slot,
        2
    );

    let metadata = bridge
        .read_voxel_conversion_source_metadata(
            protocol_voxel_conversion::VoxelConversionSourceMetadataRequest {
                source: registration_request.source.clone(),
            },
        )
        .unwrap();
    assert!(metadata.registered);
    assert_eq!(
        metadata.source.as_ref().unwrap(),
        &registration_request.source
    );
    assert_eq!(
        metadata.source_path.as_deref(),
        Some("assets/meshes/project-quad.mesh.json")
    );
    assert_eq!(metadata.vertex_count, 4);
    assert_eq!(metadata.triangle_count, 2);
    assert_eq!(metadata.groups.len(), 1);
    assert_eq!(metadata.groups[0].material_slot, 2);
    assert_eq!(metadata.groups[0].start, 0);
    assert_eq!(metadata.groups[0].count, 6);
    assert_eq!(metadata.groups[0].bounds.unwrap().max, [1.0, 1.0, 0.0]);
    assert_eq!(
        metadata.material_slots[0].source_material_id.as_deref(),
        Some("material/project-brick")
    );
    assert_eq!(
        metadata.latest_plan_id.as_deref(),
        Some(plan.plan_id.as_str())
    );
    assert_eq!(
        metadata.latest_plan_transform,
        Some(plan.settings.transform)
    );
    assert!(metadata.diagnostics.is_empty());
    assert_eq!(
        metadata.evidence[0].kind,
        protocol_voxel_conversion::VoxelConversionEvidenceKind::SourceSnapshot
    );
}

#[test]
fn voxel_conversion_source_metadata_fails_closed_for_unknown_or_stale_source() {
    let mut bridge = init_bridge();
    let request = project_mesh_asset_registration_request();
    let missing = bridge
        .read_voxel_conversion_source_metadata(
            protocol_voxel_conversion::VoxelConversionSourceMetadataRequest {
                source: request.source.clone(),
            },
        )
        .unwrap();
    assert!(!missing.registered);
    assert_eq!(
        missing.diagnostics[0].code,
        VoxelConversionDiagnosticCode::VoxelConversionUnavailable
    );

    bridge
        .register_voxel_conversion_mesh_asset(request.clone())
        .unwrap();
    let mut stale_source = request.source;
    stale_source.source_hash = "sha256:stale".to_string();
    let stale = bridge
        .read_voxel_conversion_source_metadata(
            protocol_voxel_conversion::VoxelConversionSourceMetadataRequest {
                source: stale_source,
            },
        )
        .unwrap();
    assert!(!stale.registered);
    assert_eq!(
        stale.diagnostics[0].code,
        VoxelConversionDiagnosticCode::VoxelConversionUnavailable
    );
}

#[test]
fn voxel_conversion_larger_registered_source_applies_and_reports_model_info() {
    let mut bridge = init_bridge();
    let registration_request = larger_registered_grid_source_request();
    let registration = bridge
        .register_voxel_conversion_source(registration_request.clone())
        .unwrap();
    assert!(registration.registered);
    assert_eq!(registration_request.positions.len(), 9);
    assert_eq!(registration_request.triangles.len(), 8);

    let plan = bridge
        .plan_voxel_conversion(larger_registered_grid_plan_request(&registration_request))
        .unwrap();
    assert!(plan.diagnostics.is_empty());
    assert_eq!(plan.estimated_output_voxels, 9);
    assert_eq!(plan.estimated_bounds.unwrap().max.x, 2);
    assert_eq!(plan.estimated_bounds.unwrap().max.y, 2);

    let preview = bridge
        .preview_voxel_conversion(VoxelConversionPreviewRequest {
            plan_id: plan.plan_id.clone(),
            expected_plan_hash: svc_voxel_conversion::plan_hash(&plan),
        })
        .unwrap();
    assert!(preview.diagnostics.is_empty());
    assert_eq!(preview.output_voxel_count, 9);

    let receipt = bridge
        .apply_voxel_conversion(VoxelConversionApplyRequest {
            plan_id: plan.plan_id.clone(),
            expected_plan_hash: svc_voxel_conversion::plan_hash(&plan),
            expected_preview_hash: Some(preview.output_hash),
        })
        .unwrap();
    assert!(receipt.applied);
    assert_eq!(receipt.output_voxel_count, 9);

    let model_info = bridge
        .read_voxel_model_info(VoxelModelInfoRequest {
            grid: 7,
            volume_asset_id: Some("voxel/generated".to_string()),
            include_material_counts: true,
        })
        .unwrap();
    assert!(model_info.resident);
    assert_eq!(model_info.voxel_count, 9);
    assert_eq!(
        model_info.material_counts,
        vec![VoxelModelMaterialCount {
            material: 3,
            voxel_count: 9
        }]
    );
    assert_eq!(
        model_info.source.as_ref().unwrap().asset_id,
        "mesh/registered-grid-3x3"
    );
    assert_eq!(
        model_info.latest_plan_id.as_deref(),
        Some(plan.plan_id.as_str())
    );
    assert!(model_info.latest_output_hash.is_some());
    assert!(model_info.session_hash.starts_with("fnv1a64:"));
    assert!(model_info.replay_hash.starts_with("fnv1a64:"));
    assert!(model_info.diagnostics.is_empty());

    let exported = bridge
        .export_voxel_volume_asset(VoxelVolumeAssetExportRequest {
            grid: 7,
            volume_asset_id: Some("voxel/generated".to_string()),
            target_asset_id: "voxel-volume/generated-grid".to_string(),
            label: Some("Generated grid".to_string()),
            created_by: Some("runtime-bridge-api-test".to_string()),
            source_tool: Some("svc-voxel-conversion".to_string()),
            max_sparse_runs: 16,
            expected_session_hash: Some(model_info.session_hash),
        })
        .unwrap();
    assert!(exported.exported);
    let asset = exported.asset.expect("exported larger asset");
    assert_eq!(asset.bounds.max.x, 2);
    assert_eq!(asset.bounds.max.y, 2);
    assert_eq!(asset.representation.sparse_runs.len(), 3);
    assert_eq!(
        asset
            .representation
            .sparse_runs
            .iter()
            .map(|run| run.length)
            .collect::<Vec<_>>(),
        vec![3, 3, 3]
    );
    assert!(svc_voxel_asset::decode_asset(
        exported
            .canonical_json
            .as_ref()
            .expect("larger canonical json")
    )
    .is_ok());

    let limited = bridge
        .export_voxel_volume_asset(VoxelVolumeAssetExportRequest {
            grid: 7,
            volume_asset_id: Some("voxel/generated".to_string()),
            target_asset_id: "voxel-volume/too-large".to_string(),
            label: None,
            created_by: None,
            source_tool: None,
            max_sparse_runs: 2,
            expected_session_hash: None,
        })
        .unwrap();
    assert!(!limited.exported);
    assert_eq!(
        limited.diagnostics[0].code,
        protocol_voxel_asset::VoxelAssetDiagnosticCode::ExportLimitExceeded
    );
}

#[test]
fn voxel_conversion_project_mesh_asset_registration_rejects_invalid_assets() {
    let mut bridge = init_bridge();

    let mut missing_geometry = project_mesh_asset_registration_request();
    missing_geometry.mesh_asset.positions = Vec::new();
    let rejected_missing = bridge
        .register_voxel_conversion_mesh_asset(missing_geometry)
        .unwrap();
    assert!(!rejected_missing.registered);
    assert!(rejected_missing.evidence.is_empty());
    assert_eq!(
        rejected_missing.diagnostics[0].code,
        VoxelConversionDiagnosticCode::UnsupportedSourceAsset
    );

    let mut unsupported_primitive = project_mesh_asset_registration_request();
    unsupported_primitive.source.mesh_primitive = Some("lod1".to_string());
    let rejected_primitive = bridge
        .register_voxel_conversion_mesh_asset(unsupported_primitive)
        .unwrap();
    assert!(!rejected_primitive.registered);
    assert_eq!(
        rejected_primitive.diagnostics[0].code,
        VoxelConversionDiagnosticCode::UnsupportedSourceAsset
    );

    let mut material_slot_mismatch = project_mesh_asset_registration_request();
    material_slot_mismatch.mesh_asset.groups[0].material_slot = 99;
    let rejected_material = bridge
        .register_voxel_conversion_mesh_asset(material_slot_mismatch)
        .unwrap();
    assert!(!rejected_material.registered);
    assert_eq!(
        rejected_material.diagnostics[0].code,
        VoxelConversionDiagnosticCode::UnsupportedSourceAsset
    );
}

#[test]
fn voxel_conversion_project_mesh_asset_stale_source_hash_fails_closed() {
    let mut bridge = init_bridge();
    let registration_request = project_mesh_asset_registration_request();
    let registration = bridge
        .register_voxel_conversion_mesh_asset(registration_request.clone())
        .unwrap();
    assert!(registration.registered);

    let mut plan_request = project_mesh_asset_plan_request(&registration_request);
    plan_request.source.source_hash = "sha256:stale-project-quad".to_string();
    let plan = bridge.plan_voxel_conversion(plan_request).unwrap();
    assert_eq!(
        plan.diagnostics[0].code,
        VoxelConversionDiagnosticCode::SourceHashMismatch
    );
}

#[test]
fn voxel_conversion_source_registration_missing_geometry_fails_closed() {
    let mut bridge = init_bridge();
    let mut registration_request = studio_registered_source_request();
    registration_request.positions = Vec::new();
    let registration = bridge
        .register_voxel_conversion_source(registration_request.clone())
        .unwrap();
    assert!(!registration.registered);
    assert!(registration.evidence.is_empty());
    assert_eq!(
        registration.diagnostics[0].code,
        VoxelConversionDiagnosticCode::UnsupportedSourceAsset
    );

    let plan = bridge
        .plan_voxel_conversion(registered_source_plan_request(&registration_request))
        .unwrap();
    assert_eq!(
        plan.diagnostics[0].code,
        VoxelConversionDiagnosticCode::UnsupportedSourceAsset
    );
}

#[test]
fn voxel_conversion_stale_source_hash_fails_closed() {
    let mut bridge = init_bridge();
    let mut request = project_voxel_conversion_request(7);
    request.source.source_hash = "sha256:stale".to_string();
    let plan = bridge.plan_voxel_conversion(request).unwrap();
    assert_eq!(
        plan.diagnostics[0].code,
        VoxelConversionDiagnosticCode::SourceHashMismatch
    );
}

#[test]
fn voxel_conversion_unsupported_source_fails_closed() {
    let mut bridge = init_bridge();
    let mut request = project_voxel_conversion_request(7);
    request.source.asset_id = "mesh/not-loaded".to_string();
    request.source.source_hash = "sha256:not-loaded".to_string();
    let plan = bridge.plan_voxel_conversion(request).unwrap();
    assert_eq!(
        plan.diagnostics[0].code,
        VoxelConversionDiagnosticCode::UnsupportedSourceAsset
    );
}

#[test]
fn voxel_conversion_apply_to_unregistered_target_returns_diagnostic_receipt() {
    let mut bridge = init_bridge();
    let plan = bridge
        .plan_voxel_conversion(project_voxel_conversion_request(999))
        .unwrap();
    let preview = bridge
        .preview_voxel_conversion(VoxelConversionPreviewRequest {
            plan_id: plan.plan_id.clone(),
            expected_plan_hash: svc_voxel_conversion::plan_hash(&plan),
        })
        .unwrap();
    let receipt = bridge
        .apply_voxel_conversion(VoxelConversionApplyRequest {
            plan_id: plan.plan_id.clone(),
            expected_plan_hash: svc_voxel_conversion::plan_hash(&plan),
            expected_preview_hash: Some(preview.output_hash),
        })
        .unwrap();
    assert!(!receipt.applied);
    assert_eq!(
        receipt.diagnostics[0].code,
        VoxelConversionDiagnosticCode::ConversionReplayMismatch
    );
}

#[test]
fn voxel_model_info_missing_target_fails_closed_with_diagnostic_readout() {
    let bridge = init_bridge();
    let readout = bridge
        .read_voxel_model_info(VoxelModelInfoRequest {
            grid: 999,
            volume_asset_id: Some("voxel/missing".to_string()),
            include_material_counts: true,
        })
        .unwrap();
    assert!(!readout.resident);
    assert_eq!(readout.voxel_count, 0);
    assert!(readout.material_counts.is_empty());
    assert_eq!(
        readout.diagnostics[0].code,
        VoxelConversionDiagnosticCode::VoxelConversionUnavailable
    );
    assert!(readout.session_hash.starts_with("fnv1a64:"));
    assert!(readout.replay_hash.starts_with("fnv1a64:"));
}
