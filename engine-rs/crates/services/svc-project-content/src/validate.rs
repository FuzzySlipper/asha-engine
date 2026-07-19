use std::collections::{BTreeMap, BTreeSet};

use protocol_entity_authoring::{
    EntityDefinition, EntityDefinitionCapability, EntityDefinitionValidationOutcome,
};
use protocol_game_extension::GameplayModuleBindingTarget;
use protocol_project_content::*;
use protocol_scene::{FlatSceneDocumentDto, SceneEntityReferenceDto, SceneNodeKindDto};

use crate::codec::{compiled_prefab_registry, core_catalog_from_stored};

#[derive(Default)]
struct ReferenceIndex<'a> {
    assets: BTreeSet<String>,
    entities: BTreeMap<String, &'a EntityDefinition>,
    prefabs: BTreeMap<u64, BTreeSet<String>>,
    base_prefabs: BTreeSet<u64>,
    prefab_variants: BTreeMap<u64, BTreeSet<String>>,
    scene_instances: BTreeMap<String, SceneInstanceReference>,
    presentation_resources: BTreeSet<String>,
}

#[derive(Debug, Clone)]
enum SceneInstanceReference {
    EntityDefinition {
        stable_id: String,
        transform_ok: bool,
    },
    Prefab {
        prefab_id: u64,
    },
}

pub(super) fn validate_document_set(
    documents: &[ProjectContentDocumentDto],
    scenes: &[FlatSceneDocumentDto],
    provider_schemas: &[ProjectConfigurationSchemaDto],
) -> Vec<ProjectContentDiagnosticDto> {
    let mut diagnostics = Vec::new();
    let mut index = ReferenceIndex::default();

    index_entities(documents, &mut index, &mut diagnostics);
    index_catalogs(documents, &mut index, &mut diagnostics);
    index_prefabs(documents, &mut index, &mut diagnostics);
    index_presentation(documents, &mut index, &mut diagnostics);
    index_scenes(scenes, &mut index, &mut diagnostics);
    let configuration_schemas = validate_configuration_schemas(provider_schemas, &mut diagnostics);

    validate_entities(documents, &mut diagnostics);
    validate_prefabs(documents, &index, &mut diagnostics);
    validate_gameplay(documents, &index, &configuration_schemas, &mut diagnostics);
    validate_presentation(documents, &index, &mut diagnostics);
    diagnostics
}

fn index_entities<'a>(
    documents: &'a [ProjectContentDocumentDto],
    index: &mut ReferenceIndex<'a>,
    diagnostics: &mut Vec<ProjectContentDiagnosticDto>,
) {
    for document in documents {
        if let ProjectContentDocumentDto::EntityDefinition {
            document_id,
            definition,
        } = document
        {
            if index
                .entities
                .insert(definition.stable_id.clone(), definition)
                .is_some()
            {
                push(
                    diagnostics,
                    ProjectContentDiagnosticCode::InvalidDocument,
                    Some(document_id),
                    "definition.stableId",
                    "entity definition stable ids must be unique across the project",
                );
            }
        }
    }
}

fn index_catalogs(
    documents: &[ProjectContentDocumentDto],
    index: &mut ReferenceIndex<'_>,
    diagnostics: &mut Vec<ProjectContentDiagnosticDto>,
) {
    for document in documents {
        if let ProjectContentDocumentDto::AssetCatalog {
            document_id,
            catalog,
        } = document
        {
            match core_catalog_from_stored(catalog) {
                Ok(core) => {
                    let report = core_catalog::validate(&core);
                    for error in report.errors {
                        push(
                            diagnostics,
                            ProjectContentDiagnosticCode::InvalidDocument,
                            Some(document_id),
                            "catalog",
                            &format!("{error:?}"),
                        );
                    }
                }
                Err(message) => push(
                    diagnostics,
                    ProjectContentDiagnosticCode::InvalidField,
                    Some(document_id),
                    "catalog",
                    &message,
                ),
            }
            for entry in &catalog.entries {
                if !index.assets.insert(entry.id.clone()) {
                    push(
                        diagnostics,
                        ProjectContentDiagnosticCode::InvalidDocument,
                        Some(document_id),
                        "catalog.entries",
                        "asset ids must be unique across project catalogs",
                    );
                }
            }
        }
    }
}

