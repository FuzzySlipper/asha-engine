//! ProjectBundle-authored gameplay-module binding and Session activation.
//!
//! Stored configuration selects statically linked Rust providers. Validation
//! resolves the complete binding graph before any module state exists; the
//! existing gameplay state store then initializes every facet atomically.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use core_entity::{EntityLifecycle, EntityStore};
use core_ids::{EntityId, PrefabInstanceId};
pub use gameplay_module_sdk::{
    gameplay_module_binding_registry_hash, gameplay_runtime_composition_identity,
    GameplayModuleBindingRegistryBuilder,
};
use gameplay_module_sdk::{
    GameplayConfigurationCodecRegistration, GameplayStaticComposition,
    GameplayStaticConfigurationBinding, GameplayStaticInvocationHost,
};
use protocol_diagnostics::DiagnosticSeverity;
use protocol_game_extension::{
    GameplayCompositionDiagnostic, GameplayCompositionDiagnosticCode, GameplayCompositionLoadMode,
    GameplayEventEnvelope, GameplayModuleBinding, GameplayModuleBindingActivationReceipt,
    GameplayModuleBindingDiagnostic, GameplayModuleBindingDiagnosticCode,
    GameplayModuleBindingOverride, GameplayModuleBindingReadout, GameplayModuleBindingRegistry,
    GameplayModuleBindingTarget, GameplayModuleConfiguration,
    GAMEPLAY_MODULE_BINDING_SCHEMA_VERSION,
};
use rule_gameplay_fabric::{
    adapt_trigger_overlap_fact, gameplay_module_payload_hash, FrozenGameplayViews,
    GameplayFabricCoordinator, GameplayModuleInitialization, GameplayModuleStateCheckpoint,
    GameplayModuleStateError, GameplayModuleStateScope, GameplayModuleStateStore,
    GameplayObserveReceipt, GameplayOwnerEventContext, GameplayOwnerRoutingCall,
    GameplayOwnerRoutingOutput, GameplayProposalRouter, GameplayRuntimeLimits, GameplayViewSource,
};
use rule_trigger_volume::{
    KinematicTriggerDefinition, TriggerReconcileCause, TriggerReconcileReceipt,
    TriggerVolumeDiagnostic, TriggerVolumeRule, TriggerVolumeSnapshot, TRIGGER_VOLUME_OWNER_ID,
};
use svc_gameplay_fabric::GameplayFabricRegistry;
use svc_serialization::{ArtifactEntry, ArtifactRole};

use crate::{
    compose_session_state_snapshot_with_prefabs, ProjectBundleLoadResult, SessionStateArtifact,
};

pub const GAMEPLAY_MODULE_SESSION_SNAPSHOT_PATH: &str = "session/gameplay-modules.snapshot.json";
const GAMEPLAY_MODULE_SESSION_SNAPSHOT_VERSION: u32 = 2;

/// Explicit bridge from stored EntityDefinition identity to entities already
/// created by ProjectBundle bootstrap. No display-name or path inference.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GameplayBindingEntityTargets {
    definitions: BTreeMap<String, BTreeSet<EntityId>>,
}

impl GameplayBindingEntityTargets {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn bind(&mut self, stable_id: impl Into<String>, entity: EntityId) -> &mut Self {
        self.definitions
            .entry(stable_id.into())
            .or_default()
            .insert(entity);
        self
    }

    fn entities(&self, stable_id: &str) -> impl Iterator<Item = EntityId> + '_ {
        self.definitions
            .get(stable_id)
            .into_iter()
            .flat_map(|entities| entities.iter().copied())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameplayBindingActivationError {
    Invalid {
        diagnostics: Vec<GameplayModuleBindingDiagnostic>,
    },
    State(GameplayModuleStateError),
    Snapshot(String),
    Trigger(Vec<TriggerVolumeDiagnostic>),
    EntityAuthorityUnavailable,
}

impl core::fmt::Display for GameplayBindingActivationError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for GameplayBindingActivationError {}

/// Owning Session aggregate: immutable registry/host topology, ProjectBundle
/// authority, resolved binding provenance, and live Rust-owned module state.
pub struct GameplayBoundProjectBundleSession {
    pub bundle: ProjectBundleLoadResult,
    pub activation: GameplayModuleBindingActivationReceipt,
    pub module_state: GameplayModuleStateStore,
    registry: Arc<GameplayFabricRegistry>,
    host: GameplayStaticInvocationHost,
    bindings: GameplayModuleBindingRegistry,
    triggers: TriggerVolumeRule,
}

/// Explicitly borrowed runtime cells for one gameplay-fabric transaction.
/// The owning Session keeps topology private while allowing the public host to
/// freeze reads and advance module state at deterministic wave barriers.
pub struct GameplaySessionRuntimeCells<'session> {
    pub registry: &'session GameplayFabricRegistry,
    pub invocation_host: &'session GameplayStaticInvocationHost,
    pub module_state: &'session mut GameplayModuleStateStore,
    pub triggers: &'session TriggerVolumeRule,
    pub prefab_instances: &'session crate::PrefabInstanceAuthority,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameplayTriggerSessionReceipt {
    pub collision: TriggerReconcileReceipt,
    pub gameplay_events: Vec<GameplayEventEnvelope>,
    pub reactions: Vec<GameplayObserveReceipt>,
}

