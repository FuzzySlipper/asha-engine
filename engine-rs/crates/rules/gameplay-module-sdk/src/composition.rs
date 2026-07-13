use crate::{GameplayModuleBehavior, GameplayModuleContext};
use protocol_game_extension::{GameplayContractRef, GameplayModuleManifest};
use rule_gameplay_fabric::{
    gameplay_module_payload_hash, register_standard_owner_events, FrozenGameplayViews,
    GameplayFabricCoordinator, GameplayHostError, GameplayInvocationCall,
    GameplayInvocationConfiguration, GameplayInvocationHost, GameplayInvocationInput,
    GameplayModuleStateError, GameplayModuleStateRegistration, GameplayModuleStateScope,
    GameplayObserveReceipt, GameplayOwnerRoutingCall, GameplayOwnerRoutingOutput,
    GameplayProposalRouter, GameplayRuntimeLimits, GameplayViewSource,
};
use serde::{de::DeserializeOwned, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;
use svc_gameplay_fabric::{
    stable_bytes_identity, stable_identity, GameplayEventCodecRegistration, GameplayFabricRegistry,
    GameplayFabricRegistryBuilder, GameplayLinkedProvider, GameplayProposalOwnerRegistration,
    GameplayReadViewProviderRegistration, GameplayRegistryBuildError,
    GameplayStateOwnerRegistration,
};

const GAMEPLAY_PUBLIC_CONTRACT_VERSION: &str = "gameplay-module-contract-v1";
const GAMEPLAY_PUBLIC_SDK_PACKAGE: &str = "asha-gameplay-module-sdk";

/// Computed provenance for one statically linked gameplay provider.
///
/// `source_hash` covers the caller-supplied source bytes, lockfile bytes, and
/// sorted feature set. `artifact_hash` is deliberately a linked-provenance
/// identity (source + contract + SDK + behavior type), not a claim that Rust
/// machine code is reproducible across toolchains.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameplayModuleBuildProvenance {
    package_name: String,
    package_version: String,
    features: Vec<String>,
    source_inputs_hash: String,
    lockfile_hash: String,
}

impl GameplayModuleBuildProvenance {
    pub fn from_build_inputs(
        package_name: impl Into<String>,
        package_version: impl Into<String>,
        source_inputs: &[&[u8]],
        lockfile: &[u8],
        features: &[&str],
    ) -> Self {
        assert!(
            !source_inputs.is_empty(),
            "gameplay module provenance requires at least one source input"
        );
        assert!(
            !lockfile.is_empty(),
            "gameplay module provenance requires lockfile bytes"
        );
        let mut features = features
            .iter()
            .map(|feature| (*feature).to_owned())
            .collect::<Vec<_>>();
        features.sort();
        features.dedup();
        Self {
            package_name: package_name.into(),
            package_version: package_version.into(),
            features,
            source_inputs_hash: stable_bytes_identity(source_inputs.iter().copied()),
            lockfile_hash: stable_bytes_identity([lockfile]),
        }
    }

    pub fn apply_to_manifest<B: 'static>(&self, manifest: &mut GameplayModuleManifest) {
        manifest.module_ref.sdk_hash = stable_identity([
            "asha.gameplay-sdk.v1",
            GAMEPLAY_PUBLIC_SDK_PACKAGE,
            env!("CARGO_PKG_VERSION"),
            GAMEPLAY_PUBLIC_CONTRACT_VERSION,
        ]);
        manifest.source_hash = stable_identity([
            "asha.gameplay-source-provenance.v1",
            self.package_name.as_str(),
            self.package_version.as_str(),
            self.source_inputs_hash.as_str(),
            self.lockfile_hash.as_str(),
            self.features.join(",").as_str(),
        ]);

