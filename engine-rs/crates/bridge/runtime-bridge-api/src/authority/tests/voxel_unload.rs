use super::*;

#[test]
fn voxel_volume_asset_unload_is_hash_guarded_and_preserves_unrelated_models() {
    let asset = hand_authored_voxel_volume_asset();
    let mut second_asset = asset.clone();
    second_asset.asset_id = "voxel-volume/second".to_string();
    second_asset.bounds.min.x = 4;
    second_asset.bounds.max.x = 5;
    second_asset.representation.sparse_runs[0].start.x = 4;
    second_asset = svc_voxel_asset::with_computed_hashes(&second_asset);

    let mut bridge = init_bridge();
    let first = bridge
        .load_voxel_volume_asset(VoxelVolumeAssetLoadRequest {
            asset: asset.clone(),
            target_grid: 7,
            target_volume_asset_id: Some(asset.asset_id.clone()),
            replace_existing: true,
            include_material_counts: false,
        })
        .unwrap();
    assert!(first.loaded);
    let second = bridge
        .load_voxel_volume_asset(VoxelVolumeAssetLoadRequest {
            asset: second_asset.clone(),
            target_grid: 7,
            target_volume_asset_id: Some(second_asset.asset_id.clone()),
            replace_existing: false,
            include_material_counts: false,
        })
        .unwrap();
    assert!(second.loaded);

    let missing = bridge
        .unload_voxel_volume_asset(VoxelVolumeAssetUnloadRequest {
            grid: 7,
            volume_asset_id: Some("voxel-volume/missing".to_string()),
            expected_session_hash: first.session_hash.clone(),
        })
        .unwrap();
    assert!(!missing.unloaded);
    assert_eq!(
        missing.diagnostics[0].code,
        protocol_voxel_asset::VoxelAssetDiagnosticCode::RuntimeModelUnavailable
    );

    let stale = bridge
        .unload_voxel_volume_asset(VoxelVolumeAssetUnloadRequest {
            grid: 7,
            volume_asset_id: Some(asset.asset_id.clone()),
            expected_session_hash: "fnv1a64:stale".to_string(),
        })
        .unwrap();
    assert!(!stale.unloaded);
    assert_eq!(
        stale.diagnostics[0].code,
        protocol_voxel_asset::VoxelAssetDiagnosticCode::StaleRuntimeSnapshot
    );

    let drift = bridge
        .submit_commands(CommandBatch {
            commands: vec![VoxelCommand::SetVoxel {
                grid: GridId::new(7),
                coord: VoxelCoord::new(0, 0, 0),
                value: VoxelValue::EMPTY,
            }],
        })
        .unwrap();
    assert_eq!(drift.accepted, 1);
    let drifted = bridge
        .unload_voxel_volume_asset(VoxelVolumeAssetUnloadRequest {
            grid: 7,
            volume_asset_id: Some(asset.asset_id.clone()),
            expected_session_hash: first.session_hash.clone(),
        })
        .unwrap();
    assert!(!drifted.unloaded);
    assert_eq!(
        drifted.diagnostics[0].code,
        protocol_voxel_asset::VoxelAssetDiagnosticCode::StaleRuntimeSnapshot
    );
    let restored = bridge
        .submit_commands(CommandBatch {
            commands: vec![VoxelCommand::SetVoxel {
                grid: GridId::new(7),
                coord: VoxelCoord::new(0, 0, 0),
                value: VoxelValue::solid_raw(1),
            }],
        })
        .unwrap();
    assert_eq!(restored.accepted, 1);

    let unloaded = bridge
        .unload_voxel_volume_asset(VoxelVolumeAssetUnloadRequest {
            grid: 7,
            volume_asset_id: Some(asset.asset_id.clone()),
            expected_session_hash: first.session_hash,
        })
        .unwrap();
    assert!(unloaded.unloaded);
    assert_eq!(unloaded.removed_voxel_count, 2);
    assert!(unloaded.diagnostics.is_empty());

    let first_info = bridge
        .read_voxel_model_info(VoxelModelInfoRequest {
            grid: 7,
            volume_asset_id: Some(asset.asset_id.clone()),
            include_material_counts: false,
        })
        .unwrap();
    assert!(!first_info.resident);
    let second_info = bridge
        .read_voxel_model_info(VoxelModelInfoRequest {
            grid: 7,
            volume_asset_id: Some(second_asset.asset_id.clone()),
            include_material_counts: false,
        })
        .unwrap();
    assert!(second_info.resident);
    assert_eq!(
        EngineBridge::voxel_value_at(bridge.voxel.as_ref().unwrap(), VoxelCoord::new(4, 0, 0)),
        VoxelValue::solid_raw(1)
    );

    let reloaded = bridge
        .load_voxel_volume_asset(VoxelVolumeAssetLoadRequest {
            asset,
            target_grid: 7,
            target_volume_asset_id: Some("voxel-volume/hand-authored".to_string()),
            replace_existing: false,
            include_material_counts: false,
        })
        .unwrap();
    assert!(reloaded.loaded);
}

