use napi_derive::napi;
use runtime_bridge_api::{
    GeneratedTunnelPreset, GeneratedTunnelRuntimeApplyRequest,
    GeneratedTunnelRuntimeApplyReceipt, RuntimeBridge,
};

use crate::{to_napi, with_bridge};

#[napi(object)]
pub struct NativeGeneratedTunnelRuntimeApplyReceipt {
    pub preset_id: String,
    pub seed: i64,
    pub grid: i64,
    pub config_hash: String,
    pub output_hash: String,
    pub collision_source_hash: String,
    pub collision_projection_hash: String,
}

impl From<GeneratedTunnelRuntimeApplyReceipt> for NativeGeneratedTunnelRuntimeApplyReceipt {
    fn from(value: GeneratedTunnelRuntimeApplyReceipt) -> Self {
        Self {
            preset_id: match value.preset {
                GeneratedTunnelPreset::TinyEnclosed => "tiny-enclosed".to_string(),
            },
            seed: value.seed as i64,
            grid: value.grid as i64,
            config_hash: value.config_hash,
            output_hash: value.output_hash,
            collision_source_hash: value.collision_source_hash,
            collision_projection_hash: value.collision_projection_hash,
        }
    }
}

#[napi]
pub fn apply_generated_tunnel_to_runtime_world(
    handle: i64,
    preset_id: String,
    seed: i64,
) -> napi::Result<NativeGeneratedTunnelRuntimeApplyReceipt> {
    let preset = match preset_id.as_str() {
        "tiny-enclosed" => GeneratedTunnelPreset::TinyEnclosed,
        _ => return Err(napi::Error::from_reason("unsupported generated tunnel preset")),
    };
    let seed = u64::try_from(seed)
        .map_err(|_| napi::Error::from_reason("generated tunnel seed must be non-negative"))?;
    with_bridge(handle, |bridge| {
        bridge
            .apply_generated_tunnel_to_runtime_world(GeneratedTunnelRuntimeApplyRequest {
                preset,
                seed,
            })
            .map(NativeGeneratedTunnelRuntimeApplyReceipt::from)
            .map_err(to_napi)
    })
}
