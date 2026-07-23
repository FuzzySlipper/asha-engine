use std::collections::{BTreeMap, BTreeSet};

use core_ids::SceneId;
use protocol_entity_authoring::{
    EntityDefinition, EntityDefinitionCapability, EntityDefinitionValidationOutcome,
};
use protocol_game_extension::GameplayModuleBindingTarget;
use protocol_project_content::*;
use protocol_scene::{FlatSceneDocumentDto, SceneEntityReferenceDto, SceneNodeKindDto};

use crate::codec::{compiled_prefab_registry, core_catalog_from_stored};

#[derive(Default)]
struct ReferenceIndex<'a> {
    assets: BTreeMap<String, &'a protocol_assets::StoredCatalogEntry>,
    entities: BTreeMap<String, &'a EntityDefinition>,
    prefabs: BTreeMap<u64, BTreeSet<String>>,
    base_prefabs: BTreeSet<u64>,
    prefab_variants: BTreeMap<u64, BTreeSet<String>>,
    scene_instances: BTreeMap<String, SceneInstanceReference>,
    presentation_resources: BTreeMap<String, &'a ProjectPresentationResourceDto>,
}

#[derive(Debug, Clone)]
enum SceneInstanceReference {
    EntityDefinition {
        scene_id: SceneId,
        stable_id: String,
        transform_ok: bool,
    },
    Prefab {
        scene_id: SceneId,
        prefab_id: u64,
    },
}

#[derive(Clone, Copy)]
struct SemanticReferenceContext<'a> {
    entry_scene_id: Option<SceneId>,
    gameplay: &'a dyn crate::ProjectContentGameplayAdmission,
}

pub(super) fn validate_document_set(
    documents: &[ProjectContentDocumentDto],
    scenes: &[FlatSceneDocumentDto],
    entry_scene_id: Option<SceneId>,
    gameplay: &dyn crate::ProjectContentGameplayAdmission,
) -> Vec<ProjectContentDiagnosticDto> {
    let mut diagnostics = Vec::new();
    let mut index = ReferenceIndex::default();

    index_entities(documents, &mut index, &mut diagnostics);
    index_catalogs(documents, &mut index, &mut diagnostics);
    index_prefabs(documents, &mut index, &mut diagnostics);
    index_presentation(documents, &mut index, &mut diagnostics);
    index_scenes(scenes, &mut index, &mut diagnostics);
    let configuration_schemas =
        validate_configuration_schemas(gameplay.configuration_schemas(), &mut diagnostics);

    validate_entities(documents, &index, &mut diagnostics);
    validate_prefabs(documents, &index, &mut diagnostics);
    validate_gameplay(
        documents,
        &index,
        entry_scene_id,
        gameplay,
        &configuration_schemas,
        &mut diagnostics,
    );
    validate_authored_behaviors(
        documents,
        &index,
        entry_scene_id,
        gameplay,
        &mut diagnostics,
    );
    validate_presentation(documents, &index, &mut diagnostics);
    diagnostics
}

fn validate_authored_behaviors(
    documents: &[ProjectContentDocumentDto],
    index: &ReferenceIndex<'_>,
    entry_scene_id: Option<SceneId>,
    gameplay: &dyn crate::ProjectContentGameplayAdmission,
    diagnostics: &mut Vec<ProjectContentDiagnosticDto>,
) {
    let mut package_ids = BTreeSet::new();
    for document in documents {
        let ProjectContentDocumentDto::BehaviorPackage {
            document_id,
            package,
        } = document
        else {
            continue;
        };
        let package_diagnostic_start = diagnostics.len();
        let base = "package";
        if package.schema_version != AUTHORED_BEHAVIOR_PACKAGE_SCHEMA_VERSION {
            push(
                diagnostics,
                ProjectContentDiagnosticCode::InvalidField,
                Some(document_id),
                "package.schemaVersion",
                "unsupported authored-behavior package schema version",
            );
        }
        validate_authored_id(
            &package.package_id,
            document_id,
            "package.packageId",
            diagnostics,
        );
        if !package_ids.insert(package.package_id.as_str()) {
            push(
                diagnostics,
                ProjectContentDiagnosticCode::InvalidDocument,
                Some(document_id),
                "package.packageId",
                "authored-behavior package ids must be unique across the project",
            );
        }
        if package.provenance.sdk_id != "@asha/game-workspace"
            || package.provenance.sdk_version != AUTHORED_BEHAVIOR_VOCABULARY_VERSION
            || package.provenance.vocabulary_hash != AUTHORED_BEHAVIOR_VOCABULARY_HASH
            || !valid_authored_source_module(&package.provenance.source_module)
            || !valid_authored_source_path(&package.provenance.source_path)
            || package.provenance.source_hash.trim().is_empty()
        {
            push(
                diagnostics,
                ProjectContentDiagnosticCode::InvalidField,
                Some(document_id),
                "package.provenance",
                "behavior provenance must identify the current generated Engine vocabulary, a stable source module/path, and a nonempty source hash",
            );
        }
        if package.state_machines.is_empty()
            || package.state_machines.len()
                > usize::try_from(AUTHORED_BEHAVIOR_MAX_MACHINES).unwrap_or(usize::MAX)
            || package.behaviors.is_empty()
            || package.behaviors.len()
                > usize::try_from(AUTHORED_BEHAVIOR_MAX_BEHAVIORS).unwrap_or(usize::MAX)
        {
            push(
                diagnostics,
                ProjectContentDiagnosticCode::InvalidDocument,
                Some(document_id),
                base,
                "authored behavior package exceeds its machine/behavior budget or is empty",
            );
        }

        let mut machines = BTreeMap::new();
        for (machine_index, machine) in package.state_machines.iter().enumerate() {
            let path = format!("package.stateMachines[{machine_index}]");
            validate_authored_id(
                &machine.machine_id,
                document_id,
                &format!("{path}.machineId"),
                diagnostics,
            );
            if machines
                .insert(machine.machine_id.as_str(), machine)
                .is_some()
            {
                push(
                    diagnostics,
                    ProjectContentDiagnosticCode::InvalidDocument,
                    Some(document_id),
                    &format!("{path}.machineId"),
                    "machine ids must be unique within a package",
                );
            }
            validate_authored_machine(
                document_id,
                &path,
                machine,
                index,
                entry_scene_id,
                diagnostics,
            );
        }

        let mut behavior_ids = BTreeSet::new();
        for (behavior_index, behavior) in package.behaviors.iter().enumerate() {
            let path = format!("package.behaviors[{behavior_index}]");
            validate_authored_id(
                &behavior.behavior_id,
                document_id,
                &format!("{path}.behaviorId"),
                diagnostics,
            );
            if !behavior_ids.insert(behavior.behavior_id.as_str()) {
                push(
                    diagnostics,
                    ProjectContentDiagnosticCode::InvalidDocument,
                    Some(document_id),
                    &format!("{path}.behaviorId"),
                    "behavior ids must be unique within a package",
                );
            }
            validate_authored_signal(
                document_id,
                &path,
                &behavior.signal,
                index,
                entry_scene_id,
                gameplay,
                diagnostics,
            );
            validate_authored_sequence(
                document_id,
                &path,
                behavior,
                &machines,
                index,
                entry_scene_id,
                diagnostics,
            );
        }
        if valid_authored_source_module(&package.provenance.source_module)
            && valid_authored_source_path(&package.provenance.source_path)
        {
            for diagnostic in &mut diagnostics[package_diagnostic_start..] {
                if diagnostic.document_id.as_deref() == Some(document_id.as_str()) {
                    diagnostic.message = format!(
                        "[{}:{}] {}",
                        package.provenance.source_module,
                        package.provenance.source_path,
                        diagnostic.message
                    );
                }
            }
        }
    }
}

