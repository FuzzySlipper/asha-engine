use napi_derive::napi;
use runtime_bridge_api::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

use crate::wire::parse_wire_json;
use crate::{to_napi, with_bridge};

fn encode(value: Value, operation: &str) -> napi::Result<String> {
    serde_json::to_string(&value).map_err(|error| {
        napi::Error::from_reason(format!("failed to serialize {operation} response: {error}"))
    })
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
enum DocumentKindJson {
    EntityDefinition,
    AssetCatalog,
    PrefabRegistry,
    GameplayConfiguration,
    PresentationCatalog,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SourceJson {
    source_path: String,
    document_id: String,
    kind: DocumentKindJson,
    source_text: String,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct DecodeRequestJson {
    sources: Vec<SourceJson>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct EncodeRequestJson {
    documents: Vec<Value>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AuthoringRequestJson {
    expected_workspace_id: String,
    expected_generation: u64,
    expected_working_revision: u64,
    expected_set_hash: String,
    command: AuthoringCommandJson,
}

#[derive(Deserialize, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
enum AuthoringCommandJson {
    Upsert {
        source_path: String,
        document: Value,
    },
    Delete {
        document_id: String,
        document_kind: DocumentKindJson,
    },
}

impl From<DocumentKindJson> for ProjectContentDocumentKind {
    fn from(value: DocumentKindJson) -> Self {
        match value {
            DocumentKindJson::EntityDefinition => Self::EntityDefinition,
            DocumentKindJson::AssetCatalog => Self::AssetCatalog,
            DocumentKindJson::PrefabRegistry => Self::PrefabRegistry,
            DocumentKindJson::GameplayConfiguration => Self::GameplayConfiguration,
            DocumentKindJson::PresentationCatalog => Self::PresentationCatalog,
        }
    }
}

fn source_from_document(value: &Value) -> napi::Result<ProjectContentSourceDto> {
    let object = value.as_object().ok_or_else(|| {
        napi::Error::from_reason("project-content document must be an object".to_owned())
    })?;
    let kind = object.get("kind").and_then(Value::as_str).ok_or_else(|| {
        napi::Error::from_reason("project-content document.kind must be a string".to_owned())
    })?;
    let document_id = object
        .get("documentId")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            napi::Error::from_reason(
                "project-content document.documentId must be a string".to_owned(),
            )
        })?
        .to_owned();
    let (document_kind, payload_key) = match kind {
        "entityDefinition" => (ProjectContentDocumentKind::EntityDefinition, "definition"),
        "assetCatalog" => (ProjectContentDocumentKind::AssetCatalog, "catalog"),
        "prefabRegistry" => (ProjectContentDocumentKind::PrefabRegistry, "registry"),
        "gameplayConfiguration" => (
            ProjectContentDocumentKind::GameplayConfiguration,
            "document",
        ),
        "presentationCatalog" => (ProjectContentDocumentKind::PresentationCatalog, "catalog"),
        other => {
            return Err(napi::Error::from_reason(format!(
                "unknown project-content document kind `{other}`"
            )))
        }
    };
    let allowed = ["kind", "documentId", payload_key];
    for key in object.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(napi::Error::from_reason(format!(
                "unknown field `{key}` in project-content document"
            )));
        }
    }
    let mut payload = object.get(payload_key).cloned().ok_or_else(|| {
        napi::Error::from_reason(format!("project-content document requires `{payload_key}`"))
    })?;
    if document_kind == ProjectContentDocumentKind::EntityDefinition {
        let payload_object = payload.as_object_mut().ok_or_else(|| {
            napi::Error::from_reason("entity definition must be an object".to_owned())
        })?;
        payload_object.insert(
            "kind".to_owned(),
            Value::String("EntityDefinition".to_owned()),
        );
    }
    if document_kind == ProjectContentDocumentKind::AssetCatalog {
        normalize_asset_catalog_for_storage(&mut payload)?;
    }
    let source_text = serde_json::to_string(&payload)
        .map_err(|error| napi::Error::from_reason(error.to_string()))?;
    Ok(ProjectContentSourceDto {
        source_path: document_id.clone(),
        document_id,
        kind: document_kind,
        source_text,
    })
}

fn decode_document_values(
    documents: &[Value],
) -> napi::Result<Result<Vec<ProjectContentDocumentDto>, Vec<ProjectContentDiagnosticDto>>> {
    let sources = documents
        .iter()
        .map(source_from_document)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(EngineBridge::decode_project_content_sources(&sources))
}

fn result_json(
    accepted: bool,
    canonical_files: &[ProjectContentCanonicalFileDto],
    set_hash: &Option<String>,
    provider_schemas: &[ProjectConfigurationSchemaDto],
    field_metadata: &[ProjectContentFieldMetadataDto],
    diagnostics: &[ProjectContentDiagnosticDto],
) -> napi::Result<Value> {
    let documents = canonical_files
        .iter()
        .map(document_json)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(json!({
        "accepted": accepted,
        "documents": documents,
        "canonicalFiles": canonical_files.iter().map(|file| json!({
            "sourcePath": file.source_path,
            "documentId": file.document_id,
            "kind": document_kind_tag(file.kind),
            "canonicalJson": file.canonical_json,
            "contentHash": file.content_hash,
        })).collect::<Vec<_>>(),
        "setHash": set_hash,
        "providerSchemas": provider_schemas.iter().map(configuration_schema_json).collect::<Vec<_>>(),
        "fieldMetadata": field_metadata.iter().map(|field| json!({
            "documentId": field.document_id,
            "path": field.path,
            "label": field.label,
            "valueKind": value_kind_tag(field.value_kind),
            "required": field.required,
            "editable": field.editable,
            "referenceKind": field.reference_kind.map(reference_kind_tag),
            "configurationId": field.configuration_id,
            "schemaId": field.schema_id,
            "moduleId": field.module_id,
            "providerId": field.provider_id,
            "contract": field.contract.as_ref().map(contract_json),
            "codecId": field.codec_id,
            "integerMin": field.integer_min,
            "integerMax": field.integer_max,
            "numberMin": field.number_min,
            "numberMax": field.number_max,
        })).collect::<Vec<_>>(),
        "diagnostics": diagnostics.iter().map(|diagnostic| json!({
            "code": diagnostic_code_tag(diagnostic.code),
            "documentId": diagnostic.document_id,
            "path": diagnostic.path,
            "message": diagnostic.message,
        })).collect::<Vec<_>>(),
    }))
}

pub(crate) fn codec_result_json(result: &ProjectContentCodecResultDto) -> napi::Result<Value> {
    result_json(
        result.accepted,
        &result.canonical_files,
        &result.set_hash,
        &result.provider_schemas,
        &result.field_metadata,
        &result.diagnostics,
    )
}

fn authoring_result_json(result: &ProjectContentAuthoringResultDto) -> napi::Result<Value> {
    result_json(
        result.accepted,
        &result.canonical_files,
        &result.set_hash,
        &result.provider_schemas,
        &result.field_metadata,
        &result.diagnostics,
    )
}

fn configuration_schema_json(schema: &ProjectConfigurationSchemaDto) -> Value {
    json!({
        "schemaId": schema.schema_id,
        "moduleId": schema.module_id,
        "providerId": schema.provider_id,
        "contract": contract_json(&schema.contract),
        "codecId": schema.codec_id,
        "fields": schema.fields.iter().map(|field| json!({
            "fieldId": field.field_id,
            "label": field.label,
            "valueKind": value_kind_tag(field.value_kind),
            "required": field.required,
            "referenceKind": field.reference_kind.map(reference_kind_tag),
            "integerMin": field.integer_min,
            "integerMax": field.integer_max,
            "numberMin": field.number_min,
            "numberMax": field.number_max,
        })).collect::<Vec<_>>(),
    })
}

fn contract_json(contract: &GameplayContractRef) -> Value {
    json!({
        "namespace": contract.namespace,
        "name": contract.name,
        "version": contract.version,
        "schemaHash": contract.schema_hash,
    })
}

fn document_json(file: &ProjectContentCanonicalFileDto) -> napi::Result<Value> {
    let artifact: Value = serde_json::from_str(&file.canonical_json)
        .map_err(|error| napi::Error::from_reason(error.to_string()))?;
    let artifact = artifact.as_object().ok_or_else(|| {
        napi::Error::from_reason("canonical ProjectContent artifact must be an object".to_owned())
    })?;
    let schema_version = artifact.get("schemaVersion").and_then(Value::as_u64);
    if schema_version != Some(u64::from(PROJECT_CONTENT_SCHEMA_VERSION)) {
        return Err(napi::Error::from_reason(format!(
            "canonical ProjectContent artifact has invalid schemaVersion {schema_version:?}"
        )));
    }
    let artifact_document_id = artifact.get("documentId").and_then(Value::as_str);
    if artifact_document_id != Some(file.document_id.as_str()) {
        return Err(napi::Error::from_reason(format!(
            "canonical ProjectContent artifact documentId {artifact_document_id:?} does not match {}",
            file.document_id
        )));
    }
    let expected_kind = document_kind_tag(file.kind);
    let artifact_kind = artifact.get("documentKind").and_then(Value::as_str);
    if artifact_kind != Some(expected_kind) {
        return Err(napi::Error::from_reason(format!(
            "canonical ProjectContent artifact documentKind {artifact_kind:?} does not match {expected_kind}"
        )));
    }
    let mut payload = artifact.get("document").cloned().ok_or_else(|| {
        napi::Error::from_reason("canonical ProjectContent artifact is missing document".to_owned())
    })?;
    if file.kind == ProjectContentDocumentKind::AssetCatalog {
        normalize_asset_catalog_for_public(&mut payload)?;
    }
    let (kind, payload_key) = match file.kind {
        ProjectContentDocumentKind::EntityDefinition => ("entityDefinition", "definition"),
        ProjectContentDocumentKind::AssetCatalog => ("assetCatalog", "catalog"),
        ProjectContentDocumentKind::PrefabRegistry => ("prefabRegistry", "registry"),
        ProjectContentDocumentKind::GameplayConfiguration => ("gameplayConfiguration", "document"),
        ProjectContentDocumentKind::PresentationCatalog => ("presentationCatalog", "catalog"),
    };
    let mut object = Map::new();
    object.insert("kind".to_owned(), Value::String(kind.to_owned()));
    object.insert(
        "documentId".to_owned(),
        Value::String(file.document_id.clone()),
    );
    object.insert(payload_key.to_owned(), payload);
    Ok(Value::Object(object))
}

fn normalize_asset_catalog_for_public(catalog: &mut Value) -> napi::Result<()> {
    for_each_material_style(catalog, |style, field| {
        let value = style.get_mut(field).ok_or_else(|| {
            napi::Error::from_reason(format!("stored material style is missing {field}"))
        })?;
        let components = value.as_array().ok_or_else(|| {
            napi::Error::from_reason(format!("stored material {field} must be a 4-array"))
        })?;
        if components.len() != 4 || components.iter().any(|component| !component.is_number()) {
            return Err(napi::Error::from_reason(format!(
                "stored material {field} must be a numeric 4-array"
            )));
        }
        *value = json!({
            "r": components[0],
            "g": components[1],
            "b": components[2],
            "a": components[3],
        });
        Ok(())
    })
}

fn normalize_asset_catalog_for_storage(catalog: &mut Value) -> napi::Result<()> {
    for_each_material_style(catalog, |style, field| {
        let value = style.get_mut(field).ok_or_else(|| {
            napi::Error::from_reason(format!("public material style is missing {field}"))
        })?;
        let components = value.as_object().ok_or_else(|| {
            napi::Error::from_reason(format!("public material {field} must be an RGBA object"))
        })?;
        let component = |name: &str| {
            components.get(name).and_then(Value::as_f64).ok_or_else(|| {
                napi::Error::from_reason(format!(
                    "public material {field}.{name} must be a number"
                ))
            })
        };
        *value = json!([component("r")?, component("g")?, component("b")?, component("a")?]);
        Ok(())
    })
}

fn for_each_material_style(
    catalog: &mut Value,
    mut visit: impl FnMut(&mut Map<String, Value>, &str) -> napi::Result<()>,
) -> napi::Result<()> {
    let entries = catalog
        .get_mut("entries")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| napi::Error::from_reason("asset catalog entries must be an array"))?;
    for entry in entries {
        let Some(material) = entry.get_mut("material") else {
            continue;
        };
        if material.is_null() {
            continue;
        }
        let style = material
            .get_mut("style")
            .and_then(Value::as_object_mut)
            .ok_or_else(|| napi::Error::from_reason("material style must be an object"))?;
        for field in ["color", "textureTint", "emissionColor"] {
            visit(style, field)?;
        }
    }
    Ok(())
}

