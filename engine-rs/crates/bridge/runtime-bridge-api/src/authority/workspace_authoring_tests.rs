use super::*;

use core_space::VoxelCoord;
use core_voxel::VoxelValue;

fn open_request(workspace_id: &str) -> WorkspaceAuthoringOpenRequest {
    WorkspaceAuthoringOpenRequest {
        authoring_id: "workspace-authoring.rust-authority-test".to_owned(),
        seed: 29,
        project: WorkspaceAuthoringProjectIdentity {
            game_id: "authoring-consumer".to_owned(),
            workspace_id: workspace_id.to_owned(),
        },
        project_bundle: WorkspaceAuthoringProjectBundleRef {
            bundle_schema_version: 1,
            protocol_version: 1,
            scene_id: 42,
        },
    }
}

fn initialize_volume(bridge: &mut EngineBridge) {
    let stored_fixture = tests::hand_authored_voxel_volume_asset();
    let receipt = bridge
        .initialize_voxel_volume_authoring(VoxelVolumeAuthoringInitializeRequest {
            grid: 2,
            volume_asset_id: Some("voxel-volume/workspace-authoring".to_owned()),
            seed_chunk: VoxelAssetCoord { x: 0, y: 0, z: 0 },
            material_palette: stored_fixture.material_palette,
            authoring: stored_fixture.authoring,
            max_material_bindings: 8,
        })
        .unwrap();
    assert!(receipt.initialized, "{:?}", receipt.diagnostics);
}

fn projection_request(
    workspace_id: &str,
    generation: u64,
    working_revision: u64,
    cursor: u64,
) -> WorkspaceAuthoringProjectionRequest {
    WorkspaceAuthoringProjectionRequest {
        expected_workspace_id: workspace_id.to_owned(),
        expected_generation: generation,
        expected_working_revision: working_revision,
        cursor,
    }
}

#[test]
fn authoring_cell_is_distinct_from_gameplay_runtime_and_owns_revisions() {
    let mut bridge = EngineBridge::new();
    let opened = bridge
        .open_workspace_authoring(open_request("workspace.local"))
        .unwrap();

    assert_eq!(opened.status, "open");
    assert_eq!(opened.identity.generation, 1);
    assert!(bridge.bundle.engine.is_none());
    assert_eq!(bridge.time.authority_tick, 0);
    assert_eq!(
        bridge
            .step_simulation(StepInputEnvelope { tick: 1 })
            .unwrap_err()
            .kind,
        RuntimeBridgeErrorKind::NotInitialized
    );

    initialize_volume(&mut bridge);
    let after_initialize = bridge.read_workspace_authoring_state().unwrap();
    assert_eq!(after_initialize.working_revision, 1);
    assert_eq!(after_initialize.stored_revision, 0);
    assert!(after_initialize.dirty);

    let edit = bridge
        .submit_commands(CommandBatch {
            commands: vec![VoxelCommand::SetVoxel {
                grid: GridId::new(2),
                coord: VoxelCoord::new(0, 0, 0),
                value: VoxelValue::solid_raw(1),
            }],
        })
        .unwrap();
    assert_eq!(edit.accepted, 1);
    assert_eq!(
        bridge
            .read_workspace_authoring_state()
            .unwrap()
            .working_revision,
        2
    );

    let rejected_close = bridge
        .close_workspace_authoring(WorkspaceAuthoringCloseRequest {
            expected_workspace_id: "workspace.local".to_owned(),
            expected_generation: 1,
            discard_unsaved_working_state: false,
        })
        .unwrap_err();
    assert_eq!(rejected_close.kind, RuntimeBridgeErrorKind::InvalidInput);
    let closed = bridge
        .close_workspace_authoring(WorkspaceAuthoringCloseRequest {
            expected_workspace_id: "workspace.local".to_owned(),
            expected_generation: 1,
            discard_unsaved_working_state: true,
        })
        .unwrap();
    assert!(closed.closed);

    let reopened = bridge
        .open_workspace_authoring(open_request("workspace.local"))
        .unwrap();
    assert_eq!(reopened.identity.generation, 2);
    assert_eq!(reopened.working_revision, 0);
}

