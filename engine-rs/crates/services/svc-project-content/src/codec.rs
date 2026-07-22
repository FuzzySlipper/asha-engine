use core_assets::{AssetHash, AssetId, AssetReference as CoreAssetReference, AssetVersionReq};
use core_ids::{PrefabId, PrefabPartId, TagId};
use protocol_assets::{
    Rgba as ProtocolRgba, StoredAssetCatalog, StoredAssetReference, StoredAssetVersionRequirement,
    StoredCatalogEntry, StoredMaterialAuthority, StoredMaterialDefinition, StoredMaterialStyle,
};
use protocol_entity_authoring::{
    AuthoringTransform, EntityAppearanceBinding, EntityDefinition, EntityDefinitionCapability,
    EntityDefinitionMetadataEntry, EntityDefinitionSourceTrace,
};
use protocol_project_bundle::{
    PrefabDefinition as ProtocolPrefabDefinition, PrefabOverride as ProtocolPrefabOverride,
    PrefabOverrideValue as ProtocolPrefabOverrideValue, PrefabPart as ProtocolPrefabPart,
    PrefabPartRoleBinding as ProtocolPrefabPartRoleBinding,
    PrefabPartSource as ProtocolPrefabPartSource, PrefabRegistry as ProtocolPrefabRegistry,
    PrefabTransform as ProtocolPrefabTransform, PrefabVariantDelta as ProtocolPrefabVariantDelta,
};
use protocol_project_content::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use svc_serialization::{
    PrefabDefinition, PrefabOverride, PrefabOverrideValue, PrefabPart, PrefabPartRoleBinding,
    PrefabPartSource, PrefabRegistry, PrefabRegistryValidationContext, PrefabTransform,
    PrefabVariantDelta, ValidatedPrefabRegistry, PREFAB_REGISTRY_SCHEMA_VERSION,
};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum ProjectContentDocumentKindWire {
    EntityDefinition,
    AssetCatalog,
    PrefabRegistry,
    GameplayConfiguration,
    PresentationCatalog,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ProjectContentArtifactWire {
    schema_version: u32,
    document_id: String,
    document_kind: ProjectContentDocumentKindWire,
    document: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct EntityDefinitionWire {
    kind: String,
    stable_id: String,
    display_name: String,
    source: EntityDefinitionSourceWire,
    tags: Vec<u64>,
    metadata: Vec<EntityDefinitionMetadataWire>,
    capabilities: Vec<EntityDefinitionCapabilityWire>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct EntityDefinitionSourceWire {
    project_bundle: String,
    relative_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct EntityDefinitionMetadataWire {
    key: String,
    value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
enum EntityDefinitionCapabilityWire {
    Transform {
        transform: TransformWire,
    },
    Render {
        visible: bool,
    },
    Collision {
        static_collider: bool,
    },
    Bounds {
        min: [f32; 3],
        max: [f32; 3],
    },
    Controller {
        controller_id: String,
    },
    Health {
        current: u32,
        max: u32,
    },
    WeaponMount {
        weapon_id: String,
        damage: u32,
        range_units: u32,
        ammo: u32,
        cooldown_ticks_after_fire: u32,
    },
    RenderProjection {
        projection_id: String,
        visible: bool,
        appearance: Option<EntityAppearanceBindingWire>,
    },
    PolicyBinding {
        binding_id: String,
        policy_id: String,
        view_kind: String,
        view_version: String,
        allowed_intents: Vec<String>,
        runtime_moment: String,
    },
    SpawnMarker {
        marker_id: String,
    },
    Faction {
        faction_id: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct EntityAppearanceBindingWire {
    resource_id: String,
    initial_clip_id: Option<String>,
    model_scale: [f32; 3],
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TransformWire {
    translation: [f32; 3],
    rotation: [f32; 4],
    scale: [f32; 3],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PrefabRegistryWire {
    schema_version: u32,
    definitions: Vec<PrefabDefinitionWire>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PrefabDefinitionWire {
    id: u64,
    schema_version: u32,
    display_name: String,
    parts: Vec<PrefabPartWire>,
    part_roles: Vec<PrefabPartRoleWire>,
    variant: Option<PrefabVariantWire>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PrefabPartWire {
    id: u64,
    namespace: String,
    display_name: String,
    parent: Option<u64>,
    transform: TransformWire,
    source: PrefabPartSourceWire,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
enum PrefabPartSourceWire {
    Scene { asset: String },
    EntityDefinition { stable_id: String },
    VoxelObject { asset: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PrefabPartRoleWire {
    role: String,
    part: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PrefabVariantWire {
    variant_id: String,
    base: u64,
    removed_roles: Vec<String>,
    overrides: Vec<PrefabOverrideWire>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PrefabOverrideWire {
    target_role: String,
    value: PrefabOverrideValueWire,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    tag = "field",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
enum PrefabOverrideValueWire {
    Transform { transform: TransformWire },
    EntityDefinition { stable_id: String },
    Asset { asset: String },
    Material { asset: String },
    Activation { active: bool },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct GameplayDocumentWire {
    schema_version: u32,
    configurations: Vec<GameplayConfigurationWire>,
    bindings: Vec<protocol_game_extension::GameplayModuleBinding>,
    overrides: Vec<protocol_game_extension::GameplayModuleBindingOverride>,
    triggers: Vec<protocol_project_bundle::GameplayTriggerDefinition>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum ReferenceKindWire {
    Asset,
    EntityDefinition,
    InstantiatedEntityDefinition,
    InstantiatedBoundedEntityDefinition,
    EntrySceneFpsPlayerEntityDefinition,
    SceneInstance,
    Prefab,
    PrefabPart,
    PresentationResource,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct GameplayConfigurationWire {
    configuration_id: String,
    module: protocol_game_extension::GameplayModuleRef,
    schema_id: String,
    values: Vec<ConfigurationFieldValueWire>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ConfigurationFieldValueWire {
    field_id: String,
    value: ConfigurationValueWire,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
enum ConfigurationValueWire {
    Boolean {
        value: bool,
    },
    Integer {
        value: i64,
    },
    Number {
        value: f64,
    },
    String {
        value: String,
    },
    Reference {
        reference_kind: ReferenceKindWire,
        target_id: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PresentationCatalogWire {
    schema_version: u32,
    resources: Vec<PresentationResourceWire>,
    cues: Vec<PresentationCueWire>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PresentationResourceWire {
    resource_id: String,
    kind: PresentationResourceKindWire,
    asset_id: String,
    source_path: String,
    content_hash: String,
    license_path: Option<String>,
    animated_mesh: Option<AnimatedMeshAssetWire>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AnimatedMeshAssetWire {
    asset: String,
    runtime_format: AnimatedMeshRuntimeFormatWire,
    content_hash: Option<String>,
    clips: Vec<AnimationClipDescriptorWire>,
    default_clip: Option<String>,
    material_slots: Vec<MeshMaterialSlotWire>,
    bounds: MeshBoundsDescriptorWire,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum AnimatedMeshRuntimeFormatWire {
    Glb,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AnimationClipDescriptorWire {
    id: String,
    name: Option<String>,
    duration_seconds: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MeshMaterialSlotWire {
    slot: u16,
    material: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MeshBoundsDescriptorWire {
    min: [f32; 3],
    max: [f32; 3],
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum PresentationResourceKindWire {
    AnimatedMesh,
    Audio,
    Particle,
    Font,
    Overlay,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum PresentationSignalDomainWire {
    Audio,
    Particle,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PresentationSignalWire {
    domain: PresentationSignalDomainWire,
    signal_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
enum PresentationCueWire {
    Animation {
        cue_id: String,
        resource_id: String,
        clip_id: String,
        looped: bool,
        at_seconds: f32,
        signal: PresentationSignalWire,
    },
    Audio {
        cue_id: String,
        signal_id: String,
        resource_id: String,
        gain: f32,
    },
    Particle {
        cue_id: String,
        signal_id: String,
        resource_id: String,
        scale: f32,
    },
    Overlay {
        cue_id: String,
        resource_id: String,
    },
}

pub(super) fn decode_sources(
    sources: &[ProjectContentSourceDto],
) -> Result<Vec<ProjectContentDocumentDto>, Vec<ProjectContentDiagnosticDto>> {
    let mut documents = Vec::with_capacity(sources.len());
    let mut diagnostics = Vec::new();
    for source in sources {
        match decode_source(source) {
            Ok(document) => documents.push(document),
            Err(error) => diagnostics.push(json_diagnostic(source, error)),
        }
    }
    if diagnostics.is_empty() {
        Ok(documents)
    } else {
        Err(diagnostics)
    }
}

fn decode_source(source: &ProjectContentSourceDto) -> Result<ProjectContentDocumentDto, String> {
    if let Ok(artifact) = strict_json::<ProjectContentArtifactWire>(&source.source_text) {
        if artifact.schema_version != PROJECT_CONTENT_SCHEMA_VERSION {
            return Err(format!(
                "unsupported project-content artifact schema version {}",
                artifact.schema_version
            ));
        }
        let artifact_kind: ProjectContentDocumentKind = artifact.document_kind.into();
        if artifact.document_id != source.document_id || artifact_kind != source.kind {
            return Err(
                "project-content artifact identity does not match the requested source identity"
                    .to_owned(),
            );
        }
        let inner = artifact_document_source(artifact_kind, artifact.document)?;
        return decode_source(&ProjectContentSourceDto {
            source_path: source.source_path.clone(),
            document_id: artifact.document_id,
            kind: artifact_kind,
            source_text: inner,
        });
    }
    match source.kind {
        ProjectContentDocumentKind::EntityDefinition => {
            let wire: EntityDefinitionWire = strict_json(&source.source_text)?;
            if wire.kind != "EntityDefinition" {
                return Err("field `kind` must be `EntityDefinition`".to_owned());
            }
            Ok(ProjectContentDocumentDto::EntityDefinition {
                document_id: source.document_id.clone(),
                definition: wire.into(),
            })
        }
        ProjectContentDocumentKind::AssetCatalog => {
            let catalog = core_catalog::decode(&source.source_text).map_err(|e| e.to_string())?;
            Ok(ProjectContentDocumentDto::AssetCatalog {
                document_id: source.document_id.clone(),
                catalog: stored_catalog_from_core(&catalog),
            })
        }
        ProjectContentDocumentKind::PrefabRegistry => {
            let wire: PrefabRegistryWire = strict_json(&source.source_text)?;
            Ok(ProjectContentDocumentDto::PrefabRegistry {
                document_id: source.document_id.clone(),
                registry: wire.into(),
            })
        }
        ProjectContentDocumentKind::GameplayConfiguration => {
            let wire: GameplayDocumentWire = strict_json(&source.source_text)?;
            Ok(ProjectContentDocumentDto::GameplayConfiguration {
                document_id: source.document_id.clone(),
                document: wire.into(),
            })
        }
        ProjectContentDocumentKind::PresentationCatalog => {
            let wire: PresentationCatalogWire = strict_json(&source.source_text)?;
            Ok(ProjectContentDocumentDto::PresentationCatalog {
                document_id: source.document_id.clone(),
                catalog: wire.into(),
            })
        }
    }
}

pub(super) fn decode_artifact(
    source_path: &str,
    body: &[u8],
) -> Result<ProjectContentDocumentDto, ProjectContentDiagnosticDto> {
    let source_text = std::str::from_utf8(body).map_err(|error| ProjectContentDiagnosticDto {
        code: ProjectContentDiagnosticCode::InvalidJson,
        document_id: None,
        path: source_path.to_owned(),
        message: format!("project-content artifact is not UTF-8: {error}"),
    })?;
    let artifact = strict_json::<ProjectContentArtifactWire>(source_text).map_err(|message| {
        ProjectContentDiagnosticDto {
            code: if message.contains("unknown field") {
                ProjectContentDiagnosticCode::UnknownField
            } else {
                ProjectContentDiagnosticCode::InvalidJson
            },
            document_id: None,
            path: source_path.to_owned(),
            message,
        }
    })?;
    if artifact.schema_version != PROJECT_CONTENT_SCHEMA_VERSION {
        return Err(ProjectContentDiagnosticDto {
            code: ProjectContentDiagnosticCode::InvalidField,
            document_id: Some(artifact.document_id),
            path: format!("{source_path}.schemaVersion"),
            message: format!(
                "unsupported project-content artifact schema version {}; expected {}",
                artifact.schema_version, PROJECT_CONTENT_SCHEMA_VERSION
            ),
        });
    }
    let document_id = artifact.document_id;
    let document_kind = ProjectContentDocumentKind::from(artifact.document_kind);
    let source = ProjectContentSourceDto {
        source_path: source_path.to_owned(),
        document_id: document_id.clone(),
        kind: document_kind,
        source_text: artifact_document_source(document_kind, artifact.document).map_err(
            |message| ProjectContentDiagnosticDto {
                code: ProjectContentDiagnosticCode::InvalidJson,
                document_id: Some(document_id.clone()),
                path: format!("{source_path}.document"),
                message,
            },
        )?,
    };
    decode_source(&source).map_err(|message| ProjectContentDiagnosticDto {
        code: if message.contains("unknown field") {
            ProjectContentDiagnosticCode::UnknownField
        } else {
            ProjectContentDiagnosticCode::InvalidDocument
        },
        document_id: Some(document_id),
        path: format!("{source_path}.document"),
        message,
    })
}

fn strict_json<T: for<'de> Deserialize<'de>>(source: &str) -> Result<T, String> {
    let mut deserializer = serde_json::Deserializer::from_str(source);
    let value = T::deserialize(&mut deserializer).map_err(|error| error.to_string())?;
    deserializer.end().map_err(|error| error.to_string())?;
    Ok(value)
}

fn json_diagnostic(source: &ProjectContentSourceDto, error: String) -> ProjectContentDiagnosticDto {
    let code = if error.contains("unknown field") {
        ProjectContentDiagnosticCode::UnknownField
    } else if error.contains("JSON") || error.contains("line") || error.contains("column") {
        ProjectContentDiagnosticCode::InvalidJson
    } else {
        ProjectContentDiagnosticCode::InvalidField
    };
    ProjectContentDiagnosticDto {
        code,
        document_id: Some(source.document_id.clone()),
        path: "sourceText".to_owned(),
        message: error,
    }
}

pub(super) fn canonical_files(
    documents: &[ProjectContentDocumentDto],
) -> Result<Vec<ProjectContentCanonicalFileDto>, Vec<ProjectContentDiagnosticDto>> {
    let mut files = Vec::with_capacity(documents.len());
    let mut diagnostics = Vec::new();
    for document in documents {
        match canonical_artifact(document) {
            Ok(canonical_json) => files.push(ProjectContentCanonicalFileDto {
                source_path: None,
                document_id: document.document_id().to_owned(),
                kind: document.kind(),
                content_hash: content_hash(&canonical_json),
                canonical_json,
            }),
            Err(message) => diagnostics.push(ProjectContentDiagnosticDto {
                code: ProjectContentDiagnosticCode::InvalidDocument,
                document_id: Some(document.document_id().to_owned()),
                path: "document".to_owned(),
                message,
            }),
        }
    }
    if diagnostics.is_empty() {
        files.sort_by(|left, right| left.document_id.cmp(&right.document_id));
        Ok(files)
    } else {
        Err(diagnostics)
    }
}

pub(super) fn compiled_prefab_registry(
    documents: &[ProjectContentDocumentDto],
) -> Result<ValidatedPrefabRegistry, svc_serialization::PrefabValidationReport> {
    let mut definitions = Vec::new();
    let mut asset_ids = BTreeSet::new();
    let mut entity_definition_ids = BTreeSet::new();
    for document in documents {
        match document {
            ProjectContentDocumentDto::PrefabRegistry { registry, .. } => {
                definitions.extend(serialization_prefab_registry(registry).definitions);
            }
            ProjectContentDocumentDto::AssetCatalog { catalog, .. } => {
                asset_ids.extend(catalog.entries.iter().map(|entry| entry.id.clone()));
            }
            ProjectContentDocumentDto::EntityDefinition { definition, .. } => {
                entity_definition_ids.insert(definition.stable_id.clone());
            }
            _ => {}
        }
    }
    ValidatedPrefabRegistry::new(
        PrefabRegistry {
            schema_version: PREFAB_REGISTRY_SCHEMA_VERSION,
            definitions,
        },
        &PrefabRegistryValidationContext {
            asset_ids,
            entity_definition_ids,
        },
    )
}

fn canonical_artifact(document: &ProjectContentDocumentDto) -> Result<String, String> {
    let canonical_document = canonical_document(document)?;
    let mut document_value: serde_json::Value = serde_json::from_str(&canonical_document)
        .map_err(|error| format!("canonical document could not be enveloped: {error}"))?;
    if document.kind() == ProjectContentDocumentKind::EntityDefinition {
        document_value
            .as_object_mut()
            .ok_or_else(|| "entity definition canonical body is not an object".to_owned())?
            .remove("kind");
    }
    pretty(&ProjectContentArtifactWire {
        schema_version: PROJECT_CONTENT_SCHEMA_VERSION,
        document_id: document.document_id().to_owned(),
        document_kind: document.kind().into(),
        document: document_value,
    })
}

fn artifact_document_source(
    kind: ProjectContentDocumentKind,
    mut document: serde_json::Value,
) -> Result<String, String> {
    if kind == ProjectContentDocumentKind::EntityDefinition {
        let object = document
            .as_object_mut()
            .ok_or_else(|| "entity definition artifact body must be an object".to_owned())?;
        if object
            .insert(
                "kind".to_owned(),
                serde_json::Value::String("EntityDefinition".to_owned()),
            )
            .is_some()
        {
            return Err(
                "entity definition artifact body repeats the envelope document kind".to_owned(),
            );
        }
    }
    serde_json::to_string(&document).map_err(|error| error.to_string())
}

fn canonical_document(document: &ProjectContentDocumentDto) -> Result<String, String> {
    match document {
        ProjectContentDocumentDto::EntityDefinition { definition, .. } => {
            pretty(&EntityDefinitionWire::try_from(definition.clone())?)
        }
        ProjectContentDocumentDto::AssetCatalog { catalog, .. } => {
            let core = core_catalog_from_stored(catalog)?;
            Ok(core_catalog::encode(&core))
        }
        ProjectContentDocumentDto::PrefabRegistry { registry, .. } => {
            let wire = PrefabRegistryWire::from(registry.clone());
            pretty(&wire)
        }
        ProjectContentDocumentDto::GameplayConfiguration { document, .. } => {
            let mut canonical = document.clone();
            canonical
                .configurations
                .sort_by(|a, b| a.configuration_id.cmp(&b.configuration_id));
            for configuration in &mut canonical.configurations {
                configuration
                    .values
                    .sort_by(|a, b| a.field_id.cmp(&b.field_id));
            }
            canonical
                .bindings
                .sort_by(|a, b| a.binding_id.cmp(&b.binding_id));
            canonical.overrides.sort_by(|a, b| {
                (a.binding_id.as_str(), a.scene_instance_id.as_str())
                    .cmp(&(b.binding_id.as_str(), b.scene_instance_id.as_str()))
            });
            canonical
                .triggers
                .sort_by(|a, b| a.scene_instance_id.cmp(&b.scene_instance_id));
            pretty(&GameplayDocumentWire::from(canonical))
        }
        ProjectContentDocumentDto::PresentationCatalog { catalog, .. } => {
            let mut canonical = catalog.clone();
            canonical
                .resources
                .sort_by(|a, b| a.resource_id.cmp(&b.resource_id));
            canonical.cues.sort_by(|a, b| cue_id(a).cmp(cue_id(b)));
            pretty(&PresentationCatalogWire::from(canonical))
        }
    }
}

fn pretty<T: Serialize>(value: &T) -> Result<String, String> {
    let mut encoded = serde_json::to_string_pretty(value).map_err(|error| error.to_string())?;
    encoded.push('\n');
    Ok(encoded)
}

fn cue_id(cue: &ProjectPresentationCueDto) -> &str {
    match cue {
        ProjectPresentationCueDto::Animation { cue_id, .. }
        | ProjectPresentationCueDto::Audio { cue_id, .. }
        | ProjectPresentationCueDto::Particle { cue_id, .. }
        | ProjectPresentationCueDto::Overlay { cue_id, .. } => cue_id,
    }
}

pub(super) fn document_set_hash(files: &[ProjectContentCanonicalFileDto]) -> String {
    let mut key = String::from("project-content-set-v1");
    for file in files {
        key.push('|');
        key.push_str(file.source_path.as_deref().unwrap_or("-"));
        key.push('|');
        key.push_str(&file.document_id);
        key.push('|');
        key.push_str(&file.content_hash);
    }
    content_hash(&key)
}

fn content_hash(value: &str) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("fnv1a64:{hash:016x}")
}

impl From<ProjectContentDocumentKind> for ProjectContentDocumentKindWire {
    fn from(value: ProjectContentDocumentKind) -> Self {
        match value {
            ProjectContentDocumentKind::EntityDefinition => Self::EntityDefinition,
            ProjectContentDocumentKind::AssetCatalog => Self::AssetCatalog,
            ProjectContentDocumentKind::PrefabRegistry => Self::PrefabRegistry,
            ProjectContentDocumentKind::GameplayConfiguration => Self::GameplayConfiguration,
            ProjectContentDocumentKind::PresentationCatalog => Self::PresentationCatalog,
        }
    }
}

impl From<ProjectContentDocumentKindWire> for ProjectContentDocumentKind {
    fn from(value: ProjectContentDocumentKindWire) -> Self {
        match value {
            ProjectContentDocumentKindWire::EntityDefinition => Self::EntityDefinition,
            ProjectContentDocumentKindWire::AssetCatalog => Self::AssetCatalog,
            ProjectContentDocumentKindWire::PrefabRegistry => Self::PrefabRegistry,
            ProjectContentDocumentKindWire::GameplayConfiguration => Self::GameplayConfiguration,
            ProjectContentDocumentKindWire::PresentationCatalog => Self::PresentationCatalog,
        }
    }
}

impl From<EntityDefinitionWire> for EntityDefinition {
    fn from(wire: EntityDefinitionWire) -> Self {
        Self {
            stable_id: wire.stable_id,
            display_name: wire.display_name,
            source: EntityDefinitionSourceTrace {
                project_bundle: wire.source.project_bundle,
                relative_path: wire.source.relative_path,
            },
            tags: wire.tags.into_iter().map(TagId::new).collect(),
            metadata: wire
                .metadata
                .into_iter()
                .map(|entry| EntityDefinitionMetadataEntry {
                    key: entry.key,
                    value: entry.value,
                })
                .collect(),
            capabilities: wire.capabilities.into_iter().map(Into::into).collect(),
        }
    }
}

impl TryFrom<EntityDefinition> for EntityDefinitionWire {
    type Error = String;

    fn try_from(mut definition: EntityDefinition) -> Result<Self, Self::Error> {
        definition.tags.sort_by_key(|tag| tag.raw());
        definition.metadata.sort_by(|a, b| a.key.cmp(&b.key));
        definition
            .capabilities
            .sort_by(|a, b| a.kind().cmp(b.kind()));
        let capabilities = definition
            .capabilities
            .into_iter()
            .map(EntityDefinitionCapabilityWire::try_from)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            kind: "EntityDefinition".to_owned(),
            stable_id: definition.stable_id,
            display_name: definition.display_name,
            source: EntityDefinitionSourceWire {
                project_bundle: definition.source.project_bundle,
                relative_path: definition.source.relative_path,
            },
            tags: definition.tags.into_iter().map(TagId::raw).collect(),
            metadata: definition
                .metadata
                .into_iter()
                .map(|entry| EntityDefinitionMetadataWire {
                    key: entry.key,
                    value: entry.value,
                })
                .collect(),
            capabilities,
        })
    }
}

impl From<EntityDefinitionCapabilityWire> for EntityDefinitionCapability {
    fn from(wire: EntityDefinitionCapabilityWire) -> Self {
        match wire {
            EntityDefinitionCapabilityWire::Transform { transform } => Self::Transform {
                transform: transform.into(),
            },
            EntityDefinitionCapabilityWire::Render { visible } => Self::Render { visible },
            EntityDefinitionCapabilityWire::Collision { static_collider } => {
                Self::Collision { static_collider }
            }
            EntityDefinitionCapabilityWire::Bounds { min, max } => Self::Bounds { min, max },
            EntityDefinitionCapabilityWire::Controller { controller_id } => {
                Self::Controller { controller_id }
            }
            EntityDefinitionCapabilityWire::Health { current, max } => {
                Self::Health { current, max }
            }
            EntityDefinitionCapabilityWire::WeaponMount {
                weapon_id,
                damage,
                range_units,
                ammo,
                cooldown_ticks_after_fire,
            } => Self::WeaponMount {
                weapon_id,
                damage,
                range_units,
                ammo,
                cooldown_ticks_after_fire,
            },
            EntityDefinitionCapabilityWire::RenderProjection {
                projection_id,
                visible,
                appearance,
            } => Self::RenderProjection {
                projection_id,
                visible,
                appearance: appearance.map(|binding| EntityAppearanceBinding {
                    resource_id: binding.resource_id,
                    initial_clip_id: binding.initial_clip_id,
                    model_scale: binding.model_scale,
                }),
            },
            EntityDefinitionCapabilityWire::PolicyBinding {
                binding_id,
                policy_id,
                view_kind,
                view_version,
                allowed_intents,
                runtime_moment,
            } => Self::PolicyBinding {
                binding_id,
                policy_id,
                view_kind,
                view_version,
                allowed_intents,
                runtime_moment,
            },
            EntityDefinitionCapabilityWire::SpawnMarker { marker_id } => {
                Self::SpawnMarker { marker_id }
            }
            EntityDefinitionCapabilityWire::Faction { faction_id } => Self::Faction { faction_id },
        }
    }
}

impl TryFrom<EntityDefinitionCapability> for EntityDefinitionCapabilityWire {
    type Error = String;

    fn try_from(value: EntityDefinitionCapability) -> Result<Self, Self::Error> {
        Ok(match value {
            EntityDefinitionCapability::Transform { transform } => Self::Transform {
                transform: transform.into(),
            },
            EntityDefinitionCapability::Render { visible } => Self::Render { visible },
            EntityDefinitionCapability::Collision { static_collider } => {
                Self::Collision { static_collider }
            }
            EntityDefinitionCapability::Bounds { min, max } => Self::Bounds { min, max },
            EntityDefinitionCapability::Controller { controller_id } => {
                Self::Controller { controller_id }
            }
            EntityDefinitionCapability::Health { current, max } => Self::Health { current, max },
            EntityDefinitionCapability::WeaponMount {
                weapon_id,
                damage,
                range_units,
                ammo,
                cooldown_ticks_after_fire,
            } => Self::WeaponMount {
                weapon_id,
                damage,
                range_units,
                ammo,
                cooldown_ticks_after_fire,
            },
            EntityDefinitionCapability::RenderProjection {
                projection_id,
                visible,
                appearance,
            } => Self::RenderProjection {
                projection_id,
                visible,
                appearance: appearance.map(|binding| EntityAppearanceBindingWire {
                    resource_id: binding.resource_id,
                    initial_clip_id: binding.initial_clip_id,
                    model_scale: binding.model_scale,
                }),
            },
            EntityDefinitionCapability::PolicyBinding {
                binding_id,
                policy_id,
                view_kind,
                view_version,
                allowed_intents,
                runtime_moment,
            } => Self::PolicyBinding {
                binding_id,
                policy_id,
                view_kind,
                view_version,
                allowed_intents,
                runtime_moment,
            },
            EntityDefinitionCapability::SpawnMarker { marker_id } => {
                Self::SpawnMarker { marker_id }
            }
            EntityDefinitionCapability::Faction { faction_id } => Self::Faction { faction_id },
            EntityDefinitionCapability::Unknown { capability_kind } => {
                return Err(format!(
                    "unknown entity definition capability `{capability_kind}` cannot be encoded"
                ));
            }
        })
    }
}

impl From<TransformWire> for AuthoringTransform {
    fn from(value: TransformWire) -> Self {
        Self {
            translation: value.translation,
            rotation: value.rotation,
            scale: value.scale,
        }
    }
}

impl From<AuthoringTransform> for TransformWire {
    fn from(value: AuthoringTransform) -> Self {
        Self {
            translation: value.translation,
            rotation: value.rotation,
            scale: value.scale,
        }
    }
}

impl From<ProtocolPrefabTransform> for TransformWire {
    fn from(value: ProtocolPrefabTransform) -> Self {
        Self {
            translation: value.translation,
            rotation: value.rotation,
            scale: value.scale,
        }
    }
}

impl From<TransformWire> for ProtocolPrefabTransform {
    fn from(value: TransformWire) -> Self {
        Self {
            translation: value.translation,
            rotation: value.rotation,
            scale: value.scale,
        }
    }
}

fn stored_catalog_from_core(catalog: &core_catalog::Catalog) -> StoredAssetCatalog {
    StoredAssetCatalog {
        entries: catalog
            .canonical()
            .entries
            .into_iter()
            .map(|entry| StoredCatalogEntry {
                id: entry.id.as_str().to_owned(),
                version: entry.version,
                hash: entry.hash.map(|hash| hash.as_str().to_owned()),
                source_path: entry.source_path,
                label: entry.label,
                dependencies: entry
                    .dependencies
                    .iter()
                    .map(stored_reference_from_core)
                    .collect(),
                material: entry.material.map(|material| StoredMaterialDefinition {
                    authority: StoredMaterialAuthority {
                        solid: material.authority.solid,
                        collidable: material.authority.collidable,
                        occludes: material.authority.occludes,
                        structural_class: match material.authority.structural_class {
                            core_catalog::StructuralClass::Decorative => "decorative",
                            core_catalog::StructuralClass::Solid => "solid",
                            core_catalog::StructuralClass::Structural => "structural",
                        }
                        .to_owned(),
                    },
                    style: StoredMaterialStyle {
                        color: protocol_rgba(material.style.color),
                        texture: material
                            .style
                            .texture
                            .as_ref()
                            .map(stored_reference_from_core),
                        roughness: material.style.roughness,
                        texture_tint: protocol_rgba(material.style.texture_tint),
                        emission_color: protocol_rgba(material.style.emission_color),
                        emissive: material.style.emissive,
                        uv_strategy: match material.style.uv_strategy {
                            core_catalog::UvStrategy::Flat => "flat",
                            core_catalog::UvStrategy::Planar => "planar",
                            core_catalog::UvStrategy::Atlas => "atlas",
                        }
                        .to_owned(),
                    },
                }),
            })
            .collect(),
    }
}

pub(super) fn core_catalog_from_stored(
    catalog: &StoredAssetCatalog,
) -> Result<core_catalog::Catalog, String> {
    let entries = catalog
        .entries
        .iter()
        .map(|entry| {
            let id = AssetId::parse(&entry.id).map_err(|error| error.to_string())?;
            let hash = entry
                .hash
                .as_deref()
                .map(AssetHash::parse)
                .transpose()
                .map_err(|error| error.to_string())?;
            let dependencies = entry
                .dependencies
                .iter()
                .map(core_reference_from_stored)
                .collect::<Result<Vec<_>, _>>()?;
            let material = entry
                .material
                .as_ref()
                .map(|material| {
                    Ok(core_catalog::MaterialDef {
                        authority: core_catalog::MaterialAuthority {
                            solid: material.authority.solid,
                            collidable: material.authority.collidable,
                            occludes: material.authority.occludes,
                            structural_class: match material.authority.structural_class.as_str() {
                                "decorative" => core_catalog::StructuralClass::Decorative,
                                "solid" => core_catalog::StructuralClass::Solid,
                                "structural" => core_catalog::StructuralClass::Structural,
                                other => return Err(format!("unknown structuralClass `{other}`")),
                            },
                        },
                        style: core_catalog::MaterialStyle {
                            color: core_rgba(material.style.color),
                            texture: material
                                .style
                                .texture
                                .as_ref()
                                .map(core_reference_from_stored)
                                .transpose()?,
                            roughness: material.style.roughness,
                            texture_tint: core_rgba(material.style.texture_tint),
                            emission_color: core_rgba(material.style.emission_color),
                            emissive: material.style.emissive,
                            uv_strategy: match material.style.uv_strategy.as_str() {
                                "flat" => core_catalog::UvStrategy::Flat,
                                "planar" => core_catalog::UvStrategy::Planar,
                                "atlas" => core_catalog::UvStrategy::Atlas,
                                other => return Err(format!("unknown uvStrategy `{other}`")),
                            },
                        },
                    })
                })
                .transpose()?;
            Ok(core_catalog::CatalogEntry {
                id,
                version: entry.version,
                hash,
                source_path: entry.source_path.clone(),
                label: entry.label.clone(),
                dependencies,
                material,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(core_catalog::Catalog::from_entries(entries))
}

fn stored_reference_from_core(reference: &CoreAssetReference) -> StoredAssetReference {
    StoredAssetReference {
        id: reference.id().as_str().to_owned(),
        version: match reference.version() {
            AssetVersionReq::Any => StoredAssetVersionRequirement::Any,
            AssetVersionReq::Exact(value) => StoredAssetVersionRequirement::Exact { value },
            AssetVersionReq::AtLeast(value) => StoredAssetVersionRequirement::AtLeast { value },
        },
        hash: reference.hash().map(|hash| hash.as_str().to_owned()),
    }
}

fn core_reference_from_stored(
    reference: &StoredAssetReference,
) -> Result<CoreAssetReference, String> {
    Ok(CoreAssetReference::new(
        AssetId::parse(&reference.id).map_err(|error| error.to_string())?,
        match reference.version {
            StoredAssetVersionRequirement::Any => AssetVersionReq::Any,
            StoredAssetVersionRequirement::Exact { value } => AssetVersionReq::Exact(value),
            StoredAssetVersionRequirement::AtLeast { value } => AssetVersionReq::AtLeast(value),
        },
        reference
            .hash
            .as_deref()
            .map(AssetHash::parse)
            .transpose()
            .map_err(|error| error.to_string())?,
    ))
}

fn protocol_rgba(value: core_catalog::Rgba) -> ProtocolRgba {
    ProtocolRgba {
        r: value.r,
        g: value.g,
        b: value.b,
        a: value.a,
    }
}

fn core_rgba(value: ProtocolRgba) -> core_catalog::Rgba {
    core_catalog::Rgba {
        r: value.r,
        g: value.g,
        b: value.b,
        a: value.a,
    }
}

impl From<PrefabRegistryWire> for ProtocolPrefabRegistry {
    fn from(value: PrefabRegistryWire) -> Self {
        Self {
            schema_version: value.schema_version,
            definitions: value.definitions.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<ProtocolPrefabRegistry> for PrefabRegistryWire {
    fn from(value: ProtocolPrefabRegistry) -> Self {
        Self {
            schema_version: value.schema_version,
            definitions: value.definitions.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<PrefabDefinitionWire> for ProtocolPrefabDefinition {
    fn from(value: PrefabDefinitionWire) -> Self {
        Self {
            id: PrefabId::new(value.id),
            schema_version: value.schema_version,
            display_name: value.display_name,
            parts: value.parts.into_iter().map(Into::into).collect(),
            part_roles: value.part_roles.into_iter().map(Into::into).collect(),
            variant: value.variant.map(Into::into),
        }
    }
}

impl From<ProtocolPrefabDefinition> for PrefabDefinitionWire {
    fn from(mut value: ProtocolPrefabDefinition) -> Self {
        value.parts.sort_by_key(|part| part.id.raw());
        value.part_roles.sort_by(|a, b| a.role.cmp(&b.role));
        Self {
            id: value.id.raw(),
            schema_version: value.schema_version,
            display_name: value.display_name,
            parts: value.parts.into_iter().map(Into::into).collect(),
            part_roles: value.part_roles.into_iter().map(Into::into).collect(),
            variant: value.variant.map(Into::into),
        }
    }
}

impl From<PrefabPartWire> for ProtocolPrefabPart {
    fn from(value: PrefabPartWire) -> Self {
        Self {
            id: PrefabPartId::new(value.id),
            namespace: value.namespace,
            display_name: value.display_name,
            parent: value.parent.map(PrefabPartId::new),
            transform: ProtocolPrefabTransform::from(value.transform),
            source: value.source.into(),
        }
    }
}

impl From<ProtocolPrefabPart> for PrefabPartWire {
    fn from(value: ProtocolPrefabPart) -> Self {
        Self {
            id: value.id.raw(),
            namespace: value.namespace,
            display_name: value.display_name,
            parent: value.parent.map(PrefabPartId::raw),
            transform: value.transform.into(),
            source: value.source.into(),
        }
    }
}

impl From<PrefabPartSourceWire> for ProtocolPrefabPartSource {
    fn from(value: PrefabPartSourceWire) -> Self {
        match value {
            PrefabPartSourceWire::Scene { asset } => Self::Scene { asset },
            PrefabPartSourceWire::EntityDefinition { stable_id } => {
                Self::EntityDefinition { stable_id }
            }
            PrefabPartSourceWire::VoxelObject { asset } => Self::VoxelObject { asset },
        }
    }
}

impl From<ProtocolPrefabPartSource> for PrefabPartSourceWire {
    fn from(value: ProtocolPrefabPartSource) -> Self {
        match value {
            ProtocolPrefabPartSource::Scene { asset } => Self::Scene { asset },
            ProtocolPrefabPartSource::EntityDefinition { stable_id } => {
                Self::EntityDefinition { stable_id }
            }
            ProtocolPrefabPartSource::VoxelObject { asset } => Self::VoxelObject { asset },
        }
    }
}

impl From<PrefabPartRoleWire> for ProtocolPrefabPartRoleBinding {
    fn from(value: PrefabPartRoleWire) -> Self {
        Self {
            role: value.role,
            part: PrefabPartId::new(value.part),
        }
    }
}

impl From<ProtocolPrefabPartRoleBinding> for PrefabPartRoleWire {
    fn from(value: ProtocolPrefabPartRoleBinding) -> Self {
        Self {
            role: value.role,
            part: value.part.raw(),
        }
    }
}

impl From<PrefabVariantWire> for ProtocolPrefabVariantDelta {
    fn from(value: PrefabVariantWire) -> Self {
        Self {
            variant_id: value.variant_id,
            base: PrefabId::new(value.base),
            removed_roles: value.removed_roles,
            overrides: value.overrides.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<ProtocolPrefabVariantDelta> for PrefabVariantWire {
    fn from(mut value: ProtocolPrefabVariantDelta) -> Self {
        value.removed_roles.sort();
        value.overrides.sort_by(|a, b| {
            (a.target_role.as_str(), protocol_override_field(&a.value))
                .cmp(&(b.target_role.as_str(), protocol_override_field(&b.value)))
        });
        Self {
            variant_id: value.variant_id,
            base: value.base.raw(),
            removed_roles: value.removed_roles,
            overrides: value.overrides.into_iter().map(Into::into).collect(),
        }
    }
}

fn protocol_override_field(value: &ProtocolPrefabOverrideValue) -> &'static str {
    match value {
        ProtocolPrefabOverrideValue::Transform { .. } => "transform",
        ProtocolPrefabOverrideValue::EntityDefinition { .. } => "entityDefinition",
        ProtocolPrefabOverrideValue::Asset { .. } => "asset",
        ProtocolPrefabOverrideValue::Material { .. } => "material",
        ProtocolPrefabOverrideValue::Activation { .. } => "activation",
    }
}

impl From<PrefabOverrideWire> for ProtocolPrefabOverride {
    fn from(value: PrefabOverrideWire) -> Self {
        Self {
            target_role: value.target_role,
            value: value.value.into(),
        }
    }
}

impl From<ProtocolPrefabOverride> for PrefabOverrideWire {
    fn from(value: ProtocolPrefabOverride) -> Self {
        Self {
            target_role: value.target_role,
            value: value.value.into(),
        }
    }
}

impl From<PrefabOverrideValueWire> for ProtocolPrefabOverrideValue {
    fn from(value: PrefabOverrideValueWire) -> Self {
        match value {
            PrefabOverrideValueWire::Transform { transform } => Self::Transform {
                transform: transform.into(),
            },
            PrefabOverrideValueWire::EntityDefinition { stable_id } => {
                Self::EntityDefinition { stable_id }
            }
            PrefabOverrideValueWire::Asset { asset } => Self::Asset { asset },
            PrefabOverrideValueWire::Material { asset } => Self::Material { asset },
            PrefabOverrideValueWire::Activation { active } => Self::Activation { active },
        }
    }
}

impl From<ProtocolPrefabOverrideValue> for PrefabOverrideValueWire {
    fn from(value: ProtocolPrefabOverrideValue) -> Self {
        match value {
            ProtocolPrefabOverrideValue::Transform { transform } => Self::Transform {
                transform: transform.into(),
            },
            ProtocolPrefabOverrideValue::EntityDefinition { stable_id } => {
                Self::EntityDefinition { stable_id }
            }
            ProtocolPrefabOverrideValue::Asset { asset } => Self::Asset { asset },
            ProtocolPrefabOverrideValue::Material { asset } => Self::Material { asset },
            ProtocolPrefabOverrideValue::Activation { active } => Self::Activation { active },
        }
    }
}

pub(super) fn serialization_prefab_registry(registry: &ProtocolPrefabRegistry) -> PrefabRegistry {
    PrefabRegistry {
        schema_version: registry.schema_version,
        definitions: registry
            .definitions
            .iter()
            .cloned()
            .map(|definition| PrefabDefinition {
                id: definition.id,
                schema_version: definition.schema_version,
                display_name: definition.display_name,
                parts: definition
                    .parts
                    .into_iter()
                    .map(|part| PrefabPart {
                        id: part.id,
                        namespace: part.namespace,
                        display_name: part.display_name,
                        parent: part.parent,
                        transform: PrefabTransform {
                            translation: part.transform.translation,
                            rotation: part.transform.rotation,
                            scale: part.transform.scale,
                        },
                        source: match part.source {
                            ProtocolPrefabPartSource::Scene { asset } => {
                                PrefabPartSource::Scene { asset }
                            }
                            ProtocolPrefabPartSource::EntityDefinition { stable_id } => {
                                PrefabPartSource::EntityDefinition { stable_id }
                            }
                            ProtocolPrefabPartSource::VoxelObject { asset } => {
                                PrefabPartSource::VoxelObject { asset }
                            }
                        },
                    })
                    .collect(),
                part_roles: definition
                    .part_roles
                    .into_iter()
                    .map(|binding| PrefabPartRoleBinding {
                        role: binding.role,
                        part: binding.part,
                    })
                    .collect(),
                variant: definition.variant.map(|variant| PrefabVariantDelta {
                    variant_id: variant.variant_id,
                    base: variant.base,
                    removed_roles: variant.removed_roles,
                    overrides: variant
                        .overrides
                        .into_iter()
                        .map(|item| PrefabOverride {
                            target_role: item.target_role,
                            value: match item.value {
                                ProtocolPrefabOverrideValue::Transform { transform } => {
                                    PrefabOverrideValue::Transform {
                                        transform: PrefabTransform {
                                            translation: transform.translation,
                                            rotation: transform.rotation,
                                            scale: transform.scale,
                                        },
                                    }
                                }
                                ProtocolPrefabOverrideValue::EntityDefinition { stable_id } => {
                                    PrefabOverrideValue::EntityDefinition { stable_id }
                                }
                                ProtocolPrefabOverrideValue::Asset { asset } => {
                                    PrefabOverrideValue::Asset { asset }
                                }
                                ProtocolPrefabOverrideValue::Material { asset } => {
                                    PrefabOverrideValue::Material { asset }
                                }
                                ProtocolPrefabOverrideValue::Activation { active } => {
                                    PrefabOverrideValue::Activation { active }
                                }
                            },
                        })
                        .collect(),
                }),
            })
            .collect(),
    }
}

impl From<GameplayDocumentWire> for ProjectGameplayConfigurationDocumentDto {
    fn from(value: GameplayDocumentWire) -> Self {
        Self {
            schema_version: value.schema_version,
            configurations: value.configurations.into_iter().map(Into::into).collect(),
            bindings: value.bindings,
            overrides: value.overrides,
            triggers: value.triggers,
        }
    }
}

impl From<ProjectGameplayConfigurationDocumentDto> for GameplayDocumentWire {
    fn from(value: ProjectGameplayConfigurationDocumentDto) -> Self {
        Self {
            schema_version: value.schema_version,
            configurations: value.configurations.into_iter().map(Into::into).collect(),
            bindings: value.bindings,
            overrides: value.overrides,
            triggers: value.triggers,
        }
    }
}

impl From<GameplayConfigurationWire> for ProjectGameplayConfigurationDto {
    fn from(value: GameplayConfigurationWire) -> Self {
        Self {
            configuration_id: value.configuration_id,
            module: value.module,
            schema_id: value.schema_id,
            values: value.values.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<ProjectGameplayConfigurationDto> for GameplayConfigurationWire {
    fn from(value: ProjectGameplayConfigurationDto) -> Self {
        Self {
            configuration_id: value.configuration_id,
            module: value.module,
            schema_id: value.schema_id,
            values: value.values.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<ConfigurationFieldValueWire> for ProjectConfigurationFieldValueDto {
    fn from(value: ConfigurationFieldValueWire) -> Self {
        Self {
            field_id: value.field_id,
            value: value.value.into(),
        }
    }
}

impl From<ProjectConfigurationFieldValueDto> for ConfigurationFieldValueWire {
    fn from(value: ProjectConfigurationFieldValueDto) -> Self {
        Self {
            field_id: value.field_id,
            value: value.value.into(),
        }
    }
}

macro_rules! bidirectional_enum {
    ($left:ty, $right:ty, [$($variant:ident),+ $(,)?]) => {
        impl From<$left> for $right {
            fn from(value: $left) -> Self {
                match value { $(<$left>::$variant => <$right>::$variant,)+ }
            }
        }
        impl From<$right> for $left {
            fn from(value: $right) -> Self {
                match value { $(<$right>::$variant => <$left>::$variant,)+ }
            }
        }
    };
}

bidirectional_enum!(
    ReferenceKindWire,
    ProjectContentReferenceKind,
    [
        Asset,
        EntityDefinition,
        InstantiatedEntityDefinition,
        InstantiatedBoundedEntityDefinition,
        EntrySceneFpsPlayerEntityDefinition,
        SceneInstance,
        Prefab,
        PrefabPart,
        PresentationResource
    ]
);
bidirectional_enum!(
    PresentationResourceKindWire,
    ProjectPresentationResourceKind,
    [AnimatedMesh, Audio, Particle, Font, Overlay]
);

impl From<ConfigurationValueWire> for ProjectConfigurationValueDto {
    fn from(value: ConfigurationValueWire) -> Self {
        match value {
            ConfigurationValueWire::Boolean { value } => Self::Boolean { value },
            ConfigurationValueWire::Integer { value } => Self::Integer { value },
            ConfigurationValueWire::Number { value } => Self::Number { value },
            ConfigurationValueWire::String { value } => Self::String { value },
            ConfigurationValueWire::Reference {
                reference_kind,
                target_id,
            } => Self::Reference {
                reference_kind: reference_kind.into(),
                target_id,
            },
        }
    }
}

impl From<ProjectConfigurationValueDto> for ConfigurationValueWire {
    fn from(value: ProjectConfigurationValueDto) -> Self {
        match value {
            ProjectConfigurationValueDto::Boolean { value } => Self::Boolean { value },
            ProjectConfigurationValueDto::Integer { value } => Self::Integer { value },
            ProjectConfigurationValueDto::Number { value } => Self::Number { value },
            ProjectConfigurationValueDto::String { value } => Self::String { value },
            ProjectConfigurationValueDto::Reference {
                reference_kind,
                target_id,
            } => Self::Reference {
                reference_kind: reference_kind.into(),
                target_id,
            },
        }
    }
}

impl From<PresentationCatalogWire> for ProjectPresentationCatalogDto {
    fn from(value: PresentationCatalogWire) -> Self {
        Self {
            schema_version: value.schema_version,
            resources: value.resources.into_iter().map(Into::into).collect(),
            cues: value.cues.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<ProjectPresentationCatalogDto> for PresentationCatalogWire {
    fn from(value: ProjectPresentationCatalogDto) -> Self {
        Self {
            schema_version: value.schema_version,
            resources: value.resources.into_iter().map(Into::into).collect(),
            cues: value.cues.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<PresentationResourceWire> for ProjectPresentationResourceDto {
    fn from(value: PresentationResourceWire) -> Self {
        Self {
            resource_id: value.resource_id,
            kind: value.kind.into(),
            asset_id: value.asset_id,
            source_path: value.source_path,
            content_hash: value.content_hash,
            license_path: value.license_path,
            animated_mesh: value.animated_mesh.map(Into::into),
        }
    }
}

impl From<ProjectPresentationResourceDto> for PresentationResourceWire {
    fn from(value: ProjectPresentationResourceDto) -> Self {
        Self {
            resource_id: value.resource_id,
            kind: value.kind.into(),
            asset_id: value.asset_id,
            source_path: value.source_path,
            content_hash: value.content_hash,
            license_path: value.license_path,
            animated_mesh: value.animated_mesh.map(Into::into),
        }
    }
}

impl From<AnimatedMeshAssetWire> for ProjectAnimatedMeshDescriptorDto {
    fn from(value: AnimatedMeshAssetWire) -> Self {
        Self {
            asset: value.asset,
            runtime_format: match value.runtime_format {
                AnimatedMeshRuntimeFormatWire::Glb => ProjectAnimatedMeshRuntimeFormat::Glb,
            },
            content_hash: value.content_hash,
            clips: value
                .clips
                .into_iter()
                .map(|clip| ProjectAnimationClipDescriptorDto {
                    id: clip.id,
                    name: clip.name,
                    duration_seconds: clip.duration_seconds,
                })
                .collect(),
            default_clip: value.default_clip,
            material_slots: value
                .material_slots
                .into_iter()
                .map(|slot| ProjectMeshMaterialSlotDto {
                    slot: slot.slot,
                    material: slot.material,
                })
                .collect(),
            bounds: ProjectMeshBoundsDescriptorDto {
                min: value.bounds.min,
                max: value.bounds.max,
            },
        }
    }
}

impl From<ProjectAnimatedMeshDescriptorDto> for AnimatedMeshAssetWire {
    fn from(value: ProjectAnimatedMeshDescriptorDto) -> Self {
        Self {
            asset: value.asset,
            runtime_format: match value.runtime_format {
                ProjectAnimatedMeshRuntimeFormat::Glb => AnimatedMeshRuntimeFormatWire::Glb,
            },
            content_hash: value.content_hash,
            clips: value
                .clips
                .into_iter()
                .map(|clip| AnimationClipDescriptorWire {
                    id: clip.id,
                    name: clip.name,
                    duration_seconds: clip.duration_seconds,
                })
                .collect(),
            default_clip: value.default_clip,
            material_slots: value
                .material_slots
                .into_iter()
                .map(|slot| MeshMaterialSlotWire {
                    slot: slot.slot,
                    material: slot.material,
                })
                .collect(),
            bounds: MeshBoundsDescriptorWire {
                min: value.bounds.min,
                max: value.bounds.max,
            },
        }
    }
}

impl From<PresentationCueWire> for ProjectPresentationCueDto {
    fn from(value: PresentationCueWire) -> Self {
        match value {
            PresentationCueWire::Animation {
                cue_id,
                resource_id,
                clip_id,
                looped,
                at_seconds,
                signal,
            } => Self::Animation {
                cue_id,
                resource_id,
                clip_id,
                looped,
                at_seconds,
                signal: signal.into(),
            },
            PresentationCueWire::Audio {
                cue_id,
                signal_id,
                resource_id,
                gain,
            } => Self::Audio {
                cue_id,
                signal_id,
                resource_id,
                gain,
            },
            PresentationCueWire::Particle {
                cue_id,
                signal_id,
                resource_id,
                scale,
            } => Self::Particle {
                cue_id,
                signal_id,
                resource_id,
                scale,
            },
            PresentationCueWire::Overlay {
                cue_id,
                resource_id,
            } => Self::Overlay {
                cue_id,
                resource_id,
            },
        }
    }
}

impl From<ProjectPresentationCueDto> for PresentationCueWire {
    fn from(value: ProjectPresentationCueDto) -> Self {
        match value {
            ProjectPresentationCueDto::Animation {
                cue_id,
                resource_id,
                clip_id,
                looped,
                at_seconds,
                signal,
            } => Self::Animation {
                cue_id,
                resource_id,
                clip_id,
                looped,
                at_seconds,
                signal: signal.into(),
            },
            ProjectPresentationCueDto::Audio {
                cue_id,
                signal_id,
                resource_id,
                gain,
            } => Self::Audio {
                cue_id,
                signal_id,
                resource_id,
                gain,
            },
            ProjectPresentationCueDto::Particle {
                cue_id,
                signal_id,
                resource_id,
                scale,
            } => Self::Particle {
                cue_id,
                signal_id,
                resource_id,
                scale,
            },
            ProjectPresentationCueDto::Overlay {
                cue_id,
                resource_id,
            } => Self::Overlay {
                cue_id,
                resource_id,
            },
        }
    }
}

impl From<PresentationSignalWire> for ProjectPresentationSignalDto {
    fn from(value: PresentationSignalWire) -> Self {
        Self {
            domain: value.domain.into(),
            signal_id: value.signal_id,
        }
    }
}

impl From<ProjectPresentationSignalDto> for PresentationSignalWire {
    fn from(value: ProjectPresentationSignalDto) -> Self {
        Self {
            domain: value.domain.into(),
            signal_id: value.signal_id,
        }
    }
}

impl From<PresentationSignalDomainWire> for ProjectPresentationSignalDomain {
    fn from(value: PresentationSignalDomainWire) -> Self {
        match value {
            PresentationSignalDomainWire::Audio => Self::Audio,
            PresentationSignalDomainWire::Particle => Self::Particle,
        }
    }
}

impl From<ProjectPresentationSignalDomain> for PresentationSignalDomainWire {
    fn from(value: ProjectPresentationSignalDomain) -> Self {
        match value {
            ProjectPresentationSignalDomain::Audio => Self::Audio,
            ProjectPresentationSignalDomain::Particle => Self::Particle,
        }
    }
}
