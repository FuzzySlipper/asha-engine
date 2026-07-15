use napi_derive::napi;
use runtime_bridge_api::RuntimeBridge;

use crate::{to_napi, u64_input, with_bridge};

#[napi]
pub fn read_render_diffs(handle: i64, cursor: i64) -> napi::Result<String> {
    let cursor = u64_input(cursor, "cursor")?;
    with_bridge(handle, |bridge| {
        bridge
            .read_render_diffs(cursor)
            .map(|frame| render_bridge::json::encode_frame(&frame))
            .map_err(to_napi)
    })
}