#[test]
fn projection_rejects_foreign_stale_and_future_bindings_before_drain() {
    let mut bridge = EngineBridge::new();
    let opened = bridge
        .open_workspace_authoring(open_request("workspace.local"))
        .unwrap();
    initialize_volume(&mut bridge);

    let first_request = projection_request("workspace.local", opened.identity.generation, 1, 0);
    let first = bridge
        .read_workspace_authoring_projection(first_request.clone())
        .unwrap();
    assert_eq!(first.delivery, "replace");
    assert_eq!(first.next_cursor, 1);
    assert_eq!(
        bridge
            .read_workspace_authoring_projection(first_request)
            .unwrap(),
        first,
        "an exact retry returns the cached receipt"
    );

    let edit = bridge
        .submit_commands(CommandBatch {
            commands: vec![VoxelCommand::SetVoxel {
                grid: GridId::new(2),
                coord: VoxelCoord::new(0, 0, 0),
                value: VoxelValue::solid_raw(1),
            }],
        })
        .unwrap();
    assert_eq!(edit.accepted, 1);

    for invalid in [
        projection_request("workspace.foreign", opened.identity.generation, 2, 1),
        projection_request("workspace.local", opened.identity.generation + 1, 2, 1),
        projection_request("workspace.local", opened.identity.generation, 1, 1),
        projection_request("workspace.local", opened.identity.generation, 2, 99),
    ] {
        assert_eq!(
            bridge
                .read_workspace_authoring_projection(invalid)
                .unwrap_err()
                .kind,
            RuntimeBridgeErrorKind::StaleAuthoritySnapshot
        );
    }

    let current_request = projection_request("workspace.local", opened.identity.generation, 2, 1);
    let current = bridge
        .read_workspace_authoring_projection(current_request.clone())
        .unwrap();
    assert_eq!(current.cursor, 1);
    assert_eq!(current.next_cursor, 2);
    assert!(
        current.frame_json.contains("replaceMeshPayload"),
        "rejected reads must not drain the pending edited geometry"
    );
    assert_eq!(
        bridge
            .read_workspace_authoring_projection(current_request)
            .unwrap(),
        current,
        "an accepted cursor retry remains idempotent"
    );
}

#[test]
fn stored_confirmation_consumes_only_the_current_rust_save_candidate() {
    let mut bridge = EngineBridge::new();
    let opened = bridge
        .open_workspace_authoring(open_request("workspace.local"))
        .unwrap();
    initialize_volume(&mut bridge);
    bridge
        .submit_commands(CommandBatch {
            commands: vec![VoxelCommand::SetVoxel {
                grid: GridId::new(2),
                coord: VoxelCoord::new(0, 0, 0),
                value: VoxelValue::solid_raw(1),
            }],
        })
        .unwrap();

    let save_candidate = |bridge: &mut EngineBridge| {
        bridge
            .save_voxel_volume_asset(VoxelVolumeAssetSaveRequest {
                export_request: VoxelVolumeAssetExportRequest {
                    grid: 2,
                    volume_asset_id: Some("voxel-volume/workspace-authoring".to_owned()),
                    target_asset_id: "voxel-volume/workspace-confirmation".to_owned(),
                    label: Some("Workspace confirmation".to_owned()),
                    created_by: Some("runtime-bridge-api-test".to_owned()),
                    source_tool: Some("workspace-authoring".to_owned()),
                    max_sparse_runs: 16,
                    expected_session_hash: None,
                },
                target_project_bundle: "authoring-consumer".to_owned(),
                target_asset_path: "assets/voxels/workspace-confirmation.avxl.json".to_owned(),
                representation_kind: "sparse_runs".to_owned(),
                expected_existing_canonical_json_hash: None,
                expected_canonical_json_hash: None,
                expected_voxel_data_hash: None,
            })
            .unwrap()
            .canonical_json_hash
            .expect("accepted save candidate has a canonical hash")
    };

    let first_hash = save_candidate(&mut bridge);
    bridge
        .submit_commands(CommandBatch {
            commands: vec![VoxelCommand::SetVoxel {
                grid: GridId::new(2),
                coord: VoxelCoord::new(1, 0, 0),
                value: VoxelValue::solid_raw(1),
            }],
        })
        .unwrap();
    let stale = bridge
        .confirm_workspace_authoring_stored(WorkspaceAuthoringStoredConfirmationRequest {
            expected_workspace_id: "workspace.local".to_owned(),
            expected_generation: opened.identity.generation,
            host_path: "/tmp/workspace-confirmation.avxl.json".to_owned(),
            canonical_json_hash: first_hash,
        })
        .unwrap_err();
    assert_eq!(stale.kind, RuntimeBridgeErrorKind::InvalidInput);

    let current_hash = save_candidate(&mut bridge);
    let wrong_hash = bridge
        .confirm_workspace_authoring_stored(WorkspaceAuthoringStoredConfirmationRequest {
            expected_workspace_id: "workspace.local".to_owned(),
            expected_generation: opened.identity.generation,
            host_path: "/tmp/workspace-confirmation.avxl.json".to_owned(),
            canonical_json_hash: "fnv1a64:0000000000000000".to_owned(),
        })
        .unwrap_err();
    assert_eq!(
        wrong_hash.kind,
        RuntimeBridgeErrorKind::StaleAuthoritySnapshot
    );

    let request = WorkspaceAuthoringStoredConfirmationRequest {
        expected_workspace_id: "workspace.local".to_owned(),
        expected_generation: opened.identity.generation,
        host_path: "/tmp/workspace-confirmation.avxl.json".to_owned(),
        canonical_json_hash: current_hash,
    };
    let accepted = bridge
        .confirm_workspace_authoring_stored(request.clone())
        .unwrap();
    assert!(accepted.accepted);
    assert_eq!(accepted.stored_revision, 3);
    assert!(!bridge.read_workspace_authoring_state().unwrap().dirty);

    let replayed = bridge
        .confirm_workspace_authoring_stored(request)
        .unwrap_err();
    assert_eq!(replayed.kind, RuntimeBridgeErrorKind::InvalidInput);
    assert!(!bridge.read_workspace_authoring_state().unwrap().dirty);
}