#[test]
fn voxel_volume_asset_unload_restores_disjoint_same_identity_footprints() {
    let first_asset = hand_authored_voxel_volume_asset();
    let mut moved_asset = first_asset.clone();
    moved_asset.bounds.min.x = 4;
    moved_asset.bounds.max.x = 5;
    moved_asset.representation.sparse_runs[0].start.x = 4;
    moved_asset = svc_voxel_asset::with_computed_hashes(&moved_asset);

    let mut bridge = init_bridge();
    let first = bridge
        .load_voxel_volume_asset(VoxelVolumeAssetLoadRequest {
            asset: first_asset.clone(),
            target_grid: 7,
            target_volume_asset_id: Some(first_asset.asset_id.clone()),
            replace_existing: true,
            include_material_counts: false,
        })
        .unwrap();
    assert!(first.loaded);
    let moved = bridge
        .load_voxel_volume_asset(VoxelVolumeAssetLoadRequest {
            asset: moved_asset.clone(),
            target_grid: 7,
            target_volume_asset_id: Some(first_asset.asset_id.clone()),
            replace_existing: false,
            include_material_counts: false,
        })
        .unwrap();
    assert!(moved.loaded);
    assert_ne!(first.session_hash, moved.session_hash);
    let cumulative_session_hash = moved.session_hash.clone();

    for x in [0, 1, 4, 5] {
        assert_eq!(
            EngineBridge::voxel_value_at(bridge.voxel.as_ref().unwrap(), VoxelCoord::new(x, 0, 0)),
            VoxelValue::solid_raw(1)
        );
    }

    let unloaded = bridge
        .unload_voxel_volume_asset(VoxelVolumeAssetUnloadRequest {
            grid: 7,
            volume_asset_id: Some(first_asset.asset_id.clone()),
            expected_session_hash: moved.session_hash,
        })
        .unwrap();
    assert!(unloaded.unloaded);
    assert_eq!(unloaded.removed_voxel_count, 4);
    for x in [0, 1, 4, 5] {
        assert_eq!(
            EngineBridge::voxel_value_at(bridge.voxel.as_ref().unwrap(), VoxelCoord::new(x, 0, 0)),
            VoxelValue::EMPTY
        );
    }

    bridge
        .load_voxel_volume_asset(VoxelVolumeAssetLoadRequest {
            asset: first_asset.clone(),
            target_grid: 7,
            target_volume_asset_id: Some(first_asset.asset_id.clone()),
            replace_existing: true,
            include_material_counts: false,
        })
        .unwrap();
    let replaced = bridge
        .load_voxel_volume_asset(VoxelVolumeAssetLoadRequest {
            asset: moved_asset,
            target_grid: 7,
            target_volume_asset_id: Some(first_asset.asset_id.clone()),
            replace_existing: true,
            include_material_counts: false,
        })
        .unwrap();
    assert_ne!(cumulative_session_hash, replaced.session_hash);
    let stale_cumulative = bridge
        .unload_voxel_volume_asset(VoxelVolumeAssetUnloadRequest {
            grid: 7,
            volume_asset_id: Some(first_asset.asset_id.clone()),
            expected_session_hash: cumulative_session_hash,
        })
        .unwrap();
    assert!(!stale_cumulative.unloaded);
    assert_eq!(
        stale_cumulative.diagnostics[0].code,
        protocol_voxel_asset::VoxelAssetDiagnosticCode::StaleRuntimeSnapshot
    );
    let replaced_unload = bridge
        .unload_voxel_volume_asset(VoxelVolumeAssetUnloadRequest {
            grid: 7,
            volume_asset_id: Some(first_asset.asset_id),
            expected_session_hash: replaced.session_hash,
        })
        .unwrap();
    assert!(replaced_unload.unloaded);
    assert_eq!(replaced_unload.removed_voxel_count, 2);
}

