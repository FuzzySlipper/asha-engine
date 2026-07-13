use core_ids::SceneNodeId;
use napi_derive::napi;
use protocol_assets::{
    AssetReference, CatalogEntry, CollisionMaterial, MaterialProjection, RenderMaterial, Rgba,
};
use protocol_render::{
    MeshAttribute, MeshAttributeKind, MeshAttributeName, MeshBoundsDescriptor, MeshBufferLayout,
    MeshCollisionPolicy, MeshGroupDescriptor, MeshIndexWidth, MeshMaterialSlot,
    MeshPayloadDescriptor, MeshPayloadSource, MeshProvenance, ModelMaterialPreviewRequest,
    RenderDiff, StaticMeshAsset,
};
use protocol_scene::{
    AssetReferenceDto, AssetVersionReqDto, SceneNodeKindDto, SceneNodeRecordDto,
    SceneObjectCommandDto, SceneObjectCommandRequestDto, SceneObjectCommandResultDto,
    SceneTransformDto,
};
use runtime_bridge_api::{RuntimeBridge, RuntimeBridgeError, RuntimeBridgeErrorKind};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{to_napi, with_bridge};

fn invalid(message: impl Into<String>) -> napi::Error {
    to_napi(RuntimeBridgeError::new(
        RuntimeBridgeErrorKind::InvalidInput,
        message,
    ))
}

fn internal(message: impl Into<String>) -> napi::Error {
    to_napi(RuntimeBridgeError::new(
        RuntimeBridgeErrorKind::Internal,
        message,
    ))
}

fn parse<T: serde::de::DeserializeOwned>(text: &str, operation: &str) -> napi::Result<T> {
    serde_json::from_str(text).map_err(|error| {
        invalid(format!(
            "{operation} request is not valid JSON: {error}"
        ))
    })
}

