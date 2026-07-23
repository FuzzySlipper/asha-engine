//! Rust-owned codec, validation, and compare-and-swap authoring for durable
//! non-scene project content.

#![forbid(unsafe_code)]

mod codec;
mod scene;
mod validate;

use std::collections::{BTreeMap, BTreeSet};

use protocol_game_extension::{
    GameplayModuleBinding, GameplayModuleBindingOverride, GameplayModuleConfiguration,
};
use protocol_project_bundle::GameplayTriggerDefinition;
use protocol_project_content::{
    AuthoredBehaviorArgumentDto, ProjectContentAuthoringCommandDto,
    ProjectContentAuthoringRequestDto, ProjectContentAuthoringResultDto,
    ProjectContentCodecResultDto, ProjectContentDecodeRequestDto, ProjectContentDiagnosticCode,
    ProjectContentDiagnosticDto, ProjectContentDocumentDto, ProjectContentDocumentKind,
    ProjectContentEncodeRequestDto, ProjectEntityAppearanceUpdateDto,
    ProjectPresentationResourceKind,
};
use protocol_scene::FlatSceneDocumentDto;
use svc_serialization::ValidatedPrefabRegistry;

pub use scene::project_scene_document_dto;

/// Provider-normalized gameplay content retained by project admission. The
/// canonical configuration bytes are produced by statically composed Rust
/// codecs; authored JSON can never supply them directly.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CompiledProjectGameplayContent {
    configurations: Vec<GameplayModuleConfiguration>,
    bindings: Vec<GameplayModuleBinding>,
    overrides: Vec<GameplayModuleBindingOverride>,
    triggers: Vec<GameplayTriggerDefinition>,
}

impl CompiledProjectGameplayContent {
    pub fn new(
        configurations: Vec<GameplayModuleConfiguration>,
        bindings: Vec<GameplayModuleBinding>,
        overrides: Vec<GameplayModuleBindingOverride>,
        triggers: Vec<GameplayTriggerDefinition>,
    ) -> Self {
        Self {
            configurations,
            bindings,
            overrides,
            triggers,
        }
    }

    pub fn configurations(&self) -> &[GameplayModuleConfiguration] {
        &self.configurations
    }

    pub fn bindings(&self) -> &[GameplayModuleBinding] {
        &self.bindings
    }

    pub fn overrides(&self) -> &[GameplayModuleBindingOverride] {
        &self.overrides
    }

    pub fn triggers(&self) -> &[GameplayTriggerDefinition] {
        &self.triggers
    }
}

/// Statically composed provider authority used during project-content
/// admission. Public wire requests never implement or populate this port.
pub trait ProjectContentGameplayAdmission: Send + Sync {
    fn configuration_schemas(&self) -> &[protocol_project_content::ProjectConfigurationSchemaDto];

    fn compile_gameplay(
        &self,
        documents: &[ProjectContentDocumentDto],
    ) -> Result<CompiledProjectGameplayContent, Vec<ProjectContentDiagnosticDto>>;

    /// Validate Engine-owned project input declarations through the composed
    /// Rust input rule. The service owns document closure; it does not
    /// duplicate namespace, control, or conflict semantics.
    fn validate_input_catalogs(
        &self,
        documents: &[ProjectContentDocumentDto],
    ) -> Result<(), Vec<ProjectContentDiagnosticDto>>;

    /// Resolve an authored signal against the events published by the
    /// statically composed gameplay providers. The compact semantic id is the
    /// contract namespace and name joined by a dot; `version` remains the
    /// published contract version. Schema identity comes only from Rust.
    fn resolve_authored_signal(
        &self,
        _semantic_id: &str,
        _version: u32,
    ) -> Option<protocol_game_extension::GameplayContractRef> {
        None
    }

    /// Validate the names and value kinds of authored event filters against
    /// the selected provider. Open event identity does not imply an open JSON
    /// field bag: providers must explicitly publish every supported filter.
    fn validate_authored_signal_arguments(
        &self,
        _event: &protocol_game_extension::GameplayContractRef,
        arguments: &[AuthoredBehaviorArgumentDto],
    ) -> Result<(), String> {
        if arguments.is_empty() {
            Ok(())
        } else {
            Err("the published event does not expose authored filter fields".to_owned())
        }
    }

    /// Resolve provider/domain semantics that cannot be inferred from generic
    /// project structure alone. The service remains the owner of scene and
    /// bounds checks; composed Rust gameplay authority owns role meaning.
    fn entity_definition_matches_reference(
        &self,
        _kind: protocol_project_content::ProjectContentReferenceKind,
        _definition: &protocol_entity_authoring::EntityDefinition,
    ) -> bool {
        false
    }
}

#[derive(Debug, Default)]
pub struct EmptyProjectContentGameplayAdmission {
    schemas: Vec<protocol_project_content::ProjectConfigurationSchemaDto>,
}

impl ProjectContentGameplayAdmission for EmptyProjectContentGameplayAdmission {
    fn configuration_schemas(&self) -> &[protocol_project_content::ProjectConfigurationSchemaDto] {
        &self.schemas
    }

