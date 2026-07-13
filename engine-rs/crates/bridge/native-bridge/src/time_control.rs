use napi_derive::napi;
use runtime_bridge_api::{
    RuntimeBridge, RuntimeBridgeError, RuntimeBridgeErrorKind, TimeControlCommand,
};
use serde::Serialize;

use crate::{to_napi, wire::parse_wire_json, with_bridge};

fn serialize_result<T: Serialize>(value: &T, operation: &str) -> napi::Result<String> {
    serde_json::to_string(value).map_err(|err| {
        to_napi(RuntimeBridgeError::new(
            RuntimeBridgeErrorKind::Internal,
            format!("{operation} result could not be serialized: {err}"),
        ))
    })
}

#[napi]
pub fn apply_time_control_command(handle: i64, command_json: String) -> napi::Result<String> {
    let command =
        parse_wire_json::<TimeControlCommand>("apply_time_control_command", &command_json)?;
    with_bridge(handle, |bridge| {
        let receipt = bridge
            .apply_time_control_command(command)
            .map_err(to_napi)?;
        serialize_result(&receipt, "apply time control command")
    })
}

#[napi]
pub fn read_time_control_state(handle: i64) -> napi::Result<String> {
    with_bridge(handle, |bridge| {
        let state = bridge.read_time_control_state().map_err(to_napi)?;
        serialize_result(&state, "read time control state")
    })
}