fn index_prefabs(
    documents: &[ProjectContentDocumentDto],
    index: &mut ReferenceIndex<'_>,
    diagnostics: &mut Vec<ProjectContentDiagnosticDto>,
) {
    for document in documents {
        if let ProjectContentDocumentDto::PrefabRegistry {
            document_id,
            registry,
        } = document
        {
            for definition in &registry.definitions {
                let roles = definition
                    .part_roles
                    .iter()
                    .map(|binding| binding.role.clone())
                    .collect();
                if index.prefabs.insert(definition.id.raw(), roles).is_some() {
                    push(
                        diagnostics,
                        ProjectContentDiagnosticCode::InvalidDocument,
                        Some(document_id),
                        "registry.definitions",
                        "prefab ids must be unique across project registries",
                    );
                }
                if let Some(variant) = &definition.variant {
                    if !index
                        .prefab_variants
                        .entry(variant.base.raw())
                        .or_default()
                        .insert(variant.variant_id.clone())
                    {
                        push(
                            diagnostics,
                            ProjectContentDiagnosticCode::InvalidDocument,
                            Some(document_id),
                            "registry.definitions.variant.variantId",
                            "variant ids must be unique for each base prefab",
                        );
                    }
                } else {
                    index.base_prefabs.insert(definition.id.raw());
                }
            }
        }
    }
}

fn index_presentation(
    documents: &[ProjectContentDocumentDto],
    index: &mut ReferenceIndex<'_>,
    diagnostics: &mut Vec<ProjectContentDiagnosticDto>,
) {
    for document in documents {
        if let ProjectContentDocumentDto::PresentationCatalog {
            document_id,
            catalog,
        } = document
        {
            for resource in &catalog.resources {
                if !index
                    .presentation_resources
                    .insert(resource.resource_id.clone())
                {
                    push(
                        diagnostics,
                        ProjectContentDiagnosticCode::InvalidDocument,
                        Some(document_id),
                        "catalog.resources",
                        "presentation resource ids must be unique across the project",
                    );
                }
            }
        }
    }
}

fn index_scenes(
    scenes: &[FlatSceneDocumentDto],
    index: &mut ReferenceIndex<'_>,
    diagnostics: &mut Vec<ProjectContentDiagnosticDto>,
) {
    for scene in scenes {
        let mut marker_ids = BTreeSet::new();
        let nodes = scene
            .nodes
            .iter()
            .map(|node| (node.id, node))
            .collect::<BTreeMap<_, _>>();
        for node in &scene.nodes {
            if let SceneNodeKindDto::Marker { marker_id } = &node.kind {
                if marker_id.trim().is_empty() || !marker_ids.insert(marker_id.clone()) {
                    push(
                        diagnostics,
                        ProjectContentDiagnosticCode::InvalidDocument,
                        None,
                        "workspace.scenes.nodes.markerId",
                        "scene marker ids must be non-empty and unique",
                    );
                }
            }
        }
        for node in &scene.nodes {
            let SceneNodeKindDto::EntityInstance { instance } = &node.kind else {
                continue;
            };
            if instance.instance_id.trim().is_empty()
                || index.scene_instances.contains_key(&instance.instance_id)
            {
                push(
                    diagnostics,
                    ProjectContentDiagnosticCode::InvalidDocument,
                    None,
                    "workspace.scenes.nodes.instanceId",
                    "scene instance ids must be non-empty and unique across project scenes",
                );
                continue;
            }
            if let Some(marker_id) = &instance.spawn_marker_id {
                if !marker_ids.contains(marker_id) {
                    push(
                        diagnostics,
                        ProjectContentDiagnosticCode::UnknownReference,
                        None,
                        "workspace.scenes.nodes.spawnMarkerId",
                        "scene entity instance references an unknown marker",
                    );
                }
            }
            let reference = match &instance.reference {
                SceneEntityReferenceDto::EntityDefinition { stable_id } => {
                    SceneInstanceReference::EntityDefinition {
                        stable_id: stable_id.clone(),
                        transform_ok: trigger_transform_is_supported(node, &nodes),
                    }
                }
                SceneEntityReferenceDto::Prefab {
                    prefab_id,
                    variant_id,
                    ..
                } => {
                    if !index.base_prefabs.contains(prefab_id) {
                        push(
                            diagnostics,
                            ProjectContentDiagnosticCode::UnknownReference,
                            None,
                            "workspace.scenes.nodes.reference.prefabId",
                            "scene instance references an unknown base prefab",
                        );
                    }
                    if let Some(variant_id) = variant_id {
                        let known = index
                            .prefab_variants
                            .get(prefab_id)
                            .is_some_and(|variants| variants.contains(variant_id));
                        if !known {
                            push(
                                diagnostics,
                                ProjectContentDiagnosticCode::UnknownReference,
                                None,
                                "workspace.scenes.nodes.reference.variantId",
                                "scene instance references an unknown variant for its base prefab",
                            );
                        }
                    }
                    SceneInstanceReference::Prefab {
                        prefab_id: *prefab_id,
                    }
                }
            };
            index
                .scene_instances
                .insert(instance.instance_id.clone(), reference);
        }
    }
}

