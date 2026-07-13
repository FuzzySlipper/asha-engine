use gameplay_module_sdk::*;
use rule_gameplay_fabric::{
    gameplay_module_payload_hash, FrozenGameplayViews, GameplayDecisionContinuations,
    GameplayDecisionMoment, GameplayDecisionOwner, GameplayDecisionRoutingOutput,
    GameplayDecisionStatus, GameplayFabricCoordinator, GameplayModuleInitialization,
    GameplayModuleStateError, GameplayModuleStateStore, GameplayOperationWorkspace,
    GameplayOwnerRoutingCall, GameplayOwnerRoutingOutput, GameplayProposalRouter,
    GameplayRuntimeLimits, GameplayViewSource,
};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;

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
        .apply_to_manifest::<RangeWeaponModule>(&mut behavior_changed);

    assert_eq!(
        first.module_ref.contract_hash,
        source_changed.module_ref.contract_hash
    );
    assert_ne!(first.source_hash, source_changed.source_hash);
    assert_ne!(first.source_hash, feature_changed.source_hash);
    assert_ne!(first.source_hash, lock_changed.source_hash);
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

struct RangeWeaponModule {
    manifest: GameRuleModuleManifest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
enum LegacyWeaponWorkspacePayload {
    Request(WeaponEffectHookRequest),
    Proposal(GameExtensionProposal),
}

impl GameRuleModule for RangeWeaponModule {
    fn manifest(&self) -> &GameRuleModuleManifest {
        &self.manifest
    }

    fn evaluate_weapon_effect(
        &self,
        request: &WeaponEffectHookRequest,
    ) -> GameRuleExtensionResult<GameExtensionProposal> {
        let amount_delta = if request.range_millimeters <= 500 {
            -4
        } else {
            -1
        };
        Ok(GameExtensionProposal::DamageModifier {
            proposal_id: format!("{}.range", request.request_id),
            target: request.target.unwrap(),
            channel_id: "value.health".to_owned(),
            amount_delta,
            tags: vec!["range-sensitive".to_owned()],
            proposal_hash: format!("fnv1a64:range-{amount_delta}"),
        })
    }
}

fn legacy_manifest() -> GameRuleModuleManifest {
    GameRuleModuleManifest {
        module_ref: GameRuleModuleRef {
            module_id: "game.weapon.range".to_owned(),
            version: "1.0.0".to_owned(),
            contract_hash: "sha256:legacy-contract".to_owned(),
        },
        declared_hooks: vec![GameRuleHookDeclaration {
            hook_id: "weapon.primary".to_owned(),
            kind: GameExtensionHookKind::WeaponEffect,
            input_contract: "WeaponEffectHookRequest.v0".to_owned(),
            output_contract: "GameExtensionProposal.v0".to_owned(),
            required_capabilities: vec!["health".to_owned()],
        }],
        deterministic_requirements: vec!["no-ambient-random".to_owned()],
        source_hash: "sha256:legacy-source".to_owned(),
    }
}

fn legacy_composition() -> GameplayStaticComposition {
    let workspace = contract("game.weapon", "weapon-effect-workspace");
    let operation_owner = GameplayOwnerRef {
        owner_id: "authority.weapon-effect".to_owned(),
        provider_id: "provider.weapon-effect-owner".to_owned(),
    };
    let mut manifest = GameplayModuleManifest {
        module_ref: GameplayModuleRef {
            module_id: "game.weapon.range".to_owned(),
            namespace: "game.weapon".to_owned(),
            version: "1.0.0".to_owned(),
            sdk_hash: "sha256:gameplay-sdk-v1".to_owned(),
            contract_hash: "sha256:legacy-contract".to_owned(),
            artifact_hash: "sha256:range-behavior".to_owned(),
            provider_id: "provider.game-weapon-range".to_owned(),
        },
        published_events: vec![],
        subscriptions: vec![],
        invocations: vec![GameplayInvocationDescriptor {
            invocation_id: "game.weapon.transform".to_owned(),
            family: GameplayInvocationFamily::Transform,
            input_contract: workspace.clone(),
            output_contract: workspace.clone(),
            read_requirements: Vec::new(),
            max_outputs: 1,
            max_payload_bytes: 4_096,
        }],
        read_views: vec![],
        proposal_kinds: vec![GameplayProposalDeclaration {
            proposal: workspace.clone(),
            owner: operation_owner.clone(),
        }],
        state_schemas: vec![],
        fact_schemas: vec![],
        ordering: vec![],
        budget: GameplayExecutionBudget {
            max_waves: 3,
            max_events_per_root: 4,
            max_proposals_per_root: 4,
            max_invocations_per_root: 4,
            max_payload_bytes_per_root: 8_192,
        },
        deterministic_requirements: vec!["canonical-json".to_owned()],
        source_hash: "sha256:range-source".to_owned(),
    };
    test_provenance()
        .apply_to_manifest::<LegacyWeaponEffectTransformBehavior<RangeWeaponModule>>(&mut manifest);
    let provider = GameplayStaticModuleProvider::linked_from_manifest(
        manifest,
        &test_provenance(),
        LegacyWeaponEffectTransformBehavior::new(RangeWeaponModule {
            manifest: legacy_manifest(),
        }),
    )
    .proposal_codec(codec::<LegacyWeaponWorkspacePayload>(
        workspace.clone(),
        "legacy-workspace-proposal",
    ))
    .proposal_owner(GameplayProposalOwnerRegistration {
        proposal: workspace,
        owner: operation_owner,
    });
    let mut builder = GameplayStaticCompositionBuilder::new();
    builder.add_provider(provider);
    builder.build().unwrap()
}

struct WeaponOwner {
    amount: RefCell<Option<i64>>,
}

impl GameplayDecisionOwner for WeaponOwner {
    fn revision_hash(&self, _owner: &GameplayOwnerRef) -> String {
        "revision-1".to_owned()
    }

