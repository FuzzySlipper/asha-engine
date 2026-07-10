use runtime_bridge_api::VoxelEditHistorySummary;

use super::{initialize_engine, read_voxel_edit_history, submit_commands};

#[test]
fn native_voxel_command_resource_limits_precede_session_lookup_and_preserve_history() {
    let Err(byte_error) = submit_commands(
        -99,
        " ".repeat(runtime_bridge_api::VOXEL_COMMAND_BATCH_MAX_REQUEST_BYTES + 1),
    ) else {
        panic!("byte limit must reject before unknown handle lookup");
    };
    assert!(byte_error.reason.contains("request byte limit"));

    let command =
        r#"{"op":"setVoxel","grid":1,"coord":{"x":0,"y":0,"z":0},"value":{"kind":"empty"}}"#;
    let command_over_limit = format!(
        "[{}]",
        vec![command; runtime_bridge_api::VOXEL_COMMAND_BATCH_MAX_COMMANDS + 1].join(",")
    );
    let Err(command_error) = submit_commands(-99, command_over_limit) else {
        panic!("command limit must reject before unknown handle lookup");
    };
    assert!(command_error.reason.contains("command limit"));

    let touched_over_limit = r#"[{"op":"fillRegion","grid":1,"min":{"x":0,"y":0,"z":0},"max":{"x":1000001,"y":1,"z":1},"value":{"kind":"empty"}}]"#;
    let Err(touched_error) = submit_commands(-99, touched_over_limit.to_string()) else {
        panic!("expanded work must reject before unknown handle lookup");
    };
    assert!(touched_error
        .reason
        .contains("expanded touched-voxel limit"));

    let handle = initialize_engine(88).expect("engine initializes");
    let history_before = read_history(handle);
    assert!(submit_commands(handle, touched_over_limit.to_string()).is_err());
    let history_after = read_history(handle);
    assert_eq!(history_after, history_before);
}

fn read_history(handle: i64) -> VoxelEditHistorySummary {
    let history_json = read_voxel_edit_history(
        handle,
        r#"{"historyId":"history/default","cursorId":null,"maxEntries":8,"includeRedoTail":true,"expectedHistoryHash":null}"#
            .to_string(),
    )
    .expect("voxel edit history reads");
    serde_json::from_str(&history_json).expect("voxel edit history is valid JSON")
}