        let mut contract_manifest = manifest.clone();
        contract_manifest.module_ref.sdk_hash.clear();
        contract_manifest.module_ref.contract_hash.clear();
        contract_manifest.module_ref.artifact_hash.clear();
        contract_manifest.source_hash.clear();
        let contract_bytes = serde_json::to_vec(&contract_manifest)
            .expect("GameplayModuleManifest always has a JSON representation");
        manifest.module_ref.contract_hash = stable_bytes_identity([
            b"asha.gameplay-module-contract.v1".as_slice(),
            contract_bytes.as_slice(),
        ]);
        manifest.module_ref.artifact_hash = stable_identity([
            "asha.gameplay-linked-provenance.v1",
            manifest.module_ref.sdk_hash.as_str(),
            manifest.module_ref.contract_hash.as_str(),
            manifest.source_hash.as_str(),
            core::any::type_name::<B>(),
        ]);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameplayConfigurationFieldMetadata {
    pub name: String,
    pub value_type: String,
    pub required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameplayConfigurationSchemaMetadata {
    pub module_id: String,
    pub configuration: GameplayContractRef,
    pub codec_id: String,
    pub fields: Vec<GameplayConfigurationFieldMetadata>,
}

#[derive(Clone)]
pub struct GameplayConfigurationCodecRegistration {
    metadata: GameplayConfigurationSchemaMetadata,
    validate: Rc<GameplayConfigurationValidator>,
}

type GameplayConfigurationValidator = dyn Fn(&[u8]) -> Result<(), String>;

impl GameplayConfigurationCodecRegistration {
    pub fn typed<T>(metadata: GameplayConfigurationSchemaMetadata) -> Self
    where
        T: DeserializeOwned + Serialize + 'static,
    {
        Self {
            metadata,
            validate: Rc::new(|canonical| {
                let decoded: T =
                    serde_json::from_slice(canonical).map_err(|error| error.to_string())?;
                let encoded = serde_json::to_vec(&decoded).map_err(|error| error.to_string())?;
                if encoded != canonical {
                    return Err(
                        "configuration bytes are not canonical for the typed codec".to_owned()
                    );
                }
                Ok(())
            }),
        }
    }

    pub fn metadata(&self) -> &GameplayConfigurationSchemaMetadata {
        &self.metadata
    }

    pub fn validate(&self, canonical: &[u8]) -> Result<(), String> {
        (self.validate)(canonical)
    }
}

pub struct GameplayStaticModuleProvider {
    pub manifest: GameplayModuleManifest,
    pub linked_provider: GameplayLinkedProvider,
    pub configuration_schemas: Vec<GameplayConfigurationSchemaMetadata>,
    configuration_codecs: Vec<GameplayConfigurationCodecRegistration>,
    event_codecs: Vec<GameplayEventCodecRegistration>,
    proposal_owners: Vec<GameplayProposalOwnerRegistration>,
    read_view_providers: Vec<GameplayReadViewProviderRegistration>,
    state_owners: Vec<GameplayStateOwnerRegistration>,
    state_adapters: Vec<GameplayModuleStateRegistration>,
    behavior: Box<dyn GameplayModuleBehavior>,
}

impl GameplayStaticModuleProvider {
    pub fn new(
        manifest: GameplayModuleManifest,
        linked_provider: GameplayLinkedProvider,
        behavior: impl GameplayModuleBehavior + 'static,
    ) -> Self {
        Self {
            manifest,
            linked_provider,
            configuration_schemas: Vec::new(),
            configuration_codecs: Vec::new(),
            event_codecs: Vec::new(),
            proposal_owners: Vec::new(),
            read_view_providers: Vec::new(),
            state_owners: Vec::new(),
            state_adapters: Vec::new(),
            behavior: Box::new(behavior),
        }
    }

    pub fn linked_from_manifest<B: GameplayModuleBehavior + 'static>(
        manifest: GameplayModuleManifest,
        provenance: &GameplayModuleBuildProvenance,
        behavior: B,
    ) -> Self {
        let mut linked_manifest = manifest.clone();
        provenance.apply_to_manifest::<B>(&mut linked_manifest);
        let module = &linked_manifest.module_ref;
        let linked = GameplayLinkedProvider {
            provider_id: module.provider_id.clone(),
            module_id: module.module_id.clone(),
            version: module.version.clone(),
            contract_hash: module.contract_hash.clone(),
            artifact_hash: module.artifact_hash.clone(),
            sdk_hash: module.sdk_hash.clone(),
            source_hash: linked_manifest.source_hash,
        };
        Self::new(manifest, linked, behavior)
    }

    pub fn event_codec(mut self, registration: GameplayEventCodecRegistration) -> Self {
        self.event_codecs.push(registration);
        self
    }

    /// Register the canonical typed codec for a proposal declared by this
    /// module. Proposals and events share the same closed codec table, while
    /// the named method keeps provider composition intent inspectable.
    pub fn proposal_codec(mut self, registration: GameplayEventCodecRegistration) -> Self {
        self.event_codecs.push(registration);
        self
    }

    pub fn proposal_owner(mut self, registration: GameplayProposalOwnerRegistration) -> Self {
        self.proposal_owners.push(registration);
        self
    }

    pub fn read_view_provider(
        mut self,
        registration: GameplayReadViewProviderRegistration,
    ) -> Self {
        self.read_view_providers.push(registration);
        self
    }

    pub fn state_owner(mut self, registration: GameplayStateOwnerRegistration) -> Self {
        self.state_owners.push(registration);
        self
    }

    pub fn state_adapter(mut self, registration: GameplayModuleStateRegistration) -> Self {
        self.state_adapters.push(registration);
        self
    }

    pub fn configuration_schema(mut self, schema: GameplayConfigurationSchemaMetadata) -> Self {
        self.configuration_schemas.push(schema);
        self
    }

    pub fn configuration_codec(mut self, codec: GameplayConfigurationCodecRegistration) -> Self {
        self.configuration_codecs.push(codec);
        self
    }
}

#[derive(Debug)]
pub enum GameplayStaticCompositionError {
    DuplicateBehavior(String),
    InvalidConfigurationSchema(String),
    Registry(GameplayRegistryBuildError),
    StateAdapter(GameplayModuleStateError),
}

impl core::fmt::Display for GameplayStaticCompositionError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for GameplayStaticCompositionError {}

#[derive(Default)]
pub struct GameplayStaticCompositionBuilder {
    providers: Vec<GameplayStaticModuleProvider>,
    include_standard_owner_events: bool,
}

impl GameplayStaticCompositionBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_provider(&mut self, provider: GameplayStaticModuleProvider) -> &mut Self {
        self.providers.push(provider);
        self
    }

