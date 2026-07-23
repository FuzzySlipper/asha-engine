use std::collections::BTreeMap;

use core_ids::EntityId;
use gameplay_module_sdk::{
    gameplay_module_payload_hash, GameplayConfigurationReferenceKind,
    GameplayConfigurationValueKind, GameplayModuleBindingRegistryBuilder,
    GameplayProjectConfigurationAuthority,
};
use protocol_diagnostics::DiagnosticSeverity;
use protocol_entity_authoring::{EntityDefinition, EntityDefinitionCapability};
use protocol_game_extension::{
    GameplayCompositionDiagnostic, GameplayCompositionLoadMode, GameplayModuleConfiguration,
};
use protocol_project_bundle::GAMEPLAY_TRIGGER_DEFINITION_SCHEMA_VERSION;
use protocol_project_content::{
    AuthoredBehaviorArgumentDto, AuthoredBehaviorValueDto, ProjectConfigurationFieldDto,
    ProjectConfigurationSchemaDto, ProjectConfigurationValueDto, ProjectConfigurationValueKind,
    ProjectContentDiagnosticCode, ProjectContentDiagnosticDto, ProjectContentDocumentDto,
};
use rule_trigger_volume::{validate_kinematic_trigger_definition, KinematicTriggerDefinition};
use serde_json::{Map, Number, Value};
use svc_gameplay_fabric::{GameplayEventFilterFieldShape, GameplayEventFilterValueKind};
use svc_project_content::{CompiledProjectGameplayContent, ProjectContentGameplayAdmission};

use crate::gameplay_binding::{
    validate_binding, validate_configuration, validate_override_contracts,
};

/// Closed gameplay-provider view used by pre-runtime project-content
/// authoring. It is derived only from a `GameplayStaticComposition`; wire
/// requests cannot register schemas, codecs, or module contracts.
#[derive(Clone)]
pub struct GameplayProjectContentAdmission {
    authority: GameplayProjectConfigurationAuthority,
    schemas: Vec<ProjectConfigurationSchemaDto>,
}

impl Default for GameplayProjectContentAdmission {
    fn default() -> Self {
        Self::new(GameplayProjectConfigurationAuthority::default())
    }
}

impl GameplayProjectContentAdmission {
    pub fn new(authority: GameplayProjectConfigurationAuthority) -> Self {
        let mut schemas = authority
            .schemas()
            .iter()
            .map(|schema| {
                let provider_id = authority
                    .registry()
                    .module(&schema.module_id)
                    .map(|module| module.module_ref.provider_id.clone())
                    .unwrap_or_default();
                ProjectConfigurationSchemaDto {
                    schema_id: schema.configuration.key(),
                    module_id: schema.module_id.clone(),
                    provider_id,
                    contract: schema.configuration.clone(),
                    codec_id: schema.codec_id.clone(),
                    fields: schema
                        .fields
                        .iter()
                        .map(|field| ProjectConfigurationFieldDto {
                            field_id: field.name.clone(),
                            label: field.label.clone(),
                            value_kind: value_kind(field.value_kind),
                            required: field.required,
                            reference_kind: field.reference_kind.map(reference_kind),
                            integer_min: field.integer_min,
                            integer_max: field.integer_max,
                            number_min: field.number_min,
                            number_max: field.number_max,
                        })
                        .collect(),
                }
            })
            .collect::<Vec<_>>();
        schemas.sort_by(|left, right| left.schema_id.cmp(&right.schema_id));
        Self { authority, schemas }
    }

