use core_ids::{SceneId, SceneNodeId};
use napi_derive::napi;
use protocol_voxel_asset::{VoxelAssetAuthoringMetadata, VoxelAssetMaterialBinding};
use runtime_bridge_api::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::{scene_preview::scene_document_json, to_napi, wire::parse_wire_json, with_bridge};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LimitsJson {
    max_voxels: u64,
    max_sparse_runs: u64,
    max_markers: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MarkerTargetJson {
    source_marker_id: String,
    node_id: u64,
    marker_id: String,
    child_order: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct TransformJson {
    translation: [f32; 3],
    rotation: [f32; 4],
    scale: [f32; 3],
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TargetJson {
    scene_id: u64,
    scene_path: String,
    asset_id: String,
    asset_path: String,
    voxel_node_id: u64,
    voxel_parent_id: Option<u64>,
    voxel_child_order: u32,
    voxel_label: Option<String>,
    voxel_transform: TransformJson,
    marker_targets: Vec<MarkerTargetJson>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MaterialBindingJson {
    voxel_material: u16,
    palette_entry_id: String,
    display_name: Option<String>,
    material_asset_id: String,
    material_catalog_binding_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AuthoringJson {
    label: Option<String>,
    created_by: Option<String>,
    source_tool: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PreviewRequestJson {
    expected_workspace_id: String,
    expected_generation: u64,
    expected_working_revision: u64,
    expected_scene_content_hash: String,
    provider_id: String,
    preset_id: String,
    seed: u64,
    target: TargetJson,
    material_palette: Vec<MaterialBindingJson>,
    authoring: AuthoringJson,
    limits: LimitsJson,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ApplyRequestJson {
    expected_workspace_id: String,
    expected_generation: u64,
    expected_working_revision: u64,
    candidate_hash: String,
}

impl From<MarkerTargetJson> for ProceduralEnvironmentMarkerTargetDto {
    fn from(value: MarkerTargetJson) -> Self {
        Self {
            source_marker_id: value.source_marker_id,
            node_id: SceneNodeId::new(value.node_id),
            marker_id: value.marker_id,
            child_order: value.child_order,
        }
    }
}

impl From<MaterialBindingJson> for VoxelAssetMaterialBinding {
    fn from(value: MaterialBindingJson) -> Self {
        Self {
            voxel_material: value.voxel_material,
            palette_entry_id: value.palette_entry_id,
            display_name: value.display_name,
            material_asset_id: value.material_asset_id,
            material_catalog_binding_id: value.material_catalog_binding_id,
        }
    }
}

fn preview_request(value: PreviewRequestJson) -> ProceduralEnvironmentPreviewRequestDto {
    ProceduralEnvironmentPreviewRequestDto {
        expected_workspace_id: value.expected_workspace_id,
        expected_generation: value.expected_generation,
        expected_working_revision: value.expected_working_revision,
        expected_scene_content_hash: value.expected_scene_content_hash,
        provider_id: value.provider_id,
        preset_id: value.preset_id,
        seed: value.seed,
        target: ProceduralEnvironmentTargetDto {
            scene_id: SceneId::new(value.target.scene_id),
            scene_path: value.target.scene_path,
            asset_id: value.target.asset_id,
            asset_path: value.target.asset_path,
            voxel_node_id: SceneNodeId::new(value.target.voxel_node_id),
            voxel_parent_id: value.target.voxel_parent_id.map(SceneNodeId::new),
            voxel_child_order: value.target.voxel_child_order,
            voxel_label: value.target.voxel_label,
            voxel_transform: SceneTransformDto {
                translation: value.target.voxel_transform.translation,
                rotation: value.target.voxel_transform.rotation,
                scale: value.target.voxel_transform.scale,
            },
            marker_targets: value
                .target
                .marker_targets
                .into_iter()
                .map(Into::into)
                .collect(),
        },
        material_palette: value.material_palette.into_iter().map(Into::into).collect(),
        authoring: VoxelAssetAuthoringMetadata {
            label: value.authoring.label,
            created_by: value.authoring.created_by,
            source_tool: value.authoring.source_tool,
        },
        limits: ProceduralEnvironmentLimitsDto {
            max_voxels: value.limits.max_voxels,
            max_sparse_runs: value.limits.max_sparse_runs,
            max_markers: value.limits.max_markers,
        },
    }
}

fn diagnostic_code(code: ProceduralEnvironmentDiagnosticCode) -> &'static str {
    match code {
        ProceduralEnvironmentDiagnosticCode::MissingScene => "missingScene",
        ProceduralEnvironmentDiagnosticCode::StaleScene => "staleScene",
        ProceduralEnvironmentDiagnosticCode::UnknownProvider => "unknownProvider",
        ProceduralEnvironmentDiagnosticCode::UnknownPreset => "unknownPreset",
        ProceduralEnvironmentDiagnosticCode::RecipeMismatch => "recipeMismatch",
        ProceduralEnvironmentDiagnosticCode::InvalidTarget => "invalidTarget",
        ProceduralEnvironmentDiagnosticCode::LimitExceeded => "limitExceeded",
        ProceduralEnvironmentDiagnosticCode::InvalidGeneratedAsset => "invalidGeneratedAsset",
        ProceduralEnvironmentDiagnosticCode::InvalidGeneratedScene => "invalidGeneratedScene",
        ProceduralEnvironmentDiagnosticCode::StaleCandidate => "staleCandidate",
    }
}

fn diagnostic_json(value: &ProceduralEnvironmentDiagnosticDto) -> Value {
    json!({
        "code": diagnostic_code(value.code),
        "path": value.path,
        "message": value.message,
    })
}

fn candidate_json(value: &ProceduralEnvironmentArtifactCandidateDto) -> napi::Result<Value> {
    let asset = serde_json::to_value(&value.asset).map_err(|error| {
        napi::Error::from_reason(format!(
            "failed to serialize materialized voxel asset: {error}"
        ))
    })?;
    Ok(json!({
        "candidateHash": value.candidate_hash,
        "sceneFile": {
            "path": value.scene_file.path,
            "mediaType": value.scene_file.media_type,
            "canonicalJson": value.scene_file.canonical_json,
            "contentHash": value.scene_file.content_hash,
        },
        "voxelFile": {
            "path": value.voxel_file.path,
            "mediaType": value.voxel_file.media_type,
            "canonicalJson": value.voxel_file.canonical_json,
            "contentHash": value.voxel_file.content_hash,
        },
        "artifactSetHash": value.artifact_set_hash,
        "scene": scene_document_json(&value.scene),
        "asset": asset,
        "provenance": {
            "providerId": value.provenance.provider_id,
            "providerVersion": value.provenance.provider_version,
            "presetId": value.provenance.preset_id,
            "seed": value.provenance.seed,
            "configHash": value.provenance.config_hash,
            "outputHash": value.provenance.output_hash,
        },
        "markers": value.markers.iter().map(|marker| json!({
            "sourceMarkerId": marker.source_marker_id,
            "markerId": marker.marker_id,
            "nodeId": marker.node_id.raw(),
            "localPosition": marker.local_position,
            "yawDegrees": marker.yaw_degrees,
        })).collect::<Vec<_>>(),
        "sources": {
            "voxelDataHash": value.sources.voxel_data_hash,
            "collisionSourceHash": value.sources.collision_source_hash,
            "navigationSourceHash": value.sources.navigation_source_hash,
            "solidVoxelCount": value.sources.solid_voxel_count,
            "walkableVoxelCount": value.sources.walkable_voxel_count,
        },
    }))
}

fn preview_result_json(value: &ProceduralEnvironmentPreviewResultDto) -> napi::Result<Value> {
    let preview_frame = value
        .preview_frame
        .as_ref()
        .map(|frame| {
            serde_json::from_str::<Value>(&render_bridge::json::encode_frame(frame)).map_err(
                |error| {
                    napi::Error::from_reason(format!(
                        "procedural preview frame could not be encoded: {error}"
                    ))
                },
            )
        })
        .transpose()?;
    Ok(json!({
        "accepted": value.accepted,
        "candidate": value.candidate.as_ref().map(candidate_json).transpose()?,
        "previewFrame": preview_frame,
        "previewProjectionHash": value.preview_projection_hash,
        "previewDiffCount": value.preview_diff_count,
        "diagnostics": value.diagnostics.iter().map(diagnostic_json).collect::<Vec<_>>(),
    }))
}

fn apply_result_json(value: &ProceduralEnvironmentApplyResultDto) -> napi::Result<Value> {
    Ok(json!({
        "accepted": value.accepted,
        "workingRevision": value.working_revision,
        "saveCandidateHash": value.save_candidate_hash,
        "candidate": value.candidate.as_ref().map(candidate_json).transpose()?,
        "diagnostics": value.diagnostics.iter().map(diagnostic_json).collect::<Vec<_>>(),
    }))
}

fn encode(value: Value, operation: &str) -> napi::Result<String> {
    serde_json::to_string(&value).map_err(|error| {
        napi::Error::from_reason(format!("failed to serialize {operation} response: {error}"))
    })
}

#[napi]
pub fn preview_procedural_environment(handle: i64, request_json: String) -> napi::Result<String> {
    let request =
        parse_wire_json::<PreviewRequestJson>("preview_procedural_environment", &request_json)?;
    with_bridge(handle, |bridge| {
        let result = bridge
            .preview_procedural_environment(preview_request(request))
            .map_err(to_napi)?;
        encode(
            preview_result_json(&result)?,
            "preview_procedural_environment",
        )
    })
}

#[napi]
pub fn apply_procedural_environment(handle: i64, request_json: String) -> napi::Result<String> {
    let request =
        parse_wire_json::<ApplyRequestJson>("apply_procedural_environment", &request_json)?;
    with_bridge(handle, |bridge| {
        let result = bridge
            .apply_procedural_environment(ProceduralEnvironmentApplyRequestDto {
                expected_workspace_id: request.expected_workspace_id,
                expected_generation: request.expected_generation,
                expected_working_revision: request.expected_working_revision,
                candidate_hash: request.candidate_hash,
            })
            .map_err(to_napi)?;
        encode(apply_result_json(&result)?, "apply_procedural_environment")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open_workspace() -> i64 {
        crate::open_workspace_authoring(
            -1,
            json!({
                "authoringId": "native.procedural-environment-test",
                "seed": 42,
                "project": { "gameId": "native-test", "workspaceId": "native.procedural" },
                "projectBundle": { "bundleSchemaVersion": 1, "protocolVersion": 1, "sceneId": 42 }
            })
            .to_string(),
        )
        .unwrap()
    }

    fn load_scene(handle: i64) -> String {
        let source = json!({
            "schemaVersion": 4,
            "id": 42,
            "metadata": { "name": "Native procedural", "authoringFormatVersion": 4 },
            "dependencies": [],
            "nodes": [{
                "id": 1,
                "parent": null,
                "childOrder": 0,
                "label": null,
                "tags": [],
                "transform": { "translation": [0, 0, 0], "rotation": [0, 0, 0, 1], "scale": [1, 1, 1] },
                "kind": { "kind": "bootstrap", "bindings": {
                    "generator": { "providerId": "asha.tunnel.enclosed.v2", "presetId": "tiny-enclosed", "seed": 42 },
                    "catalogs": []
                }}
            }]
        });
        let result = crate::decode_scene_document(
            handle,
            json!({ "sourceText": source.to_string() }).to_string(),
        )
        .unwrap();
        serde_json::from_str::<Value>(&result).unwrap()["contentHash"]
            .as_str()
            .unwrap()
            .to_owned()
    }

    fn preview_request(scene_hash: &str) -> Value {
        let material_palette = [1, 2, 3]
            .into_iter()
            .map(|material| {
                json!({
                    "voxelMaterial": material,
                    "paletteEntryId": format!("voxel-material/native-{material}"),
                    "displayName": null,
                    "materialAssetId": format!("material/native-{material}"),
                    "materialCatalogBindingId": format!("catalog-binding/native-{material}")
                })
            })
            .collect::<Vec<_>>();
        json!({
            "expectedWorkspaceId": "native.procedural",
            "expectedGeneration": 1,
            "expectedWorkingRevision": 0,
            "expectedSceneContentHash": scene_hash,
            "providerId": "asha.tunnel.enclosed.v2",
            "presetId": "tiny-enclosed",
            "seed": 42,
            "target": {
                "sceneId": 42,
                "scenePath": "scenes/native-procedural.scene.json",
                "assetId": "voxel-volume/native-procedural",
                "assetPath": "assets/native-procedural.avxl.json",
                "voxelNodeId": 10,
                "voxelParentId": null,
                "voxelChildOrder": 1,
                "voxelLabel": "Generated tunnel",
                "voxelTransform": { "translation": [-3.5, -1, -5.5], "rotation": [0, 0, 0, 1], "scale": [1, 1, 1] },
                "markerTargets": [
                    { "sourceMarkerId": "player_start", "nodeId": 11, "markerId": "spawn/player", "childOrder": 0 },
                    { "sourceMarkerId": "exit_hint", "nodeId": 12, "markerId": "navigation/exit", "childOrder": 1 }
                ]
            },
            "materialPalette": material_palette,
            "authoring": { "label": "Native tunnel", "createdBy": "test", "sourceTool": "native-bridge" },
            "limits": { "maxVoxels": 10000, "maxSparseRuns": 10000, "maxMarkers": 8 }
        })
    }

    #[test]
    fn native_wire_rejects_nested_unknown_fields_and_routes_real_candidate() {
        let handle = open_workspace();
        let scene_hash = load_scene(handle);
        let mut malformed = preview_request(&scene_hash);
        malformed["target"]["markerTargets"][0]["garbage"] = json!(true);
        assert!(preview_procedural_environment(handle, malformed.to_string()).is_err());
        let state: Value =
            serde_json::from_str(&crate::read_workspace_authoring_state(handle).unwrap()).unwrap();
        assert_eq!(state["workingRevision"], 0);

        let preview: Value = serde_json::from_str(
            &preview_procedural_environment(handle, preview_request(&scene_hash).to_string())
                .unwrap(),
        )
        .unwrap();
        assert_eq!(preview["accepted"], true);
        assert!(preview["previewDiffCount"].as_u64().unwrap() > 0);
        assert!(preview["previewFrame"]["ops"].as_array().is_some());
        let candidate_hash = preview["candidate"]["candidateHash"].as_str().unwrap();
        let applied: Value = serde_json::from_str(
            &apply_procedural_environment(
                handle,
                json!({
                    "expectedWorkspaceId": "native.procedural",
                    "expectedGeneration": 1,
                    "expectedWorkingRevision": 0,
                    "candidateHash": candidate_hash
                })
                .to_string(),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(applied["accepted"], true);
        assert_eq!(applied["workingRevision"], 1);
        assert!(applied["saveCandidateHash"].as_str().is_some());
    }
}