fn document_kind_tag(value: ProjectContentDocumentKind) -> &'static str {
    match value {
        ProjectContentDocumentKind::EntityDefinition => "entityDefinition",
        ProjectContentDocumentKind::AssetCatalog => "assetCatalog",
        ProjectContentDocumentKind::PrefabRegistry => "prefabRegistry",
        ProjectContentDocumentKind::GameplayConfiguration => "gameplayConfiguration",
        ProjectContentDocumentKind::PresentationCatalog => "presentationCatalog",
    }
}

fn value_kind_tag(value: ProjectConfigurationValueKind) -> &'static str {
    match value {
        ProjectConfigurationValueKind::Boolean => "boolean",
        ProjectConfigurationValueKind::Integer => "integer",
        ProjectConfigurationValueKind::Number => "number",
        ProjectConfigurationValueKind::String => "string",
        ProjectConfigurationValueKind::Reference => "reference",
    }
}

fn reference_kind_tag(value: ProjectContentReferenceKind) -> &'static str {
    match value {
        ProjectContentReferenceKind::Asset => "asset",
        ProjectContentReferenceKind::EntityDefinition => "entityDefinition",
        ProjectContentReferenceKind::InstantiatedEntityDefinition => {
            "instantiatedEntityDefinition"
        }
        ProjectContentReferenceKind::InstantiatedBoundedEntityDefinition => {
            "instantiatedBoundedEntityDefinition"
        }
        ProjectContentReferenceKind::SceneInstance => "sceneInstance",
        ProjectContentReferenceKind::Prefab => "prefab",
        ProjectContentReferenceKind::PrefabPart => "prefabPart",
        ProjectContentReferenceKind::PresentationResource => "presentationResource",
    }
}

