use core_space::{ChunkCoord, Direction6, VoxelCoord};
use napi_derive::napi;
use protocol_view::{
    CameraHandle, ScreenPoint, ScreenPointSpace, ScreenPointToPickRayRequest, ViewportSize,
    VoxelSelectionOutcome,
};
use runtime_bridge_api::{
    PickRay, PickResult, RuntimeBridge, RuntimeBridgeError, RuntimeBridgeErrorKind,
    SceneTransformDto, VoxelInstancePickHint, VoxelInstancePickOutcome, VoxelInstancePickRejection,
    VoxelInstancePickRequest, VoxelMeshEvidenceRequest, VoxelProjectionBindingRequest,
    VoxelProjectionInstanceBinding, VoxelUpdateTelemetryRequest,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::{to_napi, wire::parse_wire_json, with_bridge};

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PickRayJson {
    grid: u64,
    origin: [f64; 3],
    direction: [f64; 3],
    max_distance: f64,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SelectVoxelJson {
    camera: u64,
    grid: u64,
    viewport: Option<ViewportJson>,
    screen_point: ScreenPointJson,
    max_distance: f64,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ViewportJson {
    width: u32,
    height: u32,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ScreenPointJson {
    x: f32,
    y: f32,
    space: String,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct VoxelMeshEvidenceJson {
    grid: u64,
    chunks: Vec<CoordJson>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct CoordJson {
    x: i64,
    y: i64,
    z: i64,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ProjectionBindingJson {
    workspace_id: String,
    workspace_generation: u64,
    working_revision: u64,
    registry_digest: String,
    instances: Vec<ProjectionInstanceJson>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ProjectionInstanceJson {
    instance_id: String,
    scene_node_id: u64,
    asset_id: String,
    transform: TransformJson,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct TransformJson {
    translation: [f32; 3],
    rotation: [f32; 4],
    scale: [f32; 3],
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct InstancePickJson {
    workspace_id: String,
    workspace_generation: u64,
    working_revision: u64,
    registry_digest: String,
    binding_hash: String,
    instance_id: String,
    origin: [f64; 3],
    direction: [f64; 3],
    max_distance: f64,
    renderer_hint: InstancePickHintJson,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct InstancePickHintJson {
    local_voxel: CoordJson,
    local_face: String,
}

fn encode(value: Value, operation: &str) -> napi::Result<String> {
    serde_json::to_string(&value).map_err(|error| {
        to_napi(RuntimeBridgeError::new(
            RuntimeBridgeErrorKind::Internal,
            format!("{operation} result could not be serialized: {error}"),
        ))
    })
}

fn coord(value: VoxelCoord) -> Value {
    json!({ "x": value.x, "y": value.y, "z": value.z })
}

fn chunk(value: ChunkCoord) -> Value {
    json!({ "x": value.x, "y": value.y, "z": value.z })
}

fn face(value: Direction6) -> &'static str {
    match value {
        Direction6::PosX => "posX",
        Direction6::NegX => "negX",
        Direction6::PosY => "posY",
        Direction6::NegY => "negY",
        Direction6::PosZ => "posZ",
        Direction6::NegZ => "negZ",
    }
}

fn parse_face(value: &str) -> napi::Result<Direction6> {
    match value {
        "posX" => Ok(Direction6::PosX),
        "negX" => Ok(Direction6::NegX),
        "posY" => Ok(Direction6::PosY),
        "negY" => Ok(Direction6::NegY),
        "posZ" => Ok(Direction6::PosZ),
        "negZ" => Ok(Direction6::NegZ),
        _ => Err(to_napi(RuntimeBridgeError::new(
            RuntimeBridgeErrorKind::InvalidInput,
            format!("unknown voxel instance pick face {value:?}"),
        ))),
    }
}

fn rejection(value: VoxelInstancePickRejection) -> &'static str {
    match value {
        VoxelInstancePickRejection::StaleWorkspaceGeneration => "staleWorkspaceGeneration",
        VoxelInstancePickRejection::StaleWorkingRevision => "staleWorkingRevision",
        VoxelInstancePickRejection::RegistryDigestChanged => "registryDigestChanged",
        VoxelInstancePickRejection::BindingHashMismatch => "bindingHashMismatch",
        VoxelInstancePickRejection::UnknownInstance => "unknownInstance",
        VoxelInstancePickRejection::InvalidRay => "invalidRay",
        VoxelInstancePickRejection::NoHit => "noHit",
        VoxelInstancePickRejection::RendererHintMismatch => "rendererHintMismatch",
    }
}

#[napi]
pub fn pick_voxel(handle: i64, request_json: String) -> napi::Result<String> {
    let request = parse_wire_json::<PickRayJson>("pick_voxel", &request_json)?;
    with_bridge(handle, |bridge| {
        let result = bridge
            .pick_voxel(PickRay {
                grid: request.grid,
                origin: request.origin,
                direction: request.direction,
                max_distance: request.max_distance,
            })
            .map_err(to_napi)?;
        let value = match result {
            PickResult::Hit(hit) => json!({
                "outcome": "hit",
                "hit": {
                    "grid": hit.grid,
                    "voxel": coord(hit.voxel),
                    "chunk": chunk(hit.chunk),
                    "face": face(hit.face),
                    "point": hit.point,
                    "distance": hit.distance,
                }
            }),
            PickResult::Miss(_) => json!({
                "outcome": "miss",
                "rejection": { "reason": "noHit" }
            }),
        };
        encode(value, "voxel pick")
    })
}

#[napi]
pub fn configure_voxel_projection_instances(
    handle: i64,
    request_json: String,
) -> napi::Result<String> {
    let request = parse_wire_json::<ProjectionBindingJson>(
        "configure_voxel_projection_instances",
        &request_json,
    )?;
    with_bridge(handle, |bridge| {
        let receipt = bridge
            .configure_voxel_projection_instances(VoxelProjectionBindingRequest {
                workspace_id: request.workspace_id,
                workspace_generation: request.workspace_generation,
                working_revision: request.working_revision,
                registry_digest: request.registry_digest,
                instances: request
                    .instances
                    .into_iter()
                    .map(|instance| VoxelProjectionInstanceBinding {
                        instance_id: instance.instance_id,
                        scene_node_id: instance.scene_node_id,
                        asset_id: instance.asset_id,
                        transform: SceneTransformDto {
                            translation: instance.transform.translation,
                            rotation: instance.transform.rotation,
                            scale: instance.transform.scale,
                        },
                    })
                    .collect(),
            })
            .map_err(to_napi)?;
        encode(
            json!({
                "workspaceId": receipt.workspace_id,
                "workspaceGeneration": receipt.workspace_generation,
                "workingRevision": receipt.working_revision,
                "registryDigest": receipt.registry_digest,
                "bindingHash": receipt.binding_hash,
                "instanceCount": receipt.instance_count,
                "projectionOpCount": receipt.projection_op_count,
            }),
            "voxel projection binding",
        )
    })
}

#[napi]
pub fn pick_voxel_instance(handle: i64, request_json: String) -> napi::Result<String> {
    let request = parse_wire_json::<InstancePickJson>("pick_voxel_instance", &request_json)?;
    let local_face = parse_face(&request.renderer_hint.local_face)?;
    with_bridge(handle, |bridge| {
        let result = bridge
            .pick_voxel_instance(VoxelInstancePickRequest {
                workspace_id: request.workspace_id,
                workspace_generation: request.workspace_generation,
                working_revision: request.working_revision,
                registry_digest: request.registry_digest,
                binding_hash: request.binding_hash,
                instance_id: request.instance_id,
                origin: request.origin,
                direction: request.direction,
                max_distance: request.max_distance,
                renderer_hint: VoxelInstancePickHint {
                    local_voxel: VoxelCoord::new(
                        request.renderer_hint.local_voxel.x,
                        request.renderer_hint.local_voxel.y,
                        request.renderer_hint.local_voxel.z,
                    ),
                    local_face,
                },
            })
            .map_err(to_napi)?;
        let outcome = match result.outcome {
            VoxelInstancePickOutcome::Hit(hit) => json!({
                "outcome": "hit",
                "voxelInstancePickHit": {
                    "localVoxel": coord(hit.local_voxel),
                    "localChunk": chunk(hit.local_chunk),
                    "localFace": face(hit.local_face),
                    "localPlaceAnchor": coord(hit.local_place_anchor),
                    "worldPoint": hit.world_point,
                    "worldDistance": hit.world_distance,
                }
            }),
            VoxelInstancePickOutcome::Rejected(reason) => json!({
                "outcome": "rejected",
                "rejection": rejection(reason),
            }),
        };
        encode(
            json!({
                "workspaceId": result.workspace_id,
                "workspaceGeneration": result.workspace_generation,
                "workingRevision": result.working_revision,
                "bindingHash": result.binding_hash,
                "instanceId": result.instance_id,
                "outcome": outcome,
            }),
            "voxel instance pick",
        )
    })
}

#[napi]
pub fn select_voxel(handle: i64, request_json: String) -> napi::Result<String> {
    let request = parse_wire_json::<SelectVoxelJson>("select_voxel", &request_json)?;
    let space = match request.screen_point.space.as_str() {
        "normalized_0_1" => ScreenPointSpace::Normalized01,
        "pixel" => ScreenPointSpace::Pixel,
        other => {
            return Err(to_napi(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                format!("voxel selection screenPoint.space {other:?} is not supported"),
            )))
        }
    };
    with_bridge(handle, |bridge| {
        let snapshot = bridge
            .select_voxel(ScreenPointToPickRayRequest {
                camera: CameraHandle::new(request.camera),
                grid: request.grid,
                viewport: request.viewport.map(|viewport| ViewportSize {
                    width: viewport.width,
                    height: viewport.height,
                }),
                screen_point: ScreenPoint {
                    x: request.screen_point.x,
                    y: request.screen_point.y,
                    space,
                },
                max_distance: request.max_distance,
            })
            .map_err(to_napi)?;
        let pick_ray = &snapshot.pick_ray;
        encode(
            json!({
                "pickRay": {
                    "camera": pick_ray.camera.raw(),
                    "tick": pick_ray.tick,
                    "grid": pick_ray.grid,
                    "screenPoint": {
                        "x": pick_ray.screen_point.x,
                        "y": pick_ray.screen_point.y,
                        "space": match pick_ray.screen_point.space {
                            ScreenPointSpace::Normalized01 => "normalized_0_1",
                            ScreenPointSpace::Pixel => "pixel",
                        },
                    },
                    "origin": pick_ray.origin,
                    "direction": pick_ray.direction,
                    "maxDistance": pick_ray.max_distance,
                    "cameraProjectionHash": pick_ray.camera_projection_hash,
                    "rayHash": pick_ray.ray_hash,
                },
                "outcome": match snapshot.outcome {
                    VoxelSelectionOutcome::Hit => "hit",
                    VoxelSelectionOutcome::Miss => "miss",
                },
                "selectedVoxel": snapshot.selected_voxel.map(coord),
                "selectedFace": snapshot.selected_face.map(face),
                "editAnchor": snapshot.edit_anchor.map(coord),
                "selectionHash": snapshot.selection_hash,
            }),
            "voxel selection",
        )
    })
}

#[napi]
pub fn read_voxel_mesh_evidence(handle: i64, request_json: String) -> napi::Result<String> {
    let request =
        parse_wire_json::<VoxelMeshEvidenceJson>("read_voxel_mesh_evidence", &request_json)?;
    with_bridge(handle, |bridge| {
        let snapshot = bridge
            .read_voxel_mesh_evidence(VoxelMeshEvidenceRequest {
                grid: request.grid,
                chunks: request
                    .chunks
                    .into_iter()
                    .map(|value| ChunkCoord::new(value.x, value.y, value.z))
                    .collect(),
            })
            .map_err(to_napi)?;
        let chunks = snapshot
            .chunks
            .into_iter()
            .map(|item| {
                json!({
                    "coord": chunk(item.coord),
                    "resident": item.resident,
                    "visible": item.visible,
                    "contentHash": item.content_hash,
                    "meshHash": item.mesh_hash,
                    "stats": item.stats.map(|stats| json!({
                        "vertices": stats.vertices,
                        "indices": stats.indices,
                        "quads": stats.quads,
                        "facesEmitted": stats.faces_emitted,
                        "facesCulled": stats.faces_culled,
                    })),
                    "bounds": item.bounds.map(|bounds| json!({
                        "min": bounds.min,
                        "max": bounds.max,
                    })),
                    "materialSlots": item.material_slots,
                })
            })
            .collect::<Vec<_>>();
        encode(
            json!({
                "grid": snapshot.grid,
                "fixtureId": snapshot.fixture_id,
                "voxelStateHash": snapshot.voxel_state_hash,
                "meshingStrategy": snapshot.meshing_strategy,
                "chunks": chunks,
                "diagnostics": snapshot.diagnostics,
            }),
            "voxel mesh evidence",
        )
    })
}

#[napi]
pub fn read_voxel_update_telemetry(handle: i64, request_json: String) -> napi::Result<String> {
    let request =
        parse_wire_json::<VoxelUpdateTelemetryRequest>("read_voxel_update_telemetry", &request_json)?;
    with_bridge(handle, |bridge| {
        let readout = bridge.read_voxel_update_telemetry(request).map_err(to_napi)?;
        let value = serde_json::to_value(readout).map_err(|error| {
            to_napi(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::Internal,
                format!("voxel update telemetry could not be encoded: {error}"),
            ))
        })?;
        encode(value, "voxel update telemetry")
    })
}
