use crate::{
    GameplayConfigurationCodecRegistration, GameplayConfigurationFieldMetadata,
    GameplayConfigurationSchemaMetadata,
};
use protocol_game_extension::{
    GameplayContractRef, GameplayEventSchemaDeclaration, GameplayHeaderSelector,
    GameplayInvocationDescriptor, GameplayInvocationFamily, GameplayInvocationReadRequirement,
    GameplayModuleManifest, GameplayOwnerRef, GameplayReadSelectorCapability, GameplayReadViewKind,
    GameplayReadViewRequirement, GameplaySubscriptionDeclaration,
};
use rule_gameplay_fabric::{
    GameplayModuleStateRegistration, GameplayModuleStateScope, GameplayReadRequest,
    GameplayTypedModuleStateAdapter,
};
use serde::{de::DeserializeOwned, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::marker::PhantomData;
use svc_gameplay_fabric::{
    gameplay_canonical_codec_id, GameplayEventCodecRegistration,
    GameplayReadViewProviderRegistration, TypedGameplayEventCodec,
};

/// Typed canonical-JSON codec using the same schema descriptor that produced
/// the contract hash. The registry still rejects a descriptor/contract drift.
pub fn gameplay_serde_json_codec<T>(
    contract: GameplayContractRef,
    schema_descriptor: impl Into<String>,
) -> TypedGameplayEventCodec<T>
where
    T: Serialize + DeserializeOwned + 'static,
{
    TypedGameplayEventCodec::new(
        GameplayEventSchemaDeclaration {
            codec_id: gameplay_canonical_codec_id(&contract.schema_hash),
            event: contract,
        },
        schema_descriptor,
        |payload: &T| serde_json::to_vec(payload).map_err(|error| error.to_string()),
        |bytes| serde_json::from_slice(bytes).map_err(|error| error.to_string()),
    )
}

/// Erased registration form of [`gameplay_serde_json_codec`].
pub fn gameplay_serde_json_codec_registration<T>(
    contract: GameplayContractRef,
    schema_descriptor: impl Into<String>,
) -> GameplayEventCodecRegistration
where
    T: Serialize + DeserializeOwned + 'static,
{
    GameplayEventCodecRegistration::typed(gameplay_serde_json_codec::<T>(
        contract,
        schema_descriptor,
    ))
}

/// One typed configuration declaration. A provider consumes this value once
/// to install both reviewable metadata and its serde-backed validator.
pub struct GameplaySerdeConfiguration<T> {
    metadata: GameplayConfigurationSchemaMetadata,
    marker: PhantomData<fn() -> T>,
}

impl<T> GameplaySerdeConfiguration<T>
where
    T: Serialize + DeserializeOwned + 'static,
{
    pub fn new(
        module_id: impl Into<String>,
        configuration: GameplayContractRef,
        fields: Vec<GameplayConfigurationFieldMetadata>,
    ) -> Self {
        let codec_id = gameplay_canonical_codec_id(&configuration.schema_hash);
        Self {
            metadata: GameplayConfigurationSchemaMetadata {
                module_id: module_id.into(),
                configuration,
                codec_id,
                fields,
            },
            marker: PhantomData,
        }
    }

    pub fn metadata(&self) -> &GameplayConfigurationSchemaMetadata {
        &self.metadata
    }

    pub(crate) fn into_parts(
        self,
    ) -> (
        GameplayConfigurationSchemaMetadata,
        GameplayConfigurationCodecRegistration,
    ) {
        let metadata = self.metadata;
        let codec = GameplayConfigurationCodecRegistration::typed::<T>(metadata.clone());
        (metadata, codec)
    }
}

/// Serde-backed state adapter contract for the normal downstream path.
/// Schemas and owner identity are returned as owned values, avoiding static
/// caches and panic-based contract lookup in module code.
pub trait GameplaySerdeModuleStateAdapter {
    type Config: DeserializeOwned;
    type State: Serialize + DeserializeOwned;
    type Fact: DeserializeOwned;
    type View: Serialize;

    fn module_id(&self) -> &str;
    fn state_schema(&self) -> GameplayContractRef;
    fn fact_schema(&self) -> GameplayContractRef;
    fn owner(&self) -> GameplayOwnerRef;
    fn initialize(&self, config: &Self::Config) -> Result<Self::State, String>;
    fn apply_fact(&self, state: &Self::State, fact: &Self::Fact) -> Result<Self::State, String>;
    fn migrate(&self, from_version: u32, state: &Self::State) -> Result<Self::State, String>;

    fn view_schema(&self) -> Option<GameplayContractRef> {
        None
    }

    fn project_view(&self, _state: &Self::State) -> Result<Self::View, String> {
        Err("adapter publishes no named view".to_owned())
    }
}

struct SerdeStateAdapter<T> {
    adapter: T,
    state_schema: GameplayContractRef,
    fact_schema: GameplayContractRef,
    owner: GameplayOwnerRef,
    view_schema: Option<GameplayContractRef>,
}

impl<T> GameplayTypedModuleStateAdapter for SerdeStateAdapter<T>
where
    T: GameplaySerdeModuleStateAdapter,
{
    type Config = T::Config;
    type State = T::State;
    type Fact = T::Fact;
    type View = T::View;

    fn module_id(&self) -> &str {
        self.adapter.module_id()
    }

    fn state_schema(&self) -> &GameplayContractRef {
        &self.state_schema
    }

    fn fact_schema(&self) -> &GameplayContractRef {
        &self.fact_schema
    }

    fn owner(&self) -> &GameplayOwnerRef {
        &self.owner
    }

    fn decode_config(&self, bytes: &[u8]) -> Result<Self::Config, String> {
        serde_json::from_slice(bytes).map_err(|error| error.to_string())
    }

    fn decode_state(&self, bytes: &[u8]) -> Result<Self::State, String> {
        serde_json::from_slice(bytes).map_err(|error| error.to_string())
    }

    fn decode_fact(&self, bytes: &[u8]) -> Result<Self::Fact, String> {
        serde_json::from_slice(bytes).map_err(|error| error.to_string())
    }

    fn encode_state(&self, state: &Self::State) -> Result<Vec<u8>, String> {
        serde_json::to_vec(state).map_err(|error| error.to_string())
    }

    fn initialize(&self, config: &Self::Config) -> Result<Self::State, String> {
        self.adapter.initialize(config)
    }

    fn apply_fact(&self, state: &Self::State, fact: &Self::Fact) -> Result<Self::State, String> {
        self.adapter.apply_fact(state, fact)
    }

    fn migrate(&self, from_version: u32, state: &Self::State) -> Result<Self::State, String> {
        self.adapter.migrate(from_version, state)
    }

    fn view_schema(&self) -> Option<&GameplayContractRef> {
        self.view_schema.as_ref()
    }

    fn project_view(&self, state: &Self::State) -> Result<Self::View, String> {
        self.adapter.project_view(state)
    }

    fn encode_view(&self, view: &Self::View) -> Result<Vec<u8>, String> {
        serde_json::to_vec(view).map_err(|error| error.to_string())
    }
}

pub fn gameplay_serde_state_adapter<T>(adapter: T) -> GameplayModuleStateRegistration
where
    T: GameplaySerdeModuleStateAdapter + 'static,
{
    let state_schema = adapter.state_schema();
    let fact_schema = adapter.fact_schema();
    let owner = adapter.owner();
    let view_schema = adapter.view_schema();
    GameplayModuleStateRegistration::typed(SerdeStateAdapter {
        adapter,
        state_schema,
        fact_schema,
        owner,
        view_schema,
    })
}

/// The exact static read request delivered to one invocation, together with
/// the provider declaration needed to admit that request into the registry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameplayModuleReadTopology {
    pub request: GameplayReadRequest,
    pub provider_id: String,
    pub kind: GameplayReadViewKind,
    pub selector_capabilities: Vec<GameplayReadSelectorCapability>,
    pub max_items: u32,
    pub ordering: String,
}