impl GameplayBoundProjectBundleSession {
    pub fn runtime_cells(&mut self) -> GameplaySessionRuntimeCells<'_> {
        GameplaySessionRuntimeCells {
            registry: self.registry.as_ref(),
            invocation_host: &self.host,
            module_state: &mut self.module_state,
            triggers: &self.triggers,
            prefab_instances: &self.bundle.prefab_instances,
        }
    }

    pub fn activate(
        bundle: ProjectBundleLoadResult,
        composition: GameplayStaticComposition,
        bindings: GameplayModuleBindingRegistry,
        entity_targets: &GameplayBindingEntityTargets,
    ) -> Result<Self, GameplayBindingActivationError> {
        Self::activate_with_mode(
            bundle,
            composition,
            bindings,
            entity_targets,
            GameplayCompositionLoadMode::Compatible,
        )
    }

    pub fn activate_with_mode(
        bundle: ProjectBundleLoadResult,
        composition: GameplayStaticComposition,
        bindings: GameplayModuleBindingRegistry,
        entity_targets: &GameplayBindingEntityTargets,
        load_mode: GameplayCompositionLoadMode,
    ) -> Result<Self, GameplayBindingActivationError> {
        let parts = composition.into_parts();
        let resolved = resolve_bindings(
            &bundle,
            &bindings,
            entity_targets,
            parts.registry.as_ref(),
            &parts.configuration_codecs,
            load_mode,
        )?;
        let ResolvedBindings {
            initializations,
            readouts,
            configuration_bindings,
            compatibility_diagnostics,
        } = resolved;
        let mut host = parts.host;
        host.install_configuration_bindings(configuration_bindings);
        let mut module_state =
            GameplayModuleStateStore::new(parts.registry.clone(), parts.state_adapters)
                .map_err(GameplayBindingActivationError::State)?;
        module_state
            .initialize_atomic(initializations)
            .map_err(GameplayBindingActivationError::State)?;
        let activation = activation_receipt(
            &bindings,
            parts.registry.as_ref(),
            compatibility_diagnostics,
            readouts,
            module_state.state_hash(),
        );
        Ok(Self {
            bundle,
            activation,
            module_state,
            registry: parts.registry,
            host,
            bindings,
            triggers: TriggerVolumeRule::default(),
        })
    }

    pub fn registry(&self) -> &GameplayFabricRegistry {
        self.registry.as_ref()
    }

    pub fn invocation_host(&self) -> &GameplayStaticInvocationHost {
        &self.host
    }

    pub fn bindings(&self) -> &GameplayModuleBindingRegistry {
        &self.bindings
    }

    pub fn trigger_rule(&self) -> &TriggerVolumeRule {
        &self.triggers
    }

    /// Restore the mutable gameplay cells captured when the enclosing static
    /// RuntimeSession was activated. Immutable registry, invocation, binding,
    /// and configuration topology remains installed.
    #[doc(hidden)]
    pub fn restore_runtime_state(
        &mut self,
        module_state: GameplayModuleStateCheckpoint,
        trigger_snapshot: TriggerVolumeSnapshot,
    ) -> Result<(), GameplayBindingActivationError> {
        let triggers = TriggerVolumeRule::from_snapshot(trigger_snapshot)
            .map_err(|error| GameplayBindingActivationError::Trigger(error.diagnostics))?;
        self.module_state.restore_checkpoint(module_state);
        self.triggers = triggers;
        Ok(())
    }

    /// Install a complete authored trigger-definition set atomically. Geometry
    /// remains in the Session EntityStore; this assigns only semantic roles.
    pub fn install_trigger_definitions(
        &mut self,
        definitions: impl IntoIterator<Item = KinematicTriggerDefinition>,
    ) -> Result<(), GameplayBindingActivationError> {
        let triggers = TriggerVolumeRule::new(definitions)
            .map_err(|error| GameplayBindingActivationError::Trigger(error.diagnostics))?;
        self.triggers = triggers;
        Ok(())
    }

    /// Reconcile kinematic overlap authority, adapt accepted facts to standard
    /// trigger events, and deliver those events to the closed Session topology.
    pub fn reconcile_triggers(
        &mut self,
        tick: u64,
        cause: TriggerReconcileCause,
    ) -> Result<GameplayTriggerSessionReceipt, GameplayBindingActivationError> {
        let (collision, gameplay_events) = self.reconcile_trigger_events(tick, cause)?;
        let reactions = gameplay_events
            .iter()
            .cloned()
            .map(|event| self.observe_session_event(event))
            .collect();
        Ok(GameplayTriggerSessionReceipt {
            collision,
            gameplay_events,
            reactions,
        })
    }

    /// Reconcile trigger authority and adapt its accepted facts without choosing
    /// a gameplay invocation view source or proposal router. The owning public
    /// RuntimeSession host uses this split to supply its real read/router cells.
    pub fn reconcile_trigger_events(
        &mut self,
        tick: u64,
        cause: TriggerReconcileCause,
    ) -> Result<(TriggerReconcileReceipt, Vec<GameplayEventEnvelope>), GameplayBindingActivationError>
    {
        let entities = self
            .bundle
            .runtime_entities
            .as_ref()
            .ok_or(GameplayBindingActivationError::EntityAuthorityUnavailable)?;
        let collision = self.triggers.reconcile(entities, tick, cause);
        let mut gameplay_events = Vec::with_capacity(collision.facts.len());
        for (index, fact) in collision.facts.iter().enumerate() {
            let event = adapt_trigger_overlap_fact(
                &GameplayOwnerEventContext {
                    owner_id: TRIGGER_VOLUME_OWNER_ID.to_owned(),
                    tick,
                    root_id: format!("trigger:{}:{}:{}", tick, fact.trigger, fact.subject),
                    root_sequence: tick,
                    first_event_sequence: u32::try_from(index).map_err(|_| {
                        GameplayBindingActivationError::Snapshot(
                            "trigger event sequence overflow".to_owned(),
                        )
                    })?,
                    parent_event_id: Some(format!("trigger-fact:{}", fact.pair_hash)),
                },
                fact,
            )
            .map_err(|error| GameplayBindingActivationError::Snapshot(error.to_string()))?;
            gameplay_events.push(event);
        }
        Ok((collision, gameplay_events))
    }

    /// Executes the statically linked module set selected by the validated
    /// bindings. Shared proposals still require a RuntimeSession owner router;
    /// this narrow ProjectBundle helper fails them closed.
    pub fn observe_session_event(&self, event: GameplayEventEnvelope) -> GameplayObserveReceipt {
        GameplayFabricCoordinator::new(
            self.registry.as_ref(),
            limits_from_registry(self.registry.as_ref()),
        )
        .observe(
            event,
            &BoundSessionViews {
                registry_digest: self.registry.registry_digest(),
            },
            &self.host,
            &mut RejectSharedProposals,
        )
    }

    /// Persists module state separately while binding it to the owning Session
    /// artifact, including prefab role/override metadata.
    pub fn compose_gameplay_session_snapshot(
        &self,
    ) -> Result<SessionStateArtifact, GameplayBindingActivationError> {
        let authority = authority_artifact(&self.bundle);
        let authority_hash = authority_state_hash(&self.bundle);
        let module_session = self
            .module_state
            .encode_session_snapshot(authority.text.as_bytes(), &authority_hash)
            .map_err(GameplayBindingActivationError::State)?;
        let trigger_snapshot = self.triggers.snapshot();
        let snapshot_hash = gameplay_session_snapshot_hash(
            &self.bindings.registry_hash,
            &self.activation,
            &module_session,
            &trigger_snapshot,
        );
        let stored = StoredGameplayProjectBundleSession {
            schema_version: GAMEPLAY_MODULE_SESSION_SNAPSHOT_VERSION,
            binding_registry_hash: self.bindings.registry_hash.clone(),
            activation: self.activation.clone(),
            module_session,
            trigger_snapshot,
            snapshot_hash,
        };
        let text = serde_json::to_string(&stored)
            .map_err(|error| GameplayBindingActivationError::Snapshot(error.to_string()))?;
        let entry = ArtifactEntry::durable(
            GAMEPLAY_MODULE_SESSION_SNAPSHOT_PATH,
            ArtifactRole::Other("gameplayModuleSessionSnapshot".to_owned()),
            text.as_bytes(),
        );
        Ok(SessionStateArtifact { entry, text })
    }

    pub fn restore(
        bundle: ProjectBundleLoadResult,
        composition: GameplayStaticComposition,
        bindings: GameplayModuleBindingRegistry,
        entity_targets: &GameplayBindingEntityTargets,
        snapshot_text: &str,
    ) -> Result<Self, GameplayBindingActivationError> {
        Self::restore_with_mode(
            bundle,
            composition,
            bindings,
            entity_targets,
            snapshot_text,
            GameplayCompositionLoadMode::Compatible,
        )
    }

    pub fn restore_with_mode(
        mut bundle: ProjectBundleLoadResult,
        composition: GameplayStaticComposition,
        bindings: GameplayModuleBindingRegistry,
        entity_targets: &GameplayBindingEntityTargets,
        snapshot_text: &str,
        load_mode: GameplayCompositionLoadMode,
    ) -> Result<Self, GameplayBindingActivationError> {
        let stored: StoredGameplayProjectBundleSession = serde_json::from_str(snapshot_text)
            .map_err(|error| GameplayBindingActivationError::Snapshot(error.to_string()))?;
        if stored.schema_version != GAMEPLAY_MODULE_SESSION_SNAPSHOT_VERSION
            || stored.binding_registry_hash != bindings.registry_hash
            || stored.snapshot_hash
                != gameplay_session_snapshot_hash(
                    &stored.binding_registry_hash,
                    &stored.activation,
                    &stored.module_session,
                    &stored.trigger_snapshot,
                )
        {
            return Err(GameplayBindingActivationError::Snapshot(
                "gameplay Session snapshot version, binding registry, or hash mismatch".into(),
            ));
        }
        let parts = composition.into_parts();
        let restored = GameplayModuleStateStore::decode_session_snapshot(
            parts.registry.clone(),
            parts.state_adapters,
            &stored.module_session,
        )
        .map_err(GameplayBindingActivationError::State)?;
        let authority_text = core::str::from_utf8(&restored.authority_snapshot)
            .map_err(|error| GameplayBindingActivationError::Snapshot(error.to_string()))?;
        let entity_snapshot = core_entity::decode_snapshot(authority_text)
            .map_err(|error| GameplayBindingActivationError::Snapshot(error.to_string()))?;
        let restored_entities = EntityStore::from_snapshot(entity_snapshot);
        bundle.prefab_instances =
            match crate::prefab_snapshot::decode_embedded_prefab_snapshot(authority_text)
                .map_err(|error| GameplayBindingActivationError::Snapshot(error.to_string()))?
            {
                Some(prefabs) => {
                    crate::PrefabInstanceAuthority::restore_persisted(&prefabs, &restored_entities)
                        .map_err(|error| {
                            GameplayBindingActivationError::Snapshot(error.to_string())
                        })?
                }
                None => crate::PrefabInstanceAuthority::default(),
            };
        bundle.runtime_entities = Some(restored_entities);
        let resolved = resolve_bindings(
            &bundle,
            &bindings,
            entity_targets,
            parts.registry.as_ref(),
            &parts.configuration_codecs,
            load_mode,
        )?;
        let composition_identity =
            gameplay_runtime_composition_identity(parts.registry.as_ref(), &bindings);
        if stored.activation.binding_registry_hash != bindings.registry_hash
            || stored.activation.gameplay_registry_digest != parts.registry.registry_digest()
            || stored.activation.semantic_compatibility_digest
                != composition_identity.semantic_compatibility_digest
            || stored.activation.artifact_provenance_digest
                != composition_identity.artifact_provenance_digest
            || stored.activation.readouts != resolved.readouts
        {
            return Err(GameplayBindingActivationError::Snapshot(
                "stored activation evidence does not match resolved bindings".into(),
            ));
        }
        let authority = authority_artifact(&bundle);
        if restored.authority_snapshot != authority.text.as_bytes()
            || restored.authority_state_hash != authority_state_hash(&bundle)
        {
            return Err(GameplayBindingActivationError::Snapshot(
                "module state was saved against different ProjectBundle authority".into(),
            ));
        }
        let triggers = TriggerVolumeRule::from_snapshot(stored.trigger_snapshot)
            .map_err(|error| GameplayBindingActivationError::Trigger(error.diagnostics))?;
        let mut host = parts.host;
        host.install_configuration_bindings(resolved.configuration_bindings);
        Ok(Self {
            bundle,
            activation: stored.activation,
            module_state: restored.module_state,
            registry: parts.registry,
            host,
            bindings,
            triggers,
        })
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StoredGameplayProjectBundleSession {
    schema_version: u32,
    binding_registry_hash: String,
    activation: GameplayModuleBindingActivationReceipt,
    module_session: Vec<u8>,
    trigger_snapshot: TriggerVolumeSnapshot,
    snapshot_hash: String,
}

struct ResolvedBindings {
    initializations: Vec<GameplayModuleInitialization>,
    readouts: Vec<GameplayModuleBindingReadout>,
    configuration_bindings: Vec<GameplayStaticConfigurationBinding>,
    compatibility_diagnostics: Vec<GameplayCompositionDiagnostic>,
}

#[derive(Clone)]
struct EffectiveTarget {
    scope: GameplayModuleStateScope,
    instance: Option<PrefabInstanceId>,
    eligible: bool,
    match_entities: BTreeSet<u64>,
}

fn resolve_bindings(
    bundle: &ProjectBundleLoadResult,
    bindings: &GameplayModuleBindingRegistry,
    entity_targets: &GameplayBindingEntityTargets,
    registry: &GameplayFabricRegistry,
    codecs: &[GameplayConfigurationCodecRegistration],
    load_mode: GameplayCompositionLoadMode,
) -> Result<ResolvedBindings, GameplayBindingActivationError> {
    let mut diagnostics = Vec::new();
    let mut compatibility_diagnostics = Vec::new();
    if bindings.schema_version != GAMEPLAY_MODULE_BINDING_SCHEMA_VERSION
        || bindings.registry_hash != gameplay_module_binding_registry_hash(bindings)
    {
        diagnostics.push(diag(
            GameplayModuleBindingDiagnosticCode::InvalidRegistryHash,
            "registryHash",
            "binding registry schema version or content hash is invalid",
        ));
    }
    let mut configurations = BTreeMap::new();
    for (index, configuration) in bindings.configurations.iter().enumerate() {
        if configuration.configuration_id.trim().is_empty()
            || configurations
                .insert(configuration.configuration_id.clone(), configuration)
                .is_some()
        {
            diagnostics.push(diag(
                GameplayModuleBindingDiagnosticCode::DuplicateConfiguration,
                format!("configurations[{index}].configurationId"),
                "configuration ids must be non-empty and unique",
            ));
            continue;
        }
        validate_configuration(
            configuration,
            registry,
            codecs,
            index,
            load_mode,
            &mut diagnostics,
            &mut compatibility_diagnostics,
        );
    }
    let mut indexed_bindings = BTreeMap::new();
    for (index, binding) in bindings.bindings.iter().enumerate() {
        if binding.binding_id.trim().is_empty()
            || indexed_bindings
                .insert(binding.binding_id.clone(), binding)
                .is_some()
        {
            diagnostics.push(diag(
                GameplayModuleBindingDiagnosticCode::DuplicateBinding,
                format!("bindings[{index}].bindingId"),
                "binding ids must be non-empty and unique",
            ));
            continue;
        }
        validate_binding(
            binding,
            configurations.get(&binding.configuration_id).copied(),
            registry,
            index,
            &mut diagnostics,
        );
    }
    let overrides = validate_overrides(
        bindings,
        &indexed_bindings,
        &configurations,
        bundle,
        &mut diagnostics,
    );
    if !diagnostics.is_empty() {
        return Err(GameplayBindingActivationError::Invalid { diagnostics });
    }

    let entities = bundle.runtime_entities.as_ref();
    let mut initializations = Vec::new();
    let mut readouts = Vec::new();
    let mut configuration_bindings = Vec::new();
    let mut occupied = BTreeSet::new();
    for binding in &bindings.bindings {
        let base_configuration = configurations[&binding.configuration_id];
        let targets = resolve_target(bundle, entity_targets, entities, &binding.target);
        if targets.is_empty() && binding.enabled {
            diagnostics.push(diag(
                GameplayModuleBindingDiagnosticCode::UnresolvedTarget,
                format!("bindings[{}].target", binding.binding_id),
                "binding target did not resolve to a Session state facet",
            ));
            continue;
        }
        if targets.is_empty() {
            readouts.push(readout(
                binding,
                base_configuration,
                Vec::new(),
                false,
                &bindings.registry_hash,
            ));
        }
        for target in targets {
            let override_layer = target
                .instance
                .and_then(|instance| overrides.get(&(binding.binding_id.clone(), instance)));
            let configuration = override_layer
                .and_then(|layer| layer.configuration_id.as_ref())
                .map(|id| configurations[id])
                .unwrap_or(base_configuration);
            let enabled = override_layer
                .and_then(|layer| layer.enabled)
                .unwrap_or(binding.enabled);
            let active = enabled && target.eligible;
            let label = scope_label(&target.scope);
            readouts.push(readout(
                binding,
                configuration,
                vec![label.clone()],
                active,
                &bindings.registry_hash,
            ));
            if enabled && !target.eligible {
                diagnostics.push(diag(
                    GameplayModuleBindingDiagnosticCode::IneligibleTarget,
                    format!("bindings[{}].target", binding.binding_id),
                    "resolved target is not active and cannot initialize module state",
                ));
                continue;
            }
            if !active {
                continue;
            }
            configuration_bindings.push(GameplayStaticConfigurationBinding {
                module_id: binding.module_id.clone(),
                binding_id: binding.binding_id.clone(),
                configuration_id: configuration.configuration_id.clone(),
                scope: target.scope.clone(),
                match_entities: target.match_entities.clone(),
                canonical_config: configuration.canonical_config.clone(),
                config_hash: configuration.config_hash.clone(),
            });
            if !occupied.insert((binding.state_schema.key(), target.scope.clone())) {
                diagnostics.push(diag(
                    GameplayModuleBindingDiagnosticCode::DuplicateStateScope,
                    format!("bindings[{}]", binding.binding_id),
                    "multiple bindings resolve to the same state schema and scope",
                ));
                continue;
            }
            initializations.push(GameplayModuleInitialization {
                initialization_id: format!(
                    "binding:{}:{}:{label}",
                    binding.binding_id, configuration.configuration_id
                ),
                module_id: binding.module_id.clone(),
                state_schema: binding.state_schema.clone(),
                scope: target.scope,
                canonical_config: configuration.canonical_config.clone(),
                config_hash: configuration.config_hash.clone(),
            });
        }
    }
    if !diagnostics.is_empty() {
        return Err(GameplayBindingActivationError::Invalid { diagnostics });
    }
    readouts.sort_by(|left, right| {
        (
            left.binding_id.as_str(),
            left.resolved_scopes.as_slice(),
            left.configuration_id.as_str(),
        )
            .cmp(&(
                right.binding_id.as_str(),
                right.resolved_scopes.as_slice(),
                right.configuration_id.as_str(),
            ))
    });
    Ok(ResolvedBindings {
        initializations,
        readouts,
        configuration_bindings,
        compatibility_diagnostics,
    })
}

fn validate_configuration(
    configuration: &GameplayModuleConfiguration,
    registry: &GameplayFabricRegistry,
    codecs: &[GameplayConfigurationCodecRegistration],
    index: usize,
    load_mode: GameplayCompositionLoadMode,
    diagnostics: &mut Vec<GameplayModuleBindingDiagnostic>,
    compatibility_diagnostics: &mut Vec<GameplayCompositionDiagnostic>,
) {
    let path = format!("configurations[{index}]");
    if gameplay_module_payload_hash(&configuration.canonical_config) != configuration.config_hash {
        diagnostics.push(diag(
            GameplayModuleBindingDiagnosticCode::InvalidRegistryHash,
            format!("{path}.configHash"),
            "configuration payload hash does not match canonical bytes",
        ));
    }
    let Some(module) = registry.module(&configuration.module.module_id) else {
        diagnostics.push(diag(
            GameplayModuleBindingDiagnosticCode::ModuleMismatch,
            format!("{path}.module"),
            "configuration module is absent from the closed registry",
        ));
        return;
    };
    if !module_refs_are_semantically_compatible(&module.module_ref, &configuration.module) {
        diagnostics.push(diag(
            GameplayModuleBindingDiagnosticCode::ProviderMismatch,
            format!("{path}.module"),
            "compiled provider semantic identity differs from authored configuration",
        ));
    } else if module.module_ref.artifact_hash != configuration.module.artifact_hash {
        let provenance_diagnostic = GameplayCompositionDiagnostic {
            code: GameplayCompositionDiagnosticCode::ArtifactProvenanceMismatch,
            severity: if load_mode == GameplayCompositionLoadMode::Exact {
                DiagnosticSeverity::Error
            } else {
                DiagnosticSeverity::Warning
            },
            path: format!("{path}.module.artifactHash"),
            expected: Some(configuration.module.artifact_hash.clone()),
            actual: Some(module.module_ref.artifact_hash.clone()),
            message: "compiled artifact provenance differs while semantic module identity remains compatible"
                .to_owned(),
        };
        if load_mode == GameplayCompositionLoadMode::Exact {
            diagnostics.push(diag(
                GameplayModuleBindingDiagnosticCode::ProviderMismatch,
                format!("{path}.module.artifactHash"),
                "exact composition mode rejects compiled artifact provenance mismatch",
            ));
        }
        compatibility_diagnostics.push(provenance_diagnostic);
    }
    match codecs.iter().find(|codec| {
        let schema = codec.metadata();
        schema.module_id == configuration.module.module_id
            && schema.configuration == configuration.configuration
    }) {
        None => diagnostics.push(diag(
            GameplayModuleBindingDiagnosticCode::ConfigurationSchemaMismatch,
            format!("{path}.configuration"),
            "configuration schema is not exported by the compiled provider",
        )),
        Some(codec) if codec.metadata().codec_id != configuration.codec_id => {
            diagnostics.push(diag(
                GameplayModuleBindingDiagnosticCode::ConfigurationCodecMismatch,
                format!("{path}.codecId"),
                "configuration codec differs from the compiled provider schema",
            ))
        }
        Some(codec) => {
            if let Err(error) = codec.validate(&configuration.canonical_config) {
                diagnostics.push(diag(
                    GameplayModuleBindingDiagnosticCode::ConfigurationSchemaMismatch,
                    format!("{path}.canonicalConfig"),
                    format!("typed configuration codec rejected payload: {error}"),
                ));
            }
        }
    }
}

fn module_refs_are_semantically_compatible(
    compiled: &protocol_game_extension::GameplayModuleRef,
    authored: &protocol_game_extension::GameplayModuleRef,
) -> bool {
    compiled.module_id == authored.module_id
        && compiled.namespace == authored.namespace
        && compiled.version == authored.version
        && compiled.sdk_hash == authored.sdk_hash
        && compiled.contract_hash == authored.contract_hash
        && compiled.provider_id == authored.provider_id
}

fn validate_binding(
    binding: &GameplayModuleBinding,
    configuration: Option<&GameplayModuleConfiguration>,
    registry: &GameplayFabricRegistry,
    index: usize,
    diagnostics: &mut Vec<GameplayModuleBindingDiagnostic>,
) {
    let path = format!("bindings[{index}]");
    let Some(configuration) = configuration else {
        diagnostics.push(diag(
            GameplayModuleBindingDiagnosticCode::UnknownConfiguration,
            format!("{path}.configurationId"),
            "binding references an unknown configuration",
        ));
        return;
    };
    if binding.module_id != configuration.module.module_id {
        diagnostics.push(diag(
            GameplayModuleBindingDiagnosticCode::ModuleMismatch,
            format!("{path}.moduleId"),
            "binding and configuration select different modules",
        ));
        return;
    }
    let Some(module) = registry.module(&binding.module_id) else {
        return;
    };
    if !module
        .state_schemas
        .iter()
        .any(|owned| owned.schema == binding.state_schema)
    {
        diagnostics.push(diag(
            GameplayModuleBindingDiagnosticCode::StateSchemaMismatch,
            format!("{path}.stateSchema"),
            "state schema is not owned by the selected module",
        ));
    }
    for (read_index, read) in binding.required_reads.iter().enumerate() {
        if !module.read_views.contains(read) {
            diagnostics.push(diag(
                GameplayModuleBindingDiagnosticCode::ReadContractMismatch,
                format!("{path}.requiredReads[{read_index}]"),
                "required read contract is not declared by the selected module",
            ));
        }
    }
    for (output_index, output) in binding.output_contracts.iter().enumerate() {
        if !module
            .invocations
            .iter()
            .any(|invocation| invocation.output_contract == *output)
        {
            diagnostics.push(diag(
                GameplayModuleBindingDiagnosticCode::OutputContractMismatch,
                format!("{path}.outputContracts[{output_index}]"),
                "output contract is not produced by a selected module invocation",
            ));
        }
    }
}

type OverrideIndex<'a> = BTreeMap<(String, PrefabInstanceId), &'a GameplayModuleBindingOverride>;

fn validate_overrides<'a>(
    bindings: &'a GameplayModuleBindingRegistry,
    indexed_bindings: &BTreeMap<String, &'a GameplayModuleBinding>,
    configurations: &BTreeMap<String, &'a GameplayModuleConfiguration>,
    bundle: &ProjectBundleLoadResult,
    diagnostics: &mut Vec<GameplayModuleBindingDiagnostic>,
) -> OverrideIndex<'a> {
    let mut indexed = BTreeMap::new();
    for (index, layer) in bindings.overrides.iter().enumerate() {
        let path = format!("overrides[{index}]");
        let Some(binding) = indexed_bindings.get(&layer.binding_id).copied() else {
            diagnostics.push(diag(
                GameplayModuleBindingDiagnosticCode::InvalidOverride,
                format!("{path}.bindingId"),
                "override references an unknown binding",
            ));
            continue;
        };
        let Some(instance) = bundle.prefab_instances.instance(layer.prefab_instance) else {
            diagnostics.push(diag(
                GameplayModuleBindingDiagnosticCode::InvalidOverride,
                format!("{path}.prefabInstance"),
                "override references an unknown prefab instance",
            ));
            continue;
        };
        let target_prefab = match &binding.target {
            GameplayModuleBindingTarget::Prefab { prefab } => Some(*prefab),
            GameplayModuleBindingTarget::PrefabPart { part } => Some(part.prefab),
            _ => None,
        };
        if target_prefab != Some(instance.record.prefab) {
            diagnostics.push(diag(
                GameplayModuleBindingDiagnosticCode::InvalidOverride,
                path.clone(),
                "override instance does not belong to the binding target prefab",
            ));
        }
        if let Some(configuration_id) = &layer.configuration_id {
            match configurations.get(configuration_id) {
                Some(configuration) if configuration.module.module_id == binding.module_id => {}
                _ => diagnostics.push(diag(
                    GameplayModuleBindingDiagnosticCode::InvalidOverride,
                    format!("{path}.configurationId"),
                    "override configuration is missing or belongs to another module",
                )),
            }
        }
        if indexed
            .insert((layer.binding_id.clone(), layer.prefab_instance), layer)
            .is_some()
        {
            diagnostics.push(diag(
                GameplayModuleBindingDiagnosticCode::InvalidOverride,
                path,
                "only one override is allowed per binding and prefab instance",
            ));
        }
    }
    indexed
}