fn trigger_transform_is_supported(
    node: &protocol_scene::SceneNodeRecordDto,
    nodes: &BTreeMap<core_ids::SceneNodeId, &protocol_scene::SceneNodeRecordDto>,
) -> bool {
    let mut current = Some(node);
    let mut visited = BTreeSet::new();
    while let Some(value) = current {
        if !visited.insert(value.id)
            || !value
                .transform
                .translation
                .iter()
                .all(|number| number.is_finite())
            || value.transform.rotation != [0.0, 0.0, 0.0, 1.0]
            || value.transform.scale != [1.0, 1.0, 1.0]
        {
            return false;
        }
        current = value.parent.and_then(|parent| nodes.get(&parent).copied());
    }
    true
}

fn validate_entities(
    documents: &[ProjectContentDocumentDto],
    diagnostics: &mut Vec<ProjectContentDiagnosticDto>,
) {
    for document in documents {
        if let ProjectContentDocumentDto::EntityDefinition {
            document_id,
            definition,
        } = document
        {
            if let EntityDefinitionValidationOutcome::Invalid {
                diagnostics: failures,
            } = svc_entity_authoring::validate_entity_definition(definition)
            {
                for failure in failures {
                    push(
                        diagnostics,
                        ProjectContentDiagnosticCode::InvalidDocument,
                        Some(document_id),
                        &failure.path,
                        &failure.message,
                    );
                }
            }
        }
    }
}

fn validate_prefabs(
    documents: &[ProjectContentDocumentDto],
    _index: &ReferenceIndex<'_>,
    diagnostics: &mut Vec<ProjectContentDiagnosticDto>,
) {
    if let Err(report) = compiled_prefab_registry(documents) {
        for failure in report.diagnostics {
            push(
                diagnostics,
                ProjectContentDiagnosticCode::InvalidDocument,
                None,
                &format!("workspace.prefabRegistry.{}", failure.path),
                &failure.message,
            );
        }
    }
}