    /// Include the engine-owned asha event publisher/codecs so downstream
    /// modules can subscribe to semantic owner facts without private imports.
    /// This is explicit because pure module-unit compositions may not need them.
    pub fn include_standard_owner_events(&mut self) -> &mut Self {
        self.include_standard_owner_events = true;
        self
    }

    pub fn build(self) -> Result<GameplayStaticComposition, GameplayStaticCompositionError> {
        let mut registry_builder = GameplayFabricRegistryBuilder::new();
        if self.include_standard_owner_events {
            register_standard_owner_events(&mut registry_builder);
        }
        let mut behaviors = BTreeMap::new();
        let mut state_adapters = Vec::new();
        let mut configuration_schemas = Vec::new();
        let mut configuration_codecs = Vec::new();
        for provider in self.providers {
            let module_id = provider.manifest.module_ref.module_id.clone();
            if behaviors
                .insert(module_id.clone(), provider.behavior)
                .is_some()
            {
                return Err(GameplayStaticCompositionError::DuplicateBehavior(module_id));
            }
            validate_configuration_schemas(&provider.manifest, &provider.configuration_schemas)?;
            validate_configuration_codecs(
                &provider.configuration_schemas,
                &provider.configuration_codecs,
            )?;
            configuration_schemas.extend(provider.configuration_schemas);
            configuration_codecs.extend(provider.configuration_codecs);
            state_adapters.extend(provider.state_adapters);
            registry_builder
                .register_module(provider.manifest)
                .register_linked_provider(provider.linked_provider);
            for codec in provider.event_codecs {
                registry_builder.register_event_codec_registration(codec);
            }
            for owner in provider.proposal_owners {
                registry_builder.register_proposal_owner(owner);
            }
            for view in provider.read_view_providers {
                registry_builder.register_read_view_provider(view);
            }
            for owner in provider.state_owners {
                registry_builder.register_state_owner(owner);
            }
        }
        let registry = Rc::new(
            registry_builder
                .build()
                .map_err(GameplayStaticCompositionError::Registry)?,
        );
        for adapter in &state_adapters {
            adapter
                .validate_against_registry(registry.as_ref())
                .map_err(GameplayStaticCompositionError::StateAdapter)?;
        }
        configuration_schemas.sort_by(|left, right| {
            (left.module_id.as_str(), left.configuration.key())
                .cmp(&(right.module_id.as_str(), right.configuration.key()))
        });
        Ok(GameplayStaticComposition {
            registry,
            host: GameplayStaticInvocationHost {
                behaviors,
                configuration_bindings: Vec::new(),
            },
            state_adapters,
            configuration_schemas,
            configuration_codecs,
        })
    }
}