fn validate_authored_machine(
    document_id: &str,
    path: &str,
    machine: &AuthoredBehaviorStateMachineDto,
    index: &ReferenceIndex<'_>,
    entry_scene_id: Option<SceneId>,
    diagnostics: &mut Vec<ProjectContentDiagnosticDto>,
) {
    match index.scene_instances.get(&machine.target_scene_instance_id) {
        Some(SceneInstanceReference::EntityDefinition { scene_id, .. })
            if Some(*scene_id) == entry_scene_id => {}
        _ => push(
            diagnostics,
            ProjectContentDiagnosticCode::UnknownReference,
            Some(document_id),
            &format!("{path}.targetSceneInstanceId"),
            "state-machine target must be an entity-definition instance in the entry scene",
        ),
    }
    if machine.states.is_empty()
        || machine.states.len()
            > usize::try_from(AUTHORED_BEHAVIOR_MAX_STATES_PER_MACHINE).unwrap_or(usize::MAX)
        || machine.transitions.is_empty()
        || machine.transitions.len()
            > usize::try_from(AUTHORED_BEHAVIOR_MAX_TRANSITIONS_PER_MACHINE).unwrap_or(usize::MAX)
    {
        push(
            diagnostics,
            ProjectContentDiagnosticCode::InvalidDocument,
            Some(document_id),
            path,
            "state machine exceeds its state/transition budget or is empty",
        );
    }
    let mut states = BTreeSet::new();
    for (state_index, state) in machine.states.iter().enumerate() {
        validate_authored_id(
            &state.state_id,
            document_id,
            &format!("{path}.states[{state_index}].stateId"),
            diagnostics,
        );
        if !states.insert(state.state_id.as_str()) {
            push(
                diagnostics,
                ProjectContentDiagnosticCode::InvalidDocument,
                Some(document_id),
                &format!("{path}.states[{state_index}].stateId"),
                "state ids must be unique within a machine",
            );
        }
    }
    if !states.contains(machine.initial_state_id.as_str()) {
        push(
            diagnostics,
            ProjectContentDiagnosticCode::UnknownReference,
            Some(document_id),
            &format!("{path}.initialStateId"),
            "initial state does not resolve within the machine",
        );
    }
    let mut transitions = BTreeSet::new();
    for (transition_index, transition) in machine.transitions.iter().enumerate() {
        validate_authored_id(
            &transition.transition_id,
            document_id,
            &format!("{path}.transitions[{transition_index}].transitionId"),
            diagnostics,
        );
        if !transitions.insert(transition.transition_id.as_str())
            || transition.from_state_id == transition.to_state_id
        {
            push(
                diagnostics,
                ProjectContentDiagnosticCode::InvalidDocument,
                Some(document_id),
                &format!("{path}.transitions[{transition_index}]"),
                "transition ids must be unique and transitions must change state",
            );
        }
        if !states.contains(transition.from_state_id.as_str())
            || !states.contains(transition.to_state_id.as_str())
        {
            push(
                diagnostics,
                ProjectContentDiagnosticCode::UnknownReference,
                Some(document_id),
                &format!("{path}.transitions[{transition_index}]"),
                "transition states must resolve within the machine",
            );
        }
    }
}

fn validate_authored_signal(
    document_id: &str,
    path: &str,
    signal: &AuthoredBehaviorSignalDto,
    index: &ReferenceIndex<'_>,
    entry_scene_id: Option<SceneId>,
    gameplay: &dyn crate::ProjectContentGameplayAdmission,
    diagnostics: &mut Vec<ProjectContentDiagnosticDto>,
) {
    if signal.arguments.len()
        > usize::try_from(AUTHORED_BEHAVIOR_MAX_ARGUMENTS).unwrap_or(usize::MAX)
        || !unique_authored_arguments(&signal.arguments)
    {
        push(
            diagnostics,
            ProjectContentDiagnosticCode::InvalidField,
            Some(document_id),
            &format!("{path}.signal"),
            "signal arguments must be bounded and unique",
        );
        return;
    }
    let Some(event) =
        gameplay.resolve_authored_signal(&signal.signal.semantic_id, signal.signal.version)
    else {
        push(
            diagnostics,
            ProjectContentDiagnosticCode::UnknownReference,
            Some(document_id),
            &format!("{path}.signal.signal.semanticId"),
            "signal semantic id and version do not resolve to an event published by the statically composed Rust gameplay registry",
        );
        return;
    };
    if let Err(message) = gameplay.validate_authored_signal_arguments(&event, &signal.arguments) {
        push(
            diagnostics,
            ProjectContentDiagnosticCode::InvalidField,
            Some(document_id),
            &format!("{path}.signal.arguments"),
            &message,
        );
        return;
    }
    for (argument_index, argument) in signal.arguments.iter().enumerate() {
        let argument_path = format!("{path}.signal.arguments[{argument_index}]");
        validate_authored_id(
            &argument.name,
            document_id,
            &format!("{argument_path}.name"),
            diagnostics,
        );
        let valid = match &argument.value {
            AuthoredBehaviorValueDto::SceneEntity { scene_instance_id } => {
                entry_scene_entity(index, scene_instance_id, entry_scene_id)
            }
            AuthoredBehaviorValueDto::PrefabPart {
                scene_instance_id,
                role,
            } => matches!(
                index.scene_instances.get(scene_instance_id),
                Some(SceneInstanceReference::Prefab {
                    scene_id,
                    prefab_id,
                }) if Some(*scene_id) == entry_scene_id
                    && index
                        .prefabs
                        .get(prefab_id)
                        .is_some_and(|roles| roles.contains(role))
            ),
            AuthoredBehaviorValueDto::StateMachine { .. }
            | AuthoredBehaviorValueDto::State { .. } => false,
            AuthoredBehaviorValueDto::Text { value } => value.len() <= 1_024,
            AuthoredBehaviorValueDto::Boolean { .. } | AuthoredBehaviorValueDto::Integer { .. } => {
                true
            }
            AuthoredBehaviorValueDto::Number { value } => value.is_finite(),
            AuthoredBehaviorValueDto::Vector3 { value } => {
                value.iter().all(|component| component.is_finite())
            }
        };
        if !valid {
            push(
                diagnostics,
                ProjectContentDiagnosticCode::UnknownReference,
                Some(document_id),
                &format!("{argument_path}.value"),
                "signal argument must be a finite data value or resolve to an entry-scene entity or prefab part",
            );
        }
    }
}