/// One invocation and, for Observe, its subscription. Limits and selection
/// remain literal authored data; deriving topology does not invent budgets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameplayModuleInvocationTopology {
    pub subscription_id: Option<String>,
    pub invocation_id: String,
    pub family: GameplayInvocationFamily,
    pub input_contract: GameplayContractRef,
    pub output_contract: GameplayContractRef,
    pub selector: Option<GameplayHeaderSelector>,
    pub max_deliveries_per_root: Option<u32>,
    pub max_outputs: u32,
    pub max_payload_bytes: u32,
    pub reads: Vec<GameplayModuleReadTopology>,
}

impl GameplayModuleInvocationTopology {
    #[allow(clippy::too_many_arguments)]
    pub fn observe(
        subscription_id: impl Into<String>,
        invocation_id: impl Into<String>,
        input_contract: GameplayContractRef,
        output_contract: GameplayContractRef,
        selector: GameplayHeaderSelector,
        max_deliveries_per_root: u32,
        max_outputs: u32,
        max_payload_bytes: u32,
    ) -> Self {
        Self {
            subscription_id: Some(subscription_id.into()),
            invocation_id: invocation_id.into(),
            family: GameplayInvocationFamily::Observe,
            input_contract,
            output_contract,
            selector: Some(selector),
            max_deliveries_per_root: Some(max_deliveries_per_root),
            max_outputs,
            max_payload_bytes,
            reads: Vec::new(),
        }
    }