fn validate_gameplay(
    documents: &[ProjectContentDocumentDto],
    index: &ReferenceIndex<'_>,
    schemas: &BTreeMap<&str, &ProjectConfigurationSchemaDto>,
    diagnostics: &mut Vec<ProjectContentDiagnosticDto>,
) {
    let mut trigger_targets = BTreeSet::new();
    let mut project_configuration_ids = BTreeSet::new();
    let mut project_binding_ids = BTreeSet::new();
    let mut project_override_identities = BTreeSet::new();
    for content in documents {
        let ProjectContentDocumentDto::GameplayConfiguration {
            document_id,
            document,
        } = content
        else {
            continue;
        };
        if document.schema_version != PROJECT_CONTENT_SCHEMA_VERSION {
            push(
                diagnostics,
                ProjectContentDiagnosticCode::InvalidDocument,
                Some(document_id),
                "document.schemaVersion",
                "unsupported gameplay configuration document schema",
            );
        }
        let mut configurations = BTreeMap::new();
        for (configuration_index, configuration) in document.configurations.iter().enumerate() {
            if configuration.configuration_id.trim().is_empty()
                || configurations
                    .insert(configuration.configuration_id.as_str(), configuration)
                    .is_some()
            {
                push(
                    diagnostics,
                    ProjectContentDiagnosticCode::InvalidDocument,
                    Some(document_id),
                    &format!("document.configurations[{configuration_index}].configurationId"),
                    "configuration ids must be non-empty and unique",
                );
            }
            if !project_configuration_ids.insert(configuration.configuration_id.as_str()) {
                push(
                    diagnostics,
                    ProjectContentDiagnosticCode::InvalidDocument,
                    Some(document_id),
                    &format!("document.configurations[{configuration_index}].configurationId"),
                    "configuration ids must be unique across the project",
                );
            }
            match schemas.get(configuration.schema_id.as_str()).copied() {
                Some(schema) => validate_configuration_values(
                    document_id,
                    configuration_index,
                    configuration,
                    schema,
                    index,
                    diagnostics,
                ),
                None => push(
                    diagnostics,
                    ProjectContentDiagnosticCode::UnknownReference,
                    Some(document_id),
                    &format!("document.configurations[{configuration_index}].schemaId"),
                    "configuration references an unknown provider schema",
                ),
            }
        }

        let mut bindings = BTreeMap::new();
        for (binding_index, binding) in document.bindings.iter().enumerate() {
            if binding.binding_id.trim().is_empty()
                || bindings
                    .insert(binding.binding_id.as_str(), binding)
                    .is_some()
            {
                push(
                    diagnostics,
                    ProjectContentDiagnosticCode::InvalidDocument,
                    Some(document_id),
                    &format!("document.bindings[{binding_index}].bindingId"),
                    "binding ids must be non-empty and unique",
                );
            }
            if !project_binding_ids.insert(binding.binding_id.as_str()) {
                push(
                    diagnostics,
                    ProjectContentDiagnosticCode::InvalidDocument,
                    Some(document_id),
                    &format!("document.bindings[{binding_index}].bindingId"),
                    "binding ids must be unique across the project",
                );
            }
            match configurations
                .get(binding.configuration_id.as_str())
                .copied()
            {
                None => push(
                    diagnostics,
                    ProjectContentDiagnosticCode::UnknownReference,
                    Some(document_id),
                    &format!("document.bindings[{binding_index}].configurationId"),
                    "binding references an unknown typed configuration",
                ),
                Some(configuration) if configuration.module.module_id != binding.module_id => {
                    push(
                        diagnostics,
                        ProjectContentDiagnosticCode::ReferenceKindMismatch,
                        Some(document_id),
                        &format!("document.bindings[{binding_index}].moduleId"),
                        "binding and selected configuration belong to different modules",
                    );
                }
                Some(_) => {}
            }
            validate_binding_target(
                document_id,
                binding_index,
                &binding.target,
                index,
                diagnostics,
            );
        }
        let mut override_identities = BTreeSet::new();
        for (override_index, layer) in document.overrides.iter().enumerate() {
            if !override_identities
                .insert((layer.binding_id.as_str(), layer.scene_instance_id.as_str()))
            {
                push(
                    diagnostics,
                    ProjectContentDiagnosticCode::InvalidDocument,
                    Some(document_id),
                    &format!("document.overrides[{override_index}]"),
                    "only one override is allowed per binding and scene instance",
                );
            }
            if !project_override_identities
                .insert((layer.binding_id.as_str(), layer.scene_instance_id.as_str()))
            {
                push(
                    diagnostics,
                    ProjectContentDiagnosticCode::InvalidDocument,
                    Some(document_id),
                    &format!("document.overrides[{override_index}]"),
                    "override identity must be unique across the project",
                );
            }
            let Some(binding) = bindings.get(layer.binding_id.as_str()).copied() else {
                push(
                    diagnostics,
                    ProjectContentDiagnosticCode::UnknownReference,
                    Some(document_id),
                    &format!("document.overrides[{override_index}].bindingId"),
                    "override references an unknown binding",
                );
                continue;
            };
            let Some(SceneInstanceReference::Prefab { prefab_id }) =
                index.scene_instances.get(&layer.scene_instance_id)
            else {
                push(
                    diagnostics,
                    ProjectContentDiagnosticCode::ReferenceKindMismatch,
                    Some(document_id),
                    &format!("document.overrides[{override_index}].sceneInstanceId"),
                    "override must target a stored scene prefab instance",
                );
                continue;
            };
            let target_prefab = match &binding.target {
                GameplayModuleBindingTarget::Prefab { prefab } => Some(prefab.raw()),
                GameplayModuleBindingTarget::PrefabPart { part } => Some(part.prefab.raw()),
                _ => None,
            };
            if target_prefab != Some(*prefab_id) {
                push(
                    diagnostics,
                    ProjectContentDiagnosticCode::ReferenceKindMismatch,
                    Some(document_id),
                    &format!("document.overrides[{override_index}]"),
                    "override scene instance does not use the binding target prefab",
                );
            }
            if let Some(configuration_id) = &layer.configuration_id {
                match configurations.get(configuration_id.as_str()).copied() {
                    None => push(
                        diagnostics,
                        ProjectContentDiagnosticCode::UnknownReference,
                        Some(document_id),
                        &format!("document.overrides[{override_index}].configurationId"),
                        "override references an unknown typed configuration",
                    ),
                    Some(configuration) if configuration.module.module_id != binding.module_id => {
                        push(
                            diagnostics,
                            ProjectContentDiagnosticCode::ReferenceKindMismatch,
                            Some(document_id),
                            &format!("document.overrides[{override_index}].configurationId"),
                            "override configuration belongs to a different binding module",
                        );
                    }
                    Some(_) => {}
                }
            }
        }
        for (trigger_index, trigger) in document.triggers.iter().enumerate() {
            if !trigger_targets.insert(trigger.scene_instance_id.as_str()) {
                push(
                    diagnostics,
                    ProjectContentDiagnosticCode::InvalidDocument,
                    Some(document_id),
                    &format!("document.triggers[{trigger_index}].sceneInstanceId"),
                    "only one trigger definition may target a stored scene entity",
                );
            }
            let Some(SceneInstanceReference::EntityDefinition {
                stable_id,
                transform_ok,
            }) = index.scene_instances.get(&trigger.scene_instance_id)
            else {
                push(
                    diagnostics,
                    ProjectContentDiagnosticCode::ReferenceKindMismatch,
                    Some(document_id),
                    &format!("document.triggers[{trigger_index}].sceneInstanceId"),
                    "trigger must target a stored entity-definition scene instance",
                );
                continue;
            };
            let capable = index.entities.get(stable_id).is_some_and(|definition| {
                let bounds = definition.capabilities.iter().any(|capability| {
                    matches!(capability, EntityDefinitionCapability::Bounds { min, max }
                        if min.iter().zip(max).all(|(min, max)| min.is_finite() && max.is_finite() && min < max))
                });
                let collision = definition.capabilities.iter().any(|capability| {
                    matches!(capability, EntityDefinitionCapability::Collision { static_collider: false })
                });
                bounds && collision
            });
            if !capable || !transform_ok {
                push(
                    diagnostics,
                    ProjectContentDiagnosticCode::ReferenceKindMismatch,
                    Some(document_id),
                    &format!("document.triggers[{trigger_index}]"),
                    "trigger target requires dynamic collision, usable bounds, finite translation, identity rotation, and unit scale across its ancestor chain",
                );
            }
        }
    }
}