fn validate_authored_sequence(
    document_id: &str,
    path: &str,
    behavior: &AuthoredBehaviorDefinitionDto,
    machines: &BTreeMap<&str, &AuthoredBehaviorStateMachineDto>,
    index: &ReferenceIndex<'_>,
    entry_scene_id: Option<SceneId>,
    diagnostics: &mut Vec<ProjectContentDiagnosticDto>,
) {
    if behavior.steps.is_empty()
        || behavior.steps.len()
            > usize::try_from(AUTHORED_BEHAVIOR_MAX_STEPS_PER_BEHAVIOR).unwrap_or(usize::MAX)
    {
        push(
            diagnostics,
            ProjectContentDiagnosticCode::InvalidDocument,
            Some(document_id),
            &format!("{path}.steps"),
            "behavior transition sequence exceeds its step budget or is empty",
        );
        return;
    }
    let mut step_ids = BTreeSet::new();
    for (step_index, step) in behavior.steps.iter().enumerate() {
        validate_authored_id(
            &step.step_id,
            document_id,
            &format!("{path}.steps[{step_index}].stepId"),
            diagnostics,
        );
        if !step_ids.insert(step.step_id.as_str()) {
            push(
                diagnostics,
                ProjectContentDiagnosticCode::InvalidDocument,
                Some(document_id),
                &format!("{path}.steps"),
                "step ids must be unique within a behavior",
            );
        }
        if step.after_step_ids.len() > 1
            || step.delay_ticks > AUTHORED_BEHAVIOR_MAX_DELAY_TICKS
            || step.operations.is_empty()
            || step.operations.len()
                > usize::try_from(AUTHORED_BEHAVIOR_MAX_OPERATIONS_PER_STEP).unwrap_or(usize::MAX)
        {
            push(
                diagnostics,
                ProjectContentDiagnosticCode::InvalidDocument,
                Some(document_id),
                &format!("{path}.steps[{step_index}]"),
                "a behavior step must contain bounded operations, at most one predecessor, and a bounded delay",
            );
        }
        if step.after_step_ids.is_empty() && step.delay_ticks != 0 {
            push(
                diagnostics,
                ProjectContentDiagnosticCode::InvalidDocument,
                Some(document_id),
                &format!("{path}.steps[{step_index}].delayTicks"),
                "a root step executes immediately; only continuations may be delayed",
            );
        }
        for (operation_index, operation) in step.operations.iter().enumerate() {
            validate_authored_operation(
                document_id,
                &format!("{path}.steps[{step_index}].operations[{operation_index}]"),
                operation,
                machines,
                index,
                entry_scene_id,
                diagnostics,
            );
        }
    }
    let mut remaining = behavior.steps.iter().collect::<Vec<_>>();
    let mut resolved = BTreeSet::new();
    while !remaining.is_empty() {
        let before = remaining.len();
        remaining.retain(|step| {
            let ready = step
                .after_step_ids
                .iter()
                .all(|dependency| resolved.contains(dependency.as_str()));
            if ready {
                resolved.insert(step.step_id.as_str());
            }
            !ready
        });
        if remaining.len() == before {
            push(
                diagnostics,
                ProjectContentDiagnosticCode::InvalidDocument,
                Some(document_id),
                &format!("{path}.steps"),
                "step dependencies contain a cycle or unknown step reference",
            );
            break;
        }
    }
    for (condition_index, condition) in behavior.conditions.iter().enumerate() {
        let valid = condition.predicate.version == AUTHORED_BEHAVIOR_VOCABULARY_VERSION
            && condition.predicate.semantic_id == AUTHORED_PREDICATE_STATE_IS
            && condition.arguments.len() == 1
            && unique_authored_arguments(&condition.arguments)
            && matches!(
                authored_argument(&condition.arguments, "state"),
                Some(AuthoredBehaviorValueDto::State { machine_id, state_id })
                    if authored_state_exists(machines, machine_id, state_id)
            );
        if !valid {
            push(
                diagnostics,
                ProjectContentDiagnosticCode::UnknownReference,
                Some(document_id),
                &format!("{path}.conditions[{condition_index}]"),
                "condition must use the published state-is predicate with one valid typed state argument",
            );
        }
    }
    if !behavior
        .steps
        .iter()
        .any(|step| step.delay_ticks == 0 && step.after_step_ids.is_empty())
    {
        push(
            diagnostics,
            ProjectContentDiagnosticCode::InvalidDocument,
            Some(document_id),
            &format!("{path}.steps"),
            "behavior requires at least one immediate root step",
        );
    }
}

fn validate_authored_operation(
    document_id: &str,
    path: &str,
    operation: &AuthoredBehaviorOperationDto,
    machines: &BTreeMap<&str, &AuthoredBehaviorStateMachineDto>,
    index: &ReferenceIndex<'_>,
    entry_scene_id: Option<SceneId>,
    diagnostics: &mut Vec<ProjectContentDiagnosticDto>,
) {
    if operation.verb.version != AUTHORED_BEHAVIOR_VOCABULARY_VERSION
        || operation.arguments.len()
            > usize::try_from(AUTHORED_BEHAVIOR_MAX_ARGUMENTS).unwrap_or(usize::MAX)
        || !unique_authored_arguments(&operation.arguments)
    {
        push(
            diagnostics,
            ProjectContentDiagnosticCode::InvalidField,
            Some(document_id),
            path,
            "verb arguments must be bounded and unique and the verb version must be supported",
        );
        return;
    }
    let valid = match operation.verb.semantic_id.as_str() {
        AUTHORED_VERB_TRANSITION_STATE => {
            operation.arguments.len() == 2
                && matches!(
                    authored_argument(&operation.arguments, "machine"),
                    Some(AuthoredBehaviorValueDto::StateMachine { machine_id })
                        if machines.contains_key(machine_id.as_str())
                )
                && matches!(
                    (
                        authored_argument(&operation.arguments, "machine"),
                        authored_argument(&operation.arguments, "transition"),
                    ),
                    (
                        Some(AuthoredBehaviorValueDto::StateMachine { machine_id }),
                        Some(AuthoredBehaviorValueDto::Text { value }),
                    ) if machines.get(machine_id.as_str()).is_some_and(|machine| {
                        machine.transitions.iter().any(|transition| transition.transition_id == *value)
                    })
                )
        }
        AUTHORED_VERB_SET_RELATIVE_TRANSLATION => {
            operation.arguments.len() == 2
                && matches!(
                    authored_argument(&operation.arguments, "entity"),
                    Some(AuthoredBehaviorValueDto::SceneEntity { scene_instance_id })
                        if entry_scene_entity(index, scene_instance_id, entry_scene_id)
                )
                && matches!(
                    authored_argument(&operation.arguments, "value"),
                    Some(AuthoredBehaviorValueDto::Vector3 { value })
                        if value.iter().all(|component| component.is_finite())
                )
        }
        AUTHORED_VERB_SET_CAPABILITY_ACTIVE => {
            operation.arguments.len() == 3
                && matches!(
                    authored_argument(&operation.arguments, "entity"),
                    Some(AuthoredBehaviorValueDto::SceneEntity { scene_instance_id })
                        if entry_scene_entity(index, scene_instance_id, entry_scene_id)
                )
                && matches!(
                    authored_argument(&operation.arguments, "capability"),
                    Some(AuthoredBehaviorValueDto::Text { value }) if value == "collision"
                )
                && matches!(
                    authored_argument(&operation.arguments, "active"),
                    Some(AuthoredBehaviorValueDto::Boolean { .. })
                )
        }
        _ => false,
    };
    if !valid {
        push(
            diagnostics,
            ProjectContentDiagnosticCode::UnknownReference,
            Some(document_id),
            path,
            "verb semantic id or typed arguments are not published by the Rust authored-program catalog",
        );
    }
}