    pub fn decision(
        invocation_id: impl Into<String>,
        family: GameplayInvocationFamily,
        input_contract: GameplayContractRef,
        output_contract: GameplayContractRef,
        max_outputs: u32,
        max_payload_bytes: u32,
    ) -> Self {
        Self {
            subscription_id: None,
            invocation_id: invocation_id.into(),
            family,
            input_contract,
            output_contract,
            selector: None,
            max_deliveries_per_root: None,
            max_outputs,
            max_payload_bytes,
            reads: Vec::new(),
        }
    }

    pub fn read(mut self, read: GameplayModuleReadTopology) -> Self {
        self.reads.push(read);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameplayRuntimeDeclaredReadPlan {
    pub module_id: String,
    pub invocation_id: String,
    pub requests: Vec<GameplayReadRequest>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameplayDerivedModuleTopology {
    module_id: String,
    subscriptions: Vec<GameplaySubscriptionDeclaration>,
    invocations: Vec<GameplayInvocationDescriptor>,
    read_views: Vec<GameplayReadViewRequirement>,
    read_view_providers: Vec<GameplayReadViewProviderRegistration>,
    declared_reads: Vec<GameplayRuntimeDeclaredReadPlan>,
}

impl GameplayDerivedModuleTopology {
    pub fn derive(
        module_id: impl Into<String>,
        authored: Vec<GameplayModuleInvocationTopology>,
    ) -> Result<Self, GameplayModuleTopologyError> {
        let module_id = module_id.into();
        let mut invocation_ids = BTreeSet::new();
        let mut subscription_ids = BTreeSet::new();
        let mut subscriptions = Vec::new();
        let mut invocations = Vec::new();
        let mut read_views = BTreeMap::<String, GameplayReadViewRequirement>::new();
        let mut read_view_providers =
            BTreeMap::<String, GameplayReadViewProviderRegistration>::new();
        let mut declared_reads = Vec::new();

        for invocation in authored {
            if !invocation_ids.insert(invocation.invocation_id.clone()) {
                return Err(GameplayModuleTopologyError::DuplicateInvocation(
                    invocation.invocation_id,
                ));
            }
            match invocation.family {
                GameplayInvocationFamily::Observe => {
                    let subscription_id = invocation.subscription_id.clone().ok_or_else(|| {
                        GameplayModuleTopologyError::MissingObserveSubscription(
                            invocation.invocation_id.clone(),
                        )
                    })?;
                    if !subscription_ids.insert(subscription_id.clone()) {
                        return Err(GameplayModuleTopologyError::DuplicateSubscription(
                            subscription_id,
                        ));
                    }
                    subscriptions.push(GameplaySubscriptionDeclaration {
                        subscription_id,
                        event: invocation.input_contract.clone(),
                        invocation_id: invocation.invocation_id.clone(),
                        selector: invocation.selector.clone().ok_or_else(|| {
                            GameplayModuleTopologyError::MissingObserveSubscription(
                                invocation.invocation_id.clone(),
                            )
                        })?,
                        max_deliveries_per_root: invocation.max_deliveries_per_root.ok_or_else(
                            || {
                                GameplayModuleTopologyError::MissingObserveSubscription(
                                    invocation.invocation_id.clone(),
                                )
                            },
                        )?,
                    });
                }
                _ if invocation.subscription_id.is_some()
                    || invocation.selector.is_some()
                    || invocation.max_deliveries_per_root.is_some() =>
                {
                    return Err(GameplayModuleTopologyError::DecisionHasSubscription(
                        invocation.invocation_id,
                    ));
                }
                _ => {}
            }

            let mut request_ids = BTreeSet::new();
            let mut requirements = Vec::new();
            let mut requests = Vec::new();
            for read in invocation.reads {
                if !request_ids.insert(read.request.request_id.clone()) {
                    return Err(GameplayModuleTopologyError::DuplicateReadRequest {
                        invocation_id: invocation.invocation_id.clone(),
                        request_id: read.request.request_id,
                    });
                }
                requirements.push(GameplayInvocationReadRequirement {
                    request_id: read.request.request_id.clone(),
                    view: read.request.view.clone(),
                });
                let requirement = GameplayReadViewRequirement {
                    view: read.request.view.clone(),
                    provider_id: read.provider_id.clone(),
                    kind: read.kind,
                    fields: read.request.fields.clone(),
                    selector_capabilities: read.selector_capabilities.clone(),
                    max_items: read.max_items,
                };
                let registration = GameplayReadViewProviderRegistration {
                    view: read.request.view.clone(),
                    provider_id: read.provider_id,
                    kind: read.kind,
                    fields: read.request.fields.clone(),
                    selector_capabilities: read.selector_capabilities,
                    max_items: read.max_items,
                    ordering: read.ordering,
                };
                let view_key = requirement.view.key();
                if read_views
                    .get(&view_key)
                    .is_some_and(|existing| existing != &requirement)
                    || read_view_providers
                        .get(&view_key)
                        .is_some_and(|existing| existing != &registration)
                {
                    return Err(GameplayModuleTopologyError::ConflictingReadView(view_key));
                }
                read_views.entry(view_key.clone()).or_insert(requirement);
                read_view_providers.entry(view_key).or_insert(registration);
                requests.push(read.request);
            }
            if !requests.is_empty() {
                declared_reads.push(GameplayRuntimeDeclaredReadPlan {
                    module_id: module_id.clone(),
                    invocation_id: invocation.invocation_id.clone(),
                    requests,
                });
            }
            invocations.push(GameplayInvocationDescriptor {
                invocation_id: invocation.invocation_id,
                family: invocation.family,
                input_contract: invocation.input_contract,
                output_contract: invocation.output_contract,
                read_requirements: requirements,
                max_outputs: invocation.max_outputs,
                max_payload_bytes: invocation.max_payload_bytes,
            });
        }

        Ok(Self {
            module_id,
            subscriptions,
            invocations,
            read_views: read_views.into_values().collect(),
            read_view_providers: read_view_providers.into_values().collect(),
            declared_reads,
        })
    }

    pub fn apply_to_manifest(
        &self,
        manifest: &mut GameplayModuleManifest,
    ) -> Result<(), GameplayModuleTopologyError> {
        if manifest.module_ref.module_id != self.module_id {
            return Err(GameplayModuleTopologyError::ModuleMismatch {
                topology_module_id: self.module_id.clone(),
                manifest_module_id: manifest.module_ref.module_id.clone(),
            });
        }
        if !manifest.subscriptions.is_empty()
            || !manifest.invocations.is_empty()
            || !manifest.read_views.is_empty()
        {
            return Err(GameplayModuleTopologyError::ManifestTopologyAlreadyPopulated);
        }
        manifest.subscriptions.clone_from(&self.subscriptions);
        manifest.invocations.clone_from(&self.invocations);
        manifest.read_views.clone_from(&self.read_views);
        Ok(())
    }

    pub fn module_id(&self) -> &str {
        &self.module_id
    }

    pub fn subscriptions(&self) -> &[GameplaySubscriptionDeclaration] {
        &self.subscriptions
    }

    pub fn invocations(&self) -> &[GameplayInvocationDescriptor] {
        &self.invocations
    }

    pub fn read_views(&self) -> &[GameplayReadViewRequirement] {
        &self.read_views
    }

    pub fn read_view_providers(&self) -> &[GameplayReadViewProviderRegistration] {
        &self.read_view_providers
    }

    pub fn declared_reads(&self) -> &[GameplayRuntimeDeclaredReadPlan] {
        &self.declared_reads
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameplayModuleTopologyError {
    DuplicateInvocation(String),
    DuplicateSubscription(String),
    MissingObserveSubscription(String),
    DecisionHasSubscription(String),
    DuplicateReadRequest {
        invocation_id: String,
        request_id: String,
    },
    ConflictingReadView(String),
    ModuleMismatch {
        topology_module_id: String,
        manifest_module_id: String,
    },
    ManifestTopologyAlreadyPopulated,
}

impl core::fmt::Display for GameplayModuleTopologyError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for GameplayModuleTopologyError {}

/// Common scope for stateful Session-local named-view reads.
pub fn gameplay_session_state_read(
    request_id: impl Into<String>,
    view: GameplayContractRef,
    provider_id: impl Into<String>,
    fields: Vec<String>,
    ordering: impl Into<String>,
) -> GameplayModuleReadTopology {
    GameplayModuleReadTopology {
        request: GameplayReadRequest {
            request_id: request_id.into(),
            view,
            fields,
            selector: rule_gameplay_fabric::GameplayReadSelector::ModuleNamed {
                scope: GameplayModuleStateScope::Session,
            },
        },
        provider_id: provider_id.into(),
        kind: GameplayReadViewKind::ModuleNamed,
        selector_capabilities: vec![GameplayReadSelectorCapability::ModuleStateScope],
        max_items: 1,
        ordering: ordering.into(),
    }
}
