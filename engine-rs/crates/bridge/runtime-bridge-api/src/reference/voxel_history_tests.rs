use super::*;

fn init_bridge() -> ReferenceBridge {
    let mut bridge = ReferenceBridge::new();
    bridge.initialize_engine(EngineConfig { seed: 1 }).unwrap();
    bridge
}

fn read_request() -> VoxelEditHistoryReadRequest {
    VoxelEditHistoryReadRequest {
        history_id: "history/default".to_string(),
        cursor_id: None,
        max_entries: 12,
        include_redo_tail: true,
        expected_history_hash: None,
    }
}

fn set_voxel(coord: VoxelCoord, material: u16) -> VoxelCommand {
    VoxelCommand::SetVoxel {
        grid: GridId::new(1),
        coord,
        value: VoxelValue::solid_raw(material),
    }
}

#[test]
fn voxel_edit_history_fails_closed_before_engine_initialization() {
    let bridge = ReferenceBridge::new();
    let error = bridge.read_voxel_edit_history(read_request()).unwrap_err();
    assert_eq!(error.kind, RuntimeBridgeErrorKind::NotInitialized);
}

#[test]
fn accepted_submit_commands_populates_rust_owned_default_history() {
    let mut bridge = init_bridge();
    let empty = bridge.read_voxel_edit_history(read_request()).unwrap();
    assert_eq!(empty.history_id, "history/default");
    assert!(empty.entries.is_empty());
    assert_eq!(empty.cursor.undo_depth, 0);

    let result = bridge
        .submit_commands(CommandBatch {
            commands: vec![set_voxel(VoxelCoord::new(1, 1, 1), 2)],
        })
        .unwrap();
    assert_eq!(result.accepted, 1);
    assert_eq!(result.rejected, 0);

    let summary = bridge.read_voxel_edit_history(read_request()).unwrap();
    assert_eq!(summary.entries.len(), 1);
    assert_eq!(summary.cursor.undo_depth, 1);
    assert_eq!(summary.cursor.redo_depth, 0);
    assert_eq!(
        summary.cursor.voxel_state_hash,
        summary.entries[0].after_voxel_hash
    );
    assert_eq!(summary.entries[0].operation_label, "submit_commands");
    assert_eq!(summary.entries[0].command_count, 1);
    assert_eq!(summary.entries[0].event_count, 1);
    assert!(summary.entries[0].receipt_hash.starts_with("fnv1a64:"));
    assert!(summary.entries[0].command_hash.starts_with("fnv1a64:"));
}

#[test]
fn rejected_commands_do_not_invent_history_entries_or_undo_depth() {
    let mut bridge = init_bridge();
    let result = bridge
        .submit_commands(CommandBatch {
            commands: vec![set_voxel(VoxelCoord::new(1, 1, 1), 99)],
        })
        .unwrap();
    assert_eq!(result.accepted, 0);
    assert_eq!(result.rejected, 1);

    let summary = bridge.read_voxel_edit_history(read_request()).unwrap();
    assert!(summary.entries.is_empty());
    assert_eq!(summary.cursor.undo_depth, 0);
    let error = bridge
        .undo_voxel_edit(VoxelEditHistoryUndoRequest {
            history_id: summary.history_id,
            expected_history_hash: summary.history_hash.clone(),
            expected_cursor_hash: summary.cursor.history_hash,
            max_replay_steps: 16,
            max_diff_voxels: 32,
        })
        .unwrap_err();
    assert_eq!(error.kind, RuntimeBridgeErrorKind::InvalidInput);
    assert!(error.message.contains("EmptyUndoStack"));
}

#[test]
fn mixed_batches_preserve_partial_acceptance_and_record_one_accepted_transaction() {
    let mut bridge = init_bridge();
    let result = bridge
        .submit_commands(CommandBatch {
            commands: vec![
                set_voxel(VoxelCoord::new(1, 1, 1), 2),
                set_voxel(VoxelCoord::new(0, 0, 0), 99),
            ],
        })
        .unwrap();
    assert_eq!(result.accepted, 1);
    assert_eq!(result.rejected, 1);

    let summary = bridge.read_voxel_edit_history(read_request()).unwrap();
    assert_eq!(summary.entries.len(), 1);
    assert_eq!(summary.entries[0].command_count, 1);
    assert_eq!(summary.cursor.undo_depth, 1);
}