fn resolve_target(
    bundle: &ProjectBundleLoadResult,
    entity_targets: &GameplayBindingEntityTargets,
    entities: Option<&EntityStore>,
    target: &GameplayModuleBindingTarget,
) -> Vec<EffectiveTarget> {
    match target {
        GameplayModuleBindingTarget::Session => vec![EffectiveTarget {
            scope: GameplayModuleStateScope::Session,
            instance: None,
            eligible: true,
            match_entities: BTreeSet::new(),
        }],
        GameplayModuleBindingTarget::EntityDefinition { stable_id } => entity_targets
            .entities(stable_id)
            .map(|entity| EffectiveTarget {
                scope: GameplayModuleStateScope::Entity {
                    entity: entity.raw(),
                },
                instance: None,
                eligible: entity_is_active(entities, entity),
                match_entities: [entity.raw()].into_iter().collect(),
            })
            .collect(),
        GameplayModuleBindingTarget::Prefab { prefab } => bundle
            .prefab_instances
            .instances()
            .filter(|instance| instance.record.prefab == *prefab)
            .map(|instance| EffectiveTarget {
                scope: GameplayModuleStateScope::PrefabInstance {
                    instance: instance.record.instance.raw(),
                },
                instance: Some(instance.record.instance),
                eligible: instance
                    .parts
                    .iter()
                    .any(|part| part.active && entity_is_active(entities, part.entity)),
                match_entities: instance
                    .parts
                    .iter()
                    .map(|part| part.entity.raw())
                    .collect(),
            })
            .collect(),
        GameplayModuleBindingTarget::PrefabPart { part } => bundle
            .prefab_instances
            .instances()
            .filter(|instance| instance.record.prefab == part.prefab)
            .filter_map(|instance| {
                let stored_part = svc_serialization::PrefabPartReference {
                    prefab: part.prefab,
                    role: part.role.clone(),
                };
                bundle
                    .prefab_instances
                    .resolve_part(instance.record.instance, &stored_part)
                    .map(|resolution| EffectiveTarget {
                        scope: GameplayModuleStateScope::Entity {
                            entity: resolution.entity.raw(),
                        },
                        instance: Some(instance.record.instance),
                        eligible: entity_is_active(entities, resolution.entity),
                        match_entities: [resolution.entity.raw()].into_iter().collect(),
                    })
            })
            .collect(),
    }
}