fn authored_state_exists(
    machines: &BTreeMap<&str, &AuthoredBehaviorStateMachineDto>,
    machine_id: &str,
    state_id: &str,
) -> bool {
    machines.get(machine_id).is_some_and(|machine| {
        machine
            .states
            .iter()
            .any(|state| state.state_id == state_id)
    })
}

fn authored_argument<'a>(
    arguments: &'a [AuthoredBehaviorArgumentDto],
    name: &str,
) -> Option<&'a AuthoredBehaviorValueDto> {
    arguments
        .iter()
        .find(|argument| argument.name == name)
        .map(|argument| &argument.value)
}

fn unique_authored_arguments(arguments: &[AuthoredBehaviorArgumentDto]) -> bool {
    arguments
        .iter()
        .map(|argument| argument.name.as_str())
        .collect::<BTreeSet<_>>()
        .len()
        == arguments.len()
}

fn entry_scene_entity(
    index: &ReferenceIndex<'_>,
    scene_instance_id: &str,
    entry_scene_id: Option<SceneId>,
) -> bool {
    matches!(
        index.scene_instances.get(scene_instance_id),
        Some(SceneInstanceReference::EntityDefinition { scene_id, .. })
            if Some(*scene_id) == entry_scene_id
    )
}

fn validate_authored_id(
    value: &str,
    document_id: &str,
    path: &str,
    diagnostics: &mut Vec<ProjectContentDiagnosticDto>,
) {
    let valid = !value.is_empty()
        && value.len() <= 96
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_'));
    if !valid {
        push(
            diagnostics,
            ProjectContentDiagnosticCode::InvalidField,
            Some(document_id),
            path,
            "authored behavior ids must use 1-96 ASCII letters, digits, dot, dash, or underscore",
        );
    }
}

fn valid_authored_source_module(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'@' | b'/' | b'.' | b'-' | b'_')
        })
}