    fn validate_document(
        &self,
        document_id: &str,
        document: &protocol_project_content::ProjectGameplayConfigurationDocumentDto,
    ) -> Result<CompiledProjectGameplayContent, Vec<ProjectContentDiagnosticDto>> {
        let mut diagnostics = Vec::new();
        let mut runtime_configurations = Vec::new();

        for (index, configuration) in document.configurations.iter().enumerate() {
            let Some(schema) = self
                .authority
                .schemas()
                .iter()
                .find(|schema| schema.configuration.key() == configuration.schema_id)
            else {
                continue;
            };
            if schema.module_id != configuration.module.module_id {
                diagnostics.push(project_diagnostic(
                    document_id,
                    format!("document.configurations[{index}].module"),
                    "configuration schema belongs to a different statically composed module",
                ));
                continue;
            }
            let Some(codec) = self.authority.codecs().iter().find(|codec| {
                codec.metadata().module_id == schema.module_id
                    && codec.metadata().configuration == schema.configuration
            }) else {
                diagnostics.push(project_diagnostic(
                    document_id,
                    format!("document.configurations[{index}].schemaId"),
                    "statically composed provider did not register the configuration codec",
                ));
                continue;
            };
            let source = configuration_values_json(&configuration.values);
            let source = match serde_json::to_vec(&source) {
                Ok(source) => source,
                Err(error) => {
                    diagnostics.push(project_diagnostic(
                        document_id,
                        format!("document.configurations[{index}].values"),
                        format!("configuration values could not be encoded: {error}"),
                    ));
                    continue;
                }
            };
            let canonical_config = match codec.canonicalize(&source) {
                Ok(canonical) => canonical,
                Err(error) => {
                    diagnostics.push(project_diagnostic(
                        document_id,
                        format!("document.configurations[{index}].values"),
                        format!("typed provider codec rejected configuration: {error}"),
                    ));
                    continue;
                }
            };
            runtime_configurations.push(GameplayModuleConfiguration {
                configuration_id: configuration.configuration_id.clone(),
                module: configuration.module.clone(),
                configuration: schema.configuration.clone(),
                codec_id: schema.codec_id.clone(),
                config_hash: gameplay_module_payload_hash(&canonical_config),
                canonical_config,
            });
        }

        let mut registry_builder = GameplayModuleBindingRegistryBuilder::new();
        for configuration in runtime_configurations.iter().cloned() {
            registry_builder.configuration(configuration);
        }
        for binding in document.bindings.iter().cloned() {
            registry_builder.binding(binding);
        }
        for layer in document.overrides.iter().cloned() {
            registry_builder.instance_override(layer);
        }
        let registry = registry_builder.build();
        let configurations = runtime_configurations
            .iter()
            .map(|configuration| (configuration.configuration_id.clone(), configuration))
            .collect::<BTreeMap<_, _>>();
        let bindings = document
            .bindings
            .iter()
            .map(|binding| (binding.binding_id.clone(), binding))
            .collect::<BTreeMap<_, _>>();
        let mut runtime_diagnostics = Vec::new();
        let mut compatibility = Vec::<GameplayCompositionDiagnostic>::new();
        for (index, configuration) in registry.configurations.iter().enumerate() {
            validate_configuration(
                configuration,
                self.authority.registry(),
                self.authority.codecs(),
                index,
                GameplayCompositionLoadMode::Compatible,
                &mut runtime_diagnostics,
                &mut compatibility,
            );
        }
        for (index, binding) in registry.bindings.iter().enumerate() {
            validate_binding(
                binding,
                configurations
                    .get(binding.configuration_id.as_str())
                    .copied(),
                self.authority.registry(),
                index,
                &mut runtime_diagnostics,
            );
        }
        validate_override_contracts(
            &document.overrides,
            &bindings,
            &configurations,
            &mut runtime_diagnostics,
        );
        diagnostics.extend(runtime_diagnostics.into_iter().map(|diagnostic| {
            ProjectContentDiagnosticDto {
                code: ProjectContentDiagnosticCode::InvalidDocument,
                document_id: Some(document_id.to_owned()),
                path: format!("document.runtimeAdmission.{}", diagnostic.path),
                message: diagnostic.message,
            }
        }));
        diagnostics.extend(compatibility.into_iter().filter_map(|diagnostic| {
            (diagnostic.severity == DiagnosticSeverity::Error).then(|| {
                ProjectContentDiagnosticDto {
                    code: ProjectContentDiagnosticCode::InvalidDocument,
                    document_id: Some(document_id.to_owned()),
                    path: format!("document.runtimeAdmission.{}", diagnostic.path),
                    message: diagnostic.message,
                }
            })
        }));

        for (index, trigger) in document.triggers.iter().enumerate() {
            if trigger.schema_version != GAMEPLAY_TRIGGER_DEFINITION_SCHEMA_VERSION {
                diagnostics.push(project_diagnostic(
                    document_id,
                    format!("document.triggers[{index}].schemaVersion"),
                    "trigger schema version is not accepted by RuntimeSession admission",
                ));
            }
            let definition = KinematicTriggerDefinition::new(
                EntityId::new(1),
                trigger.scope.clone(),
                trigger.tags.clone(),
            );
            for diagnostic in validate_kinematic_trigger_definition(&definition) {
                diagnostics.push(project_diagnostic(
                    document_id,
                    format!("document.triggers[{index}]"),
                    diagnostic.message,
                ));
            }
        }
        if diagnostics.is_empty() {
            Ok(CompiledProjectGameplayContent::new(
                runtime_configurations,
                document.bindings.clone(),
                document.overrides.clone(),
                document.triggers.clone(),
            ))
        } else {
            Err(diagnostics)
        }
    }
}