#[test]
fn voxel_volume_asset_session_hash_captures_unload_restoration_state() {
    let asset = hand_authored_voxel_volume_asset();
    let load_request = |replace_existing| VoxelVolumeAssetLoadRequest {
        asset: asset.clone(),
        target_grid: 7,
        target_volume_asset_id: Some(asset.asset_id.clone()),
        replace_existing,
        include_material_counts: false,
    };

    let mut empty_prior_bridge = init_bridge();
    let empty_prior = empty_prior_bridge
        .load_voxel_volume_asset(load_request(true))
        .unwrap();

    let mut solid_prior_bridge = init_bridge();
    let mut grid_seed_asset = asset.clone();
    grid_seed_asset.asset_id = "voxel-volume/grid-seed".to_string();
    grid_seed_asset.bounds.min.x = 10;
    grid_seed_asset.bounds.max.x = 11;
    grid_seed_asset.representation.sparse_runs[0].start.x = 10;
    grid_seed_asset = svc_voxel_asset::with_computed_hashes(&grid_seed_asset);
    let grid_seed = solid_prior_bridge
        .load_voxel_volume_asset(VoxelVolumeAssetLoadRequest {
            asset: grid_seed_asset.clone(),
            target_grid: 7,
            target_volume_asset_id: Some(grid_seed_asset.asset_id.clone()),
            replace_existing: true,
            include_material_counts: false,
        })
        .unwrap();
    let grid_seed_unload = solid_prior_bridge
        .unload_voxel_volume_asset(VoxelVolumeAssetUnloadRequest {
            grid: 7,
            volume_asset_id: Some(grid_seed_asset.asset_id),
            expected_session_hash: grid_seed.session_hash,
        })
        .unwrap();
    assert!(grid_seed_unload.unloaded);
    let target = solid_prior_bridge
        .voxel_asset_load_target(&load_request(false))
        .unwrap();
    EngineBridge::ensure_candidate_chunks_for_asset(
        &asset,
        &target.spec,
        solid_prior_bridge.voxel.as_mut().unwrap(),
    );
    solid_prior_bridge.reset_voxel_edit_history(solid_prior_bridge.voxel.as_ref().unwrap().clone());
    let seeded = solid_prior_bridge
        .submit_commands(CommandBatch {
            commands: vec![
                VoxelCommand::SetVoxel {
                    grid: GridId::new(7),
                    coord: VoxelCoord::new(0, 0, 0),
                    value: VoxelValue::solid_raw(2),
                },
                VoxelCommand::SetVoxel {
                    grid: GridId::new(7),
                    coord: VoxelCoord::new(1, 0, 0),
                    value: VoxelValue::solid_raw(2),
                },
            ],
        })
        .unwrap();
    assert_eq!(seeded.accepted, 2);
    for x in [0, 1] {
        assert_eq!(
            EngineBridge::voxel_value_at(
                solid_prior_bridge.voxel.as_ref().unwrap(),
                VoxelCoord::new(x, 0, 0),
            ),
            VoxelValue::solid_raw(2)
        );
    }
    let solid_prior = solid_prior_bridge
        .load_voxel_volume_asset(load_request(false))
        .unwrap();
    let solid_prior_info = solid_prior_bridge
        .voxel_model_infos
        .get(&EngineBridge::voxel_model_key(
            7,
            &Some(asset.asset_id.clone()),
        ))
        .unwrap();
    assert_eq!(
        solid_prior_info.prior_voxels.get(&VoxelCoord::new(0, 0, 0)),
        Some(&VoxelValue::solid_raw(2))
    );

    assert_eq!(empty_prior.voxel_count, solid_prior.voxel_count);
    assert_eq!(empty_prior.voxel_data_hash, solid_prior.voxel_data_hash);
    assert_ne!(empty_prior.session_hash, solid_prior.session_hash);
    assert_ne!(empty_prior.replay_hash, solid_prior.replay_hash);

    let unloaded = solid_prior_bridge
        .unload_voxel_volume_asset(VoxelVolumeAssetUnloadRequest {
            grid: 7,
            volume_asset_id: Some(asset.asset_id),
            expected_session_hash: solid_prior.session_hash,
        })
        .unwrap();
    assert!(unloaded.unloaded);
    for x in [0, 1] {
        assert_eq!(
            EngineBridge::voxel_value_at(
                solid_prior_bridge.voxel.as_ref().unwrap(),
                VoxelCoord::new(x, 0, 0),
            ),
            VoxelValue::solid_raw(2)
        );
    }
}