fn validate_configuration_schemas<'a>(
    provider_schemas: &'a [ProjectConfigurationSchemaDto],
    diagnostics: &mut Vec<ProjectContentDiagnosticDto>,
) -> BTreeMap<&'a str, &'a ProjectConfigurationSchemaDto> {
    let mut schemas = BTreeMap::new();
    for (schema_index, schema) in provider_schemas.iter().enumerate() {
        let path = format!("composition.providerSchemas[{schema_index}]");
        if schema.schema_id.trim().is_empty()
            || schema.module_id.trim().is_empty()
            || schema.provider_id.trim().is_empty()
            || schema.codec_id.trim().is_empty()
            || schemas.insert(schema.schema_id.as_str(), schema).is_some()
        {
            push(
                diagnostics,
                ProjectContentDiagnosticCode::InvalidDocument,
                None,
                &path,
                "composed provider schemas require unique ids and non-empty module/provider/codec identities",
            );
        }
        validate_schema_fields(schema_index, schema, diagnostics);
    }
    schemas
}

fn validate_schema_fields(
    schema_index: usize,
    schema: &ProjectConfigurationSchemaDto,
    diagnostics: &mut Vec<ProjectContentDiagnosticDto>,
) {
    let mut fields = BTreeSet::new();
    for (field_index, field) in schema.fields.iter().enumerate() {
        let path = format!("composition.providerSchemas[{schema_index}].fields[{field_index}]");
        if field.field_id.trim().is_empty()
            || field.label.trim().is_empty()
            || !fields.insert(field.field_id.as_str())
        {
            push(
                diagnostics,
                ProjectContentDiagnosticCode::InvalidDocument,
                None,
                &path,
                "provider fields require unique ids and non-empty labels",
            );
        }
        if (field.value_kind == ProjectConfigurationValueKind::Reference)
            != field.reference_kind.is_some()
        {
            push(
                diagnostics,
                ProjectContentDiagnosticCode::InvalidField,
                None,
                &path,
                "referenceKind is required only for reference fields",
            );
        }
        if field
            .integer_min
            .zip(field.integer_max)
            .is_some_and(|(min, max)| min > max)
            || field.number_min.is_some_and(|min| !min.is_finite())
            || field.number_max.is_some_and(|max| !max.is_finite())
            || field
                .number_min
                .zip(field.number_max)
                .is_some_and(|(min, max)| min > max)
        {
            push(
                diagnostics,
                ProjectContentDiagnosticCode::InvalidField,
                None,
                &path,
                "provider field bounds are invalid",
            );
        }
    }
}

fn validate_configuration_values(
    document_id: &str,
    configuration_index: usize,
    configuration: &ProjectGameplayConfigurationDto,
    schema: &ProjectConfigurationSchemaDto,
    index: &ReferenceIndex<'_>,
    diagnostics: &mut Vec<ProjectContentDiagnosticDto>,
) {
    if configuration.module.module_id != schema.module_id {
        push(
            diagnostics,
            ProjectContentDiagnosticCode::ReferenceKindMismatch,
            Some(document_id),
            &format!("document.configurations[{configuration_index}].module.moduleId"),
            "configuration module does not own the selected schema",
        );
    }
    if configuration.module.provider_id != schema.provider_id {
        push(
            diagnostics,
            ProjectContentDiagnosticCode::ReferenceKindMismatch,
            Some(document_id),
            &format!("document.configurations[{configuration_index}].module.providerId"),
            "configuration module provider does not own the selected schema",
        );
    }
    let fields = schema
        .fields
        .iter()
        .map(|field| (field.field_id.as_str(), field))
        .collect::<BTreeMap<_, _>>();
    let mut seen = BTreeSet::new();
    for (value_index, entry) in configuration.values.iter().enumerate() {
        let path = format!("document.configurations[{configuration_index}].values[{value_index}]");
        if !seen.insert(entry.field_id.as_str()) {
            push(
                diagnostics,
                ProjectContentDiagnosticCode::InvalidDocument,
                Some(document_id),
                &path,
                "configuration field values must be unique",
            );
            continue;
        }
        let Some(field) = fields.get(entry.field_id.as_str()).copied() else {
            push(
                diagnostics,
                ProjectContentDiagnosticCode::UnknownReference,
                Some(document_id),
                &path,
                "configuration supplies an undeclared provider field",
            );
            continue;
        };
        validate_value(document_id, &path, field, &entry.value, index, diagnostics);
    }
    for field in &schema.fields {
        if field.required && !seen.contains(field.field_id.as_str()) {
            push(
                diagnostics,
                ProjectContentDiagnosticCode::InvalidField,
                Some(document_id),
                &format!("document.configurations[{configuration_index}].values"),
                &format!("missing required provider field `{}`", field.field_id),
            );
        }
    }
}