#[test]
fn preview_then_apply_revert_uses_rust_replay_without_preview_mutation() {
    let mut bridge = init_bridge();
    let base_hash = rule_voxel_edit::voxel_world_hash(bridge.voxel.as_ref().unwrap());
    bridge
        .submit_commands(CommandBatch {
            commands: vec![set_voxel(VoxelCoord::new(1, 1, 1), 2)],
        })
        .unwrap();
    let edited = bridge.read_voxel_edit_history(read_request()).unwrap();
    let edited_hash = rule_voxel_edit::voxel_world_hash(bridge.voxel.as_ref().unwrap());
    let request = VoxelEditHistoryRevertRequest {
        history_id: edited.history_id.clone(),
        mode: protocol_voxel_edit_history::VoxelEditHistoryRevertMode::PreviewRevert,
        target: protocol_voxel_edit_history::VoxelEditHistoryRevertTarget {
            transaction_id: None,
            cursor_id: None,
            cursor_index: Some(0),
        },
        expected_history_hash: edited.history_hash.clone(),
        expected_cursor_hash: edited.cursor.history_hash.clone(),
        max_replay_steps: 16,
        max_diff_voxels: 32,
        include_sample_window: true,
    };

    let preview = bridge.preview_voxel_edit_revert(request.clone()).unwrap();
    assert!(preview.preview);
    assert!(!preview.applied);
    assert_eq!(
        preview.diff_summary.as_ref().unwrap().changed_voxel_count,
        1
    );
    assert_eq!(
        rule_voxel_edit::voxel_world_hash(bridge.voxel.as_ref().unwrap()),
        edited_hash,
        "preview must not mutate live authority"
    );

    let applied = bridge
        .apply_voxel_edit_revert(VoxelEditHistoryRevertRequest {
            mode: protocol_voxel_edit_history::VoxelEditHistoryRevertMode::ApplyRevert,
            ..request
        })
        .unwrap();
    assert!(applied.applied);
    assert!(!applied.preview);
    assert_eq!(
        rule_voxel_edit::voxel_world_hash(bridge.voxel.as_ref().unwrap()),
        base_hash
    );
}

#[test]
fn undo_and_redo_move_the_rust_history_cursor_and_live_voxel_state_together() {
    let mut bridge = init_bridge();
    let base_hash = rule_voxel_edit::voxel_world_hash(bridge.voxel.as_ref().unwrap());
    bridge
        .submit_commands(CommandBatch {
            commands: vec![set_voxel(VoxelCoord::new(1, 1, 1), 2)],
        })
        .unwrap();
    let edited = bridge.read_voxel_edit_history(read_request()).unwrap();
    let edited_hash = rule_voxel_edit::voxel_world_hash(bridge.voxel.as_ref().unwrap());
    assert_ne!(edited_hash, base_hash);

    let undo = bridge
        .undo_voxel_edit(VoxelEditHistoryUndoRequest {
            history_id: edited.history_id.clone(),
            expected_history_hash: edited.history_hash.clone(),
            expected_cursor_hash: edited.cursor.history_hash.clone(),
            max_replay_steps: 16,
            max_diff_voxels: 32,
        })
        .unwrap();
    assert!(undo.receipt.applied);
    assert_eq!(
        rule_voxel_edit::voxel_world_hash(bridge.voxel.as_ref().unwrap()),
        base_hash
    );
    let undone = bridge.read_voxel_edit_history(read_request()).unwrap();
    assert_eq!(undone.cursor.undo_depth, 0);
    assert_eq!(undone.cursor.redo_depth, 1);

    let redo = bridge
        .redo_voxel_edit(VoxelEditHistoryRedoRequest {
            history_id: undone.history_id,
            expected_history_hash: undone.history_hash.clone(),
            expected_cursor_hash: undone.cursor.history_hash,
            max_replay_steps: 16,
            max_diff_voxels: 32,
        })
        .unwrap();
    assert!(redo.receipt.applied);
    assert_eq!(
        rule_voxel_edit::voxel_world_hash(bridge.voxel.as_ref().unwrap()),
        edited_hash
    );
}

#[test]
fn stale_or_unknown_history_requests_fail_closed_without_mutation() {
    let mut bridge = init_bridge();
    bridge
        .submit_commands(CommandBatch {
            commands: vec![set_voxel(VoxelCoord::new(1, 1, 1), 2)],
        })
        .unwrap();
    let before_hash = rule_voxel_edit::voxel_world_hash(bridge.voxel.as_ref().unwrap());

    let mut unknown = read_request();
    unknown.history_id = "history/unknown".to_string();
    assert_eq!(
        bridge.read_voxel_edit_history(unknown).unwrap_err().kind,
        RuntimeBridgeErrorKind::InvalidInput
    );

    let mut stale = read_request();
    stale.expected_history_hash = Some("fnv1a64:stale".to_string());
    assert_eq!(
        bridge.read_voxel_edit_history(stale).unwrap_err().kind,
        RuntimeBridgeErrorKind::InvalidInput
    );
    assert_eq!(
        rule_voxel_edit::voxel_world_hash(bridge.voxel.as_ref().unwrap()),
        before_hash
    );
}