fn entity_is_active(entities: Option<&EntityStore>, entity: EntityId) -> bool {
    entities
        .and_then(|entities| entities.lifecycle(entity))
        .is_some_and(|lifecycle| lifecycle == EntityLifecycle::Active)
}

fn readout(
    binding: &GameplayModuleBinding,
    configuration: &GameplayModuleConfiguration,
    resolved_scopes: Vec<String>,
    active: bool,
    registry_hash: &str,
) -> GameplayModuleBindingReadout {
    let encoded = serde_json::to_vec(&(
        registry_hash,
        &binding.binding_id,
        &configuration.configuration_id,
        &binding.target,
        &resolved_scopes,
        active,
    ))
    .expect("binding readout values serialize");
    GameplayModuleBindingReadout {
        binding_id: binding.binding_id.clone(),
        module_id: binding.module_id.clone(),
        configuration_id: configuration.configuration_id.clone(),
        target: binding.target.clone(),
        resolved_scopes,
        active,
        provenance_hash: gameplay_module_payload_hash(&encoded),
    }
}

fn activation_receipt(
    bindings: &GameplayModuleBindingRegistry,
    registry: &GameplayFabricRegistry,
    compatibility_diagnostics: Vec<GameplayCompositionDiagnostic>,
    readouts: Vec<GameplayModuleBindingReadout>,
    module_state_hash: String,
) -> GameplayModuleBindingActivationReceipt {
    let composition_identity = gameplay_runtime_composition_identity(registry, bindings);
    let bytes = serde_json::to_vec(&(
        &bindings.registry_hash,
        registry.registry_digest(),
        &composition_identity.semantic_compatibility_digest,
        &composition_identity.artifact_provenance_digest,
        &compatibility_diagnostics,
        &readouts,
        &module_state_hash,
    ))
    .expect("activation receipt values serialize");
    GameplayModuleBindingActivationReceipt {
        binding_registry_hash: bindings.registry_hash.clone(),
        gameplay_registry_digest: registry.registry_digest().to_owned(),
        semantic_compatibility_digest: composition_identity.semantic_compatibility_digest,
        artifact_provenance_digest: composition_identity.artifact_provenance_digest,
        compatibility_diagnostics,
        readouts,
        module_state_hash,
        receipt_hash: gameplay_module_payload_hash(&bytes),
    }
}