fn validate_value(
    document_id: &str,
    path: &str,
    field: &ProjectConfigurationFieldDto,
    value: &ProjectConfigurationValueDto,
    index: &ReferenceIndex<'_>,
    diagnostics: &mut Vec<ProjectContentDiagnosticDto>,
) {
    let kind_ok = matches!(
        (field.value_kind, value),
        (
            ProjectConfigurationValueKind::Boolean,
            ProjectConfigurationValueDto::Boolean { .. }
        ) | (
            ProjectConfigurationValueKind::Integer,
            ProjectConfigurationValueDto::Integer { .. }
        ) | (
            ProjectConfigurationValueKind::Number,
            ProjectConfigurationValueDto::Number { .. }
        ) | (
            ProjectConfigurationValueKind::String,
            ProjectConfigurationValueDto::String { .. }
        ) | (
            ProjectConfigurationValueKind::Reference,
            ProjectConfigurationValueDto::Reference { .. }
        )
    );
    if !kind_ok {
        push(
            diagnostics,
            ProjectContentDiagnosticCode::ReferenceKindMismatch,
            Some(document_id),
            path,
            "configuration value does not match provider field metadata",
        );
        return;
    }
    match value {
        ProjectConfigurationValueDto::Integer { value } => {
            if field.integer_min.is_some_and(|min| *value < min)
                || field.integer_max.is_some_and(|max| *value > max)
            {
                push(
                    diagnostics,
                    ProjectContentDiagnosticCode::InvalidField,
                    Some(document_id),
                    path,
                    "integer configuration value is outside provider bounds",
                );
            }
        }
        ProjectConfigurationValueDto::Number { value } => {
            if !value.is_finite()
                || field.number_min.is_some_and(|min| *value < min)
                || field.number_max.is_some_and(|max| *value > max)
            {
                push(
                    diagnostics,
                    ProjectContentDiagnosticCode::InvalidField,
                    Some(document_id),
                    path,
                    "number configuration value is non-finite or outside provider bounds",
                );
            }
        }
        ProjectConfigurationValueDto::String { value } if value.trim().is_empty() => push(
            diagnostics,
            ProjectContentDiagnosticCode::InvalidField,
            Some(document_id),
            path,
            "string configuration values must be non-empty",
        ),
        ProjectConfigurationValueDto::Reference {
            reference_kind,
            target_id,
        } if Some(*reference_kind) != field.reference_kind
            || !reference_exists(*reference_kind, target_id, index) =>
        {
            push(
                diagnostics,
                ProjectContentDiagnosticCode::UnknownReference,
                Some(document_id),
                path,
                "configuration reference is unknown or has the wrong kind",
            );
        }
        _ => {}
    }
}

fn reference_exists(
    kind: ProjectContentReferenceKind,
    target_id: &str,
    index: &ReferenceIndex<'_>,
) -> bool {
    match kind {
        ProjectContentReferenceKind::Asset => index.assets.contains(target_id),
        ProjectContentReferenceKind::EntityDefinition => index.entities.contains_key(target_id),
        ProjectContentReferenceKind::SceneInstance => index.scene_instances.contains_key(target_id),
        ProjectContentReferenceKind::Prefab => target_id
            .parse::<u64>()
            .is_ok_and(|id| index.prefabs.contains_key(&id)),
        ProjectContentReferenceKind::PrefabPart => {
            let Some((prefab, role)) = target_id.split_once(':') else {
                return false;
            };
            prefab
                .parse::<u64>()
                .ok()
                .and_then(|id| index.prefabs.get(&id))
                .is_some_and(|roles| roles.contains(role))
        }
        ProjectContentReferenceKind::PresentationResource => {
            index.presentation_resources.contains(target_id)
        }
    }
}