fn diagnostic_code_tag(value: ProjectContentDiagnosticCode) -> &'static str {
    match value {
        ProjectContentDiagnosticCode::InvalidJson => "invalidJson",
        ProjectContentDiagnosticCode::UnknownField => "unknownField",
        ProjectContentDiagnosticCode::InvalidField => "invalidField",
        ProjectContentDiagnosticCode::DuplicateDocument => "duplicateDocument",
        ProjectContentDiagnosticCode::InvalidDocument => "invalidDocument",
        ProjectContentDiagnosticCode::UnknownReference => "unknownReference",
        ProjectContentDiagnosticCode::ReferenceKindMismatch => "referenceKindMismatch",
        ProjectContentDiagnosticCode::StaleRevision => "staleRevision",
    }
}

#[napi]
pub fn decode_project_content(handle: i64, request_json: String) -> napi::Result<String> {
    let request = parse_wire_json::<DecodeRequestJson>("decode_project_content", &request_json)?;
    with_bridge(handle, |bridge| {
        let result = bridge
            .decode_project_content(ProjectContentDecodeRequestDto {
                sources: request
                    .sources
                    .into_iter()
                    .map(|source| ProjectContentSourceDto {
                        source_path: source.source_path,
                        document_id: source.document_id,
                        kind: source.kind.into(),
                        source_text: source.source_text,
                    })
                    .collect(),
            })
            .map_err(to_napi)?;
        encode(codec_result_json(&result)?, "project-content decode")
    })
}