fn authority_artifact(bundle: &ProjectBundleLoadResult) -> SessionStateArtifact {
    bundle.compose_session_state_snapshot().unwrap_or_else(|| {
        let empty = EntityStore::new();
        compose_session_state_snapshot_with_prefabs(
            &empty.snapshot_durable(),
            &bundle.prefab_instances.snapshot(&empty),
        )
    })
}

fn authority_state_hash(bundle: &ProjectBundleLoadResult) -> String {
    match &bundle.runtime_entities {
        Some(entities) => format!("entity:{:016x}", entities.hash().0),
        None => format!("entity:{:016x}", EntityStore::new().hash().0),
    }
}

fn gameplay_session_snapshot_hash(
    binding_registry_hash: &str,
    activation: &GameplayModuleBindingActivationReceipt,
    module_session: &[u8],
    trigger_snapshot: &TriggerVolumeSnapshot,
) -> String {
    let activation = serde_json::to_vec(activation).expect("activation receipt serializes");
    let trigger_snapshot =
        serde_json::to_vec(trigger_snapshot).expect("trigger snapshot serializes");
    gameplay_module_payload_hash(
        &[
            binding_registry_hash.as_bytes(),
            activation.as_slice(),
            module_session,
            trigger_snapshot.as_slice(),
        ]
        .concat(),
    )
}

