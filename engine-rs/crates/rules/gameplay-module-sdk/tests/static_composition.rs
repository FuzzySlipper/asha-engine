use gameplay_module_sdk::*;
use rule_gameplay_fabric::{
    gameplay_module_payload_hash, FrozenGameplayViews, GameplayFabricCoordinator,
    GameplayModuleInitialization, GameplayModuleStateError, GameplayModuleStateStore,
    GameplayOwnerRoutingCall, GameplayOwnerRoutingOutput, GameplayProposalRouter,
    GameplayRuntimeLimits, GameplayViewSource,
};
use serde::{Deserialize, Serialize};

fn contract(namespace: &str, name: &str) -> GameplayContractRef {
    gameplay_contract(namespace, name, 1, &schema_descriptor(namespace, name))
}

fn schema_descriptor(namespace: &str, name: &str) -> String {
    format!("fixture:{namespace}.{name};canonical-json-v1")
}

fn declaration(event: GameplayContractRef) -> GameplayEventSchemaDeclaration {
    GameplayEventSchemaDeclaration {
        codec_id: gameplay_canonical_codec_id(&event.schema_hash),
        event,
    }
}

fn owner(namespace: &str) -> GameplayOwnerRef {
    GameplayOwnerRef {
        owner_id: format!("authority.{namespace}"),
        provider_id: format!("provider.{namespace}"),
    }
}

fn codec<T>(event: GameplayContractRef, codec_id: &str) -> GameplayEventCodecRegistration
where
    T: Serialize + for<'de> Deserialize<'de> + 'static,
{
    let _legacy_codec_label = codec_id;
    GameplayEventCodecRegistration::typed(typed_codec::<T>(event))
}

fn typed_codec<T>(event: GameplayContractRef) -> TypedGameplayEventCodec<T>
where
    T: Serialize + for<'de> Deserialize<'de> + 'static,
{
    let descriptor = schema_descriptor(&event.namespace, &event.name);
    gameplay_serde_json_codec(event, descriptor)
}