impl ProjectContentGameplayAdmission for GameplayProjectContentAdmission {
    fn configuration_schemas(&self) -> &[ProjectConfigurationSchemaDto] {
        &self.schemas
    }

    fn compile_gameplay(
        &self,
        documents: &[ProjectContentDocumentDto],
    ) -> Result<CompiledProjectGameplayContent, Vec<ProjectContentDiagnosticDto>> {
        let mut configurations = Vec::new();
        let mut bindings = Vec::new();
        let mut overrides = Vec::new();
        let mut triggers = Vec::new();
        let mut diagnostics = Vec::new();
        for content in documents {
            let compiled = match content {
                ProjectContentDocumentDto::GameplayConfiguration {
                    document_id,
                    document,
                } => self.validate_document(document_id, document),
                _ => continue,
            };
            match compiled {
                Ok(compiled) => {
                    configurations.extend_from_slice(compiled.configurations());
                    bindings.extend_from_slice(compiled.bindings());
                    overrides.extend_from_slice(compiled.overrides());
                    triggers.extend_from_slice(compiled.triggers());
                }
                Err(mut document_diagnostics) => diagnostics.append(&mut document_diagnostics),
            }
        }
        if diagnostics.is_empty() {
            configurations
                .sort_by(|left, right| left.configuration_id.cmp(&right.configuration_id));
            bindings.sort_by(|left, right| left.binding_id.cmp(&right.binding_id));
            overrides.sort_by(|left, right| {
                (left.binding_id.as_str(), left.scene_instance_id.as_str())
                    .cmp(&(right.binding_id.as_str(), right.scene_instance_id.as_str()))
            });
            triggers.sort_by(|left, right| left.scene_instance_id.cmp(&right.scene_instance_id));
            Ok(CompiledProjectGameplayContent::new(
                configurations,
                bindings,
                overrides,
                triggers,
            ))
        } else {
            Err(diagnostics)
        }
    }

    fn validate_input_catalogs(
        &self,
        documents: &[ProjectContentDocumentDto],
    ) -> Result<(), Vec<ProjectContentDiagnosticDto>> {
        let input_documents = documents
            .iter()
            .filter_map(|document| match document {
                ProjectContentDocumentDto::InputCatalog {
                    document_id,
                    catalog,
                } => Some((document_id, catalog.clone())),
                _ => None,
            })
            .collect::<Vec<_>>();
        let catalogs = input_documents
            .iter()
            .map(|(_, catalog)| catalog.clone())
            .collect::<Vec<_>>();
        rule_input::compose_project_input_catalog(
            rule_input::default_browser_input_catalog(),
            &catalogs,
        )
        .map(|_| ())
        .map_err(|error| {
            let document_id = (input_documents.len() == 1).then(|| input_documents[0].0.clone());
            error
                .diagnostics()
                .iter()
                .map(|diagnostic| ProjectContentDiagnosticDto {
                    code: ProjectContentDiagnosticCode::InvalidDocument,
                    document_id: document_id.clone(),
                    path: diagnostic.path.clone(),
                    message: format!("{:?}: {}", diagnostic.code, diagnostic.message),
                })
                .collect()
        })
    }

    fn resolve_authored_signal(
        &self,
        semantic_id: &str,
        version: u32,
    ) -> Option<protocol_game_extension::GameplayContractRef> {
        self.authority
            .registry()
            .published_event(&format!("{semantic_id}.v{version}"))
            .cloned()
    }

