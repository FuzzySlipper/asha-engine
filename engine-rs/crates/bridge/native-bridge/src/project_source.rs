use napi::bindgen_prelude::Buffer;

use super::*;

#[napi(object)]
pub struct NativeProjectResourceTransaction {
    pub generation: i64,
    pub manifest_hash: String,
}

#[napi(object)]
pub struct NativeStagedProjectResource {
    pub handle: i64,
    pub generation: i64,
    pub version: u32,
    pub byte_len: i64,
}

/// Start a manifest-bound project resource transaction. The manifest is strict
/// decoded and validated before any binary bytes can be staged.
#[napi]
pub fn begin_runtime_project_source_resources(
    handle: i64,
    request_json: String,
) -> napi::Result<NativeProjectResourceTransaction> {
    let request = wire::parse_wire_json::<runtime_bridge_api::ProjectResourceBeginRequest>(
        "begin_runtime_project_source_resources",
        &request_json,
    )?;
    with_bridge(handle, |bridge| {
        let transaction = bridge
            .begin_runtime_project_source_resources(&request.manifest_json)
            .map_err(to_napi)?;
        Ok(NativeProjectResourceTransaction {
            generation: transaction.generation() as i64,
            manifest_hash: transaction.manifest_hash().to_hex(),
        })
    })
}

/// Stage one large/binary body through a real Node Buffer. The bytes never
/// enter JSON or base64; only the returned opaque handle enters the source batch.
#[napi]
pub fn stage_runtime_project_source_resource(
    handle: i64,
    generation: i64,
    path: String,
    bytes: Buffer,
) -> napi::Result<NativeStagedProjectResource> {
    let generation = u64_input(generation, "generation")?;
    with_bridge(handle, |bridge| {
        let staged = bridge
            .stage_runtime_project_source_resource_generation(generation, &path, bytes.to_vec())
            .map_err(to_napi)?;
        Ok(NativeStagedProjectResource {
            handle: staged.handle.raw() as i64,
            generation: staged.generation as i64,
            version: staged.version,
            byte_len: staged.byte_len as i64,
        })
    })
}

/// Validate the compact manifest/body-index JSON after binary bodies have been
/// replaced by opaque handles.
#[napi]
pub fn admit_runtime_project_source_batch(
    handle: i64,
    request_json: String,
) -> napi::Result<String> {
    let request = wire::parse_wire_json::<runtime_bridge_api::RuntimeProjectSourceBatch>(
        "admit_runtime_project_source_batch",
        &request_json,
    )?;
    with_bridge(handle, |bridge| {
        let receipt = bridge
            .admit_runtime_project_source_batch(request)
            .map_err(to_napi)?;
        serde_json::to_string(&receipt).map_err(|error| {
            to_napi(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::Internal,
                format!("failed to serialize project source receipt: {error}"),
            ))
        })
    })
}