fn test_provenance() -> GameplayModuleBuildProvenance {
    GameplayModuleBuildProvenance::from_build_inputs(
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
        &[include_bytes!("static_composition.rs")],
        include_bytes!("../../../../Cargo.lock"),
        &[],
    )
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RootPayload {
    amount: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ResultPayload {
    amount: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CounterConfiguration {
    multiplier: u64,
}

struct CounterAdapter {
    module_id: &'static str,
    state_schema: GameplayContractRef,
    fact_schema: GameplayContractRef,
    view_schema: GameplayContractRef,
    owner: GameplayOwnerRef,
}

impl GameplaySerdeModuleStateAdapter for CounterAdapter {
    type Config = CounterConfiguration;
    type State = u64;
    type Fact = u64;
    type View = ResultPayload;

    fn module_id(&self) -> &str {
        self.module_id
    }

    fn state_schema(&self) -> GameplayContractRef {
        self.state_schema.clone()
    }

    fn fact_schema(&self) -> GameplayContractRef {
        self.fact_schema.clone()
    }

    fn owner(&self) -> GameplayOwnerRef {
        self.owner.clone()
    }

    fn initialize(&self, config: &Self::Config) -> Result<Self::State, String> {
        Ok(config.multiplier)
    }

    fn apply_fact(&self, state: &Self::State, fact: &Self::Fact) -> Result<Self::State, String> {
        Ok(state.saturating_add(*fact))
    }

    fn migrate(&self, _from_version: u32, state: &Self::State) -> Result<Self::State, String> {
        Ok(*state)
    }

    fn view_schema(&self) -> Option<GameplayContractRef> {
        Some(self.view_schema.clone())
    }

    fn project_view(&self, state: &Self::State) -> Result<Self::View, String> {
        Ok(ResultPayload { amount: *state })
    }
}

struct CounterBehavior {
    namespace: &'static str,
    multiplier: u64,
    proposes: bool,
}

impl GameplayModuleBehavior for CounterBehavior {
    fn invoke(
        &self,
        context: &GameplayModuleContext<'_>,
    ) -> Result<GameplayModuleActions, GameplayModuleError> {
        let input: RootPayload = context.event_payload()?;
        let amount = input.amount.saturating_mul(self.multiplier);
        let mut actions = context.actions();
        actions.emit(
            &typed_codec::<ResultPayload>(contract(self.namespace, "result")),
            &ResultPayload { amount },
            context.source(),
            vec![],
            context.target(0).into_iter().collect(),
        )?;
        if self.proposes {
            actions.propose(
                &typed_codec::<ResultPayload>(contract(self.namespace, "shared-delta")),
                &ResultPayload { amount },
                context.source(),
                context.target(0).into_iter().collect(),
            )?;
        }
        actions.record_local_fact_json(
            contract(self.namespace, "counter-fact"),
            contract(self.namespace, "counter-state"),
            GameplayModuleStateScope::Session,
            0,
            &amount,
        )?;
        actions.trace(format!("{}.ran", self.namespace));
        Ok(actions)
    }
}

fn manifest(
    namespace: &'static str,
    root: &GameplayContractRef,
    proposes: bool,
) -> GameplayModuleManifest {
    let module_id = format!("{namespace}.module");
    let provider_id = format!("provider.{namespace}");
    let state_owner = owner(namespace);
    let mut proposal_kinds = Vec::new();
    if proposes {
        proposal_kinds.push(GameplayProposalDeclaration {
            proposal: contract(namespace, "shared-delta"),
            owner: state_owner.clone(),
        });
    }
    let mut manifest = GameplayModuleManifest {
        module_ref: GameplayModuleRef {
            module_id: module_id.clone(),
            namespace: namespace.to_owned(),
            version: "1.0.0".to_owned(),
            sdk_hash: "sha256:gameplay-sdk-v1".to_owned(),
            contract_hash: format!("sha256:{namespace}-contract"),
            artifact_hash: format!("sha256:{namespace}-artifact"),
            provider_id: provider_id.clone(),
        },
        published_events: vec![declaration(contract(namespace, "result"))],
        subscriptions: vec![GameplaySubscriptionDeclaration {
            subscription_id: format!("{namespace}.root.observe"),
            event: root.clone(),
            invocation_id: format!("{namespace}.observe"),
            selector: GameplayHeaderSelector {
                source: None,
                target: None,
                scope: None,
                required_tags: vec![],
            },
            max_deliveries_per_root: 4,
        }],
        invocations: vec![GameplayInvocationDescriptor {
            invocation_id: format!("{namespace}.observe"),
            family: GameplayInvocationFamily::Observe,
            input_contract: root.clone(),
            output_contract: contract(namespace, "result"),
            read_requirements: Vec::new(),
            max_outputs: 3,
            max_payload_bytes: 4_096,
        }],
        read_views: vec![GameplayReadViewRequirement {
            view: contract(namespace, "counter-view"),
            provider_id: provider_id.clone(),
            kind: GameplayReadViewKind::ModuleNamed,
            fields: vec!["amount".to_owned()],
            selector_capabilities: vec![GameplayReadSelectorCapability::ModuleStateScope],
            max_items: 1,
        }],
        proposal_kinds,
        state_schemas: vec![GameplayOwnedSchemaDeclaration {
            schema: contract(namespace, "counter-state"),
            owner: state_owner.clone(),
        }],
        fact_schemas: vec![GameplayOwnedSchemaDeclaration {
            schema: contract(namespace, "counter-fact"),
            owner: state_owner,
        }],
        ordering: Vec::new(),
        budget: GameplayExecutionBudget {
            max_waves: 4,
            max_events_per_root: 16,
            max_proposals_per_root: 8,
            max_invocations_per_root: 16,
            max_payload_bytes_per_root: 16_384,
        },
        deterministic_requirements: vec!["canonical-json".to_owned()],
        source_hash: format!("sha256:{namespace}-source"),
    };
    test_provenance().apply_to_manifest::<CounterBehavior>(&mut manifest);
    manifest
}

fn provider(
    namespace: &'static str,
    root: &GameplayContractRef,
    multiplier: u64,
    proposes: bool,
) -> GameplayStaticModuleProvider {
    provider_with_adapter_view(namespace, root, multiplier, proposes, "counter-view")
}

fn provider_with_adapter_view(
    namespace: &'static str,
    root: &GameplayContractRef,
    multiplier: u64,
    proposes: bool,
    adapter_view_name: &str,
) -> GameplayStaticModuleProvider {
    let manifest = manifest(namespace, root, proposes);
    let owner = owner(namespace);
    let configuration = GameplaySerdeConfiguration::<CounterConfiguration>::new(
        format!("{namespace}.module"),
        contract(namespace, "configuration"),
        vec![GameplayConfigurationFieldMetadata {
            name: "multiplier".to_owned(),
            value_type: "u64".to_owned(),
            required: true,
        }],
    );
    let mut provider = GameplayStaticModuleProvider::linked_from_manifest(
        manifest,
        &test_provenance(),
        CounterBehavior {
            namespace,
            multiplier,
            proposes,
        },
    )
    .event_codec(codec::<ResultPayload>(
        contract(namespace, "result"),
        &format!("codec.{namespace}.result"),
    ))
    .read_view_provider(GameplayReadViewProviderRegistration {
        view: contract(namespace, "counter-view"),
        provider_id: format!("provider.{namespace}"),
        kind: GameplayReadViewKind::ModuleNamed,
        fields: vec!["amount".to_owned()],
        selector_capabilities: vec![GameplayReadSelectorCapability::ModuleStateScope],
        max_items: 1,
        ordering: "singleValue".to_owned(),
    })
    .state_owner(GameplayStateOwnerRegistration {
        schema: contract(namespace, "counter-state"),
        owner: owner.clone(),
    })
    .state_owner(GameplayStateOwnerRegistration {
        schema: contract(namespace, "counter-fact"),
        owner: owner.clone(),
    })
    .state_adapter(gameplay_serde_state_adapter(CounterAdapter {
        module_id: if namespace == "game.alpha" {
            "game.alpha.module"
        } else {
            "game.beta.module"
        },
        state_schema: contract(namespace, "counter-state"),
        fact_schema: contract(namespace, "counter-fact"),
        view_schema: contract(namespace, adapter_view_name),
        owner: owner.clone(),
    }))
    .serde_configuration(configuration);
    if proposes {
        provider = provider
            .proposal_codec(codec::<ResultPayload>(
                contract(namespace, "shared-delta"),
                "proposal",
            ))
            .proposal_owner(GameplayProposalOwnerRegistration {
                proposal: contract(namespace, "shared-delta"),
                owner,
            });
    }
    provider
}

fn standalone_provider(
    namespace: &'static str,
    multiplier: u64,
    proposes: bool,
) -> GameplayStaticModuleProvider {
    standalone_provider_with_adapter_view(namespace, multiplier, proposes, "counter-view")
}

fn standalone_provider_with_adapter_view(
    namespace: &'static str,
    multiplier: u64,
    proposes: bool,
    adapter_view_name: &str,
) -> GameplayStaticModuleProvider {
    let root = contract(namespace, "root");
    let mut provider =
        provider_with_adapter_view(namespace, &root, multiplier, proposes, adapter_view_name);
    provider
        .manifest
        .published_events
        .push(declaration(root.clone()));
    provider.event_codec(codec::<RootPayload>(
        root,
        &format!("codec.{namespace}.root"),
    ))
}

fn composition(alpha_multiplier: u64) -> GameplayStaticComposition {
    let root = contract("game.alpha", "root");
    let mut alpha = provider("game.alpha", &root, alpha_multiplier, true);
    alpha
        .manifest
        .published_events
        .push(declaration(root.clone()));
    alpha = alpha.event_codec(codec::<RootPayload>(root.clone(), "codec.game.alpha.root"));
    let beta = provider("game.beta", &root, 5, false);
    let mut builder = GameplayStaticCompositionBuilder::new();
    builder.add_provider(beta).add_provider(alpha);
    builder.build().unwrap()
}

struct Views;

impl GameplayViewSource for Views {
    fn freeze(&self, _root_id: &str, wave: u32) -> FrozenGameplayViews {
        FrozenGameplayViews {
            epoch: u64::from(wave),
            view_hash: format!("fixture-wave-{wave}"),
        }
    }
}

struct Router;

impl GameplayProposalRouter for Router {
    fn route(&mut self, _call: &GameplayOwnerRoutingCall) -> GameplayOwnerRoutingOutput {
        GameplayOwnerRoutingOutput {
            accepted: true,
            ..GameplayOwnerRoutingOutput::default()
        }
    }
}

fn root_event(registry: &svc_gameplay_fabric::GameplayFabricRegistry) -> GameplayEventEnvelope {
    registry
        .event(
            &contract("game.alpha", "root"),
            &RootPayload { amount: 4 },
            GameplayEventMetadata {
                event_id: "root-event".to_owned(),
                tick: 3,
                root_sequence: 1,
                wave: 0,
                event_sequence: 0,
                phase: GameplayEventPhase::PostCommit,
                emitter: GameplayEmitterRef::Owner {
                    owner_id: "authority.fixture".to_owned(),
                },
                causation: GameplayCausationRef {
                    root_id: "root-1".to_owned(),
                    parent_event_id: None,
                    decision_id: None,
                },
                source: Some(GameplayEntityRef {
                    entity: EntityId::new(1),
                }),
                subjects: vec![],
                targets: vec![GameplayEntityRef {
                    entity: EntityId::new(2),
                }],
                scope: None,
                tags: vec![],
            },
        )
        .expect("typed root event")
}

fn limits() -> GameplayRuntimeLimits {
    GameplayRuntimeLimits {
        max_waves: 4,
        max_events_per_root: 32,
        max_proposals_per_root: 16,
        max_invocations_per_root: 32,
        max_payload_bytes_per_root: 65_536,
    }
}

#[test]
fn two_real_modules_execute_and_behavior_changes_evidence() {
    let first = composition(2);
    assert_eq!(first.registry().readout().module_ids.len(), 2);
    assert_eq!(first.configuration_schemas().len(), 2);
    let receipt = GameplayFabricCoordinator::new(first.registry(), limits()).observe(
        root_event(first.registry()),
        &Views,
        first.invocation_host(),
        &mut Router,
    );
    assert!(receipt.accepted(), "{:?}", receipt.diagnostics);
    assert_eq!(receipt.invocations.len(), 2);
    assert_eq!(receipt.module_facts.len(), 2);
    assert_eq!(receipt.routing.len(), 1);

    let changed = composition(3);
    let changed_receipt = GameplayFabricCoordinator::new(changed.registry(), limits()).observe(
        root_event(changed.registry()),
        &Views,
        changed.invocation_host(),
        &mut Router,
    );
    assert_ne!(receipt.receipt_hash, changed_receipt.receipt_hash);
    assert_ne!(
        receipt.invocations[0].output_hash,
        changed_receipt.invocations[0].output_hash
    );
}

#[test]
fn provider_state_adapters_initialize_and_apply_recorded_local_facts() {
    let parts = composition(2).into_parts();
    let mut state =
        GameplayModuleStateStore::new(parts.registry.clone(), parts.state_adapters).unwrap();
    for namespace in ["game.alpha", "game.beta"] {
        let config = serde_json::to_vec(&CounterConfiguration { multiplier: 10 }).unwrap();
        state
            .initialize_atomic(vec![GameplayModuleInitialization {
                initialization_id: format!("init-{namespace}"),
                module_id: format!("{namespace}.module"),
                state_schema: contract(namespace, "counter-state"),
                scope: GameplayModuleStateScope::Session,
                config_hash: gameplay_module_payload_hash(&config),
                canonical_config: config,
            }])
            .unwrap();
    }
    let receipt = GameplayFabricCoordinator::new(&parts.registry, limits()).observe(
        root_event(&parts.registry),
        &Views,
        &parts.host,
        &mut Router,
    );
    for fact in receipt.module_facts {
        state.apply_fact(fact).unwrap();
    }
    let alpha = state
        .named_view(
            &contract("game.alpha", "counter-state"),
            &GameplayModuleStateScope::Session,
        )
        .unwrap();
    let beta = state
        .named_view(
            &contract("game.beta", "counter-state"),
            &GameplayModuleStateScope::Session,
        )
        .unwrap();
    assert_eq!(
        serde_json::from_slice::<ResultPayload>(&alpha.canonical_payload)
            .unwrap()
            .amount,
        18
    );
    assert_eq!(
        serde_json::from_slice::<ResultPayload>(&beta.canonical_payload)
            .unwrap()
            .amount,
        30
    );
}

#[test]
fn mismatched_link_identity_and_configuration_schema_fail_before_activation() {
    let mut bad_link = standalone_provider("game.alpha", 2, true);
    bad_link.linked_provider.artifact_hash = "sha256:wrong-artifact".to_owned();
    let mut builder = GameplayStaticCompositionBuilder::new();
    builder.add_provider(bad_link);
    assert!(matches!(
        builder.build(),
        Err(GameplayStaticCompositionError::Registry(_))
    ));

    let mut bad_sdk = standalone_provider("game.alpha", 2, true);
    bad_sdk.linked_provider.sdk_hash = "sha256:wrong-sdk".to_owned();
    let mut builder = GameplayStaticCompositionBuilder::new();
    builder.add_provider(bad_sdk);
    assert!(matches!(
        builder.build(),
        Err(GameplayStaticCompositionError::Registry(_))
    ));

    let mut bad_source = standalone_provider("game.alpha", 2, true);
    bad_source.linked_provider.source_hash = "sha256:wrong-source".to_owned();
    let mut builder = GameplayStaticCompositionBuilder::new();
    builder.add_provider(bad_source);
    assert!(matches!(
        builder.build(),
        Err(GameplayStaticCompositionError::Registry(_))
    ));

    let bad_view = standalone_provider_with_adapter_view("game.alpha", 2, true, "wrong-view");
    let mut builder = GameplayStaticCompositionBuilder::new();
    builder.add_provider(bad_view);
    let error = match builder.build() {
        Ok(_) => panic!("view mismatch must fail"),
        Err(error) => error,
    };
    assert!(
        matches!(
            error,
            GameplayStaticCompositionError::StateAdapter(GameplayModuleStateError::UndeclaredView)
        ),
        "unexpected view mismatch error: {error:?}"
    );

    let mut bad_schema = standalone_provider("game.alpha", 2, true);
    bad_schema.configuration_schemas[0].module_id = "game.foreign.module".to_owned();
    let mut builder = GameplayStaticCompositionBuilder::new();
    builder.add_provider(bad_schema);
    assert!(matches!(
        builder.build(),
        Err(GameplayStaticCompositionError::InvalidConfigurationSchema(
            _
        ))
    ));
}

#[test]
fn computed_provenance_changes_with_source_features_lock_and_behavior_type() {
    let root = contract("game.alpha", "root");
    let base = manifest("game.alpha", &root, false);
    let provenance = |source: &'static [u8], lock: &'static [u8], features: &[&str]| {
        GameplayModuleBuildProvenance::from_build_inputs(
            "fixture-package",
            "1.2.3",
            &[source],
            lock,
            features,
        )
    };
    let mut first = base.clone();
    provenance(b"source-a", b"lock-a", &["feature-a"])
        .apply_to_manifest::<CounterBehavior>(&mut first);
    let mut source_changed = base.clone();
    provenance(b"source-b", b"lock-a", &["feature-a"])
        .apply_to_manifest::<CounterBehavior>(&mut source_changed);
    let mut feature_changed = base.clone();
    provenance(b"source-a", b"lock-a", &["feature-b"])
        .apply_to_manifest::<CounterBehavior>(&mut feature_changed);
    let mut lock_changed = base.clone();
    provenance(b"source-a", b"lock-b", &["feature-a"])
        .apply_to_manifest::<CounterBehavior>(&mut lock_changed);
    let mut behavior_changed = base;
    provenance(b"source-a", b"lock-a", &["feature-a"])
        .apply_to_manifest::<CounterAdapter>(&mut behavior_changed);
    let mut toolchain_changed = manifest("game.alpha", &root, false);
    GameplayModuleBuildProvenance::from_build_inputs_with_environment(
        "fixture-package",
        "1.2.3",
        &[b"source-a"],
        b"lock-a",
        &["feature-a"],
        &[("rustc", "1.99.0"), ("target", "fixture-target")],
    )
    .apply_to_manifest::<CounterBehavior>(&mut toolchain_changed);

    assert_eq!(
        first.module_ref.contract_hash,
        source_changed.module_ref.contract_hash
    );
    assert_ne!(first.source_hash, source_changed.source_hash);
    assert_ne!(first.source_hash, feature_changed.source_hash);
    assert_ne!(first.source_hash, lock_changed.source_hash);
    assert_ne!(first.source_hash, toolchain_changed.source_hash);
    assert_eq!(
        first.module_ref.contract_hash,
        toolchain_changed.module_ref.contract_hash
    );
    assert_eq!(
        first.module_ref.sdk_hash,
        toolchain_changed.module_ref.sdk_hash
    );
    assert_ne!(
        first.module_ref.artifact_hash,
        behavior_changed.module_ref.artifact_hash
    );

    let stale_provider = GameplayStaticModuleProvider::linked_from_manifest(
        first,
        &provenance(b"source-b", b"lock-a", &["feature-a"]),
        CounterBehavior {
            namespace: "game.alpha",
            multiplier: 1,
            proposes: false,
        },
    );
    assert_ne!(
        stale_provider.manifest.source_hash,
        stale_provider.linked_provider.source_hash
    );
    let mut builder = GameplayStaticCompositionBuilder::new();
    builder.add_provider(stale_provider);
    let Err(GameplayStaticCompositionError::Registry(error)) = builder.build() else {
        panic!("stale source identity must fail composition");
    };
    assert!(error.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == GameplayRegistryDiagnosticCode::ProviderManifestMismatch
    }));
}
