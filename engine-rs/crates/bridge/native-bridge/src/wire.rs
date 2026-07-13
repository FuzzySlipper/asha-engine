use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::value::Value;

use crate::{generated::native_wire_limits, to_napi};
use runtime_bridge_api::{RuntimeBridgeError, RuntimeBridgeErrorKind};

const DEFAULT_MAX_WIRE_BYTES: usize = 8 * 1024 * 1024;

fn wire_error(operation: &str, path: &str, message: impl Into<String>) -> napi::Error {
    to_napi(
        RuntimeBridgeError::new(RuntimeBridgeErrorKind::InvalidInput, message)
            .at_path(path)
            .with_detail(format!("operation={operation}")),
    )
}

fn reject_unknown_fields(
    operation: &str,
    input: &Value,
    canonical: &Value,
    path: &str,
) -> napi::Result<()> {
    match (input, canonical) {
        (Value::Object(input_fields), Value::Object(canonical_fields)) => {
            for (field, value) in input_fields {
                let field_path = format!("{path}.{field}");
                let Some(canonical_value) = canonical_fields.get(field) else {
                    return Err(wire_error(operation, &field_path, "unknown field"));
                };
                reject_unknown_fields(operation, value, canonical_value, &field_path)?;
            }
        }
        (Value::Array(input_items), Value::Array(canonical_items)) => {
            for (index, (value, canonical_value)) in
                input_items.iter().zip(canonical_items.iter()).enumerate()
            {
                reject_unknown_fields(
                    operation,
                    value,
                    canonical_value,
                    &format!("{path}[{index}]"),
                )?;
            }
        }
        _ => {}
    }
    Ok(())
}

pub(crate) fn parse_wire_json<T>(operation: &str, payload: &str) -> napi::Result<T>
where
    T: DeserializeOwned + Serialize,
{
    let max_bytes = native_wire_limits(operation)
        .map(|limits| limits.0)
        .unwrap_or(DEFAULT_MAX_WIRE_BYTES);
    if payload.len() > max_bytes {
        return Err(wire_error(
            operation,
            "$",
            format!(
                "request has {} bytes; operation limit is {max_bytes}",
                payload.len()
            ),
        ));
    }
    let input: Value = serde_json::from_str(payload).map_err(|error| {
        wire_error(
            operation,
            "$",
            format!("request is not valid JSON: {error}"),
        )
    })?;
    let decoded: T = serde_json::from_value(input.clone()).map_err(|error| {
        wire_error(
            operation,
            "$",
            format!("request does not match its wire contract: {error}"),
        )
    })?;
    let canonical = serde_json::to_value(&decoded).map_err(|error| {
        to_napi(RuntimeBridgeError::new(
            RuntimeBridgeErrorKind::Internal,
            format!("failed to verify decoded {operation} request: {error}"),
        ))
    })?;
    reject_unknown_fields(operation, &input, &canonical, "$")?;
    Ok(decoded)
}

#[cfg(test)]
mod tests {
    use super::*;
    use runtime_bridge_api::TimeControlCommand;

    #[test]
    fn rejects_unknown_fields_and_unknown_tagged_union_variants() {
        let extra = parse_wire_json::<TimeControlCommand>(
            "apply_time_control_command",
            r#"{"operation":"pause","extra":true}"#,
        )
        .unwrap_err();
        assert!(extra.reason.contains("unknown field"));

        let variant = parse_wire_json::<TimeControlCommand>(
            "apply_time_control_command",
            r#"{"operation":"rewind"}"#,
        )
        .unwrap_err();
        assert!(variant.reason.contains("wire contract"));
    }

    #[test]
    fn rejects_oversized_payload_before_decoding() {
        let payload = "x".repeat(8 * 1024 * 1024 + 1);
        let error = parse_wire_json::<TimeControlCommand>("apply_time_control_command", &payload)
            .unwrap_err();
        assert!(error.reason.contains("operation limit"));
    }
}