    fn compile_gameplay(
        &self,
        documents: &[ProjectContentDocumentDto],
    ) -> Result<CompiledProjectGameplayContent, Vec<ProjectContentDiagnosticDto>> {
        let diagnostics = documents
            .iter()
            .filter_map(|document| match document {
                ProjectContentDocumentDto::GameplayConfiguration { document_id, .. } => {
                    Some(ProjectContentDiagnosticDto {
                        code: ProjectContentDiagnosticCode::UnknownReference,
                        document_id: Some(document_id.clone()),
                        path: "document.configurations".to_owned(),
                        message:
                            "gameplay configuration requires a statically composed Rust provider"
                                .to_owned(),
                    })
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        if diagnostics.is_empty() {
            Ok(CompiledProjectGameplayContent::default())
        } else {
            Err(diagnostics)
        }
    }

    fn validate_input_catalogs(
        &self,
        documents: &[ProjectContentDocumentDto],
    ) -> Result<(), Vec<ProjectContentDiagnosticDto>> {
        let diagnostics = documents
            .iter()
            .filter_map(|document| match document {
                ProjectContentDocumentDto::InputCatalog { document_id, .. } => {
                    Some(ProjectContentDiagnosticDto {
                        code: ProjectContentDiagnosticCode::InvalidDocument,
                        document_id: Some(document_id.clone()),
                        path: "catalog".to_owned(),
                        message: "project input catalog requires the composed Rust input rule"
                            .to_owned(),
                    })
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        if diagnostics.is_empty() {
            Ok(())
        } else {
            Err(diagnostics)
        }
    }
}

pub struct ProjectContentValidationContext<'a> {
    pub scenes: &'a [FlatSceneDocumentDto],
    pub entry_scene_id: Option<core_ids::SceneId>,
    pub gameplay: &'a dyn ProjectContentGameplayAdmission,
    pub reference_revision: u64,
}

/// The only set accepted as current authoring state. Its fields are private so
/// callers cannot manufacture a current set from arbitrary documents/hashes.
#[derive(Debug, Clone)]
pub struct ValidatedProjectContentSet {
    result: ProjectContentCodecResultDto,
    compiled_gameplay: CompiledProjectGameplayContent,
    prefab_registry: ValidatedPrefabRegistry,
    reference_revision: u64,
}

impl ValidatedProjectContentSet {
    pub fn result(&self) -> &ProjectContentCodecResultDto {
        &self.result
    }

    pub fn set_hash(&self) -> &str {
        self.result
            .set_hash
            .as_deref()
            .expect("validated project-content set has an identity")
    }

    pub fn compiled_gameplay(&self) -> &CompiledProjectGameplayContent {
        &self.compiled_gameplay
    }

    pub fn prefab_registry(&self) -> &ValidatedPrefabRegistry {
        &self.prefab_registry
    }
}

pub struct ProjectContentValidationOutcome {
    pub result: ProjectContentCodecResultDto,
    pub validated: Option<ValidatedProjectContentSet>,
}

pub fn decode_project_content_sources(
    sources: &[protocol_project_content::ProjectContentSourceDto],
) -> Result<Vec<ProjectContentDocumentDto>, Vec<ProjectContentDiagnosticDto>> {
    codec::decode_sources(sources)
}

/// Compile a stored catalog into the validated core representation used by
/// authority and render projection. Callers still validate the complete
/// ProjectContent set first; this function prevents downstream cells from
/// reimplementing the durable catalog codec.
pub fn compile_stored_asset_catalog(
    catalog: &protocol_assets::StoredAssetCatalog,
) -> Result<core_catalog::Catalog, String> {
    codec::core_catalog_from_stored(catalog)
}

/// Decode one manifest `projectContent` body. Unlike the authoring DTO seam,
/// the artifact is self-identifying so runtime admission never infers a
/// document kind or stable id from its path.
pub fn decode_project_content_artifact(
    source_path: &str,
    body: &[u8],
) -> Result<ProjectContentDocumentDto, ProjectContentDiagnosticDto> {
    codec::decode_artifact(source_path, body)
}

pub fn reject_project_content_parse(
    diagnostics: Vec<ProjectContentDiagnosticDto>,
    gameplay: &dyn ProjectContentGameplayAdmission,
) -> ProjectContentCodecResultDto {
    rejected_codec(diagnostics, gameplay.configuration_schemas())
}

pub fn decode_project_content(
    request: ProjectContentDecodeRequestDto,
    context: ProjectContentValidationContext<'_>,
) -> ProjectContentValidationOutcome {
    let source_paths = match source_path_map(&request.sources) {
        Ok(paths) => paths,
        Err(diagnostics) => {
            return outcome(rejected_codec(
                diagnostics,
                context.gameplay.configuration_schemas(),
            ))
        }
    };
    match codec::decode_sources(&request.sources) {
        Ok(documents) => encode_documents(documents, Some(&source_paths), context),
        Err(diagnostics) => outcome(rejected_codec(
            diagnostics,
            context.gameplay.configuration_schemas(),
        )),
    }
}

/// Compile already-decoded, manifest-discovered documents against the same
/// closed validation and provider authority used by authoring operations.
pub fn validate_project_content_documents(
    documents: Vec<ProjectContentDocumentDto>,
    context: ProjectContentValidationContext<'_>,
) -> ProjectContentValidationOutcome {
    encode_documents(documents, None, context)
}

pub fn encode_project_content(
    request: ProjectContentEncodeRequestDto,
    context: ProjectContentValidationContext<'_>,
) -> ProjectContentCodecResultDto {
    encode_documents(request.documents, None, context).result
}

pub fn apply_project_content_authoring(
    current: &ValidatedProjectContentSet,
    request: ProjectContentAuthoringRequestDto,
    context: ProjectContentValidationContext<'_>,
) -> (
    ProjectContentAuthoringResultDto,
    Option<ValidatedProjectContentSet>,
) {
    if current.set_hash() != request.expected_set_hash
        || current.reference_revision != context.reference_revision
    {
        let result = ProjectContentAuthoringResultDto {
            accepted: false,
            documents: Vec::new(),
            canonical_files: Vec::new(),
            set_hash: Some(current.set_hash().to_owned()),
            provider_schemas: context.gameplay.configuration_schemas().to_vec(),
            field_metadata: Vec::new(),
            diagnostics: vec![ProjectContentDiagnosticDto {
                code: ProjectContentDiagnosticCode::StaleRevision,
                document_id: None,
                path: "expectedSetHash".to_owned(),
                message: "project-content authoring targeted a stale document set".to_owned(),
            }],
        };
        return (result, None);
    }

    let mut documents = current.result.documents.clone();
    let mut source_paths = current
        .result
        .canonical_files
        .iter()
        .filter_map(|file| {
            file.source_path
                .as_ref()
                .map(|path| (document_key(file.kind, &file.document_id), path.clone()))
        })
        .collect::<BTreeMap<_, _>>();
    match request.command {
        ProjectContentAuthoringCommandDto::Upsert {
            source_path,
            document,
        } => {
            if let Err(message) = validate_source_path(&source_path) {
                return authoring_rejection(
                    current,
                    context.gameplay.configuration_schemas(),
                    ProjectContentDiagnosticDto {
                        code: ProjectContentDiagnosticCode::InvalidField,
                        document_id: Some(document.document_id().to_owned()),
                        path: "command.sourcePath".to_owned(),
                        message,
                    },
                );
            }
            let source_key = document_key(document.kind(), document.document_id());
            if let Some(conflicting_document_id) = source_paths.iter().find_map(|(key, path)| {
                (key != &source_key && path == &source_path).then(|| key.1.clone())
            }) {
                return authoring_rejection(
                    current,
                    context.gameplay.configuration_schemas(),
                    ProjectContentDiagnosticDto {
                        code: ProjectContentDiagnosticCode::DuplicateDocument,
                        document_id: Some(document.document_id().to_owned()),
                        path: "command.sourcePath".to_owned(),
                        message: format!(
                            "project-content sourcePath is already owned by document `{conflicting_document_id}`"
                        ),
                    },
                );
            }
            let document_identity = (document.kind(), document.document_id().to_owned());
            documents.retain(|current| {
                (current.kind(), current.document_id().to_owned()) != document_identity
            });
            source_paths.insert(source_key, source_path);
            documents.push(document);
        }
        ProjectContentAuthoringCommandDto::Delete {
            document_id,
            document_kind,
        } => {
            let before = documents.len();
            documents.retain(|document| {
                document.kind() != document_kind || document.document_id() != document_id
            });
            if documents.len() == before {
                let result = ProjectContentAuthoringResultDto {
                    accepted: false,
                    documents: Vec::new(),
                    canonical_files: Vec::new(),
                    set_hash: Some(current.set_hash().to_owned()),
                    provider_schemas: context.gameplay.configuration_schemas().to_vec(),
                    field_metadata: Vec::new(),
                    diagnostics: vec![ProjectContentDiagnosticDto {
                        code: ProjectContentDiagnosticCode::UnknownReference,
                        document_id: Some(document_id),
                        path: "command.documentId".to_owned(),
                        message: "delete targeted an unknown project-content document".to_owned(),
                    }],
                };
                return (result, None);
            }
            source_paths.remove(&document_key(document_kind, &document_id));
        }
        ProjectContentAuthoringCommandDto::UpdateEntityAppearance {
            document_id,
            projection_id,
            update,
        } => {
            if let Err(diagnostic) =
                update_entity_appearance(&mut documents, &document_id, &projection_id, update)
            {
                return authoring_rejection(
                    current,
                    context.gameplay.configuration_schemas(),
                    diagnostic,
                );
            }
        }
    }
    let encoded = encode_documents(documents, Some(&source_paths), context);
    let result = authoring_from_codec(encoded.result.clone());
    (result, encoded.validated)
}

fn update_entity_appearance(
    documents: &mut [ProjectContentDocumentDto],
    document_id: &str,
    projection_id: &str,
    update: ProjectEntityAppearanceUpdateDto,
) -> Result<(), ProjectContentDiagnosticDto> {
    let reject = |path: &str, message: String| ProjectContentDiagnosticDto {
        code: ProjectContentDiagnosticCode::InvalidField,
        document_id: Some(document_id.to_owned()),
        path: path.to_owned(),
        message,
    };
    let resource = match &update {
        ProjectEntityAppearanceUpdateDto::Resource { resource_id } => Some(
            documents
                .iter()
                .find_map(|document| match document {
                    ProjectContentDocumentDto::PresentationCatalog { catalog, .. } => catalog
                        .resources
                        .iter()
                        .find(|resource| resource.resource_id == *resource_id)
                        .cloned(),
                    _ => None,
                })
                .filter(|resource| {
                    resource.kind == ProjectPresentationResourceKind::AnimatedMesh
                        && resource.animated_mesh.is_some()
                })
                .ok_or_else(|| {
                    reject(
                        "command.update.resourceId",
                        format!(
                            "entity appearance resource `{resource_id}` is not an admitted animated mesh"
                        ),
                    )
                })?,
        ),
        _ => None,
    };
    let target = documents
        .iter_mut()
        .find_map(|document| match document {
            ProjectContentDocumentDto::EntityDefinition {
                document_id: candidate,
                definition,
            } if candidate == document_id => Some(definition),
            _ => None,
        })
        .ok_or_else(|| {
            reject(
                "command.documentId",
                "entity appearance update targeted an unknown EntityDefinition".to_owned(),
            )
        })?;
    let appearance = target
        .capabilities
        .iter_mut()
        .find_map(|capability| match capability {
            protocol_entity_authoring::EntityDefinitionCapability::RenderProjection {
                projection_id: candidate,
                appearance,
                ..
            } if candidate == projection_id => Some(appearance),
            _ => None,
        })
        .ok_or_else(|| {
            reject(
                "command.projectionId",
                format!("unknown render projection `{projection_id}`"),
            )
        })?;

    match update {
        ProjectEntityAppearanceUpdateDto::Resource { .. } => {
            let resource = resource.expect("resource update resolved above");
            let descriptor = resource
                .animated_mesh
                .expect("compatible resource has an animated-mesh descriptor");
            let retained_clip = appearance
                .as_ref()
                .and_then(|binding| binding.initial_clip_id.as_ref())
                .filter(|clip| {
                    descriptor
                        .clips
                        .iter()
                        .any(|candidate| candidate.id == **clip)
                })
                .cloned();
            let model_scale = appearance
                .as_ref()
                .map_or([1.0, 1.0, 1.0], |binding| binding.model_scale);
            *appearance = Some(protocol_entity_authoring::EntityAppearanceBinding {
                resource_id: resource.resource_id,
                initial_clip_id: retained_clip.or(descriptor.default_clip),
                model_scale,
            });
        }
        ProjectEntityAppearanceUpdateDto::InitialClip { initial_clip_id } => {
            let binding = appearance.as_mut().ok_or_else(|| {
                reject(
                    "command.update.initialClipId",
                    "select an appearance resource before selecting a clip".to_owned(),
                )
            })?;
            binding.initial_clip_id = initial_clip_id;
        }
        ProjectEntityAppearanceUpdateDto::ModelScale { axis, value } => {
            if axis > 2 || !value.is_finite() || !(0.0001..=1000.0).contains(&value) {
                return Err(reject(
                    "command.update.modelScale",
                    "model scale axis must be 0..=2 and value must be finite in 0.0001..=1000"
                        .to_owned(),
                ));
            }
            let binding = appearance.as_mut().ok_or_else(|| {
                reject(
                    "command.update.modelScale",
                    "select an appearance resource before editing model scale".to_owned(),
                )
            })?;
            binding.model_scale[usize::from(axis)] = value;
        }
    }
    Ok(())
}

fn encode_documents(
    mut documents: Vec<ProjectContentDocumentDto>,
    source_paths: Option<&BTreeMap<(u8, String), String>>,
    context: ProjectContentValidationContext<'_>,
) -> ProjectContentValidationOutcome {
    documents.sort_by(|left, right| {
        (left.kind() as u8, left.document_id()).cmp(&(right.kind() as u8, right.document_id()))
    });
    let mut identities = BTreeMap::new();
    let mut duplicate_diagnostics = Vec::new();
    for document in &documents {
        let key = (document.kind() as u8, document.document_id().to_owned());
        if document.document_id().trim().is_empty() || identities.insert(key, ()).is_some() {
            duplicate_diagnostics.push(ProjectContentDiagnosticDto {
                code: ProjectContentDiagnosticCode::DuplicateDocument,
                document_id: Some(document.document_id().to_owned()),
                path: "documents".to_owned(),
                message: "document ids must be non-empty and unique within each kind".to_owned(),
            });
        }
    }
    if !duplicate_diagnostics.is_empty() {
        return outcome(rejected_codec(
            duplicate_diagnostics,
            context.gameplay.configuration_schemas(),
        ));
    }

    let mut diagnostics = validate::validate_document_set(
        &documents,
        context.scenes,
        context.entry_scene_id,
        context.gameplay,
    );
    let compiled_gameplay = match context.gameplay.compile_gameplay(&documents) {
        Ok(compiled) => compiled,
        Err(gameplay_diagnostics) => {
            diagnostics.extend(gameplay_diagnostics);
            CompiledProjectGameplayContent::default()
        }
    };
    if let Err(input_diagnostics) = context.gameplay.validate_input_catalogs(&documents) {
        diagnostics.extend(input_diagnostics);
    }
    if !diagnostics.is_empty() {
        return outcome(rejected_codec(
            diagnostics,
            context.gameplay.configuration_schemas(),
        ));
    }
    match codec::canonical_files(&documents) {
        Ok(mut canonical_files) => {
            if let Some(source_paths) = source_paths {
                let mut path_diagnostics = Vec::new();
                let mut assigned_paths = BTreeMap::<String, String>::new();
                for file in &mut canonical_files {
                    let key = document_key(file.kind, &file.document_id);
                    match source_paths.get(&key) {
                        Some(path) => {
                            if let Some(conflicting_document_id) =
                                assigned_paths.insert(path.clone(), file.document_id.clone())
                            {
                                path_diagnostics.push(ProjectContentDiagnosticDto {
                                    code: ProjectContentDiagnosticCode::DuplicateDocument,
                                    document_id: Some(file.document_id.clone()),
                                    path: "sourcePath".to_owned(),
                                    message: format!(
                                        "project-content sourcePath is already owned by document `{conflicting_document_id}`"
                                    ),
                                });
                            }
                            file.source_path = Some(path.clone());
                        }
                        None => path_diagnostics.push(ProjectContentDiagnosticDto {
                            code: ProjectContentDiagnosticCode::InvalidDocument,
                            document_id: Some(file.document_id.clone()),
                            path: "sourcePath".to_owned(),
                            message: "opened authoring content has no retained manifest path"
                                .to_owned(),
                        }),
                    }
                }
                if !path_diagnostics.is_empty() {
                    return outcome(rejected_codec(
                        path_diagnostics,
                        context.gameplay.configuration_schemas(),
                    ));
                }
            }
            let prefab_registry = match codec::compiled_prefab_registry(&documents) {
                Ok(registry) => registry,
                Err(prefab_report) => {
                    let diagnostics = prefab_report
                        .diagnostics
                        .into_iter()
                        .map(|diagnostic| ProjectContentDiagnosticDto {
                            code: ProjectContentDiagnosticCode::InvalidDocument,
                            document_id: None,
                            path: diagnostic.path,
                            message: format!(
                                "prefab compilation rejected {}: {}",
                                diagnostic.code.as_str(),
                                diagnostic.message
                            ),
                        })
                        .collect();
                    return outcome(rejected_codec(
                        diagnostics,
                        context.gameplay.configuration_schemas(),
                    ));
                }
            };
            let set_hash = Some(codec::document_set_hash(&canonical_files));
            let field_metadata = validate::field_metadata(
                &documents,
                context.scenes,
                context.entry_scene_id,
                context.gameplay,
            );
            let result = ProjectContentCodecResultDto {
                accepted: true,
                documents,
                canonical_files,
                set_hash,
                provider_schemas: context.gameplay.configuration_schemas().to_vec(),
                field_metadata,
                diagnostics: Vec::new(),
            };
            ProjectContentValidationOutcome {
                validated: Some(ValidatedProjectContentSet {
                    result: result.clone(),
                    compiled_gameplay,
                    prefab_registry,
                    reference_revision: context.reference_revision,
                }),
                result,
            }
        }
        Err(diagnostics) => outcome(rejected_codec(
            diagnostics,
            context.gameplay.configuration_schemas(),
        )),
    }
}

fn document_key(kind: ProjectContentDocumentKind, document_id: &str) -> (u8, String) {
    (kind as u8, document_id.to_owned())
}

fn source_path_map(
    sources: &[protocol_project_content::ProjectContentSourceDto],
) -> Result<BTreeMap<(u8, String), String>, Vec<ProjectContentDiagnosticDto>> {
    let mut paths = BTreeMap::new();
    let mut seen_paths = BTreeSet::new();
    let mut diagnostics = Vec::new();
    for (index, source) in sources.iter().enumerate() {
        if let Err(message) = validate_source_path(&source.source_path) {
            diagnostics.push(ProjectContentDiagnosticDto {
                code: ProjectContentDiagnosticCode::InvalidField,
                document_id: Some(source.document_id.clone()),
                path: format!("sources[{index}].sourcePath"),
                message,
            });
            continue;
        }
        if !seen_paths.insert(source.source_path.clone()) {
            diagnostics.push(ProjectContentDiagnosticDto {
                code: ProjectContentDiagnosticCode::DuplicateDocument,
                document_id: Some(source.document_id.clone()),
                path: format!("sources[{index}].sourcePath"),
                message: "manifest source path is assigned to more than one document".to_owned(),
            });
            continue;
        }
        paths.insert(
            document_key(source.kind, &source.document_id),
            source.source_path.clone(),
        );
    }
    if diagnostics.is_empty() {
        Ok(paths)
    } else {
        Err(diagnostics)
    }
}

fn validate_source_path(path: &str) -> Result<(), String> {
    if path.is_empty()
        || path.starts_with('/')
        || path.contains('\\')
        || path
            .split('/')
            .any(|component| component.is_empty() || component == "." || component == "..")
    {
        Err("project-content sourcePath must be a normalized project-relative path".to_owned())
    } else {
        Ok(())
    }
}

fn authoring_rejection(
    current: &ValidatedProjectContentSet,
    provider_schemas: &[protocol_project_content::ProjectConfigurationSchemaDto],
    diagnostic: ProjectContentDiagnosticDto,
) -> (
    ProjectContentAuthoringResultDto,
    Option<ValidatedProjectContentSet>,
) {
    (
        ProjectContentAuthoringResultDto {
            accepted: false,
            documents: Vec::new(),
            canonical_files: Vec::new(),
            set_hash: Some(current.set_hash().to_owned()),
            provider_schemas: provider_schemas.to_vec(),
            field_metadata: Vec::new(),
            diagnostics: vec![diagnostic],
        },
        None,
    )
}

fn outcome(result: ProjectContentCodecResultDto) -> ProjectContentValidationOutcome {
    ProjectContentValidationOutcome {
        result,
        validated: None,
    }
}

fn rejected_codec(
    diagnostics: Vec<ProjectContentDiagnosticDto>,
    provider_schemas: &[protocol_project_content::ProjectConfigurationSchemaDto],
) -> ProjectContentCodecResultDto {
    ProjectContentCodecResultDto {
        accepted: false,
        documents: Vec::new(),
        canonical_files: Vec::new(),
        set_hash: None,
        provider_schemas: provider_schemas.to_vec(),
        field_metadata: Vec::new(),
        diagnostics,
    }
}

fn authoring_from_codec(result: ProjectContentCodecResultDto) -> ProjectContentAuthoringResultDto {
    ProjectContentAuthoringResultDto {
        accepted: result.accepted,
        documents: result.documents,
        canonical_files: result.canonical_files,
        set_hash: result.set_hash,
        provider_schemas: result.provider_schemas,
        field_metadata: result.field_metadata,
        diagnostics: result.diagnostics,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        apply_project_content_authoring, decode_project_content, decode_project_content_sources,
        validate, CompiledProjectGameplayContent, ProjectContentGameplayAdmission,
        ProjectContentValidationContext, ProjectContentValidationOutcome,
    };
    use core_ids::{SceneId, SceneNodeId};
    use protocol_project_content::*;
    use protocol_scene::{
        FlatSceneDocumentDto, SceneEntityInstanceDto, SceneEntityReferenceDto, SceneMetadataDto,
        SceneNodeKindDto, SceneNodeRecordDto, SceneTransformDto,
    };

    fn source(
        document_id: &str,
        kind: ProjectContentDocumentKind,
        source_text: &str,
    ) -> ProjectContentSourceDto {
        ProjectContentSourceDto {
            source_path: format!("content/{document_id}.json"),
            document_id: document_id.to_owned(),
            kind,
            source_text: source_text.to_owned(),
        }
    }

    fn scene() -> FlatSceneDocumentDto {
        let node = |id, instance_id: &str, reference| SceneNodeRecordDto {
            id: SceneNodeId::new(id),
            parent: None,
            child_order: id as u32,
            label: None,
            tags: Vec::new(),
            transform: SceneTransformDto {
                translation: [0.0, 0.0, 0.0],
                rotation: [0.0, 0.0, 0.0, 1.0],
                scale: [1.0, 1.0, 1.0],
            },
            kind: SceneNodeKindDto::EntityInstance {
                instance: SceneEntityInstanceDto {
                    instance_id: instance_id.to_owned(),
                    reference,
                    spawn_marker_id: None,
                },
            },
        };
        FlatSceneDocumentDto {
            schema_version: 4,
            id: SceneId::new(41),
            metadata: SceneMetadataDto {
                name: Some("Reference room".to_owned()),
                authoring_format_version: 4,
            },
            dependencies: Vec::new(),
            nodes: vec![
                node(
                    1,
                    "reference.trigger.instance",
                    SceneEntityReferenceDto::EntityDefinition {
                        stable_id: "reference.trigger".to_owned(),
                    },
                ),
                node(
                    2,
                    "reference.console.blue",
                    SceneEntityReferenceDto::Prefab {
                        prefab_id: 70,
                        variant_id: Some("blue".to_owned()),
                        instantiation_seed: 11,
                    },
                ),
            ],
        }
    }

    fn marker_scene(
        scene_id: u64,
        marker_id: Option<&str>,
        spawn_marker_id: Option<&str>,
    ) -> FlatSceneDocumentDto {
        let mut nodes = Vec::new();
        if let Some(marker_id) = marker_id {
            nodes.push(SceneNodeRecordDto {
                id: SceneNodeId::new(1),
                parent: None,
                child_order: 0,
                label: None,
                tags: Vec::new(),
                transform: SceneTransformDto {
                    translation: [0.0, 0.0, 0.0],
                    rotation: [0.0, 0.0, 0.0, 1.0],
                    scale: [1.0, 1.0, 1.0],
                },
                kind: SceneNodeKindDto::Marker {
                    marker_id: marker_id.to_owned(),
                },
            });
        }
        if let Some(spawn_marker_id) = spawn_marker_id {
            nodes.push(SceneNodeRecordDto {
                id: SceneNodeId::new(2),
                parent: None,
                child_order: 1,
                label: None,
                tags: Vec::new(),
                transform: SceneTransformDto {
                    translation: [0.0, 0.0, 0.0],
                    rotation: [0.0, 0.0, 0.0, 1.0],
                    scale: [1.0, 1.0, 1.0],
                },
                kind: SceneNodeKindDto::EntityInstance {
                    instance: SceneEntityInstanceDto {
                        instance_id: format!("scene.{scene_id}.instance"),
                        reference: SceneEntityReferenceDto::EntityDefinition {
                            stable_id: "reference.console".to_owned(),
                        },
                        spawn_marker_id: Some(spawn_marker_id.to_owned()),
                    },
                },
            });
        }
        FlatSceneDocumentDto {
            schema_version: 4,
            id: SceneId::new(scene_id),
            metadata: SceneMetadataDto {
                name: Some(format!("Marker scene {scene_id}")),
                authoring_format_version: 4,
            },
            dependencies: Vec::new(),
            nodes,
        }
    }

    struct FixtureAdmission {
        schemas: Vec<ProjectConfigurationSchemaDto>,
    }

    impl ProjectContentGameplayAdmission for FixtureAdmission {
        fn configuration_schemas(&self) -> &[ProjectConfigurationSchemaDto] {
            &self.schemas
        }

        fn compile_gameplay(
            &self,
            _documents: &[ProjectContentDocumentDto],
        ) -> Result<CompiledProjectGameplayContent, Vec<ProjectContentDiagnosticDto>> {
            Ok(CompiledProjectGameplayContent::default())
        }

        fn validate_input_catalogs(
            &self,
            _documents: &[ProjectContentDocumentDto],
        ) -> Result<(), Vec<ProjectContentDiagnosticDto>> {
            Ok(())
        }

        fn resolve_authored_signal(
            &self,
            semantic_id: &str,
            version: u32,
        ) -> Option<protocol_game_extension::GameplayContractRef> {
            (semantic_id == AUTHORED_SIGNAL_PREFAB_PART_INTERACTED && version == 1).then(|| {
                protocol_game_extension::GameplayContractRef {
                    namespace: "asha.prefab".to_owned(),
                    name: "part-interacted".to_owned(),
                    version,
                    schema_hash: "fixture-prefab-part-interacted-v1".to_owned(),
                }
            })
        }

        fn validate_authored_signal_arguments(
            &self,
            _event: &protocol_game_extension::GameplayContractRef,
            arguments: &[AuthoredBehaviorArgumentDto],
        ) -> Result<(), String> {
            if matches!(
                arguments,
                [AuthoredBehaviorArgumentDto {
                    name,
                    value: protocol_project_content::AuthoredBehaviorValueDto::PrefabPart { .. },
                }] if name == "part"
            ) {
                Ok(())
            } else {
                Err(
                    "prefab interaction requires the provider-owned `part: prefabPart` filter"
                        .to_owned(),
                )
            }
        }

        fn entity_definition_matches_reference(
            &self,
            kind: ProjectContentReferenceKind,
            definition: &protocol_entity_authoring::EntityDefinition,
        ) -> bool {
            kind == ProjectContentReferenceKind::EntrySceneFpsPlayerEntityDefinition
                && definition.capabilities.iter().any(|capability| {
                    matches!(
                        capability,
                        protocol_entity_authoring::EntityDefinitionCapability::Controller {
                            controller_id
                        } if controller_id == "player_input"
                    )
                })
        }
    }

    fn admission() -> FixtureAdmission {
        FixtureAdmission {
            schemas: vec![ProjectConfigurationSchemaDto {
                schema_id: "reference.primary-action.v1".to_owned(),
                module_id: "reference.primary-action".to_owned(),
                provider_id: "provider.reference.primary-action".to_owned(),
                contract: protocol_game_extension::GameplayContractRef {
                    namespace: "reference.primary-action".to_owned(),
                    name: "configuration".to_owned(),
                    version: 1,
                    schema_hash: "fnv1a64:config".to_owned(),
                },
                codec_id: "asha.project-configuration.canonical-json.v1".to_owned(),
                fields: vec![
                    ProjectConfigurationFieldDto {
                        field_id: "cooldownTicks".to_owned(),
                        label: "Cooldown ticks".to_owned(),
                        value_kind: ProjectConfigurationValueKind::Integer,
                        required: true,
                        reference_kind: None,
                        integer_min: Some(0),
                        integer_max: Some(120),
                        number_min: None,
                        number_max: None,
                    },
                    ProjectConfigurationFieldDto {
                        field_id: "entryPlayer".to_owned(),
                        label: "Entry-scene FPS player".to_owned(),
                        value_kind: ProjectConfigurationValueKind::Reference,
                        required: false,
                        reference_kind: Some(
                            ProjectContentReferenceKind::EntrySceneFpsPlayerEntityDefinition,
                        ),
                        integer_min: None,
                        integer_max: None,
                        number_min: None,
                        number_max: None,
                    },
                    ProjectConfigurationFieldDto {
                        field_id: "requiredActor".to_owned(),
                        label: "Required actor".to_owned(),
                        value_kind: ProjectConfigurationValueKind::Reference,
                        required: false,
                        reference_kind: Some(
                            ProjectContentReferenceKind::InstantiatedEntityDefinition,
                        ),
                        integer_min: None,
                        integer_max: None,
                        number_min: None,
                        number_max: None,
                    },
                    ProjectConfigurationFieldDto {
                        field_id: "requiredBoundedActor".to_owned(),
                        label: "Required bounded actor".to_owned(),
                        value_kind: ProjectConfigurationValueKind::Reference,
                        required: false,
                        reference_kind: Some(
                            ProjectContentReferenceKind::InstantiatedBoundedEntityDefinition,
                        ),
                        integer_min: None,
                        integer_max: None,
                        number_min: None,
                        number_max: None,
                    },
                ],
            }],
        }
    }

    fn decode(request: ProjectContentDecodeRequestDto) -> ProjectContentValidationOutcome {
        let scenes = vec![scene()];
        let admission = admission();
        decode_project_content(
            request,
            ProjectContentValidationContext {
                scenes: &scenes,
                entry_scene_id: Some(scenes[0].id),
                gameplay: &admission,
                reference_revision: 0,
            },
        )
    }

    fn request() -> ProjectContentDecodeRequestDto {
        ProjectContentDecodeRequestDto {
            sources: vec![
                source(
                    "entities/reference-trigger.json",
                    ProjectContentDocumentKind::EntityDefinition,
                    r#"{
                      "kind":"EntityDefinition",
                      "stableId":"reference.trigger",
                      "displayName":"Reference Trigger",
                      "source":{"projectBundle":"reference-project","relativePath":"entities/reference-trigger.json"},
                      "tags":[],
                      "metadata":[],
                      "capabilities":[
                        {"kind":"transform","transform":{"translation":[0,0,0],"rotation":[0,0,0,1],"scale":[1,1,1]}},
                        {"kind":"bounds","min":[-1,-1,-1],"max":[1,1,1]},
                        {"kind":"collision","staticCollider":false},
                        {"kind":"controller","controllerId":"player_input"},
                        {"kind":"renderProjection","projectionId":"reference.trigger.appearance","visible":true,"appearance":{"resourceId":"reference.character.mesh","initialClipId":"idle","modelScale":[1,1,1]}}
                      ]
                    }"#,
                ),
                source(
                    "entities/reference-console.json",
                    ProjectContentDocumentKind::EntityDefinition,
                    r#"{
                      "kind":"EntityDefinition",
                      "stableId":"reference.console",
                      "displayName":"Reference Console",
                      "source":{"projectBundle":"reference-project","relativePath":"entities/reference-console.json"},
                      "tags":[],"metadata":[],
                      "capabilities":[{"kind":"render","visible":true}]
                    }"#,
                ),
                source(
                    "catalogs/reference-assets.json",
                    ProjectContentDocumentKind::AssetCatalog,
                    r#"{
                      "entries":[
                        {"id":"audio/reference-confirm","version":1,"hash":"aabb","sourcePath":"assets/confirm.wav","label":"Confirm","dependencies":[],"material":null},
                        {"id":"mesh/reference-character","version":1,"hash":"ccdd","sourcePath":"assets/character.glb","label":"Character","dependencies":[],"material":null}
                      ]
                    }"#,
                ),
                source(
                    "prefabs/reference-registry.json",
                    ProjectContentDocumentKind::PrefabRegistry,
                    r#"{
                      "schemaVersion":1,
                      "definitions":[{
                        "id":70,"schemaVersion":1,"displayName":"Reference Console",
                        "parts":[{"id":1,"namespace":"body","displayName":"Body","parent":null,"transform":{"translation":[0,0,0],"rotation":[0,0,0,1],"scale":[1,1,1]},"source":{"kind":"entityDefinition","stableId":"reference.console"}}],
                        "partRoles":[{"role":"interaction/body","part":1}],"variant":null
                      },{
                        "id":71,"schemaVersion":1,"displayName":"Reference Console Blue",
                        "parts":[],"partRoles":[],
                        "variant":{"variantId":"blue","base":70,"removedRoles":[],"overrides":[]}
                      }]
                    }"#,
                ),
                source(
                    "gameplay/reference-config.json",
                    ProjectContentDocumentKind::GameplayConfiguration,
                    r#"{
                      "schemaVersion":1,
                      "configurations":[{
                        "configurationId":"reference.primary-action.default",
                        "module":{"moduleId":"reference.primary-action","namespace":"reference.primary-action","version":"0.1.0","sdkHash":"fnv1a64:sdk","contractHash":"fnv1a64:contract","artifactHash":"fnv1a64:artifact","providerId":"provider.reference.primary-action"},
                        "schemaId":"reference.primary-action.v1",
                        "values":[{"fieldId":"cooldownTicks","value":{"kind":"integer","value":4}}]
                      }],
                      "bindings":[{
                        "bindingId":"reference.console.binding","moduleId":"reference.primary-action","configurationId":"reference.primary-action.default",
                        "stateSchema":{"namespace":"reference.primary-action","name":"state","version":1,"schemaHash":"fnv1a64:state"},
                        "target":{"kind":"prefabPart","part":{"prefab":70,"role":"interaction/body"}},
                        "requiredReads":[],"outputContracts":[],"enabled":true
                      }],
                      "overrides":[{"bindingId":"reference.console.binding","sceneInstanceId":"reference.console.blue","configurationId":null,"enabled":null}],
                      "triggers":[{"schemaVersion":2,"sceneInstanceId":"reference.trigger.instance","scope":"reference.nearby","tags":["reference"]}]
                    }"#,
                ),
                source(
                    "presentation/reference-cues.json",
                    ProjectContentDocumentKind::PresentationCatalog,
                    r#"{
                      "schemaVersion":1,
                      "resources":[
                        {"resourceId":"reference.confirm.audio","kind":"audio","assetId":"audio/reference-confirm","sourcePath":"assets/confirm.wav","contentHash":"aabb","licensePath":null,"animatedMesh":null},
                        {"resourceId":"reference.character.mesh","kind":"animatedMesh","assetId":"mesh/reference-character","sourcePath":"assets/character.glb","contentHash":"ccdd","licensePath":null,"animatedMesh":{"asset":"mesh/reference-character","runtimeFormat":"glb","contentHash":"ccdd","clips":[{"id":"idle","name":"Idle","durationSeconds":1.0},{"id":"walk","name":"Walk","durationSeconds":0.5}],"defaultClip":"idle","materialSlots":[],"bounds":{"min":[-0.5,0,-0.5],"max":[0.5,2,0.5]}}}
                      ],
                      "cues":[{"kind":"audio","cueId":"reference.confirm","signalId":"reference.confirm","resourceId":"reference.confirm.audio","gain":0.8}]
                    }"#,
                ),
            ],
        }
    }

    fn authored_behavior_source(source_text: impl Into<String>) -> ProjectContentSourceDto {
        source(
            "behaviors/reference-door.json",
            ProjectContentDocumentKind::BehaviorPackage,
            &source_text.into(),
        )
    }

    fn authored_behavior_json() -> String {
        format!(
            r#"{{
              "schemaVersion":1,
              "packageId":"reference.doors",
              "provenance":{{
                "sdkId":"@asha/game-workspace",
                "sdkVersion":1,
                "vocabularyHash":"{}",
                "sourceModule":"@fixture/gameplay",
                "sourcePath":"src/doors.ts",
                "sourceHash":"fnv1a64:0123456789abcdef"
              }},
              "stateMachines":[{{
                "machineId":"door",
                "targetSceneInstanceId":"reference.trigger.instance",
                "initialStateId":"closed",
                "states":[
                  {{"stateId":"open"}},
                  {{"stateId":"closed"}}
                ],
                "transitions":[
                  {{"transitionId":"open","fromStateId":"closed","toStateId":"open"}},
                  {{"transitionId":"close","fromStateId":"open","toStateId":"closed"}}
                ]
              }}],
              "behaviors":[{{
                "behaviorId":"switch-opens-door",
                "signal":{{"signal":{{"semanticId":"asha.prefab.part-interacted","version":1}},"arguments":[{{"name":"part","value":{{"kind":"prefabPart","sceneInstanceId":"reference.console.blue","role":"interaction/body"}}}}]}},
                "conditions":[{{"predicate":{{"semanticId":"asha.predicate.state-is","version":1}},"arguments":[{{"name":"state","value":{{"kind":"state","machineId":"door","stateId":"closed"}}}}]}}],
                "steps":[
                  {{"stepId":"open-now","afterStepIds":[],"delayTicks":0,"operations":[
                    {{"verb":{{"semanticId":"asha.verb.transition-state","version":1}},"arguments":[{{"name":"machine","value":{{"kind":"stateMachine","machineId":"door"}}}},{{"name":"transition","value":{{"kind":"text","value":"open"}}}}]}},
                    {{"verb":{{"semanticId":"asha.verb.set-relative-translation","version":1}},"arguments":[{{"name":"entity","value":{{"kind":"sceneEntity","sceneInstanceId":"reference.trigger.instance"}}}},{{"name":"value","value":{{"kind":"vector3","value":[0,3,0]}}}}]}},
                    {{"verb":{{"semanticId":"asha.verb.set-capability-active","version":1}},"arguments":[{{"name":"entity","value":{{"kind":"sceneEntity","sceneInstanceId":"reference.trigger.instance"}}}},{{"name":"capability","value":{{"kind":"text","value":"collision"}}}},{{"name":"active","value":{{"kind":"boolean","value":false}}}}]}}
                  ]}},
                  {{"stepId":"close-later","afterStepIds":["open-now"],"delayTicks":120,"operations":[
                    {{"verb":{{"semanticId":"asha.verb.transition-state","version":1}},"arguments":[{{"name":"machine","value":{{"kind":"stateMachine","machineId":"door"}}}},{{"name":"transition","value":{{"kind":"text","value":"close"}}}}]}},
                    {{"verb":{{"semanticId":"asha.verb.set-relative-translation","version":1}},"arguments":[{{"name":"entity","value":{{"kind":"sceneEntity","sceneInstanceId":"reference.trigger.instance"}}}},{{"name":"value","value":{{"kind":"vector3","value":[0,0,0]}}}}]}},
                    {{"verb":{{"semanticId":"asha.verb.set-capability-active","version":1}},"arguments":[{{"name":"entity","value":{{"kind":"sceneEntity","sceneInstanceId":"reference.trigger.instance"}}}},{{"name":"capability","value":{{"kind":"text","value":"collision"}}}},{{"name":"active","value":{{"kind":"boolean","value":true}}}}]}}
                  ]}}
                ]
              }}]
            }}"#,
            AUTHORED_BEHAVIOR_VOCABULARY_HASH
        )
    }

    #[test]
    fn demo_shaped_documents_decode_validate_and_reopen_as_a_canonical_set() {
        let decoded = decode(request());
        assert!(decoded.result.accepted, "{:?}", decoded.result.diagnostics);
        assert_eq!(decoded.result.documents.len(), 6);
        assert_eq!(decoded.result.canonical_files.len(), 6);
        assert!(decoded.result.field_metadata.iter().any(|field| field.path
            == "document.configurations[0].values.cooldownTicks"
            && field.schema_id.as_deref() == Some("reference.primary-action.v1")));
        assert_eq!(decoded.result.provider_schemas.len(), 1);
        assert_eq!(
            decoded.result.provider_schemas[0].fields[0].integer_max,
            Some(120)
        );

        let reopened = decode(ProjectContentDecodeRequestDto {
            sources: decoded
                .result
                .canonical_files
                .iter()
                .map(|file| ProjectContentSourceDto {
                    source_path: file.source_path.clone().expect("opened source path"),
                    document_id: file.document_id.clone(),
                    kind: file.kind,
                    source_text: file.canonical_json.clone(),
                })
                .collect(),
        });
        assert!(
            reopened.result.accepted,
            "{:?}",
            reopened.result.diagnostics
        );
        assert_eq!(reopened.result.set_hash, decoded.result.set_hash);

        let mut moved_request = request();
        let stable_document_id = moved_request.sources[0].document_id.clone();
        moved_request.sources[0].source_path = "content/relocated-entity.json".to_owned();
        let moved = decode(moved_request);
        assert!(moved.result.accepted, "{:?}", moved.result.diagnostics);
        assert!(moved
            .result
            .documents
            .iter()
            .any(|document| document.document_id() == stable_document_id));
        assert_ne!(moved.result.set_hash, decoded.result.set_hash);
    }

    #[test]
    fn authored_behavior_is_strict_canonical_project_content() {
        let mut request = request();
        request
            .sources
            .push(authored_behavior_source(authored_behavior_json()));
        let decoded = decode(request);
        assert!(decoded.result.accepted, "{:?}", decoded.result.diagnostics);
        assert!(matches!(
            decoded.result.documents.last(),
            Some(ProjectContentDocumentDto::BehaviorPackage { package, .. })
                if package.package_id == "reference.doors"
        ));
        assert!(decoded
            .result
            .field_metadata
            .iter()
            .all(|field| { field.document_id != "behaviors/reference-door.json" }));

        let behavior_file = decoded
            .result
            .canonical_files
            .iter()
            .find(|file| file.kind == ProjectContentDocumentKind::BehaviorPackage)
            .expect("canonical behavior package");
        let reopened = decode(ProjectContentDecodeRequestDto {
            sources: decoded
                .result
                .canonical_files
                .iter()
                .map(|file| ProjectContentSourceDto {
                    source_path: file.source_path.clone().expect("canonical source path"),
                    document_id: file.document_id.clone(),
                    kind: file.kind,
                    source_text: file.canonical_json.clone(),
                })
                .collect(),
        });
        assert!(
            reopened.result.accepted,
            "{:?}",
            reopened.result.diagnostics
        );
        assert_eq!(
            reopened
                .result
                .canonical_files
                .iter()
                .find(|file| file.kind == ProjectContentDocumentKind::BehaviorPackage)
                .expect("reopened behavior package")
                .canonical_json,
            behavior_file.canonical_json
        );
    }

    #[test]
    fn authored_behavior_rejects_unknown_code_references_cycles_versions_and_budgets() {
        let cases = [
            (
                authored_behavior_json().replace(
                    "\"stateId\":\"open\"",
                    "\"stateId\":\"open\",\"callback\":\"runTs\"",
                ),
                ProjectContentDiagnosticCode::UnknownField,
            ),
            (
                authored_behavior_json().replace(
                    "\"targetSceneInstanceId\":\"reference.trigger.instance\"",
                    "\"targetSceneInstanceId\":\"missing.door\"",
                ),
                ProjectContentDiagnosticCode::UnknownReference,
            ),
            (
                authored_behavior_json().replace("\"schemaVersion\":1", "\"schemaVersion\":2"),
                ProjectContentDiagnosticCode::InvalidField,
            ),
            (
                authored_behavior_json().replace(
                    "\"stepId\":\"open-now\",\"afterStepIds\":[]",
                    "\"stepId\":\"open-now\",\"afterStepIds\":[\"close-later\"]",
                ),
                ProjectContentDiagnosticCode::InvalidDocument,
            ),
            (
                authored_behavior_json().replace("\"delayTicks\":120", "\"delayTicks\":3601"),
                ProjectContentDiagnosticCode::InvalidDocument,
            ),
        ];
        for (source_text, expected) in cases {
            let mut request = request();
            request.sources.push(authored_behavior_source(source_text));
            let rejected = decode(request);
            assert!(!rejected.result.accepted);
            assert!(
                rejected
                    .result
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.code == expected),
                "expected {expected:?}: {:?}",
                rejected.result.diagnostics
            );
            if expected != ProjectContentDiagnosticCode::UnknownField {
                assert!(rejected.result.diagnostics.iter().any(|diagnostic| {
                    diagnostic.document_id.as_deref() == Some("behaviors/reference-door.json")
                        && diagnostic
                            .message
                            .starts_with("[@fixture/gameplay:src/doors.ts]")
                }));
            }
        }
    }

    #[test]
    fn entity_appearance_metadata_and_updates_are_rust_owned() {
        let decoded = decode(request());
        assert!(decoded.result.accepted, "{:?}", decoded.result.diagnostics);
        let fields = decoded
            .result
            .field_metadata
            .iter()
            .filter(|field| field.schema_id.as_deref() == Some("asha.entity-appearance.v1"))
            .collect::<Vec<_>>();
        assert_eq!(
            fields
                .iter()
                .map(|field| field.field_id.as_str())
                .collect::<Vec<_>>(),
            vec![
                "initialClipId",
                "modelScaleX",
                "modelScaleY",
                "modelScaleZ",
                "resourceId",
            ]
        );
        let resource = fields
            .iter()
            .find(|field| field.field_id == "resourceId")
            .expect("resource descriptor");
        assert_eq!(resource.reference_options.len(), 1);
        assert_eq!(
            resource.reference_options[0].target_id,
            "reference.character.mesh"
        );
        let clip = fields
            .iter()
            .find(|field| field.field_id == "initialClipId")
            .expect("clip descriptor");
        assert_eq!(
            clip.reference_options
                .iter()
                .map(|option| option.target_id.as_str())
                .collect::<Vec<_>>(),
            vec!["", "idle", "walk"]
        );

        let current = decoded.validated.expect("validated set");
        let scenes = vec![scene()];
        let admission = admission();
        let context = ProjectContentValidationContext {
            scenes: &scenes,
            entry_scene_id: Some(scenes[0].id),
            gameplay: &admission,
            reference_revision: 0,
        };
        let (updated, next) = apply_project_content_authoring(
            &current,
            ProjectContentAuthoringRequestDto {
                expected_workspace_id: "fixture".to_owned(),
                expected_generation: 1,
                expected_working_revision: 1,
                expected_set_hash: current.set_hash().to_owned(),
                command: ProjectContentAuthoringCommandDto::UpdateEntityAppearance {
                    document_id: "entities/reference-trigger.json".to_owned(),
                    projection_id: "reference.trigger.appearance".to_owned(),
                    update: ProjectEntityAppearanceUpdateDto::InitialClip {
                        initial_clip_id: None,
                    },
                },
            },
            context,
        );
        assert!(updated.accepted, "{:?}", updated.diagnostics);
        let definition = updated
            .documents
            .iter()
            .find_map(|document| match document {
                ProjectContentDocumentDto::EntityDefinition {
                    document_id,
                    definition,
                } if document_id == "entities/reference-trigger.json" => Some(definition),
                _ => None,
            })
            .expect("updated definition");
        assert!(definition.capabilities.iter().any(|capability| matches!(
            capability,
            protocol_entity_authoring::EntityDefinitionCapability::RenderProjection {
                appearance: Some(protocol_entity_authoring::EntityAppearanceBinding {
                    initial_clip_id: None,
                    ..
                }),
                ..
            }
        )));
        assert!(next.is_some());

        let context = ProjectContentValidationContext {
            scenes: &scenes,
            entry_scene_id: Some(scenes[0].id),
            gameplay: &admission,
            reference_revision: 0,
        };
        let (rejected, rejected_next) = apply_project_content_authoring(
            &current,
            ProjectContentAuthoringRequestDto {
                expected_workspace_id: "fixture".to_owned(),
                expected_generation: 1,
                expected_working_revision: 1,
                expected_set_hash: current.set_hash().to_owned(),
                command: ProjectContentAuthoringCommandDto::UpdateEntityAppearance {
                    document_id: "entities/reference-trigger.json".to_owned(),
                    projection_id: "reference.trigger.appearance".to_owned(),
                    update: ProjectEntityAppearanceUpdateDto::Resource {
                        resource_id: "reference.confirm.audio".to_owned(),
                    },
                },
            },
            context,
        );
        assert!(!rejected.accepted);
        assert!(rejected_next.is_none());
        assert_eq!(rejected.set_hash.as_deref(), Some(current.set_hash()));
    }

    #[test]
    fn instantiated_entity_definition_reference_requires_a_scene_instance() {
        let with_target = |target_id: &str| {
            let mut request = request();
            let gameplay = request
                .sources
                .iter_mut()
                .find(|source| source.kind == ProjectContentDocumentKind::GameplayConfiguration)
                .expect("gameplay source");
            let mut value: serde_json::Value =
                serde_json::from_str(&gameplay.source_text).expect("fixture JSON");
            value["configurations"][0]["values"]
                .as_array_mut()
                .expect("configuration values")
                .push(serde_json::json!({
                    "fieldId": "requiredActor",
                    "value": {
                        "kind": "reference",
                        "referenceKind": "instantiatedEntityDefinition",
                        "targetId": target_id,
                    }
                }));
            gameplay.source_text = serde_json::to_string(&value).expect("fixture serializes");
            request
        };

        let instantiated = decode(with_target("reference.trigger"));
        assert!(
            instantiated.result.accepted,
            "{:?}",
            instantiated.result.diagnostics
        );

        let uninstantiated = decode(with_target("reference.console"));
        assert!(!uninstantiated.result.accepted);
        assert!(uninstantiated.result.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == ProjectContentDiagnosticCode::UnknownReference
                && diagnostic.message.contains("requiredActor")
        }));
    }

    #[test]
    fn instantiated_bounded_entity_definition_reference_requires_usable_bounds() {
        let with_bounds = |bounds: Option<([f32; 3], [f32; 3])>| {
            let mut request = request();
            let entity = request
                .sources
                .iter_mut()
                .find(|source| source.document_id == "entities/reference-trigger.json")
                .expect("instantiated entity source");
            let mut definition: serde_json::Value =
                serde_json::from_str(&entity.source_text).expect("entity fixture JSON");
            let capabilities = definition["capabilities"]
                .as_array_mut()
                .expect("entity capabilities");
            capabilities.retain(|capability| capability["kind"] != "bounds");
            if let Some((min, max)) = bounds {
                capabilities.push(serde_json::json!({
                    "kind": "bounds",
                    "min": min,
                    "max": max,
                }));
            }
            entity.source_text = serde_json::to_string(&definition).expect("entity serializes");

            let gameplay = request
                .sources
                .iter_mut()
                .find(|source| source.kind == ProjectContentDocumentKind::GameplayConfiguration)
                .expect("gameplay source");
            let mut document: serde_json::Value =
                serde_json::from_str(&gameplay.source_text).expect("gameplay fixture JSON");
            document["configurations"][0]["values"]
                .as_array_mut()
                .expect("configuration values")
                .push(serde_json::json!({
                    "fieldId": "requiredBoundedActor",
                    "value": {
                        "kind": "reference",
                        "referenceKind": "instantiatedBoundedEntityDefinition",
                        "targetId": "reference.trigger",
                    }
                }));
            gameplay.source_text = serde_json::to_string(&document).expect("gameplay serializes");
            request
        };
        let reference_diagnostic = |outcome: &ProjectContentValidationOutcome| {
            outcome.result.diagnostics.iter().any(|diagnostic| {
                diagnostic.code == ProjectContentDiagnosticCode::UnknownReference
                    && diagnostic.path == "document.configurations[0].values[1]"
            })
        };

        let valid = decode(with_bounds(Some(([-1.0; 3], [1.0; 3]))));
        assert!(valid.result.accepted, "{:?}", valid.result.diagnostics);

        let missing = decode(with_bounds(None));
        assert!(!missing.result.accepted);
        assert!(
            reference_diagnostic(&missing),
            "{:?}",
            missing.result.diagnostics
        );

        let zero_width = decode(with_bounds(Some(([0.0, -1.0, -1.0], [0.0, 1.0, 1.0]))));
        assert!(!zero_width.result.accepted);
        assert!(
            reference_diagnostic(&zero_width),
            "{:?}",
            zero_width.result.diagnostics
        );
    }

    #[test]
    fn entry_scene_fps_player_reference_uses_rust_semantics_and_catalogs_only_valid_targets() {
        let with_entry_player = |controller_id: &str| {
            let mut request = request();
            let entity = request
                .sources
                .iter_mut()
                .find(|source| source.document_id == "entities/reference-trigger.json")
                .expect("entry player entity source");
            let mut definition: serde_json::Value =
                serde_json::from_str(&entity.source_text).expect("entity fixture JSON");
            let controller = definition["capabilities"]
                .as_array_mut()
                .expect("entity capabilities")
                .iter_mut()
                .find(|capability| capability["kind"] == "controller")
                .expect("controller capability");
            controller["controllerId"] = serde_json::json!(controller_id);
            entity.source_text = serde_json::to_string(&definition).expect("entity serializes");

            let gameplay = request
                .sources
                .iter_mut()
                .find(|source| source.kind == ProjectContentDocumentKind::GameplayConfiguration)
                .expect("gameplay source");
            let mut document: serde_json::Value =
                serde_json::from_str(&gameplay.source_text).expect("gameplay fixture JSON");
            document["configurations"][0]["values"]
                .as_array_mut()
                .expect("configuration values")
                .push(serde_json::json!({
                    "fieldId": "entryPlayer",
                    "value": {
                        "kind": "reference",
                        "referenceKind": "entrySceneFpsPlayerEntityDefinition",
                        "targetId": "reference.trigger",
                    }
                }));
            gameplay.source_text = serde_json::to_string(&document).expect("gameplay serializes");
            request
        };

        let valid = decode(with_entry_player("player_input"));
        assert!(valid.result.accepted, "{:?}", valid.result.diagnostics);
        let metadata = valid
            .result
            .field_metadata
            .iter()
            .find(|field| field.path.ends_with(".entryPlayer"))
            .expect("entry-player field metadata");
        assert_eq!(
            metadata.reference_options,
            vec![ProjectContentReferenceOptionDto {
                target_id: "reference.trigger".to_owned(),
                label: "Reference Trigger".to_owned(),
            }]
        );

        let enemy = decode(with_entry_player("enemy_policy"));
        assert!(!enemy.result.accepted);
        assert!(enemy.result.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == ProjectContentDiagnosticCode::UnknownReference
                && diagnostic.message.contains("entryPlayer")
        }));

        let scenes = vec![marker_scene(99, None, None), scene()];
        let admission = admission();
        let outside_entry = decode_project_content(
            with_entry_player("player_input"),
            ProjectContentValidationContext {
                scenes: &scenes,
                entry_scene_id: Some(SceneId::new(99)),
                gameplay: &admission,
                reference_revision: 0,
            },
        );
        assert!(!outside_entry.result.accepted);
        assert!(outside_entry.result.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == ProjectContentDiagnosticCode::UnknownReference
                && diagnostic.message.contains("entryPlayer")
        }));

        let ambiguous_request = with_entry_player("player_input");
        let mut second_player_source = ambiguous_request
            .sources
            .iter()
            .find(|source| source.document_id == "entities/reference-trigger.json")
            .expect("first player source")
            .clone();
        second_player_source.document_id = "entities/reference-second-player.json".to_owned();
        second_player_source.source_path =
            "content/entities/reference-second-player.json".to_owned();
        let mut second_player: serde_json::Value =
            serde_json::from_str(&second_player_source.source_text).expect("player fixture JSON");
        second_player["stableId"] = serde_json::json!("reference.second-player");
        second_player["displayName"] = serde_json::json!("Reference Second Player");
        second_player["source"]["relativePath"] =
            serde_json::json!("entities/reference-second-player.json");
        second_player_source.source_text =
            serde_json::to_string(&second_player).expect("second player serializes");
        let mut ambiguous_request = ambiguous_request;
        ambiguous_request.sources.push(second_player_source);

        let mut ambiguous_scenes = vec![scene()];
        let mut second_player_node = ambiguous_scenes[0].nodes[0].clone();
        second_player_node.id = SceneNodeId::new(3);
        second_player_node.child_order = 2;
        second_player_node.label = Some("Reference Second Player".to_owned());
        let SceneNodeKindDto::EntityInstance { instance } = &mut second_player_node.kind else {
            panic!("player fixture node is an entity instance");
        };
        instance.instance_id = "reference.second-player.instance".to_owned();
        instance.reference = SceneEntityReferenceDto::EntityDefinition {
            stable_id: "reference.second-player".to_owned(),
        };
        ambiguous_scenes[0].nodes.push(second_player_node);

        let ambiguous_documents = decode_project_content_sources(&ambiguous_request.sources)
            .expect("ambiguous sources remain structurally decodable");
        let ambiguous_metadata = validate::field_metadata(
            &ambiguous_documents,
            &ambiguous_scenes,
            Some(ambiguous_scenes[0].id),
            &admission,
        );
        let ambiguous_player_field = ambiguous_metadata
            .iter()
            .find(|field| field.path.ends_with(".entryPlayer"))
            .expect("ambiguous player field metadata");
        assert!(ambiguous_player_field.reference_options.is_empty());

        let ambiguous = decode_project_content(
            ambiguous_request,
            ProjectContentValidationContext {
                scenes: &ambiguous_scenes,
                entry_scene_id: Some(ambiguous_scenes[0].id),
                gameplay: &admission,
                reference_revision: 0,
            },
        );
        assert!(!ambiguous.result.accepted);
        assert!(ambiguous.result.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == ProjectContentDiagnosticCode::UnknownReference
                && diagnostic.message.contains("entryPlayer")
        }));
    }

    #[test]
    fn strict_decode_rejects_unknown_nested_fields() {
        let result = decode(ProjectContentDecodeRequestDto {
            sources: vec![source(
                "entities/invalid.json",
                ProjectContentDocumentKind::EntityDefinition,
                r#"{"kind":"EntityDefinition","stableId":"reference.invalid","displayName":"Invalid","source":{"projectBundle":"reference","relativePath":"invalid.json","browserAccepted":true},"tags":[],"metadata":[],"capabilities":[]}"#,
            )],
        });
        assert!(!result.result.accepted);
        assert_eq!(
            result.result.diagnostics[0].code,
            ProjectContentDiagnosticCode::UnknownField
        );
    }

    #[test]
    fn project_files_cannot_redefine_provider_schemas() {
        let result = decode(ProjectContentDecodeRequestDto {
            sources: vec![source(
                "gameplay/invalid-provider-schema.json",
                ProjectContentDocumentKind::GameplayConfiguration,
                r#"{
                  "schemaVersion":1,"schemas":[],"configurations":[],
                  "bindings":[],"overrides":[],"triggers":[]
                }"#,
            )],
        });
        assert!(!result.result.accepted);
        assert_eq!(
            result.result.diagnostics[0].code,
            ProjectContentDiagnosticCode::UnknownField
        );
    }

    #[test]
    fn scene_variant_ids_resolve_against_named_registry_variants() {
        let request = request();
        let mut scenes = vec![scene()];
        let SceneNodeKindDto::EntityInstance { instance } = &mut scenes[0].nodes[1].kind else {
            panic!("fixture node is not an entity instance");
        };
        let SceneEntityReferenceDto::Prefab { variant_id, .. } = &mut instance.reference else {
            panic!("fixture node is not a prefab instance");
        };
        *variant_id = Some("missing".to_owned());

        let admission = admission();
        let result = decode_project_content(
            request,
            ProjectContentValidationContext {
                scenes: &scenes,
                entry_scene_id: Some(scenes[0].id),
                gameplay: &admission,
                reference_revision: 0,
            },
        );
        assert!(!result.result.accepted);
        assert!(result.result.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == ProjectContentDiagnosticCode::UnknownReference
                && diagnostic.path.ends_with("variantId")
        }));
    }

    #[test]
    fn marker_ids_are_scene_local_and_never_resolve_across_scenes() {
        let admission = admission();
        let valid_scenes = vec![
            scene(),
            marker_scene(42, Some("shared.spawn"), None),
            marker_scene(43, Some("shared.spawn"), None),
        ];
        let valid = decode_project_content(
            request(),
            ProjectContentValidationContext {
                scenes: &valid_scenes,
                entry_scene_id: Some(valid_scenes[0].id),
                gameplay: &admission,
                reference_revision: 0,
            },
        );
        assert!(valid.result.accepted, "{:?}", valid.result.diagnostics);

        let cross_scene = vec![
            scene(),
            marker_scene(42, Some("only.in.scene.42"), None),
            marker_scene(43, None, Some("only.in.scene.42")),
        ];
        let rejected = decode_project_content(
            request(),
            ProjectContentValidationContext {
                scenes: &cross_scene,
                entry_scene_id: Some(cross_scene[0].id),
                gameplay: &admission,
                reference_revision: 0,
            },
        );
        assert!(!rejected.result.accepted);
        assert!(rejected.result.diagnostics.iter().any(|diagnostic| {
            diagnostic.path.ends_with("spawnMarkerId")
                && diagnostic.code == ProjectContentDiagnosticCode::UnknownReference
        }));
    }

    #[test]
    fn stale_authoring_rejects_before_returning_a_save_candidate() {
        let decoded = decode(request());
        let scenes = vec![scene()];
        let admission = admission();
        let (result, next) = apply_project_content_authoring(
            decoded.validated.as_ref().unwrap(),
            ProjectContentAuthoringRequestDto {
                expected_workspace_id: "workspace/reference".to_owned(),
                expected_generation: 1,
                expected_working_revision: 0,
                expected_set_hash: "fnv1a64:stale".to_owned(),
                command: ProjectContentAuthoringCommandDto::Delete {
                    document_id: "presentation/reference-cues.json".to_owned(),
                    document_kind: ProjectContentDocumentKind::PresentationCatalog,
                },
            },
            ProjectContentValidationContext {
                scenes: &scenes,
                entry_scene_id: Some(scenes[0].id),
                gameplay: &admission,
                reference_revision: 0,
            },
        );
        assert!(!result.accepted);
        assert!(next.is_none());
        assert!(result.canonical_files.is_empty());
        assert_eq!(
            result.diagnostics[0].code,
            ProjectContentDiagnosticCode::StaleRevision
        );
    }

    #[test]
    fn duplicate_trigger_targets_reject_before_returning_a_save_candidate() {
        let decoded = decode(request());
        assert!(decoded.result.accepted, "{:?}", decoded.result.diagnostics);
        let expected_set_hash = decoded.result.set_hash.clone().expect("accepted set hash");
        let mut gameplay = decoded
            .result
            .documents
            .iter()
            .find_map(|document| match document {
                ProjectContentDocumentDto::GameplayConfiguration {
                    document_id,
                    document,
                } => Some((document_id.clone(), document.clone())),
                _ => None,
            })
            .expect("gameplay document");
        gameplay.1.triggers.push(gameplay.1.triggers[0].clone());

        let scenes = vec![scene()];
        let admission = admission();
        let (authored, next) = apply_project_content_authoring(
            decoded.validated.as_ref().unwrap(),
            ProjectContentAuthoringRequestDto {
                expected_workspace_id: "workspace/reference".to_owned(),
                expected_generation: 1,
                expected_working_revision: 0,
                expected_set_hash,
                command: ProjectContentAuthoringCommandDto::Upsert {
                    source_path: format!("content/{}.json", gameplay.0),
                    document: ProjectContentDocumentDto::GameplayConfiguration {
                        document_id: gameplay.0,
                        document: gameplay.1,
                    },
                },
            },
            ProjectContentValidationContext {
                scenes: &scenes,
                entry_scene_id: Some(scenes[0].id),
                gameplay: &admission,
                reference_revision: 0,
            },
        );
        assert!(!authored.accepted);
        assert!(next.is_none());
        assert!(authored.canonical_files.is_empty());
        assert!(authored.diagnostics.iter().any(|diagnostic| {
            diagnostic.path == "document.triggers[1].sceneInstanceId"
                && diagnostic.message.contains("only one trigger definition")
        }));
    }

    #[test]
    fn typed_authoring_returns_a_canonical_reopenable_candidate() {
        let decoded = decode(request());
        assert!(decoded.result.accepted, "{:?}", decoded.result.diagnostics);
        let expected_set_hash = decoded.result.set_hash.clone().expect("accepted set hash");
        let mut changed = decoded
            .result
            .documents
            .iter()
            .find_map(|document| match document {
                ProjectContentDocumentDto::PresentationCatalog {
                    document_id,
                    catalog,
                } => Some((document_id.clone(), catalog.clone())),
                _ => None,
            })
            .expect("presentation catalog");
        changed.1.cues[0] = ProjectPresentationCueDto::Audio {
            cue_id: "reference.confirm".to_owned(),
            signal_id: "reference.confirm".to_owned(),
            resource_id: "reference.confirm.audio".to_owned(),
            gain: 0.65,
        };

        let scenes = vec![scene()];
        let admission = admission();
        let (authored, next) = apply_project_content_authoring(
            decoded.validated.as_ref().unwrap(),
            ProjectContentAuthoringRequestDto {
                expected_workspace_id: "workspace/reference".to_owned(),
                expected_generation: 1,
                expected_working_revision: 0,
                expected_set_hash,
                command: ProjectContentAuthoringCommandDto::Upsert {
                    source_path: format!("content/{}.json", changed.0),
                    document: ProjectContentDocumentDto::PresentationCatalog {
                        document_id: changed.0,
                        catalog: changed.1,
                    },
                },
            },
            ProjectContentValidationContext {
                scenes: &scenes,
                entry_scene_id: Some(scenes[0].id),
                gameplay: &admission,
                reference_revision: 0,
            },
        );
        assert!(authored.accepted, "{:?}", authored.diagnostics);
        assert!(next.is_some());
        assert_ne!(authored.set_hash, decoded.result.set_hash);

        let reopened = decode(ProjectContentDecodeRequestDto {
            sources: authored
                .canonical_files
                .iter()
                .map(|file| ProjectContentSourceDto {
                    source_path: file.source_path.clone().expect("authored source path"),
                    document_id: file.document_id.clone(),
                    kind: file.kind,
                    source_text: file.canonical_json.clone(),
                })
                .collect(),
        });
        assert!(
            reopened.result.accepted,
            "{:?}",
            reopened.result.diagnostics
        );
        assert_eq!(reopened.result.set_hash, authored.set_hash);
    }
}