fn encode(value: Value, operation: &str) -> napi::Result<String> {
    serde_json::to_string(&value).map_err(|error| {
        internal(format!(
            "{operation} result could not be serialized: {error}"
        ))
    })
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ModelPreviewJson {
    catalog_entry: CatalogEntryJson,
    mesh_asset: StaticMeshAssetJson,
    instance_handle: u64,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CatalogEntryJson {
    id: String,
    kind: String,
    version: u64,
    hash: Option<String>,
    source_path: Option<String>,
    label: Option<String>,
    dependencies: Vec<SceneAssetReferenceJson>,
    material: Option<MaterialProjectionJson>,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct SceneAssetReferenceJson {
    id: String,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct RgbaJson {
    r: f32,
    g: f32,
    b: f32,
    a: f32,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RenderMaterialJson {
    color: RgbaJson,
    texture: Option<SceneAssetReferenceJson>,
    roughness: f32,
    texture_tint: RgbaJson,
    emission_color: RgbaJson,
    emissive: f32,
    uv_strategy: String,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CollisionMaterialJson {
    solid: bool,
    collidable: bool,
    occludes: bool,
    structural_class: String,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct MaterialProjectionJson {
    render: RenderMaterialJson,
    collision: CollisionMaterialJson,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StaticMeshAssetJson {
    asset: String,
    payload: MeshPayloadJson,
    material_slots: Vec<MeshMaterialSlotJson>,
    collision: MeshCollisionJson,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MeshPayloadJson {
    layout: MeshLayoutJson,
    groups: Vec<MeshGroupJson>,
    bounds: MeshBoundsJson,
    source: MeshSourceJson,
    provenance: String,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MeshLayoutJson {
    vertex_count: u32,
    index_count: u32,
    index_width: String,
    attributes: Vec<MeshAttributeJson>,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct MeshAttributeJson {
    name: String,
    components: u8,
    kind: String,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MeshGroupJson {
    material_slot: u16,
    start: u32,
    count: u32,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct MeshBoundsJson {
    min: [f32; 3],
    max: [f32; 3],
}

#[derive(Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
enum MeshSourceJson {
    Inline {
        positions: Vec<f32>,
        normals: Vec<f32>,
        indices: Vec<u32>,
    },
    Handle {
        buffer: u64,
        #[serde(rename = "positionsByteOffset")]
        positions_byte_offset: u32,
        #[serde(rename = "normalsByteOffset")]
        normals_byte_offset: u32,
        #[serde(rename = "indicesByteOffset")]
        indices_byte_offset: u32,
    },
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct MeshMaterialSlotJson {
    slot: u16,
    material: String,
}

#[derive(Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
enum MeshCollisionJson {
    VisualOnly,
    Proxy {
        #[serde(rename = "proxyAsset")]
        proxy_asset: String,
    },
    AabbFallback,
}

impl RgbaJson {
    fn protocol(&self) -> Rgba {
        Rgba {
            r: self.r,
            g: self.g,
            b: self.b,
            a: self.a,
        }
    }
}

impl MaterialProjectionJson {
    fn protocol(&self) -> MaterialProjection {
        MaterialProjection {
            render: RenderMaterial {
                color: self.render.color.protocol(),
                texture: self
                    .render
                    .texture
                    .as_ref()
                    .map(|asset| AssetReference {
                        id: asset.id.clone(),
                        kind: asset_kind(&asset.id),
                    }),
                roughness: self.render.roughness,
                texture_tint: self.render.texture_tint.protocol(),
                emission_color: self.render.emission_color.protocol(),
                emissive: self.render.emissive,
                uv_strategy: self.render.uv_strategy.clone(),
            },
            collision: CollisionMaterial {
                solid: self.collision.solid,
                collidable: self.collision.collidable,
                occludes: self.collision.occludes,
                structural_class: self.collision.structural_class.clone(),
            },
        }
    }
}

fn asset_kind(id: &str) -> String {
    id.split(['/', ':']).next().unwrap_or_default().to_string()
}

impl CatalogEntryJson {
    fn protocol(&self) -> CatalogEntry {
        CatalogEntry {
            id: self.id.clone(),
            kind: self.kind.clone(),
            version: self.version,
            hash: self.hash.clone(),
            source_path: self.source_path.clone(),
            label: self.label.clone(),
            dependencies: self
                .dependencies
                .iter()
                .map(|asset| AssetReference {
                    id: asset.id.clone(),
                    kind: asset_kind(&asset.id),
                })
                .collect(),
            material: self
                .material
                .as_ref()
                .map(MaterialProjectionJson::protocol),
        }
    }
}

impl StaticMeshAssetJson {
    fn protocol(&self) -> napi::Result<StaticMeshAsset> {
        let attributes = self
            .payload
            .layout
            .attributes
            .iter()
            .map(|attribute| {
                let name = match attribute.name.as_str() {
                    "position" => MeshAttributeName::Position,
                    "normal" => MeshAttributeName::Normal,
                    "uv" => MeshAttributeName::Uv,
                    "color" => MeshAttributeName::Color,
                    other => {
                        return Err(invalid(format!(
                            "unsupported mesh attribute {other:?}"
                        )))
                    }
                };
                if attribute.kind != "f32" {
                    return Err(invalid(format!(
                        "unsupported mesh attribute kind {:?}",
                        attribute.kind
                    )));
                }
                Ok(MeshAttribute {
                    name,
                    components: attribute.components,
                    kind: MeshAttributeKind::F32,
                })
            })
            .collect::<napi::Result<Vec<_>>>()?;
        if self.payload.layout.index_width != "u32" {
            return Err(invalid(format!(
                "unsupported mesh index width {:?}",
                self.payload.layout.index_width
            )));
        }
        let source = match &self.payload.source {
            MeshSourceJson::Inline {
                positions,
                normals,
                indices,
            } => MeshPayloadSource::Inline {
                positions: positions.clone(),
                normals: normals.clone(),
                indices: indices.clone(),
            },
            MeshSourceJson::Handle {
                buffer,
                positions_byte_offset,
                normals_byte_offset,
                indices_byte_offset,
            } => MeshPayloadSource::Handle {
                buffer: *buffer,
                positions_byte_offset: *positions_byte_offset,
                normals_byte_offset: *normals_byte_offset,
                indices_byte_offset: *indices_byte_offset,
            },
        };
        let provenance = match self.payload.provenance.as_str() {
            "voxelChunk" => MeshProvenance::VoxelChunk,
            "staticAsset" => MeshProvenance::StaticAsset,
            "generated" => MeshProvenance::Generated,
            "debug" => MeshProvenance::Debug,
            other => {
                return Err(invalid(format!(
                    "unsupported mesh provenance {other:?}"
                )))
            }
        };
        let collision = match &self.collision {
            MeshCollisionJson::VisualOnly => MeshCollisionPolicy::VisualOnly,
            MeshCollisionJson::Proxy { proxy_asset } => MeshCollisionPolicy::Proxy {
                proxy_asset: proxy_asset.clone(),
            },
            MeshCollisionJson::AabbFallback => MeshCollisionPolicy::AabbFallback,
        };
        Ok(StaticMeshAsset {
            asset: self.asset.clone(),
            payload: MeshPayloadDescriptor {
                layout: MeshBufferLayout {
                    vertex_count: self.payload.layout.vertex_count,
                    index_count: self.payload.layout.index_count,
                    index_width: MeshIndexWidth::U32,
                    attributes,
                },
                groups: self
                    .payload
                    .groups
                    .iter()
                    .map(|group| MeshGroupDescriptor {
                        material_slot: group.material_slot,
                        start: group.start,
                        count: group.count,
                    })
                    .collect(),
                bounds: MeshBoundsDescriptor {
                    min: self.payload.bounds.min,
                    max: self.payload.bounds.max,
                },
                source,
                provenance,
            },
            material_slots: self
                .material_slots
                .iter()
                .map(|slot| MeshMaterialSlot {
                    slot: slot.slot,
                    material: slot.material.clone(),
                })
                .collect(),
            collision,
        })
    }
}

#[napi]
pub fn read_model_material_preview(handle: i64, request_json: String) -> napi::Result<String> {
    let source: Value = parse(&request_json, "model material preview")?;
    let request: ModelPreviewJson = parse(&request_json, "model material preview")?;
    let protocol_request = ModelMaterialPreviewRequest {
        catalog_entry: request.catalog_entry.protocol(),
        mesh_asset: request.mesh_asset.protocol()?,
        instance_handle: protocol_render::RenderHandle::new(request.instance_handle),
    };
    with_bridge(handle, |bridge| {
        let snapshot = bridge
            .read_model_material_preview(protocol_request)
            .map_err(to_napi)?;
        let catalog_entry = source
            .get("catalogEntry")
            .cloned()
            .ok_or_else(|| invalid("missing catalogEntry"))?;
        let mesh_asset = source
            .get("meshAsset")
            .cloned()
            .ok_or_else(|| invalid("missing meshAsset"))?;
        let material = catalog_entry
            .get("material")
            .cloned()
            .ok_or_else(|| invalid("catalogEntry.material is required"))?;
        let ops = snapshot
            .preview_diff
            .ops
            .iter()
            .map(|operation| match operation {
                RenderDiff::DefineMaterial { material } => Ok(json!({
                    "op": "defineMaterial",
                    "material": {
                        "schemaVersion": material.schema_version,
                        "id": material.id,
                        "color": material.color,
                        "texture": material.texture,
                        "roughness": material.roughness,
                        "textureTint": material.texture_tint,
                        "emissionColor": material.emission_color,
                        "emissionIntensity": material.emission_intensity,
                        "uvStrategy": material.uv_strategy.label(),
                    }
                })),
                RenderDiff::DefineStaticMesh { .. } => {
                    Ok(json!({ "op": "defineStaticMesh", "asset": mesh_asset }))
                }
                RenderDiff::CreateStaticMeshInstance {
                    handle,
                    parent,
                    instance,
                } => Ok(json!({
                    "op": "createStaticMeshInstance",
                    "handle": handle.raw(),
                    "parent": parent.map(|value| value.raw()),
                    "instance": {
                        "asset": instance.asset,
                        "transform": {
                            "translation": instance.transform.translation,
                            "rotation": instance.transform.rotation,
                            "scale": instance.transform.scale,
                        },
                        "materialOverrides": instance.material_overrides.iter().map(|slot| json!({
                            "slot": slot.slot, "material": slot.material,
                        })).collect::<Vec<_>>(),
                        "metadata": {
                            "source": instance.metadata.source.map(|value| value.raw()),
                            "tags": instance.metadata.tags.iter().map(|value| value.raw()).collect::<Vec<_>>(),
                            "label": instance.metadata.label,
                        },
                    }
                })),
                other => Err(internal(format!(
                    "model preview emitted unsupported render operation {other:?}"
                ))),
            })
            .collect::<napi::Result<Vec<_>>>()?;
        encode(
            json!({
                "catalogEntry": catalog_entry,
                "material": material,
                "meshAsset": mesh_asset,
                "previewDiff": { "ops": ops },
                "rendererClassification": snapshot.renderer_classification,
                "diagnostics": snapshot.diagnostics,
            }),
            "model material preview",
        )
    })
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SceneCommandRequestJson {
    expected_document_hash: u64,
    command: SceneCommandJson,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
enum SceneCommandJson {
    Create {
        record: SceneRecordJson,
    },
    Delete {
        id: u64,
    },
    Rename {
        id: u64,
        label: Option<String>,
    },
    Reparent {
        id: u64,
        parent: Option<u64>,
        child_order: u32,
    },
    Translate {
        id: u64,
        delta: [f32; 3],
    },
    Rotate {
        id: u64,
        rotation: [f32; 4],
    },
    Select {
        id: Option<u64>,
    },
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SceneRecordJson {
    id: u64,
    parent: Option<u64>,
    child_order: u32,
    label: Option<String>,
    tags: Vec<String>,
    transform: SceneTransformJson,
    kind: SceneKindJson,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SceneTransformJson {
    translation: [f32; 3],
    rotation: [f32; 4],
    scale: [f32; 3],
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
enum SceneKindJson {
    EmptyGroup,
    StaticMesh { asset: SceneAssetDtoJson },
    Sprite { asset: SceneAssetDtoJson },
    VoxelVolume { asset: SceneAssetDtoJson },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SceneAssetDtoJson {
    id: String,
    version: SceneVersionJson,
    hash: Option<String>,
}

#[derive(Deserialize)]
#[serde(tag = "req", rename_all = "camelCase", deny_unknown_fields)]
enum SceneVersionJson {
    Any,
    Exact { value: u32 },
    AtLeast { value: u32 },
}

impl SceneAssetDtoJson {
    fn protocol(self) -> AssetReferenceDto {
        AssetReferenceDto {
            id: self.id,
            version: match self.version {
                SceneVersionJson::Any => AssetVersionReqDto::Any,
                SceneVersionJson::Exact { value } => AssetVersionReqDto::Exact(value),
                SceneVersionJson::AtLeast { value } => AssetVersionReqDto::AtLeast(value),
            },
            hash: self.hash,
        }
    }
}

impl SceneRecordJson {
    fn protocol(self) -> SceneNodeRecordDto {
        SceneNodeRecordDto {
            id: SceneNodeId::new(self.id),
            parent: self.parent.map(SceneNodeId::new),
            child_order: self.child_order,
            label: self.label,
            tags: self.tags,
            transform: SceneTransformDto {
                translation: self.transform.translation,
                rotation: self.transform.rotation,
                scale: self.transform.scale,
            },
            kind: match self.kind {
                SceneKindJson::EmptyGroup => SceneNodeKindDto::EmptyGroup,
                SceneKindJson::StaticMesh { asset } => {
                    SceneNodeKindDto::StaticMesh(asset.protocol())
                }
                SceneKindJson::Sprite { asset } => SceneNodeKindDto::Sprite(asset.protocol()),
                SceneKindJson::VoxelVolume { asset } => {
                    SceneNodeKindDto::VoxelVolume(asset.protocol())
                }
            },
        }
    }
}

impl SceneCommandJson {
    fn protocol(self) -> SceneObjectCommandDto {
        match self {
            SceneCommandJson::Create { record } => SceneObjectCommandDto::Create {
                record: record.protocol(),
            },
            SceneCommandJson::Delete { id } => SceneObjectCommandDto::Delete {
                id: SceneNodeId::new(id),
            },
            SceneCommandJson::Rename { id, label } => SceneObjectCommandDto::Rename {
                id: SceneNodeId::new(id),
                label,
            },
            SceneCommandJson::Reparent {
                id,
                parent,
                child_order,
            } => SceneObjectCommandDto::Reparent {
                id: SceneNodeId::new(id),
                parent: parent.map(SceneNodeId::new),
                child_order,
            },
            SceneCommandJson::Translate { id, delta } => SceneObjectCommandDto::Translate {
                id: SceneNodeId::new(id),
                delta,
            },
            SceneCommandJson::Rotate { id, rotation } => SceneObjectCommandDto::Rotate {
                id: SceneNodeId::new(id),
                rotation,
            },
            SceneCommandJson::Select { id } => SceneObjectCommandDto::Select {
                id: id.map(SceneNodeId::new),
            },
        }
    }
}

fn scene_snapshot_json(snapshot: &protocol_scene::SceneObjectSnapshotDto) -> Value {
    json!({
        "documentHash": snapshot.document_hash,
        "objects": snapshot.objects.iter().map(|object| json!({
            "id": object.id.raw(),
            "parent": object.parent.map(|value| value.raw()),
            "childOrder": object.child_order,
            "label": object.label,
            "kind": object.kind.as_str(),
            "hasRenderableAsset": object.has_renderable_asset,
        })).collect::<Vec<_>>(),
    })
}

fn scene_asset_json(asset: &AssetReferenceDto) -> Value {
    let version = match asset.version {
        AssetVersionReqDto::Any => json!({ "req": "any" }),
        AssetVersionReqDto::Exact(value) => json!({ "req": "exact", "value": value }),
        AssetVersionReqDto::AtLeast(value) => json!({ "req": "atLeast", "value": value }),
    };
    json!({ "id": asset.id, "version": version, "hash": asset.hash })
}

fn scene_document_json(document: &protocol_scene::FlatSceneDocumentDto) -> Value {
    json!({
        "schemaVersion": document.schema_version,
        "id": document.id.raw(),
        "metadata": {
            "name": document.metadata.name,
            "authoringFormatVersion": document.metadata.authoring_format_version,
        },
        "dependencies": document.dependencies.iter().map(scene_asset_json).collect::<Vec<_>>(),
        "nodes": document.nodes.iter().map(|record| {
            let kind = match &record.kind {
                SceneNodeKindDto::EmptyGroup => json!({ "kind": "emptyGroup" }),
                SceneNodeKindDto::StaticMesh(asset) => json!({ "kind": "staticMesh", "asset": scene_asset_json(asset) }),
                SceneNodeKindDto::Sprite(asset) => json!({ "kind": "sprite", "asset": scene_asset_json(asset) }),
                SceneNodeKindDto::VoxelVolume(asset) => json!({ "kind": "voxelVolume", "asset": scene_asset_json(asset) }),
            };
            json!({
                "id": record.id.raw(),
                "parent": record.parent.map(|value| value.raw()),
                "childOrder": record.child_order,
                "label": record.label,
                "tags": record.tags,
                "transform": {
                    "translation": record.transform.translation,
                    "rotation": record.transform.rotation,
                    "scale": record.transform.scale,
                },
                "kind": kind,
            })
        }).collect::<Vec<_>>(),
    })
}

fn scene_result_json(result: &SceneObjectCommandResultDto) -> Value {
    json!({
        "accepted": result.accepted,
        "outcome": result.outcome.as_ref().map(|outcome| json!({
            "document": scene_document_json(&outcome.document),
            "snapshot": scene_snapshot_json(&outcome.snapshot),
            "selected": outcome.selected.map(|value| value.raw()),
        })),
        "rejection": result.rejection.as_ref().map(|rejection| json!({
            "code": rejection.code.as_str(),
            "id": rejection.id.map(|value| value.raw()),
            "parent": rejection.parent.map(|value| value.raw()),
            "expectedHash": rejection.expected_hash,
            "actualHash": rejection.actual_hash,
            "validationErrors": rejection.validation_errors.iter().map(|error| json!({
                "code": error.code.as_str(),
                "node": error.node.map(|value| value.raw()),
                "parent": error.parent.map(|value| value.raw()),
                "expectedKind": error.expected_kind,
                "actualKind": error.actual_kind,
                "transformReason": error.transform_reason,
                "cyclePath": error.cycle_path.iter().map(|value| value.raw()).collect::<Vec<_>>(),
            })).collect::<Vec<_>>(),
        })),
    })
}

#[napi]
pub fn read_scene_object_snapshot(handle: i64) -> napi::Result<String> {
    with_bridge(handle, |bridge| {
        let snapshot = bridge.read_scene_object_snapshot().map_err(to_napi)?;
        encode(scene_snapshot_json(&snapshot), "scene object snapshot")
    })
}

#[napi]
pub fn apply_scene_object_command(handle: i64, request_json: String) -> napi::Result<String> {
    let request: SceneCommandRequestJson = parse(&request_json, "scene object command")?;
    with_bridge(handle, |bridge| {
        let result = bridge
            .apply_scene_object_command(SceneObjectCommandRequestDto {
                expected_document_hash: request.expected_document_hash,
                command: request.command.protocol(),
            })
            .map_err(to_napi)?;
        encode(scene_result_json(&result), "scene object command")
    })
}

#[cfg(test)]
pub(crate) fn model_preview_test_request_json() -> String {
    json!({
        "catalogEntry": {
            "id": "material/copper",
            "kind": "material",
            "version": 1,
            "hash": "sha256-material-copper",
            "sourcePath": null,
            "label": "Copper",
            "dependencies": [],
            "material": {
                "render": {
                    "color": { "r": 0.8, "g": 0.4, "b": 0.2, "a": 1.0 },
                    "texture": null,
                    "roughness": 0.6,
                    "textureTint": { "r": 1.0, "g": 1.0, "b": 1.0, "a": 1.0 },
                    "emissionColor": { "r": 0.8, "g": 0.4, "b": 0.2, "a": 1.0 },
                    "emissive": 0.0,
                    "uvStrategy": "flat"
                },
                "collision": {
                    "solid": true,
                    "collidable": true,
                    "occludes": true,
                    "structuralClass": "solid"
                }
            }
        },
        "meshAsset": {
            "asset": "mesh/preview-triangle",
            "payload": {
                "layout": {
                    "vertexCount": 3,
                    "indexCount": 3,
                    "indexWidth": "u32",
                    "attributes": [
                        { "name": "position", "components": 3, "kind": "f32" }
                    ]
                },
                "groups": [{ "materialSlot": 0, "start": 0, "count": 3 }],
                "bounds": { "min": [0.0, 0.0, 0.0], "max": [1.0, 1.0, 0.0] },
                "source": {
                    "kind": "inline",
                    "positions": [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0],
                    "normals": [0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0],
                    "indices": [0, 1, 2]
                },
                "provenance": "staticAsset"
            },
            "materialSlots": [{ "slot": 0, "material": "material/copper" }],
            "collision": { "kind": "aabbFallback" }
        },
        "instanceHandle": 7001
    })
    .to_string()
}