fn validate_binding_target(
    document_id: &str,
    binding_index: usize,
    target: &GameplayModuleBindingTarget,
    index: &ReferenceIndex<'_>,
    diagnostics: &mut Vec<ProjectContentDiagnosticDto>,
) {
    let known = match target {
        GameplayModuleBindingTarget::Session => true,
        GameplayModuleBindingTarget::EntityDefinition { stable_id } => {
            index.entities.contains_key(stable_id)
        }
        GameplayModuleBindingTarget::Prefab { prefab } => index.prefabs.contains_key(&prefab.raw()),
        GameplayModuleBindingTarget::PrefabPart { part } => index
            .prefabs
            .get(&part.prefab.raw())
            .is_some_and(|roles| roles.contains(&part.role)),
    };
    if !known {
        push(
            diagnostics,
            ProjectContentDiagnosticCode::UnknownReference,
            Some(document_id),
            &format!("document.bindings[{binding_index}].target"),
            "binding target does not resolve in the project content set",
        );
    }
}

fn validate_presentation(
    documents: &[ProjectContentDocumentDto],
    index: &ReferenceIndex<'_>,
    diagnostics: &mut Vec<ProjectContentDiagnosticDto>,
) {
    for document in documents {
        let ProjectContentDocumentDto::PresentationCatalog {
            document_id,
            catalog,
        } = document
        else {
            continue;
        };
        if catalog.schema_version != PROJECT_CONTENT_SCHEMA_VERSION {
            push(
                diagnostics,
                ProjectContentDiagnosticCode::InvalidDocument,
                Some(document_id),
                "catalog.schemaVersion",
                "unsupported presentation catalog schema",
            );
        }
        for (resource_index, resource) in catalog.resources.iter().enumerate() {
            if resource.resource_id.trim().is_empty()
                || resource.source_path.trim().is_empty()
                || resource.content_hash.trim().is_empty()
                || !index.assets.contains(&resource.asset_id)
            {
                push(
                    diagnostics,
                    ProjectContentDiagnosticCode::UnknownReference,
                    Some(document_id),
                    &format!("catalog.resources[{resource_index}]"),
                    "presentation resources require ids, source/content identity, and a catalog asset",
                );
            }
            let mut clips = BTreeSet::new();
            if resource
                .clip_ids
                .iter()
                .any(|clip| clip.trim().is_empty() || !clips.insert(clip.as_str()))
            {
                push(
                    diagnostics,
                    ProjectContentDiagnosticCode::InvalidDocument,
                    Some(document_id),
                    &format!("catalog.resources[{resource_index}].clipIds"),
                    "presentation clip ids must be non-empty and unique",
                );
            }
        }
        let resources = catalog
            .resources
            .iter()
            .map(|resource| (resource.resource_id.as_str(), resource))
            .collect::<BTreeMap<_, _>>();
        let mut cues = BTreeSet::new();
        for (cue_index, cue) in catalog.cues.iter().enumerate() {
            let (cue_id, resource_id) = match cue {
                ProjectPresentationCueDto::Animation {
                    cue_id,
                    resource_id,
                    clip_id,
                    ..
                } => {
                    if !resources
                        .get(resource_id.as_str())
                        .is_some_and(|resource| resource.clip_ids.contains(clip_id))
                    {
                        push(
                            diagnostics,
                            ProjectContentDiagnosticCode::UnknownReference,
                            Some(document_id),
                            &format!("catalog.cues[{cue_index}].clipId"),
                            "animation cue references an unknown resource clip",
                        );
                    }
                    (cue_id, resource_id)
                }
                ProjectPresentationCueDto::Audio {
                    cue_id,
                    resource_id,
                    gain,
                } => {
                    if !gain.is_finite() || *gain < 0.0 {
                        push(
                            diagnostics,
                            ProjectContentDiagnosticCode::InvalidField,
                            Some(document_id),
                            &format!("catalog.cues[{cue_index}].gain"),
                            "audio cue gain must be finite and non-negative",
                        );
                    }
                    (cue_id, resource_id)
                }
                ProjectPresentationCueDto::Particle {
                    cue_id,
                    resource_id,
                    scale,
                } => {
                    if !scale.is_finite() || *scale <= 0.0 {
                        push(
                            diagnostics,
                            ProjectContentDiagnosticCode::InvalidField,
                            Some(document_id),
                            &format!("catalog.cues[{cue_index}].scale"),
                            "particle cue scale must be finite and positive",
                        );
                    }
                    (cue_id, resource_id)
                }
                ProjectPresentationCueDto::Overlay {
                    cue_id,
                    resource_id,
                } => (cue_id, resource_id),
            };
            if cue_id.trim().is_empty() || !cues.insert(cue_id.as_str()) {
                push(
                    diagnostics,
                    ProjectContentDiagnosticCode::InvalidDocument,
                    Some(document_id),
                    &format!("catalog.cues[{cue_index}].cueId"),
                    "presentation cue ids must be non-empty and unique",
                );
            }
            if !resources.contains_key(resource_id.as_str()) {
                push(
                    diagnostics,
                    ProjectContentDiagnosticCode::UnknownReference,
                    Some(document_id),
                    &format!("catalog.cues[{cue_index}].resourceId"),
                    "presentation cue references an unknown resource",
                );
            }
        }
    }
}

