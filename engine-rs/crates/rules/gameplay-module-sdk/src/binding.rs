use protocol_game_extension::{
    GameplayModuleBinding, GameplayModuleBindingOverride, GameplayModuleBindingRegistry,
    GameplayModuleConfiguration, GAMEPLAY_MODULE_BINDING_SCHEMA_VERSION,
};
use rule_gameplay_fabric::gameplay_module_payload_hash;
use svc_gameplay_fabric::GameplayFabricRegistry;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameplayRuntimeCompositionIdentity {
    pub semantic_compatibility_digest: String,
    pub artifact_provenance_digest: String,
}

/// Deterministic public builder for durable authored binding registries. The
/// owning RuntimeSession still performs provider/schema/target validation.
#[derive(Debug, Clone, Default)]
pub struct GameplayModuleBindingRegistryBuilder {
    configurations: Vec<GameplayModuleConfiguration>,
    bindings: Vec<GameplayModuleBinding>,
    overrides: Vec<GameplayModuleBindingOverride>,
}

impl GameplayModuleBindingRegistryBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn configuration(&mut self, value: GameplayModuleConfiguration) -> &mut Self {
        self.configurations.push(value);
        self
    }

    pub fn binding(&mut self, value: GameplayModuleBinding) -> &mut Self {
        self.bindings.push(value);
        self
    }

    pub fn instance_override(&mut self, value: GameplayModuleBindingOverride) -> &mut Self {
        self.overrides.push(value);
        self
    }

    pub fn build(mut self) -> GameplayModuleBindingRegistry {
        self.configurations
            .sort_by(|left, right| left.configuration_id.cmp(&right.configuration_id));
        self.bindings
            .sort_by(|left, right| left.binding_id.cmp(&right.binding_id));
        self.overrides.sort_by(|left, right| {
            (left.binding_id.as_str(), left.prefab_instance.raw())
                .cmp(&(right.binding_id.as_str(), right.prefab_instance.raw()))
        });
        let mut registry = GameplayModuleBindingRegistry {
            schema_version: GAMEPLAY_MODULE_BINDING_SCHEMA_VERSION,
            configurations: self.configurations,
            bindings: self.bindings,
            overrides: self.overrides,
            registry_hash: String::new(),
        };
        registry.registry_hash = gameplay_module_binding_registry_hash(&registry);
        registry
    }
}

pub fn gameplay_module_binding_registry_hash(registry: &GameplayModuleBindingRegistry) -> String {
    let bytes = serde_json::to_vec(&(
        registry.schema_version,
        &registry.configurations,
        &registry.bindings,
        &registry.overrides,
    ))
    .expect("binding registry values serialize");
    gameplay_module_payload_hash(&bytes)
}

/// Hashes the authored behavior selection while excluding only exact module
/// artifact bytes. Configuration values, public module contracts, binding
/// targets, overrides, and enabled state remain semantic compatibility inputs.
pub fn gameplay_module_binding_registry_semantic_hash(
    registry: &GameplayModuleBindingRegistry,
) -> String {
    let configurations = registry
        .configurations
        .iter()
        .map(|configuration| {
            (
                &configuration.configuration_id,
                (
                    &configuration.module.module_id,
                    &configuration.module.namespace,
                    &configuration.module.version,
                    &configuration.module.sdk_hash,
                    &configuration.module.contract_hash,
                    &configuration.module.provider_id,
                ),
                &configuration.configuration,
                &configuration.codec_id,
                &configuration.config_hash,
                &configuration.canonical_config,
            )
        })
        .collect::<Vec<_>>();
    let bytes = serde_json::to_vec(&(
        registry.schema_version,
        configurations,
        &registry.bindings,
        &registry.overrides,
    ))
    .expect("binding registry semantic values serialize");
    gameplay_module_payload_hash(&bytes)
}

/// One RuntimeSession identity binds the closed module topology and the
/// authored bindings. Exact provenance additionally binds concrete module
/// artifacts while the semantic identity deliberately does not.
pub fn gameplay_runtime_composition_identity(
    registry: &GameplayFabricRegistry,
    bindings: &GameplayModuleBindingRegistry,
) -> GameplayRuntimeCompositionIdentity {
    let semantic_binding_hash = gameplay_module_binding_registry_semantic_hash(bindings);
    let semantic_compatibility_digest = gameplay_module_payload_hash(
        format!(
            "{}|{}",
            registry.semantic_compatibility_digest(),
            semantic_binding_hash
        )
        .as_bytes(),
    );
    let artifact_provenance_digest = gameplay_module_payload_hash(
        format!(
            "{}|{}",
            registry.artifact_provenance_digest(),
            bindings.registry_hash
        )
        .as_bytes(),
    );
    GameplayRuntimeCompositionIdentity {
        semantic_compatibility_digest,
        artifact_provenance_digest,
    }
}
