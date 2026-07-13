use core_space::{ChunkCoord, Direction6, VoxelCoord};
use napi_derive::napi;
use protocol_view::{
    CameraHandle, ScreenPoint, ScreenPointSpace, ScreenPointToPickRayRequest,
    VoxelSelectionOutcome, ViewportSize,
};
use runtime_bridge_api::{
    PickRay, PickResult, RuntimeBridge, RuntimeBridgeError, RuntimeBridgeErrorKind,
    VoxelMeshEvidenceRequest,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{to_napi, with_bridge};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PickRayJson {
    grid: u64,
    origin: [f64; 3],
    direction: [f64; 3],
    max_distance: f64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SelectVoxelJson {
    camera: u64,
    grid: u64,
    viewport: Option<ViewportJson>,
    screen_point: ScreenPointJson,
    max_distance: f64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ViewportJson {
    width: u32,
    height: u32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ScreenPointJson {
    x: f32,
    y: f32,
    space: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct VoxelMeshEvidenceJson {
    grid: u64,
    chunks: Vec<CoordJson>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CoordJson {
    x: i64,
    y: i64,
    z: i64,
}

fn parse<T: serde::de::DeserializeOwned>(text: &str, operation: &str) -> napi::Result<T> {
    serde_json::from_str(text).map_err(|error| {
        to_napi(RuntimeBridgeError::new(
            RuntimeBridgeErrorKind::InvalidInput,
            format!("{operation} request is not valid JSON: {error}"),
        ))
    })
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

#[napi]
pub fn pick_voxel(handle: i64, request_json: String) -> napi::Result<String> {
    let request = parse::<PickRayJson>(&request_json, "voxel pick")?;
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
pub fn select_voxel(handle: i64, request_json: String) -> napi::Result<String> {
    let request = parse::<SelectVoxelJson>(&request_json, "voxel selection")?;
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
    let request = parse::<VoxelMeshEvidenceJson>(&request_json, "voxel mesh evidence")?;
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
