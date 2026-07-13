use napi_derive::napi;
use runtime_bridge_api::{
    InputContextCommand, InputSessionConfigureRequest, RawInputSample, RecordedInputAction,
    RuntimeBridge, RuntimeBridgeError, RuntimeBridgeErrorKind,
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
pub fn configure_input_session(handle: i64, request_json: String) -> napi::Result<String> {
    let request =
        parse_wire_json::<InputSessionConfigureRequest>("configure_input_session", &request_json)?;
    with_bridge(handle, |bridge| {
        let snapshot = bridge.configure_input_session(request).map_err(to_napi)?;
        serialize_result(&snapshot, "configure input session")
    })
}

#[napi]
pub fn apply_input_context_command(handle: i64, command_json: String) -> napi::Result<String> {
    let command =
        parse_wire_json::<InputContextCommand>("apply_input_context_command", &command_json)?;
    with_bridge(handle, |bridge| {
        let receipt = bridge
            .apply_input_context_command(command)
            .map_err(to_napi)?;
        serialize_result(&receipt, "apply input context command")
    })
}

#[napi]
pub fn submit_raw_input(handle: i64, sample_json: String) -> napi::Result<String> {
    let sample = parse_wire_json::<RawInputSample>("submit_raw_input", &sample_json)?;
    with_bridge(handle, |bridge| {
        let receipt = bridge.submit_raw_input(sample).map_err(to_napi)?;
        serialize_result(&receipt, "submit raw input")
    })
}

#[napi]
pub fn replay_resolved_input_action(handle: i64, record_json: String) -> napi::Result<String> {
    let record =
        parse_wire_json::<RecordedInputAction>("replay_resolved_input_action", &record_json)?;
    with_bridge(handle, |bridge| {
        let receipt = bridge
            .replay_resolved_input_action(record)
            .map_err(to_napi)?;
        serialize_result(&receipt, "replay resolved input action")
    })
}

#[napi]
pub fn read_input_context_state(handle: i64) -> napi::Result<String> {
    with_bridge(handle, |bridge| {
        let state = bridge.read_input_context_state().map_err(to_napi)?;
        serialize_result(&state, "read input context state")
    })
}