#[napi]
pub fn encode_project_content(handle: i64, request_json: String) -> napi::Result<String> {
    let request = parse_wire_json::<EncodeRequestJson>("encode_project_content", &request_json)?;
    with_bridge(handle, |bridge| {
        let decoded = match decode_document_values(&request.documents)? {
            Ok(documents) => documents,
            Err(diagnostics) => {
                let rejection = bridge
                    .reject_project_content_parse(diagnostics)
                    .map_err(to_napi)?;
                return encode(codec_result_json(&rejection)?, "project-content encode");
            }
        };
        let result = bridge
            .encode_project_content(ProjectContentEncodeRequestDto { documents: decoded })
            .map_err(to_napi)?;
        encode(codec_result_json(&result)?, "project-content encode")
    })
}

#[napi]
pub fn apply_project_content_authoring(handle: i64, request_json: String) -> napi::Result<String> {
    let request =
        parse_wire_json::<AuthoringRequestJson>("apply_project_content_authoring", &request_json)?;
    with_bridge(handle, |bridge| {
        let command = match request.command {
            AuthoringCommandJson::Delete {
                document_id,
                document_kind,
            } => ProjectContentAuthoringCommandDto::Delete {
                document_id,
                document_kind: document_kind.into(),
            },
            AuthoringCommandJson::Upsert {
                source_path,
                document,
            } => {
                let source = source_from_document(&document)?;
                let parsed =
                    EngineBridge::decode_project_content_sources(std::slice::from_ref(&source));
                let document = match parsed {
                    Ok(documents) => documents,
                    Err(diagnostics) => {
                        let rejection = bridge
                            .reject_project_content_parse(diagnostics)
                            .map_err(to_napi)?;
                        return encode(codec_result_json(&rejection)?, "project-content authoring");
                    }
                }
                .into_iter()
                .find(|document| {
                    document.kind() == source.kind && document.document_id() == source.document_id
                })
                .ok_or_else(|| {
                    napi::Error::from_reason(
                        "accepted upsert document was not returned by Rust".to_owned(),
                    )
                })?;
                ProjectContentAuthoringCommandDto::Upsert {
                    source_path,
                    document,
                }
            }
        };
        let result = bridge
            .apply_project_content_authoring(ProjectContentAuthoringRequestDto {
                expected_workspace_id: request.expected_workspace_id,
                expected_generation: request.expected_generation,
                expected_working_revision: request.expected_working_revision,
                expected_set_hash: request.expected_set_hash,
                command,
            })
            .map_err(to_napi)?;
        encode(authoring_result_json(&result)?, "project-content authoring")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn canonical_file(canonical_json: String) -> ProjectContentCanonicalFileDto {
        ProjectContentCanonicalFileDto {
            source_path: Some("content/catalog.json".to_owned()),
            document_id: "catalog/test".to_owned(),
            kind: ProjectContentDocumentKind::AssetCatalog,
            canonical_json,
            content_hash: "fnv1a64:test".to_owned(),
        }
    }

    #[test]
    fn canonical_document_json_exposes_only_the_typed_document_body() {
        let value = document_json(&canonical_file(
            json!({
                "schemaVersion": PROJECT_CONTENT_SCHEMA_VERSION,
                "documentId": "catalog/test",
                "documentKind": "assetCatalog",
                "document": { "entries": [] },
            })
            .to_string(),
        ))
        .expect("canonical artifact should convert");

        assert_eq!(value["kind"], "assetCatalog");
        assert_eq!(value["documentId"], "catalog/test");
        assert_eq!(value["catalog"]["entries"], json!([]));
        assert!(value.get("schemaVersion").is_none());
        assert!(value["catalog"].get("document").is_none());
    }

    #[test]
    fn canonical_document_json_rejects_envelope_identity_drift() {
        let result = document_json(&canonical_file(
            json!({
                "schemaVersion": PROJECT_CONTENT_SCHEMA_VERSION,
                "documentId": "catalog/other",
                "documentKind": "assetCatalog",
                "document": {},
            })
            .to_string(),
        ));

        assert!(result.is_err());
    }

    #[test]
    fn asset_catalog_boundary_translates_canonical_color_arrays_to_public_rgba_objects() {
        let value = document_json(&canonical_file(
            json!({
                "schemaVersion": PROJECT_CONTENT_SCHEMA_VERSION,
                "documentId": "catalog/test",
                "documentKind": "assetCatalog",
                "document": {
                    "entries": [{
                        "id": "material/test",
                        "version": 1,
                        "hash": null,
                        "sourcePath": null,
                        "label": "Test",
                        "dependencies": [],
                        "material": {
                            "authority": {
                                "solid": true,
                                "collidable": true,
                                "occludes": true,
                                "structuralClass": "structural"
                            },
                            "style": {
                                "color": [0.1, 0.2, 0.3, 1.0],
                                "texture": null,
                                "roughness": 0.8,
                                "textureTint": [1.0, 1.0, 1.0, 1.0],
                                "emissionColor": [0.0, 0.0, 0.0, 1.0],
                                "emissive": 0.0,
                                "uvStrategy": "flat"
                            }
                        }
                    }]
                },
            })
            .to_string(),
        ))
        .expect("canonical material should convert");

        assert_eq!(
            value["catalog"]["entries"][0]["material"]["style"]["color"],
            json!({ "r": 0.1, "g": 0.2, "b": 0.3, "a": 1.0 })
        );
        let source = source_from_document(&value).expect("public material should encode");
        let source: Value = serde_json::from_str(&source.source_text).expect("stored source json");
        assert_eq!(
            source["entries"][0]["material"]["style"]["color"],
            json!([0.1, 0.2, 0.3, 1.0])
        );
    }
}