    fn validate_authored_signal_arguments(
        &self,
        event: &protocol_game_extension::GameplayContractRef,
        arguments: &[AuthoredBehaviorArgumentDto],
    ) -> Result<(), String> {
        let fields = arguments
            .iter()
            .map(|argument| {
                let value_kind = match argument.value {
                    AuthoredBehaviorValueDto::SceneEntity { .. } => {
                        GameplayEventFilterValueKind::Entity
                    }
                    AuthoredBehaviorValueDto::PrefabPart { .. } => {
                        GameplayEventFilterValueKind::PrefabPart
                    }
                    AuthoredBehaviorValueDto::Text { .. } => GameplayEventFilterValueKind::Text,
                    AuthoredBehaviorValueDto::Boolean { .. } => {
                        GameplayEventFilterValueKind::Boolean
                    }
                    AuthoredBehaviorValueDto::Integer { .. } => {
                        GameplayEventFilterValueKind::Integer
                    }
                    AuthoredBehaviorValueDto::Number { .. } => GameplayEventFilterValueKind::Number,
                    AuthoredBehaviorValueDto::Vector3 { .. } => {
                        GameplayEventFilterValueKind::Vector3
                    }
                    AuthoredBehaviorValueDto::StateMachine { .. }
                    | AuthoredBehaviorValueDto::State { .. } => {
                        return Err(format!(
                            "filter field `{}` cannot use symbolic state as an event payload value",
                            argument.name
                        ));
                    }
                };
                Ok(GameplayEventFilterFieldShape {
                    name: argument.name.clone(),
                    value_kind,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        self.authority
            .registry()
            .validate_event_filter_shape(event, &fields)
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    fn entity_definition_matches_reference(
        &self,
        kind: protocol_project_content::ProjectContentReferenceKind,
        definition: &EntityDefinition,
    ) -> bool {
        if kind
            != protocol_project_content::ProjectContentReferenceKind::EntrySceneFpsPlayerEntityDefinition
        {
            return false;
        }
        let controller = definition
            .capabilities
            .iter()
            .find_map(|capability| match capability {
                EntityDefinitionCapability::Controller { controller_id } => {
                    Some(controller_id.as_str())
                }
                _ => None,
            });
        let faction = definition
            .capabilities
            .iter()
            .find_map(|capability| match capability {
                EntityDefinitionCapability::Faction { faction_id } => Some(faction_id.as_str()),
                _ => None,
            });
        rule_lifecycle::classify_fps_runtime_role(controller, faction)
            == Ok(rule_lifecycle::FpsRuntimeRole::Player)
    }
}

fn project_diagnostic(
    document_id: &str,
    path: String,
    message: impl Into<String>,
) -> ProjectContentDiagnosticDto {
    ProjectContentDiagnosticDto {
        code: ProjectContentDiagnosticCode::InvalidDocument,
        document_id: Some(document_id.to_owned()),
        path,
        message: message.into(),
    }
}

fn value_kind(value: GameplayConfigurationValueKind) -> ProjectConfigurationValueKind {
    match value {
        GameplayConfigurationValueKind::Boolean => ProjectConfigurationValueKind::Boolean,
        GameplayConfigurationValueKind::Integer => ProjectConfigurationValueKind::Integer,
        GameplayConfigurationValueKind::Number => ProjectConfigurationValueKind::Number,
        GameplayConfigurationValueKind::String => ProjectConfigurationValueKind::String,
        GameplayConfigurationValueKind::Reference => ProjectConfigurationValueKind::Reference,
    }
}

fn reference_kind(
    value: GameplayConfigurationReferenceKind,
) -> protocol_project_content::ProjectContentReferenceKind {
    use protocol_project_content::ProjectContentReferenceKind;
    match value {
        GameplayConfigurationReferenceKind::Asset => ProjectContentReferenceKind::Asset,
        GameplayConfigurationReferenceKind::EntityDefinition => {
            ProjectContentReferenceKind::EntityDefinition
        }
        GameplayConfigurationReferenceKind::InstantiatedEntityDefinition => {
            ProjectContentReferenceKind::InstantiatedEntityDefinition
        }
        GameplayConfigurationReferenceKind::InstantiatedBoundedEntityDefinition => {
            ProjectContentReferenceKind::InstantiatedBoundedEntityDefinition
        }
        GameplayConfigurationReferenceKind::EntrySceneFpsPlayerEntityDefinition => {
            ProjectContentReferenceKind::EntrySceneFpsPlayerEntityDefinition
        }
        GameplayConfigurationReferenceKind::SceneInstance => {
            ProjectContentReferenceKind::SceneInstance
        }
        GameplayConfigurationReferenceKind::Prefab => ProjectContentReferenceKind::Prefab,
        GameplayConfigurationReferenceKind::PrefabPart => ProjectContentReferenceKind::PrefabPart,
        GameplayConfigurationReferenceKind::PresentationResource => {
            ProjectContentReferenceKind::PresentationResource
        }
    }
}

fn configuration_values_json(
    values: &[protocol_project_content::ProjectConfigurationFieldValueDto],
) -> Value {
    let mut object = Map::new();
    for field in values {
        let value = match &field.value {
            ProjectConfigurationValueDto::Boolean { value } => Value::Bool(*value),
            ProjectConfigurationValueDto::Integer { value } => Value::Number(Number::from(*value)),
            ProjectConfigurationValueDto::Number { value } => Number::from_f64(*value)
                .map(Value::Number)
                .unwrap_or(Value::Null),
            ProjectConfigurationValueDto::String { value } => Value::String(value.clone()),
            ProjectConfigurationValueDto::Reference { target_id, .. } => {
                Value::String(target_id.clone())
            }
        };
        object.insert(field.field_id.clone(), value);
    }
    Value::Object(object)
}
