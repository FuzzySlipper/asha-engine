use super::*;

pub(super) struct CanonicalFpsProjectSeed {
    pub(super) input: FpsProjectBundleLoadInput,
    pub(super) sources: BTreeMap<EntityId, CanonicalFpsDefinitionSource>,
}

pub(super) struct CanonicalFpsDefinitionSource {
    pub(super) document_id: String,
    pub(super) source_path: String,
}

pub(super) fn runtime_project_seed_domain_error(
    code: &str,
    seed: &gameplay_runtime_host::RuntimeProjectEntitySeed,
    field_path: &str,
    message: String,
) -> RuntimeProjectLoadError {
    RuntimeProjectLoadError::Domain {
        code: code.to_owned(),
        document_id: Some(seed.document_id.clone()),
        path: Some(format!(
            "{}.document.definition.{field_path}",
            seed.source_path
        )),
        message,
    }
}

pub(super) fn runtime_project_fps_activation_error(
    seed: &CanonicalFpsProjectSeed,
    error: FpsRuntimeError,
) -> RuntimeProjectLoadError {
    let definitions = &seed.input.definitions;
    let by_entity = |entity: EntityId| {
        definitions
            .iter()
            .find(|definition| definition.entity == entity)
    };
    let neutral_definition = || {
        definitions
            .iter()
            .find(|definition| definition.role == FpsRuntimeRole::Neutral)
            .or_else(|| definitions.first())
    };
    let debug_message = format!("FPS domain activation rejected stored content: {error:?}");
    let (code, definition, field_path, message) = match &error {
        FpsRuntimeError::MissingPlayer => {
            let definition = definitions
                .iter()
                .find(|definition| {
                    definition.role == FpsRuntimeRole::Neutral && definition.weapon.is_some()
                })
                .or_else(neutral_definition);
            let stable_id = definition
                .map(|definition| definition.definition.stable_id.as_str())
                .unwrap_or("<missing>");
            (
                "missingPlayerRole",
                definition,
                "capabilities".to_owned(),
                format!(
                    "FPS player definition `{stable_id}` must declare controller `player_input` or faction `player`"
                ),
            )
        }
        FpsRuntimeError::MissingEnemy => {
            let definition = definitions
                .iter()
                .find(|definition| {
                    definition.role == FpsRuntimeRole::Neutral
                        && definition.policy_binding.is_some()
                })
                .or_else(neutral_definition);
            let stable_id = definition
                .map(|definition| definition.definition.stable_id.as_str())
                .unwrap_or("<missing>");
            (
                "missingEnemyRole",
                definition,
                "capabilities".to_owned(),
                format!(
                    "FPS enemy definition `{stable_id}` must declare controller `enemy_policy` or faction `hostile`"
                ),
            )
        }
        FpsRuntimeError::MissingPlayerWeapon { entity } => (
            "missingPlayerWeapon",
            by_entity(*entity),
            "capabilities".to_owned(),
            by_entity(*entity)
                .map(|definition| {
                    format!(
                        "FPS player definition `{}` must declare a weaponMount capability",
                        definition.definition.stable_id
                    )
                })
                .unwrap_or_else(|| debug_message.clone()),
        ),
        FpsRuntimeError::MissingEnemyHealth { entity } => (
            "missingEnemyHealth",
            by_entity(*entity),
            "capabilities".to_owned(),
            debug_message.clone(),
        ),
        FpsRuntimeError::MissingEnemyBounds { entity } => (
            "missingEnemyBounds",
            by_entity(*entity),
            "capabilities".to_owned(),
            debug_message.clone(),
        ),
        FpsRuntimeError::InvalidHealth { entity } => (
            "invalidHealth",
            by_entity(*entity),
            "capabilities".to_owned(),
            debug_message.clone(),
        ),
        FpsRuntimeError::InvalidPolicyBinding { entity, field } => (
            "invalidPolicyBinding",
            by_entity(*entity),
            format!("capabilities.{field}"),
            debug_message.clone(),
        ),
        FpsRuntimeError::DuplicateEntity { entity } => (
            "duplicateFpsEntity",
            by_entity(*entity),
            "stableId".to_owned(),
            debug_message.clone(),
        ),
        FpsRuntimeError::DuplicateStableId { stable_id } => (
            "duplicateFpsStableId",
            definitions
                .iter()
                .find(|definition| definition.definition.stable_id == *stable_id),
            "stableId".to_owned(),
            debug_message.clone(),
        ),
        FpsRuntimeError::DuplicateRole {
            role,
            first_entity,
            duplicate_entity,
        } => (
            "duplicateFpsRole",
            by_entity(*duplicate_entity),
            "capabilities".to_owned(),
            format!(
                "FPS role {role:?} must resolve to exactly one entity; entities {} and {} both claim it",
                first_entity.raw(),
                duplicate_entity.raw()
            ),
        ),
        FpsRuntimeError::MissingProjectBundle => (
            "missingProjectBundle",
            definitions.first(),
            "source.projectBundle".to_owned(),
            debug_message.clone(),
        ),
        FpsRuntimeError::EmptyDefinitions => (
            "missingEntityDefinitions",
            None,
            "capabilities".to_owned(),
            debug_message.clone(),
        ),
        _ => (
            "fpsDomainRejected",
            definitions.first(),
            "capabilities".to_owned(),
            debug_message,
        ),
    };
    let source = definition.and_then(|definition| seed.sources.get(&definition.entity));
    RuntimeProjectLoadError::Domain {
        code: code.to_owned(),
        document_id: source.map(|source| source.document_id.clone()),
        path: source
            .map(|source| format!("{}.document.definition.{field_path}", source.source_path)),
        message,
    }
}