pub struct GameplayStaticComposition {
    registry: Rc<GameplayFabricRegistry>,
    host: GameplayStaticInvocationHost,
    state_adapters: Vec<GameplayModuleStateRegistration>,
    configuration_schemas: Vec<GameplayConfigurationSchemaMetadata>,
    configuration_codecs: Vec<GameplayConfigurationCodecRegistration>,
}

impl GameplayStaticComposition {
    pub fn registry(&self) -> &GameplayFabricRegistry {
        self.registry.as_ref()
    }

    pub fn invocation_host(&self) -> &GameplayStaticInvocationHost {
        &self.host
    }

    pub fn configuration_schemas(&self) -> &[GameplayConfigurationSchemaMetadata] {
        &self.configuration_schemas
    }

    /// Executes one static Session Observe root for modules that emit only
    /// events and module-local facts. Shared proposals fail closed here; the
    /// owning RuntimeSession supplies its private owner router for them.
    pub fn observe_session_event(
        &self,
        event: protocol_game_extension::GameplayEventEnvelope,
    ) -> GameplayObserveReceipt {
        GameplayFabricCoordinator::new(&self.registry, limits_from_registry(&self.registry))
            .observe(
                event,
                &StaticSessionViews {
                    registry_digest: self.registry.registry_digest(),
                },
                &self.host,
                &mut RejectSharedProposals,
            )
    }

    pub fn into_parts(self) -> GameplayStaticCompositionParts {
        GameplayStaticCompositionParts {
            registry: self.registry,
            host: self.host,
            state_adapters: self.state_adapters,
            configuration_schemas: self.configuration_schemas,
            configuration_codecs: self.configuration_codecs,
        }
    }
}

struct StaticSessionViews<'a> {
    registry_digest: &'a str,
}