fn scope_label(scope: &GameplayModuleStateScope) -> String {
    match scope {
        GameplayModuleStateScope::Session => "session".to_owned(),
        GameplayModuleStateScope::Entity { entity } => format!("entity:{entity}"),
        GameplayModuleStateScope::PrefabInstance { instance } => {
            format!("prefabInstance:{instance}")
        }
    }
}

fn diag(
    code: GameplayModuleBindingDiagnosticCode,
    path: impl Into<String>,
    message: impl Into<String>,
) -> GameplayModuleBindingDiagnostic {
    GameplayModuleBindingDiagnostic {
        code,
        path: path.into(),
        message: message.into(),
    }
}

struct BoundSessionViews<'a> {
    registry_digest: &'a str,
}

impl GameplayViewSource for BoundSessionViews<'_> {
    fn freeze(&self, root_id: &str, wave: u32) -> FrozenGameplayViews {
        FrozenGameplayViews {
            epoch: u64::from(wave),
            view_hash: gameplay_module_payload_hash(
                format!("{}|{root_id}|{wave}", self.registry_digest).as_bytes(),
            ),
        }
    }
}

struct RejectSharedProposals;

impl GameplayProposalRouter for RejectSharedProposals {
    fn route(&mut self, _call: &GameplayOwnerRoutingCall) -> GameplayOwnerRoutingOutput {
        GameplayOwnerRoutingOutput {
            accepted: false,
            diagnostic_codes: vec!["privateOwnerRouterRequired".to_owned()],
            ..GameplayOwnerRoutingOutput::default()
        }
    }
}

fn limits_from_registry(registry: &GameplayFabricRegistry) -> GameplayRuntimeLimits {
    let mut limits = GameplayRuntimeLimits {
        max_waves: 1,
        max_events_per_root: 1,
        max_proposals_per_root: 1,
        max_invocations_per_root: 1,
        max_payload_bytes_per_root: 1,
    };
    for module_id in registry.module_order() {
        let budget = &registry
            .module(module_id)
            .expect("closed module order")
            .budget;
        limits.max_waves = limits.max_waves.max(budget.max_waves);
        limits.max_events_per_root = limits
            .max_events_per_root
            .saturating_add(budget.max_events_per_root);
        limits.max_proposals_per_root = limits
            .max_proposals_per_root
            .saturating_add(budget.max_proposals_per_root);
        limits.max_invocations_per_root = limits
            .max_invocations_per_root
            .saturating_add(budget.max_invocations_per_root);
        limits.max_payload_bytes_per_root = limits
            .max_payload_bytes_per_root
            .saturating_add(budget.max_payload_bytes_per_root);
    }
    limits
}