pub(super) fn field_metadata(
    documents: &[ProjectContentDocumentDto],
    configuration_schemas: &[ProjectConfigurationSchemaDto],
) -> Vec<ProjectContentFieldMetadataDto> {
    let mut fields = Vec::new();
    for document in documents {
        let document_id = document.document_id().to_owned();
        match document {
            ProjectContentDocumentDto::EntityDefinition { .. } => {
                for (path, label, kind, reference) in [
                    (
                        "definition.stableId",
                        "Stable id",
                        ProjectConfigurationValueKind::String,
                        None,
                    ),
                    (
                        "definition.displayName",
                        "Display name",
                        ProjectConfigurationValueKind::String,
                        None,
                    ),
                    (
                        "definition.source.relativePath",
                        "Source path",
                        ProjectConfigurationValueKind::String,
                        None,
                    ),
                ] {
                    fields.push(metadata(&document_id, path, label, kind, true, reference));
                }
            }
            ProjectContentDocumentDto::AssetCatalog { catalog, .. } => {
                for (index, _) in catalog.entries.iter().enumerate() {
                    fields.push(metadata(
                        &document_id,
                        &format!("catalog.entries[{index}].id"),
                        "Asset id",
                        ProjectConfigurationValueKind::Reference,
                        true,
                        Some(ProjectContentReferenceKind::Asset),
                    ));
                }
            }
            ProjectContentDocumentDto::PrefabRegistry { registry, .. } => {
                for (index, _) in registry.definitions.iter().enumerate() {
                    fields.push(metadata(
                        &document_id,
                        &format!("registry.definitions[{index}].displayName"),
                        "Prefab display name",
                        ProjectConfigurationValueKind::String,
                        true,
                        None,
                    ));
                }
            }
            ProjectContentDocumentDto::GameplayConfiguration { document, .. } => {
                for (configuration_index, configuration) in
                    document.configurations.iter().enumerate()
                {
                    let Some(schema) = configuration_schemas
                        .iter()
                        .find(|schema| schema.schema_id == configuration.schema_id)
                    else {
                        continue;
                    };
                    for field in &schema.fields {
                        fields.push(ProjectContentFieldMetadataDto {
                            document_id: document_id.clone(),
                            path: format!(
                                "document.configurations[{configuration_index}].values.{}",
                                field.field_id
                            ),
                            label: field.label.clone(),
                            value_kind: field.value_kind,
                            required: field.required,
                            editable: true,
                            reference_kind: field.reference_kind,
                            configuration_id: Some(configuration.configuration_id.clone()),
                            schema_id: Some(schema.schema_id.clone()),
                            module_id: Some(schema.module_id.clone()),
                            provider_id: Some(schema.provider_id.clone()),
                            contract: Some(schema.contract.clone()),
                            codec_id: Some(schema.codec_id.clone()),
                            integer_min: field.integer_min,
                            integer_max: field.integer_max,
                            number_min: field.number_min,
                            number_max: field.number_max,
                        });
                    }
                }
            }
            ProjectContentDocumentDto::PresentationCatalog { catalog, .. } => {
                for (index, _) in catalog.resources.iter().enumerate() {
                    fields.push(metadata(
                        &document_id,
                        &format!("catalog.resources[{index}].assetId"),
                        "Resource asset",
                        ProjectConfigurationValueKind::Reference,
                        true,
                        Some(ProjectContentReferenceKind::Asset),
                    ));
                }
            }
        }
    }
    fields.sort_by(|left, right| {
        (left.document_id.as_str(), left.path.as_str())
            .cmp(&(right.document_id.as_str(), right.path.as_str()))
    });
    fields
}

fn metadata(
    document_id: &str,
    path: &str,
    label: &str,
    value_kind: ProjectConfigurationValueKind,
    required: bool,
    reference_kind: Option<ProjectContentReferenceKind>,
) -> ProjectContentFieldMetadataDto {
    ProjectContentFieldMetadataDto {
        document_id: document_id.to_owned(),
        path: path.to_owned(),
        label: label.to_owned(),
        value_kind,
        required,
        editable: true,
        reference_kind,
        configuration_id: None,
        schema_id: None,
        module_id: None,
        provider_id: None,
        contract: None,
        codec_id: None,
        integer_min: None,
        integer_max: None,
        number_min: None,
        number_max: None,
    }
}

fn push(
    diagnostics: &mut Vec<ProjectContentDiagnosticDto>,
    code: ProjectContentDiagnosticCode,
    document_id: Option<&str>,
    path: &str,
    message: &str,
) {
    diagnostics.push(ProjectContentDiagnosticDto {
        code,
        document_id: document_id.map(str::to_owned),
        path: path.to_owned(),
        message: message.to_owned(),
    });
}