impl GameplayViewSource for StaticSessionViews<'_> {
    fn freeze(&self, root_id: &str, wave: u32) -> FrozenGameplayViews {
        let key = format!("{}|{root_id}|{wave}", self.registry_digest);
        FrozenGameplayViews {
            epoch: u64::from(wave),
            view_hash: gameplay_module_payload_hash(key.as_bytes()),
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

pub struct GameplayStaticCompositionParts {
    pub registry: Rc<GameplayFabricRegistry>,
    pub host: GameplayStaticInvocationHost,
    pub state_adapters: Vec<GameplayModuleStateRegistration>,
    pub configuration_schemas: Vec<GameplayConfigurationSchemaMetadata>,
    pub configuration_codecs: Vec<GameplayConfigurationCodecRegistration>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameplayStaticConfigurationBinding {
    pub module_id: String,
    pub binding_id: String,
    pub configuration_id: String,
    pub scope: GameplayModuleStateScope,
    pub match_entities: BTreeSet<u64>,
    pub canonical_config: Vec<u8>,
    pub config_hash: String,
}

pub struct GameplayStaticInvocationHost {
    behaviors: BTreeMap<String, Box<dyn GameplayModuleBehavior>>,
    configuration_bindings: Vec<GameplayStaticConfigurationBinding>,
}

impl GameplayStaticInvocationHost {
    pub fn install_configuration_bindings(
        &mut self,
        mut bindings: Vec<GameplayStaticConfigurationBinding>,
    ) {
        bindings.sort_by(|left, right| {
            (
                left.module_id.as_str(),
                left.binding_id.as_str(),
                left.configuration_id.as_str(),
                &left.scope,
            )
                .cmp(&(
                    right.module_id.as_str(),
                    right.binding_id.as_str(),
                    right.configuration_id.as_str(),
                    &right.scope,
                ))
        });
        self.configuration_bindings = bindings;
    }
}

impl GameplayInvocationHost for GameplayStaticInvocationHost {
    fn resolve_configuration(
        &self,
        call: &GameplayInvocationCall,
    ) -> Result<Option<GameplayInvocationConfiguration>, GameplayHostError> {
        let mut identities = BTreeSet::new();
        match &call.input {
            GameplayInvocationInput::Observe(event) => {
                identities.extend(event.source.iter().map(|item| item.entity.raw()));
                identities.extend(event.subjects.iter().map(|item| item.entity.raw()));
                identities.extend(event.targets.iter().map(|item| item.entity.raw()));
            }
            GameplayInvocationInput::Decision(moment) => {
                identities.extend(moment.operation.source.iter().map(|item| item.entity.raw()));
                identities.extend(
                    moment
                        .operation
                        .targets
                        .iter()
                        .map(|item| item.entity.raw()),
                );
            }
        }
        let module = self
            .configuration_bindings
            .iter()
            .filter(|binding| binding.module_id == call.module_id)
            .collect::<Vec<_>>();
        let specific = module
            .iter()
            .copied()
            .filter(|binding| {
                !binding.match_entities.is_empty()
                    && binding
                        .match_entities
                        .iter()
                        .any(|entity| identities.contains(entity))
            })
            .collect::<Vec<_>>();
        let selected = if specific.len() == 1 {
            specific.first().copied()
        } else if specific.len() > 1 {
            return Err(GameplayHostError {
                code: "ambiguousInvocationConfiguration".to_owned(),
                message: format!(
                    "invocation `{}` matches multiple authored configuration scopes",
                    call.invocation_id
                ),
            });
        } else {
            let session = module
                .iter()
                .copied()
                .filter(|binding| binding.scope == GameplayModuleStateScope::Session)
                .collect::<Vec<_>>();
            if session.len() > 1 {
                return Err(GameplayHostError {
                    code: "ambiguousInvocationConfiguration".to_owned(),
                    message: "module has multiple Session configuration bindings".to_owned(),
                });
            }
            session.first().copied()
        };
        Ok(selected.map(|binding| GameplayInvocationConfiguration {
            binding_id: binding.binding_id.clone(),
            configuration_id: binding.configuration_id.clone(),
            scope: binding.scope.clone(),
            canonical_config: binding.canonical_config.clone(),
            config_hash: binding.config_hash.clone(),
        }))
    }

    fn invoke(
        &self,
        call: &GameplayInvocationCall,
    ) -> Result<rule_gameplay_fabric::GameplayInvocationOutput, GameplayHostError> {
        let behavior = self
            .behaviors
            .get(&call.module_id)
            .ok_or_else(|| GameplayHostError {
                code: "missingStaticBehavior".to_owned(),
                message: format!("no behavior instance for module `{}`", call.module_id),
            })?;
        behavior
            .invoke(&GameplayModuleContext::new(call))
            .map(|actions| actions.finish())
            .map_err(Into::into)
    }
}

fn validate_configuration_schemas(
    manifest: &GameplayModuleManifest,
    schemas: &[GameplayConfigurationSchemaMetadata],
) -> Result<(), GameplayStaticCompositionError> {
    let mut seen = BTreeSet::new();
    for schema in schemas {
        if schema.module_id != manifest.module_ref.module_id
            || schema.configuration.namespace != manifest.module_ref.namespace
            || schema.configuration.version == 0
            || schema.configuration.schema_hash.trim().is_empty()
            || schema.codec_id.trim().is_empty()
            || !seen.insert(schema.configuration.key())
        {
            return Err(GameplayStaticCompositionError::InvalidConfigurationSchema(
                schema.configuration.key(),
            ));
        }
        let mut field_names = BTreeSet::new();
        if schema.fields.iter().any(|field| {
            field.name.trim().is_empty()
                || field.value_type.trim().is_empty()
                || !field_names.insert(field.name.as_str())
        }) {
            return Err(GameplayStaticCompositionError::InvalidConfigurationSchema(
                schema.configuration.key(),
            ));
        }
    }
    Ok(())
}

fn validate_configuration_codecs(
    schemas: &[GameplayConfigurationSchemaMetadata],
    codecs: &[GameplayConfigurationCodecRegistration],
) -> Result<(), GameplayStaticCompositionError> {
    for schema in schemas {
        let matching = codecs
            .iter()
            .filter(|codec| codec.metadata() == schema)
            .count();
        if matching != 1 {
            return Err(GameplayStaticCompositionError::InvalidConfigurationSchema(
                schema.configuration.key(),
            ));
        }
    }
    if codecs.len() != schemas.len() {
        return Err(GameplayStaticCompositionError::InvalidConfigurationSchema(
            "unmatchedConfigurationCodec".to_owned(),
        ));
    }
    Ok(())
}