    fn route_precommit(
        &mut self,
        call: &GameplayOwnerRoutingCall,
    ) -> GameplayDecisionRoutingOutput {
        let proposal: GameExtensionProposal =
            serde_json::from_slice(&call.proposal.canonical_payload).unwrap();
        let GameExtensionProposal::DamageModifier { amount_delta, .. } = proposal else {
            panic!("expected damage modifier")
        };
        self.amount.replace(Some(amount_delta));
        GameplayDecisionRoutingOutput {
            accepted: true,
            ..GameplayDecisionRoutingOutput::default()
        }
    }
}

fn weapon_moment(range_millimeters: u32) -> GameplayDecisionMoment {
    let workspace_contract = contract("game.weapon", "weapon-effect-workspace");
    let request = WeaponEffectHookRequest {
        module_ref: legacy_manifest().module_ref,
        hook_id: "weapon.primary".to_owned(),
        request_id: format!("range-{range_millimeters}"),
        tick: 4,
        source: EntityId::new(1),
        target: Some(EntityId::new(2)),
        base_damage: -8,
        range_millimeters,
        tags: vec!["primary-fire".to_owned()],
        input_hash: format!("fnv1a64:input-{range_millimeters}"),
    };
    let workspace_payload = serde_json::to_vec(&request).unwrap();
    GameplayDecisionMoment {
        decision_id: format!("weapon-{range_millimeters}"),
        operation: GameplayProposalEnvelope {
            proposal_id: format!("weapon-{range_millimeters}"),
            proposal: workspace_contract.clone(),
            tick: 4,
            root_sequence: 1,
            wave: 0,
            proposal_sequence: 0,
            emitter: GameplayEmitterRef::Owner {
                owner_id: "authority.combat".to_owned(),
            },
            causation: GameplayCausationRef {
                root_id: "weapon-root".to_owned(),
                parent_event_id: None,
                decision_id: Some(format!("weapon-{range_millimeters}")),
            },
            originating_event_id: None,
            source: Some(GameplayEntityRef {
                entity: EntityId::new(1),
            }),
            targets: vec![GameplayEntityRef {
                entity: EntityId::new(2),
            }],
            payload_hash: gameplay_canonical_payload_hash(&workspace_payload),
            canonical_payload: workspace_payload.clone(),
        },
        expected_owner_revision: "revision-1".to_owned(),
        workspace: GameplayOperationWorkspace::from_payload(workspace_contract, workspace_payload),
        resume_token: None,
    }
}

#[test]
fn legacy_weapon_compatibility_executes_real_range_sensitive_module() {
    let composition = legacy_composition();
    let coordinator = GameplayFabricCoordinator::new(composition.registry(), limits());
    let mut close_owner = WeaponOwner {
        amount: RefCell::new(None),
    };
    let close = coordinator.decide(
        weapon_moment(400),
        &mut GameplayDecisionContinuations::default(),
        &Views,
        composition.invocation_host(),
        &mut close_owner,
    );
    let mut far_owner = WeaponOwner {
        amount: RefCell::new(None),
    };
    let far = coordinator.decide(
        weapon_moment(1_200),
        &mut GameplayDecisionContinuations::default(),
        &Views,
        composition.invocation_host(),
        &mut far_owner,
    );
    assert_eq!(
        close.status,
        GameplayDecisionStatus::Accepted,
        "{:?}",
        close.diagnostics
    );
    assert_eq!(
        far.status,
        GameplayDecisionStatus::Accepted,
        "{:?}",
        far.diagnostics
    );
    assert_eq!(*close_owner.amount.borrow(), Some(-4));
    assert_eq!(*far_owner.amount.borrow(), Some(-1));
    assert_ne!(close.final_workspace_hash, far.final_workspace_hash);
}