fn valid_authored_source_path(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 512
        && !value.starts_with('/')
        && !value.contains('\\')
        && value.bytes().all(|byte| byte.is_ascii_graphic())
        && value
            .split('/')
            .all(|segment| !segment.is_empty() && segment != "." && segment != "..")
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

fn index_catalogs<'a>(
    documents: &'a [ProjectContentDocumentDto],
    index: &mut ReferenceIndex<'a>,
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
                if index.assets.insert(entry.id.clone(), entry).is_some() {
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

fn index_presentation<'a>(
    documents: &'a [ProjectContentDocumentDto],
    index: &mut ReferenceIndex<'a>,
    diagnostics: &mut Vec<ProjectContentDiagnosticDto>,
) {
    for document in documents {
        if let ProjectContentDocumentDto::PresentationCatalog {
            document_id,
            catalog,
        } = document
        {
            for resource in &catalog.resources {
                if index
                    .presentation_resources
                    .insert(resource.resource_id.clone(), resource)
                    .is_some()
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
                        scene_id: scene.id,
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
                        scene_id: scene.id,
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
    index: &ReferenceIndex<'_>,
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
            for (capability_index, capability) in definition.capabilities.iter().enumerate() {
                let EntityDefinitionCapability::RenderProjection {
                    appearance: Some(appearance),
                    ..
                } = capability
                else {
                    continue;
                };
                let path =
                    format!("definition.capabilities[{capability_index}].appearance.resourceId");
                let Some(resource) = index
                    .presentation_resources
                    .get(appearance.resource_id.as_str())
                else {
                    push(
                        diagnostics,
                        ProjectContentDiagnosticCode::UnknownReference,
                        Some(document_id),
                        &path,
                        "entity appearance references an unknown presentation resource",
                    );
                    continue;
                };
                let Some(animated_mesh) = resource.animated_mesh.as_ref() else {
                    push(
                        diagnostics,
                        ProjectContentDiagnosticCode::InvalidField,
                        Some(document_id),
                        &path,
                        "entity appearance requires an animated-mesh presentation resource",
                    );
                    continue;
                };
                if appearance.initial_clip_id.as_ref().is_some_and(|clip| {
                    !animated_mesh
                        .clips
                        .iter()
                        .any(|descriptor| descriptor.id == *clip)
                }) {
                    push(
                        diagnostics,
                        ProjectContentDiagnosticCode::UnknownReference,
                        Some(document_id),
                        &format!(
                            "definition.capabilities[{capability_index}].appearance.initialClipId"
                        ),
                        "entity appearance initial clip is absent from its presentation resource",
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
    entry_scene_id: Option<SceneId>,
    gameplay: &dyn crate::ProjectContentGameplayAdmission,
    schemas: &BTreeMap<&str, &ProjectConfigurationSchemaDto>,
    diagnostics: &mut Vec<ProjectContentDiagnosticDto>,
) {
    let reference_context = SemanticReferenceContext {
        entry_scene_id,
        gameplay,
    };
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
                    reference_context,
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
            let Some(SceneInstanceReference::Prefab { prefab_id, .. }) =
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
                ..
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
    reference_context: SemanticReferenceContext<'_>,
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
        validate_value(
            document_id,
            &path,
            field,
            &entry.value,
            index,
            reference_context,
            diagnostics,
        );
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
    reference_context: SemanticReferenceContext<'_>,
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
            || !reference_exists(
                *reference_kind,
                target_id,
                index,
                reference_context.entry_scene_id,
                reference_context.gameplay,
            ) =>
        {
            push(
                diagnostics,
                ProjectContentDiagnosticCode::UnknownReference,
                Some(document_id),
                path,
                &format!(
                    "configuration reference `{target_id}` is unknown or does not satisfy the required {reference_kind:?} reference for field `{}`",
                    field.field_id
                ),
            );
        }
        _ => {}
    }
}

fn reference_exists(
    kind: ProjectContentReferenceKind,
    target_id: &str,
    index: &ReferenceIndex<'_>,
    entry_scene_id: Option<SceneId>,
    gameplay: &dyn crate::ProjectContentGameplayAdmission,
) -> bool {
    match kind {
        ProjectContentReferenceKind::Asset => index.assets.contains_key(target_id),
        ProjectContentReferenceKind::EntityDefinition => index.entities.contains_key(target_id),
        ProjectContentReferenceKind::InstantiatedEntityDefinition => {
            index.entities.contains_key(target_id)
                && index.scene_instances.values().any(|reference| {
                    matches!(
                        reference,
                        SceneInstanceReference::EntityDefinition { stable_id, .. }
                            if stable_id == target_id
                    )
                })
        }
        ProjectContentReferenceKind::InstantiatedBoundedEntityDefinition => {
            index
                .entities
                .get(target_id)
                .is_some_and(|definition| has_usable_bounds(definition))
                && index.scene_instances.values().any(|reference| {
                    matches!(
                        reference,
                        SceneInstanceReference::EntityDefinition { stable_id, .. }
                            if stable_id == target_id
                    )
                })
        }
        ProjectContentReferenceKind::EntrySceneFpsPlayerEntityDefinition => {
            is_unique_entry_scene_fps_player_target(target_id, index, entry_scene_id, gameplay)
        }
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
            index.presentation_resources.contains_key(target_id)
        }
    }
}

fn is_unique_entry_scene_fps_player_target(
    target_id: &str,
    index: &ReferenceIndex<'_>,
    entry_scene_id: Option<SceneId>,
    gameplay: &dyn crate::ProjectContentGameplayAdmission,
) -> bool {
    let Some(entry_scene_id) = entry_scene_id else {
        return false;
    };
    let mut player_instances = index.scene_instances.values().filter_map(|reference| {
        let SceneInstanceReference::EntityDefinition {
            scene_id,
            stable_id,
            ..
        } = reference
        else {
            return None;
        };
        if *scene_id != entry_scene_id {
            return None;
        }
        let definition = index.entities.get(stable_id)?;
        gameplay
            .entity_definition_matches_reference(
                ProjectContentReferenceKind::EntrySceneFpsPlayerEntityDefinition,
                definition,
            )
            .then_some(stable_id.as_str())
    });
    player_instances.next() == Some(target_id)
        && player_instances.next().is_none()
        && index
            .entities
            .get(target_id)
            .is_some_and(|definition| has_usable_bounds(definition))
}

fn has_usable_bounds(definition: &EntityDefinition) -> bool {
    definition.capabilities.iter().any(|capability| {
        matches!(capability, EntityDefinitionCapability::Bounds { min, max }
            if min.iter().zip(max).all(|(min, max)|
                min.is_finite() && max.is_finite() && min < max))
    })
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
            let asset = index.assets.get(&resource.asset_id).copied();
            if resource.resource_id.trim().is_empty()
                || resource.source_path.trim().is_empty()
                || resource.content_hash.trim().is_empty()
                || asset.is_none()
            {
                push(
                    diagnostics,
                    ProjectContentDiagnosticCode::UnknownReference,
                    Some(document_id),
                    &format!("catalog.resources[{resource_index}]"),
                    "presentation resources require ids, source/content identity, and a catalog asset",
                );
            }
            if let Some(asset) = asset {
                let expected_prefix = presentation_resource_asset_prefix(resource.kind);
                if !asset.id.starts_with(&format!("{expected_prefix}/")) {
                    push(
                        diagnostics,
                        ProjectContentDiagnosticCode::InvalidField,
                        Some(document_id),
                        &format!("catalog.resources[{resource_index}].assetId"),
                        "presentation resource kind does not match its catalog asset kind",
                    );
                }
                if asset.source_path.as_deref() != Some(resource.source_path.as_str())
                    || asset.hash.as_deref() != Some(resource.content_hash.as_str())
                {
                    push(
                        diagnostics,
                        ProjectContentDiagnosticCode::InvalidField,
                        Some(document_id),
                        &format!("catalog.resources[{resource_index}]"),
                        "presentation resource source path and content hash must match its catalog asset",
                    );
                }
            }
            let animated_descriptor_matches = match resource.kind {
                ProjectPresentationResourceKind::AnimatedMesh => {
                    resource.animated_mesh.as_ref().is_some_and(|descriptor| {
                        descriptor.asset == resource.asset_id
                            && descriptor.content_hash.as_deref()
                                == Some(resource.content_hash.as_str())
                            && valid_project_animated_mesh_descriptor(descriptor)
                            && descriptor
                                .bounds
                                .min
                                .iter()
                                .chain(descriptor.bounds.max.iter())
                                .all(|value| value.is_finite())
                            && descriptor
                                .bounds
                                .min
                                .iter()
                                .zip(descriptor.bounds.max.iter())
                                .all(|(min, max)| min <= max)
                    })
                }
                _ => resource.animated_mesh.is_none(),
            };
            if !animated_descriptor_matches {
                push(
                    diagnostics,
                    ProjectContentDiagnosticCode::InvalidField,
                    Some(document_id),
                    &format!("catalog.resources[{resource_index}].animatedMesh"),
                    "animated-mesh resources require a valid matching renderer-neutral descriptor and other resource kinds forbid one",
                );
            }
        }
        let resources = catalog
            .resources
            .iter()
            .map(|resource| (resource.resource_id.as_str(), resource))
            .collect::<BTreeMap<_, _>>();
        let realizable_signals = catalog
            .cues
            .iter()
            .filter_map(|cue| match cue {
                ProjectPresentationCueDto::Audio { signal_id, .. } => {
                    Some((ProjectPresentationSignalDomain::Audio, signal_id.as_str()))
                }
                ProjectPresentationCueDto::Particle { signal_id, .. } => Some((
                    ProjectPresentationSignalDomain::Particle,
                    signal_id.as_str(),
                )),
                _ => None,
            })
            .collect::<BTreeSet<_>>();
        let mut cues = BTreeSet::new();
        let mut signal_bindings = BTreeSet::new();
        for (cue_index, cue) in catalog.cues.iter().enumerate() {
            let (cue_id, resource_id) = match cue {
                ProjectPresentationCueDto::Animation {
                    cue_id,
                    resource_id,
                    clip_id,
                    at_seconds,
                    signal,
                    ..
                } => {
                    if !resources.get(resource_id.as_str()).is_some_and(|resource| {
                        resource.kind == ProjectPresentationResourceKind::AnimatedMesh
                            && resource.animated_mesh.as_ref().is_some_and(|descriptor| {
                                descriptor.clips.iter().any(|clip| clip.id == *clip_id)
                            })
                    }) {
                        push(
                            diagnostics,
                            ProjectContentDiagnosticCode::UnknownReference,
                            Some(document_id),
                            &format!("catalog.cues[{cue_index}].clipId"),
                            "animation cue references an unknown resource clip",
                        );
                    }
                    if !at_seconds.is_finite() || *at_seconds < 0.0 {
                        push(
                            diagnostics,
                            ProjectContentDiagnosticCode::InvalidField,
                            Some(document_id),
                            &format!("catalog.cues[{cue_index}].atSeconds"),
                            "animation cue sample time must be finite and non-negative",
                        );
                    }
                    if signal.signal_id.trim().is_empty()
                        || !realizable_signals.contains(&(signal.domain, signal.signal_id.as_str()))
                    {
                        push(
                            diagnostics,
                            ProjectContentDiagnosticCode::UnknownReference,
                            Some(document_id),
                            &format!("catalog.cues[{cue_index}].signal"),
                            "animation cue signal must resolve to a typed audio or particle cue binding",
                        );
                    }
                    (cue_id, resource_id)
                }
                ProjectPresentationCueDto::Audio {
                    cue_id,
                    signal_id,
                    resource_id,
                    gain,
                } => {
                    if !gain.is_finite() || !(0.0..=1.0).contains(gain) {
                        push(
                            diagnostics,
                            ProjectContentDiagnosticCode::InvalidField,
                            Some(document_id),
                            &format!("catalog.cues[{cue_index}].gain"),
                            "audio cue gain must be finite and between zero and one",
                        );
                    }
                    validate_signal_binding(
                        document_id,
                        cue_index,
                        ProjectPresentationSignalDomain::Audio,
                        signal_id,
                        &mut signal_bindings,
                        diagnostics,
                    );
                    if !resources.get(resource_id.as_str()).is_some_and(|resource| {
                        resource.kind == ProjectPresentationResourceKind::Audio
                    }) {
                        push(
                            diagnostics,
                            ProjectContentDiagnosticCode::InvalidField,
                            Some(document_id),
                            &format!("catalog.cues[{cue_index}].resourceId"),
                            "audio cue requires an audio presentation resource",
                        );
                    }
                    (cue_id, resource_id)
                }
                ProjectPresentationCueDto::Particle {
                    cue_id,
                    signal_id,
                    resource_id,
                    scale,
                } => {
                    if !scale.is_finite() || *scale <= 0.0 || *scale > 16.0 {
                        push(
                            diagnostics,
                            ProjectContentDiagnosticCode::InvalidField,
                            Some(document_id),
                            &format!("catalog.cues[{cue_index}].scale"),
                            "particle cue scale must be finite, positive, and at most 16",
                        );
                    }
                    validate_signal_binding(
                        document_id,
                        cue_index,
                        ProjectPresentationSignalDomain::Particle,
                        signal_id,
                        &mut signal_bindings,
                        diagnostics,
                    );
                    if !resources.get(resource_id.as_str()).is_some_and(|resource| {
                        resource.kind == ProjectPresentationResourceKind::Particle
                    }) {
                        push(
                            diagnostics,
                            ProjectContentDiagnosticCode::InvalidField,
                            Some(document_id),
                            &format!("catalog.cues[{cue_index}].resourceId"),
                            "particle cue requires a particle presentation resource",
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

fn presentation_resource_asset_prefix(kind: ProjectPresentationResourceKind) -> &'static str {
    match kind {
        ProjectPresentationResourceKind::AnimatedMesh => "mesh",
        ProjectPresentationResourceKind::Audio => "audio",
        ProjectPresentationResourceKind::Particle | ProjectPresentationResourceKind::Overlay => {
            "sprite"
        }
        ProjectPresentationResourceKind::Font => "font",
    }
}

fn valid_project_animated_mesh_descriptor(descriptor: &ProjectAnimatedMeshDescriptorDto) -> bool {
    if descriptor.asset.trim().is_empty() {
        return false;
    }
    let mut clips = BTreeSet::new();
    if descriptor.clips.iter().any(|clip| {
        clip.id.trim().is_empty()
            || !clips.insert(clip.id.as_str())
            || clip
                .duration_seconds
                .is_some_and(|duration| !duration.is_finite() || duration <= 0.0)
    }) {
        return false;
    }
    if descriptor
        .default_clip
        .as_ref()
        .is_some_and(|clip| !clips.contains(clip.as_str()))
    {
        return false;
    }
    let mut slots = BTreeSet::new();
    !descriptor
        .material_slots
        .iter()
        .any(|slot| slot.material.trim().is_empty() || !slots.insert(slot.slot))
}

fn validate_signal_binding<'a>(
    document_id: &str,
    cue_index: usize,
    domain: ProjectPresentationSignalDomain,
    signal_id: &'a str,
    signal_bindings: &mut BTreeSet<(ProjectPresentationSignalDomain, &'a str)>,
    diagnostics: &mut Vec<ProjectContentDiagnosticDto>,
) {
    if signal_id.trim().is_empty() || !signal_bindings.insert((domain, signal_id)) {
        push(
            diagnostics,
            ProjectContentDiagnosticCode::InvalidDocument,
            Some(document_id),
            &format!("catalog.cues[{cue_index}].signalId"),
            "presentation signal ids must be non-empty and unique within each signal domain",
        );
    }
}

pub(super) fn field_metadata(
    documents: &[ProjectContentDocumentDto],
    scenes: &[FlatSceneDocumentDto],
    entry_scene_id: Option<SceneId>,
    gameplay: &dyn crate::ProjectContentGameplayAdmission,
) -> Vec<ProjectContentFieldMetadataDto> {
    let configuration_schemas = gameplay.configuration_schemas();
    let mut index = ReferenceIndex::default();
    let mut accepted_diagnostics = Vec::new();
    index_entities(documents, &mut index, &mut accepted_diagnostics);
    index_catalogs(documents, &mut index, &mut accepted_diagnostics);
    index_prefabs(documents, &mut index, &mut accepted_diagnostics);
    index_presentation(documents, &mut index, &mut accepted_diagnostics);
    index_scenes(scenes, &mut index, &mut accepted_diagnostics);
    debug_assert!(accepted_diagnostics.is_empty());
    let mut fields = Vec::new();
    for document in documents {
        let document_id = document.document_id().to_owned();
        match document {
            ProjectContentDocumentDto::EntityDefinition { definition, .. } => {
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
                fields.extend(entity_appearance_field_metadata(
                    &document_id,
                    definition,
                    documents,
                ));
            }
            ProjectContentDocumentDto::AssetCatalog { catalog, .. } => {
                for (index, entry) in catalog.entries.iter().enumerate() {
                    fields.push(metadata(
                        &document_id,
                        &format!("catalog.entries[{index}].id"),
                        "Asset id",
                        ProjectConfigurationValueKind::Reference,
                        true,
                        Some(ProjectContentReferenceKind::Asset),
                    ));
                    if entry.material.is_some() {
                        for (suffix, label) in [
                            ("color.r", "Base color red"),
                            ("color.g", "Base color green"),
                            ("color.b", "Base color blue"),
                            ("color.a", "Base color alpha"),
                            ("roughness", "Roughness"),
                            ("emissionColor.r", "Emission red"),
                            ("emissionColor.g", "Emission green"),
                            ("emissionColor.b", "Emission blue"),
                            ("emissionColor.a", "Emission alpha"),
                            ("emissive", "Emission intensity"),
                        ] {
                            fields.push(ProjectContentFieldMetadataDto {
                                document_id: document_id.clone(),
                                field_id: suffix.to_owned(),
                                path: format!("catalog.entries[{index}].material.style.{suffix}"),
                                label: format!(
                                    "{} · {label}",
                                    entry.label.as_deref().unwrap_or(&entry.id)
                                ),
                                value_kind: ProjectConfigurationValueKind::Number,
                                required: true,
                                editable: true,
                                reference_kind: None,
                                reference_options: Vec::new(),
                                configuration_id: Some(entry.id.clone()),
                                schema_id: Some("asha.material.v1".to_owned()),
                                module_id: None,
                                provider_id: Some("provider.asha.material-catalog".to_owned()),
                                contract: None,
                                codec_id: Some("svc-project-content.material.v1".to_owned()),
                                integer_min: None,
                                integer_max: None,
                                number_min: Some(0.0),
                                number_max: if suffix == "emissive" {
                                    Some(16.0)
                                } else {
                                    Some(1.0)
                                },
                            });
                        }
                    }
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
                            field_id: field.field_id.clone(),
                            path: format!(
                                "document.configurations[{configuration_index}].values.{}",
                                field.field_id
                            ),
                            label: field.label.clone(),
                            value_kind: field.value_kind,
                            required: field.required,
                            editable: true,
                            reference_kind: field.reference_kind,
                            reference_options: field.reference_kind.map_or_else(Vec::new, |kind| {
                                reference_options(
                                    kind,
                                    documents,
                                    scenes,
                                    &index,
                                    entry_scene_id,
                                    gameplay,
                                )
                            }),
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
                for (index, cue) in catalog.cues.iter().enumerate() {
                    let (cue_id, fields_for_cue): (&str, &[(&str, &str, f64, f64)]) = match cue {
                        ProjectPresentationCueDto::Animation { cue_id, .. } => (
                            cue_id,
                            &[("atSeconds", "Animation sample time", 0.0, 3_600.0)],
                        ),
                        ProjectPresentationCueDto::Audio { cue_id, .. } => {
                            (cue_id, &[("gain", "Audio gain", 0.0, 1.0)])
                        }
                        ProjectPresentationCueDto::Particle { cue_id, .. } => {
                            (cue_id, &[("scale", "Particle scale", 0.000_001, 16.0)])
                        }
                        ProjectPresentationCueDto::Overlay { cue_id, .. } => (cue_id, &[]),
                    };
                    for (field, label, minimum, maximum) in fields_for_cue {
                        fields.push(ProjectContentFieldMetadataDto {
                            document_id: document_id.clone(),
                            field_id: (*field).to_owned(),
                            path: format!("catalog.cues[{index}].{field}"),
                            label: format!("{cue_id} · {label}"),
                            value_kind: ProjectConfigurationValueKind::Number,
                            required: true,
                            editable: true,
                            reference_kind: None,
                            reference_options: Vec::new(),
                            configuration_id: Some(cue_id.to_owned()),
                            schema_id: Some("asha.presentation-cue.v1".to_owned()),
                            module_id: None,
                            provider_id: Some("provider.asha.presentation-catalog".to_owned()),
                            contract: None,
                            codec_id: Some("svc-project-content.presentation-cue.v1".to_owned()),
                            integer_min: None,
                            integer_max: None,
                            number_min: Some(*minimum),
                            number_max: Some(*maximum),
                        });
                    }
                }
            }
            ProjectContentDocumentDto::InputCatalog { catalog, .. } => {
                for (index, action) in catalog.actions.iter().enumerate() {
                    fields.push(metadata(
                        &document_id,
                        &format!("catalog.actions[{index}].actionId"),
                        "Input action id",
                        ProjectConfigurationValueKind::String,
                        true,
                        None,
                    ));
                    fields.push(metadata(
                        &document_id,
                        &format!("catalog.actions[{index}].acceptedPhases"),
                        &format!("{} accepted phases", action.action_id),
                        ProjectConfigurationValueKind::String,
                        true,
                        None,
                    ));
                }
                for (index, context) in catalog.contexts.iter().enumerate() {
                    fields.push(metadata(
                        &document_id,
                        &format!("catalog.contexts[{index}].contextId"),
                        &format!("{} context id", context.context_id),
                        ProjectConfigurationValueKind::String,
                        true,
                        None,
                    ));
                }
                for (index, binding) in catalog.bindings.iter().enumerate() {
                    for (suffix, label) in [
                        ("actionId", "Input action"),
                        ("contextId", "Input context"),
                        ("control", "Platform control"),
                    ] {
                        fields.push(metadata(
                            &document_id,
                            &format!("catalog.bindings[{index}].{suffix}"),
                            &format!("{} · {label}", binding.binding_id),
                            ProjectConfigurationValueKind::String,
                            true,
                            None,
                        ));
                    }
                }
            }
            ProjectContentDocumentDto::BehaviorPackage { .. } => {}
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
        field_id: path.rsplit('.').next().unwrap_or(path).to_owned(),
        path: path.to_owned(),
        label: label.to_owned(),
        value_kind,
        required,
        editable: true,
        reference_kind,
        reference_options: Vec::new(),
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

fn entity_appearance_field_metadata(
    document_id: &str,
    definition: &EntityDefinition,
    documents: &[ProjectContentDocumentDto],
) -> Vec<ProjectContentFieldMetadataDto> {
    let animated_meshes = documents
        .iter()
        .flat_map(|document| match document {
            ProjectContentDocumentDto::PresentationCatalog { catalog, .. } => catalog
                .resources
                .iter()
                .filter(|resource| {
                    resource.kind == ProjectPresentationResourceKind::AnimatedMesh
                        && resource.animated_mesh.is_some()
                })
                .collect::<Vec<_>>(),
            _ => Vec::new(),
        })
        .collect::<Vec<_>>();
    let resource_options = animated_meshes
        .iter()
        .map(|resource| ProjectContentReferenceOptionDto {
            target_id: resource.resource_id.clone(),
            label: format!("{} · {}", resource.resource_id, resource.asset_id),
        })
        .collect::<Vec<_>>();
    let mut fields = Vec::new();

    for (capability_index, capability) in definition.capabilities.iter().enumerate() {
        let EntityDefinitionCapability::RenderProjection {
            projection_id,
            appearance,
            ..
        } = capability
        else {
            continue;
        };
        let path = format!("definition.capabilities[{capability_index}].appearance");
        let common =
            |field_id: &str,
             path: String,
             label: &str,
             value_kind: ProjectConfigurationValueKind,
             required: bool,
             reference_kind: Option<ProjectContentReferenceKind>,
             reference_options: Vec<ProjectContentReferenceOptionDto>,
             number_min: Option<f64>,
             number_max: Option<f64>| ProjectContentFieldMetadataDto {
                document_id: document_id.to_owned(),
                field_id: field_id.to_owned(),
                path,
                label: label.to_owned(),
                value_kind,
                required,
                editable: true,
                reference_kind,
                reference_options,
                configuration_id: Some(projection_id.clone()),
                schema_id: Some("asha.entity-appearance.v1".to_owned()),
                module_id: None,
                provider_id: Some("provider.asha.entity-appearance".to_owned()),
                contract: None,
                codec_id: Some("svc-project-content.entity-appearance.v1".to_owned()),
                integer_min: None,
                integer_max: None,
                number_min,
                number_max,
            };
        fields.push(common(
            "resourceId",
            format!("{path}.resourceId"),
            "Appearance resource",
            ProjectConfigurationValueKind::Reference,
            true,
            Some(ProjectContentReferenceKind::PresentationResource),
            resource_options.clone(),
            None,
            None,
        ));
        let Some(appearance) = appearance else {
            continue;
        };
        let selected = animated_meshes
            .iter()
            .find(|resource| resource.resource_id == appearance.resource_id)
            .and_then(|resource| resource.animated_mesh.as_ref());
        let mut clip_options = vec![ProjectContentReferenceOptionDto {
            target_id: String::new(),
            label: format!(
                "Resource default ({})",
                selected
                    .and_then(|descriptor| descriptor.default_clip.as_deref())
                    .unwrap_or("none")
            ),
        }];
        if let Some(descriptor) = selected {
            clip_options.extend(descriptor.clips.iter().map(|clip| {
                ProjectContentReferenceOptionDto {
                    target_id: clip.id.clone(),
                    label: clip
                        .name
                        .as_ref()
                        .map_or_else(|| clip.id.clone(), |name| format!("{} · {name}", clip.id)),
                }
            }));
        }
        fields.push(common(
            "initialClipId",
            format!("{path}.initialClipId"),
            "Initial animation clip",
            ProjectConfigurationValueKind::String,
            false,
            None,
            clip_options,
            None,
            None,
        ));
        for (axis, label) in ["X", "Y", "Z"].into_iter().enumerate() {
            fields.push(common(
                &format!("modelScale{label}"),
                format!("{path}.modelScale[{axis}]"),
                &format!("Model scale {label}"),
                ProjectConfigurationValueKind::Number,
                true,
                None,
                Vec::new(),
                Some(0.0001),
                Some(1000.0),
            ));
        }
    }
    fields
}

fn reference_options(
    kind: ProjectContentReferenceKind,
    documents: &[ProjectContentDocumentDto],
    scenes: &[FlatSceneDocumentDto],
    index: &ReferenceIndex<'_>,
    entry_scene_id: Option<SceneId>,
    gameplay: &dyn crate::ProjectContentGameplayAdmission,
) -> Vec<ProjectContentReferenceOptionDto> {
    let mut options: Vec<ProjectContentReferenceOptionDto> = match kind {
        ProjectContentReferenceKind::Asset => documents
            .iter()
            .flat_map(|document| match document {
                ProjectContentDocumentDto::AssetCatalog { catalog, .. } => catalog
                    .entries
                    .iter()
                    .map(|entry| ProjectContentReferenceOptionDto {
                        target_id: entry.id.clone(),
                        label: entry.label.clone().unwrap_or_else(|| entry.id.clone()),
                    })
                    .collect(),
                _ => Vec::new(),
            })
            .collect(),
        ProjectContentReferenceKind::EntityDefinition
        | ProjectContentReferenceKind::InstantiatedEntityDefinition
        | ProjectContentReferenceKind::InstantiatedBoundedEntityDefinition
        | ProjectContentReferenceKind::EntrySceneFpsPlayerEntityDefinition => documents
            .iter()
            .filter_map(|document| match document {
                ProjectContentDocumentDto::EntityDefinition { definition, .. }
                    if reference_exists(
                        kind,
                        &definition.stable_id,
                        index,
                        entry_scene_id,
                        gameplay,
                    ) =>
                {
                    Some(ProjectContentReferenceOptionDto {
                        target_id: definition.stable_id.clone(),
                        label: definition.display_name.clone(),
                    })
                }
                _ => None,
            })
            .collect(),
        ProjectContentReferenceKind::SceneInstance => scenes
            .iter()
            .flat_map(|scene| {
                scene.nodes.iter().filter_map(|node| match &node.kind {
                    SceneNodeKindDto::EntityInstance { instance } => {
                        Some(ProjectContentReferenceOptionDto {
                            target_id: instance.instance_id.clone(),
                            label: node
                                .label
                                .clone()
                                .unwrap_or_else(|| instance.instance_id.clone()),
                        })
                    }
                    _ => None,
                })
            })
            .collect(),
        ProjectContentReferenceKind::Prefab => documents
            .iter()
            .flat_map(|document| match document {
                ProjectContentDocumentDto::PrefabRegistry { registry, .. } => registry
                    .definitions
                    .iter()
                    .map(|definition| ProjectContentReferenceOptionDto {
                        target_id: definition.id.raw().to_string(),
                        label: definition.display_name.clone(),
                    })
                    .collect(),
                _ => Vec::new(),
            })
            .collect(),
        ProjectContentReferenceKind::PrefabPart => documents
            .iter()
            .flat_map(|document| match document {
                ProjectContentDocumentDto::PrefabRegistry { registry, .. } => registry
                    .definitions
                    .iter()
                    .flat_map(|definition| {
                        definition
                            .part_roles
                            .iter()
                            .map(|role| ProjectContentReferenceOptionDto {
                                target_id: format!("{}:{}", definition.id.raw(), role.role),
                                label: format!("{} · {}", definition.display_name, role.role),
                            })
                    })
                    .collect(),
                _ => Vec::new(),
            })
            .collect(),
        ProjectContentReferenceKind::PresentationResource => documents
            .iter()
            .flat_map(|document| match document {
                ProjectContentDocumentDto::PresentationCatalog { catalog, .. } => catalog
                    .resources
                    .iter()
                    .map(|resource| ProjectContentReferenceOptionDto {
                        target_id: resource.resource_id.clone(),
                        label: resource.resource_id.clone(),
                    })
                    .collect(),
                _ => Vec::new(),
            })
            .collect(),
    };
    options.sort_by(|left, right| {
        (left.target_id.as_str(), left.label.as_str())
            .cmp(&(right.target_id.as_str(), right.label.as_str()))
    });
    options.dedup_by(|left, right| left.target_id == right.target_id);
    options
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
