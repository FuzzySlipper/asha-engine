//! Public-facade provider regression: this crate imports only approved surfaces.

use asha_gameplay_module_sdk::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pulse {
    pub amount: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TriggerReactionProposal {
    pub action: String,
    pub trigger: u64,
    pub subject: u64,
    pub pair_hash: String,
    pub overlap_read_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct PrefabInteractionPulse {
    actor: u64,
    instance: u64,
    prefab: u64,
    role: String,
    target: u64,
    tick: u64,
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

fn typed_codec<T>(event: GameplayContractRef) -> TypedGameplayEventCodec<T>
where
    T: Serialize + for<'de> Deserialize<'de> + 'static,
{
    let descriptor = schema_descriptor(&event.namespace, &event.name);
    gameplay_serde_json_codec(event, descriptor)
}

fn build_provenance() -> GameplayModuleBuildProvenance {
    GameplayModuleBuildProvenance::from_build_inputs(
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
        &[include_bytes!("lib.rs")],
        include_bytes!("../Cargo.lock"),
        &[],
    )
}

fn capability_activation_codec() -> TypedGameplayEventCodec<CapabilityActivationGameplayProposal> {
    let kind = StandardGameplayProposalKind::SetCapabilityActivation;
    let event = kind.contract();
    gameplay_serde_json_codec(event, kind.schema_descriptor())
}

pub struct PrimaryFireDamageBehavior;

impl GameplayModuleBehavior for PrimaryFireDamageBehavior {
    fn invoke(
        &self,
        context: &GameplayModuleContext<'_>,
    ) -> Result<GameplayModuleActions, GameplayModuleError> {
        let mut workspace: PrimaryFireGameplayDecisionWorkspace = context.decision_workspace()?;
        let is_close_hit = workspace.target.is_some()
            && workspace
                .range_millimeters
                .is_some_and(|range| range <= 4_000);
        let mut actions = context.actions();
        if is_close_hit {
            workspace.damage = workspace.damage.saturating_mul(2);
        }
        actions.transform_workspace_json(
            StandardGameplayProposalKind::ResolvePrimaryFire.contract(),
            context
                .decision_workspace_hash()
                .expect("primary-fire Transform receives a Workspace hash"),
            &workspace,
        )?;
        Ok(actions)
    }
}

fn primary_fire_topology() -> GameplayDerivedModuleTopology {
    let proposal = StandardGameplayProposalKind::ResolvePrimaryFire.contract();
    GameplayDerivedModuleTopology::derive(
        "fixture.primary-fire.module",
        vec![GameplayModuleInvocationTopology::decision(
            "fixture.primary-fire.close-range-transform",
            GameplayInvocationFamily::Transform,
            proposal.clone(),
            proposal,
            1,
            4_096,
        )],
    )
    .expect("primary-fire topology is unambiguous")
}

fn primary_fire_provider_with_behavior<B>(behavior: B) -> GameplayStaticModuleProvider
where
    B: GameplayModuleBehavior + 'static,
{
    let proposal = StandardGameplayProposalKind::ResolvePrimaryFire;
    let topology = primary_fire_topology();
    let mut manifest = GameplayModuleManifest {
        module_ref: GameplayModuleRef {
            module_id: "fixture.primary-fire.module".to_owned(),
            namespace: "fixture.primary-fire".to_owned(),
            version: "1.0.0".to_owned(),
            sdk_hash: "sha256:gameplay-sdk-v1".to_owned(),
            contract_hash: "sha256:fixture-primary-fire-contract-v1".to_owned(),
            artifact_hash: "sha256:fixture-primary-fire-artifact-v1".to_owned(),
            provider_id: "provider.fixture-primary-fire".to_owned(),
        },
        published_events: Vec::new(),
        subscriptions: Vec::new(),
        invocations: Vec::new(),
        read_views: Vec::new(),
        proposal_kinds: vec![GameplayProposalDeclaration {
            proposal: proposal.contract(),
            owner: proposal.owner(),
        }],
        state_schemas: Vec::new(),
        fact_schemas: Vec::new(),
        ordering: Vec::new(),
        budget: GameplayExecutionBudget {
            max_waves: 1,
            max_events_per_root: 4,
            max_proposals_per_root: 1,
            max_invocations_per_root: 2,
            max_payload_bytes_per_root: 4_096,
        },
        deterministic_requirements: vec!["canonical-json".to_owned()],
        source_hash: "sha256:fixture-primary-fire-source-v1".to_owned(),
    };
    topology
        .apply_to_manifest(&mut manifest)
        .expect("primary-fire topology belongs to its manifest");
    build_provenance().apply_to_manifest::<B>(&mut manifest);
    GameplayStaticModuleProvider::linked_from_manifest(manifest, &build_provenance(), behavior)
        .derived_topology(&topology)
}

fn primary_fire_provider() -> GameplayStaticModuleProvider {
    primary_fire_provider_with_behavior(PrimaryFireDamageBehavior)
}

pub fn primary_fire_composition() -> GameplayStaticComposition {
    let mut builder = GameplayStaticCompositionBuilder::new();
    builder.include_standard_owner_events();
    builder.add_provider(primary_fire_provider());
    builder.build().expect("primary-fire provider composes")
}

pub struct PulseBehavior {
    pub multiplier: u64,
}

impl GameplayModuleBehavior for PulseBehavior {
    fn invoke(
        &self,
        context: &GameplayModuleContext<'_>,
    ) -> Result<GameplayModuleActions, GameplayModuleError> {
        if context.event_contract() == Some(&StandardGameplayEventKind::TriggerEntered.contract()) {
            let entered: TriggerOverlapGameplayPayload = context.event_payload()?;
            let overlap_read_hash = context
                .read("current-trigger-overlaps")
                .map(|overlaps| match &overlaps.value {
                    GameplayReadValue::OwnerQuery {
                        result:
                            GameplayOwnerQueryResult::CurrentTriggerOverlaps {
                                trigger, subjects, ..
                            },
                    } if *trigger == entered.trigger && subjects.contains(&entered.subject) => {
                        Ok(overlaps.value_hash.clone())
                    }
                    _ => Err(GameplayModuleError {
                        code: "triggerOverlapReadMismatch".to_owned(),
                        message: "frozen trigger overlap read did not contain the accepted pair"
                            .to_owned(),
                    }),
                })
                .transpose()?;
            let mut actions = context.actions();
            actions.emit(
                &typed_codec::<TriggerReactionProposal>(contract("trigger-reaction-proposed")),
                &TriggerReactionProposal {
                    action: "door.open".to_owned(),
                    trigger: entered.trigger,
                    subject: entered.subject,
                    pair_hash: entered.pair_hash,
                    overlap_read_hash,
                },
                Some(entered.trigger),
                vec![entered.subject],
                Vec::new(),
            )?;
            actions.propose(
                &capability_activation_codec(),
                &CapabilityActivationGameplayProposal {
                    entity: entered.trigger,
                    capability: "collision".to_owned(),
                    action: "deactivate".to_owned(),
                },
                Some(entered.subject),
                vec![entered.trigger],
            )?;
            actions.trace("triggerReactionProposed");
            return Ok(actions);
        }
        if context.event_contract()
            == Some(&StandardGameplayEventKind::PrefabPartInteracted.contract())
        {
            let interaction: PrefabInteractionPulse = context.event_payload()?;
            let mut actions = context.actions();
            actions.emit(
                &typed_codec::<Pulse>(contract("pulse-result")),
                &Pulse { amount: 1 },
                Some(interaction.actor),
                vec![interaction.target],
                vec![interaction.target],
            )?;
            actions.record_local_fact_json(
                contract("pulse-fact"),
                contract("pulse-state"),
                GameplayModuleStateScope::Session,
                0,
                &1_u64,
            )?;
            actions.trace("prefabInteractionObserved");
            return Ok(actions);
        }
        let pulse: Pulse = context.event_payload()?;
        let mut actions = context.actions();
        let current_state = context
            .read("pulse-state")
            .map(|_| context.named_view::<u64>("pulse-state"))
            .transpose()?
            .unwrap_or(0);
        let result = pulse
            .amount
            .saturating_mul(self.multiplier)
            .saturating_add(current_state);
        actions.emit(
            &typed_codec::<Pulse>(contract("pulse-result")),
            &Pulse { amount: result },
            context.source(),
            vec![],
            context.target(0).into_iter().collect(),
        )?;
        actions.record_local_fact_json(
            contract("pulse-fact"),
            contract("pulse-state"),
            GameplayModuleStateScope::Session,
            0,
            &result,
        )?;
        Ok(actions)
    }
}

pub struct PulseStateAdapter;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PulseConfiguration {
    multiplier: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    asset_id: Option<String>,
}

impl GameplaySerdeModuleStateAdapter for PulseStateAdapter {
    type Config = PulseConfiguration;
    type State = u64;
    type Fact = u64;
    type View = u64;

    fn module_id(&self) -> &str {
        "fixture.pulse.module"
    }

    fn state_schema(&self) -> GameplayContractRef {
        contract("pulse-state")
    }

    fn fact_schema(&self) -> GameplayContractRef {
        contract("pulse-fact")
    }

    fn owner(&self) -> GameplayOwnerRef {
        GameplayOwnerRef {
            owner_id: "authority.fixture-pulse".to_owned(),
            provider_id: "provider.fixture-pulse".to_owned(),
        }
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
        Some(contract("pulse-state-view"))
    }

    fn project_view(&self, state: &Self::State) -> Result<Self::View, String> {
        Ok(*state)
    }
}

fn pulse_topology() -> GameplayDerivedModuleTopology {
    let selector = GameplayHeaderSelector {
        source: None,
        target: None,
        scope: None,
        required_tags: Vec::new(),
    };
    let pulse = GameplayModuleInvocationTopology::observe(
        "fixture.pulse.observe",
        "fixture.pulse.observe",
        contract("pulse"),
        contract("pulse-result"),
        selector.clone(),
        4,
        2,
        1_024,
    )
    .read(gameplay_session_state_read(
        "pulse-state",
        contract("pulse-state-view"),
        "provider.fixture-pulse",
        vec!["amount".to_owned()],
        "single-module-state",
    ));
    let trigger = GameplayModuleInvocationTopology::observe(
        "fixture.trigger-enter.observe",
        "fixture.trigger-enter.observe",
        StandardGameplayEventKind::TriggerEntered.contract(),
        contract("trigger-reaction-proposed"),
        selector,
        4,
        2,
        1_024,
    )
    .read(GameplayModuleReadTopology {
        request: GameplayReadRequest {
            request_id: "current-trigger-overlaps".to_owned(),
            view: contract("trigger-overlaps-view"),
            fields: vec!["trigger".to_owned(), "subjects".to_owned()],
            selector: GameplayReadSelector::OwnerQuery {
                query: GameplayOwnerQuery::CurrentTriggerOverlaps {
                    trigger: GameplayEventEntityBinding::Source,
                    max_items: 8,
                },
            },
        },
        provider_id: "provider.fixture-trigger-overlaps".to_owned(),
        kind: GameplayReadViewKind::OwnerQuery,
        selector_capabilities: vec![
            GameplayReadSelectorCapability::EventSource,
            GameplayReadSelectorCapability::OwnerQuery,
        ],
        max_items: 8,
        ordering: "entity-id-ascending".to_owned(),
    });
    let prefab_interaction = GameplayModuleInvocationTopology::observe(
        "fixture.prefab-part-interacted.observe",
        "fixture.prefab-part-interacted.observe",
        StandardGameplayEventKind::PrefabPartInteracted.contract(),
        contract("pulse-result"),
        GameplayHeaderSelector {
            source: None,
            target: None,
            scope: None,
            required_tags: Vec::new(),
        },
        4,
        2,
        1_024,
    );
    GameplayDerivedModuleTopology::derive(
        "fixture.pulse.module",
        vec![pulse, trigger, prefab_interaction],
    )
    .expect("fixture topology is unambiguous")
}

fn provider_with_behavior<B>(multiplier: u64, behavior: B) -> GameplayStaticModuleProvider
where
    B: GameplayModuleBehavior + 'static,
{
    let topology = pulse_topology();
    let mut manifest = GameplayModuleManifest {
        module_ref: GameplayModuleRef {
            module_id: "fixture.pulse.module".to_owned(),
            namespace: "fixture.pulse".to_owned(),
            version: "1.0.0".to_owned(),
            sdk_hash: "sha256:gameplay-sdk-v1".to_owned(),
            contract_hash: "sha256:fixture-pulse-contract".to_owned(),
            artifact_hash: format!("sha256:fixture-pulse-behavior-{multiplier}"),
            provider_id: "provider.fixture-pulse".to_owned(),
        },
        published_events: vec![
            declaration(contract("pulse")),
            declaration(contract("pulse-result")),
            declaration(contract("trigger-reaction-proposed")),
        ],
        subscriptions: Vec::new(),
        invocations: Vec::new(),
        read_views: Vec::new(),
        proposal_kinds: vec![GameplayProposalDeclaration {
            proposal: StandardGameplayProposalKind::SetCapabilityActivation.contract(),
            owner: StandardGameplayProposalKind::SetCapabilityActivation.owner(),
        }],
        state_schemas: vec![GameplayOwnedSchemaDeclaration {
            schema: contract("pulse-state"),
            owner: pulse_owner(),
        }],
        fact_schemas: vec![GameplayOwnedSchemaDeclaration {
            schema: contract("pulse-fact"),
            owner: pulse_owner(),
        }],
        ordering: vec![],
        budget: GameplayExecutionBudget {
            max_waves: 2,
            max_events_per_root: 8,
            max_proposals_per_root: 1,
            max_invocations_per_root: 8,
            max_payload_bytes_per_root: 8_192,
        },
        deterministic_requirements: vec!["canonical-json".to_owned()],
        source_hash: format!("sha256:fixture-pulse-source-{multiplier}"),
    };
    topology
        .apply_to_manifest(&mut manifest)
        .expect("Pulse topology belongs to its manifest");
    build_provenance().apply_to_manifest::<B>(&mut manifest);
    let configuration = GameplaySerdeConfiguration::<PulseConfiguration>::new(
        "fixture.pulse.module",
        contract("configuration"),
        vec![
            GameplayConfigurationFieldMetadata {
                name: "multiplier".to_owned(),
                label: "Pulse multiplier".to_owned(),
                value_kind: GameplayConfigurationValueKind::Integer,
                required: true,
                reference_kind: None,
                integer_min: Some(0),
                integer_max: Some(64),
                number_min: None,
                number_max: None,
            },
            GameplayConfigurationFieldMetadata {
                name: "assetId".to_owned(),
                label: "Pulse presentation asset".to_owned(),
                value_kind: GameplayConfigurationValueKind::Reference,
                required: false,
                reference_kind: Some(GameplayConfigurationReferenceKind::Asset),
                integer_min: None,
                integer_max: None,
                number_min: None,
                number_max: None,
            },
        ],
    );
    GameplayStaticModuleProvider::linked_from_manifest(manifest, &build_provenance(), behavior)
        .event_codec(json_codec(contract("pulse")))
        .event_codec(json_codec(contract("pulse-result")))
        .event_codec(gameplay_serde_json_codec_registration::<
            TriggerReactionProposal,
        >(
            contract("trigger-reaction-proposed"),
            schema_descriptor("fixture.pulse", "trigger-reaction-proposed"),
        ))
        .derived_topology(&topology)
        .state_owner(GameplayStateOwnerRegistration {
            schema: contract("pulse-state"),
            owner: pulse_owner(),
        })
        .state_owner(GameplayStateOwnerRegistration {
            schema: contract("pulse-fact"),
            owner: pulse_owner(),
        })
        .state_adapter(gameplay_serde_state_adapter(PulseStateAdapter))
        .serde_configuration(configuration)
}

pub fn provider(multiplier: u64) -> GameplayStaticModuleProvider {
    provider_with_behavior(multiplier, PulseBehavior { multiplier })
}

pub fn root_event(amount: u64) -> GameplayEventEnvelope {
    composition(4)
        .registry()
        .event(
            &contract("pulse"),
            &Pulse { amount },
            GameplayEventMetadata {
                event_id: "fixture-root".to_owned(),
                tick: 0,
                root_sequence: 0,
                wave: 0,
                event_sequence: 0,
                phase: GameplayEventPhase::PostCommit,
                emitter: GameplayEmitterRef::Owner {
                    owner_id: "authority.fixture".to_owned(),
                },
                causation: GameplayCausationRef {
                    root_id: "fixture-root".to_owned(),
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
        .expect("typed pulse root event")
}

pub fn binding_registry(multiplier: u64) -> GameplayModuleBindingRegistry {
    let module = provider(multiplier).manifest.module_ref;
    binding_registry_for_module(module, multiplier)
}

fn binding_registry_for_module(
    module: GameplayModuleRef,
    multiplier: u64,
) -> GameplayModuleBindingRegistry {
    let canonical_config = serde_json::to_vec(&PulseConfiguration {
        multiplier,
        asset_id: None,
    })
    .expect("multiplier serializes");
    let configuration = GameplayModuleConfiguration {
        configuration_id: "fixture.pulse.default".to_owned(),
        module,
        configuration: contract("configuration"),
        codec_id: gameplay_canonical_codec_id(&contract("configuration").schema_hash),
        config_hash: gameplay_module_payload_hash(&canonical_config),
        canonical_config,
    };
    let binding = GameplayModuleBinding {
        binding_id: "fixture.pulse.session".to_owned(),
        module_id: "fixture.pulse.module".to_owned(),
        configuration_id: configuration.configuration_id.clone(),
        state_schema: contract("pulse-state"),
        target: GameplayModuleBindingTarget::Session,
        required_reads: Vec::new(),
        output_contracts: vec![contract("pulse-result")],
        enabled: true,
    };
    let mut builder = GameplayModuleBindingRegistryBuilder::new();
    builder.configuration(configuration).binding(binding);
    builder.build()
}

pub fn composition(multiplier: u64) -> GameplayStaticComposition {
    let mut builder = GameplayStaticCompositionBuilder::new();
    builder.include_standard_owner_events();
    builder.add_provider(provider(multiplier));
    builder.build().expect("public provider composes")
}

pub fn composed_static_composition(multiplier: u64) -> GameplayStaticComposition {
    let mut composition = GameplayStaticCompositionBuilder::new();
    composition.include_standard_owner_events();
    composition.add_provider(primary_fire_provider());
    composition.add_provider(provider(multiplier));
    composition
        .build()
        .expect("composed fixture providers compose")
}

pub fn pulse_state_view_contract() -> GameplayContractRef {
    contract("pulse-state-view")
}

pub fn provider_requirements_json() -> String {
    include_str!("../project/provider-requirements.json").to_owned()
}

pub fn conformance_reachable_surfaces(
) -> Vec<asha_gameplay_module_conformance::GameplayModuleConformanceReachableSurface> {
    vec![
        asha_gameplay_module_conformance::GameplayModuleConformanceReachableSurface::gameplay_module_sdk(),
        asha_gameplay_module_conformance::GameplayModuleConformanceReachableSurface::gameplay_module_conformance(),
    ]
}

pub fn trigger_entered_event(trigger: u64, subject: u64) -> GameplayEventEnvelope {
    let payload = TriggerOverlapGameplayPayload {
        trigger,
        subject,
        action: "enter".to_owned(),
        scope: "zone.exit".to_owned(),
        tags: vec!["door".to_owned(), "exit".to_owned()],
        tick: 7,
        cause: "teleport".to_owned(),
        pair_hash: gameplay_module_payload_hash(format!("{trigger}|{subject}").as_bytes()),
    };
    composition(4)
        .registry()
        .event(
            &StandardGameplayEventKind::TriggerEntered.contract(),
            &payload,
            GameplayEventMetadata {
                event_id: "fixture-trigger-enter".to_owned(),
                tick: 7,
                root_sequence: 2,
                wave: 0,
                event_sequence: 0,
                phase: GameplayEventPhase::PostCommit,
                emitter: GameplayEmitterRef::Owner {
                    owner_id: "rule-trigger-volume".to_owned(),
                },
                causation: GameplayCausationRef {
                    root_id: "fixture-trigger-root".to_owned(),
                    parent_event_id: Some("accepted-trigger-fact".to_owned()),
                    decision_id: None,
                },
                source: Some(GameplayEntityRef {
                    entity: EntityId::new(trigger),
                }),
                subjects: vec![GameplayEntityRef {
                    entity: EntityId::new(subject),
                }],
                targets: Vec::new(),
                scope: Some("zone.exit".to_owned()),
                tags: vec!["door".to_owned(), "enter".to_owned(), "exit".to_owned()],
            },
        )
        .expect("typed trigger root event")
}

fn contract(name: &str) -> GameplayContractRef {
    gameplay_contract(
        "fixture.pulse",
        name,
        1,
        &schema_descriptor("fixture.pulse", name),
    )
}

fn pulse_owner() -> GameplayOwnerRef {
    GameplayOwnerRef {
        owner_id: "authority.fixture-pulse".to_owned(),
        provider_id: "provider.fixture-pulse".to_owned(),
    }
}


fn json_codec(event: GameplayContractRef) -> GameplayEventCodecRegistration {
    GameplayEventCodecRegistration::typed(typed_codec::<Pulse>(event))
}

#[cfg(test)]
mod tests {
    use super::*;
    use asha_gameplay_module_conformance::{
        run_gameplay_module_conformance, GameplayModuleConformanceCase,
        GameplayModuleConformanceNeedsManifest, GameplayModuleConformanceProject,
        GameplayModuleConformanceReachableSurface,
    };

    fn conformance_composition() -> Result<GameplayStaticComposition, GameplayStaticCompositionError>
    {
        Ok(composition(4))
    }

    #[test]
    fn authored_topology_is_the_manifest_source() {
        let topology = pulse_topology();
        let provider = provider(4);
        assert_eq!(provider.manifest.subscriptions, topology.subscriptions());
        assert_eq!(provider.manifest.invocations, topology.invocations());
        assert_eq!(provider.manifest.read_views, topology.read_views());
    }

    fn conformance_project() -> GameplayModuleConformanceProject {
        serde_json::from_str(include_str!("../project/gameplay-project.json")).unwrap()
    }

    #[test]
    fn committed_project_uses_the_linked_provider_identity() {
        assert_eq!(
            conformance_project().gameplay_module_bindings,
            binding_registry(4),
        );
    }

    fn conformance_needs_manifest() -> GameplayModuleConformanceNeedsManifest {
        serde_json::from_str(&provider_requirements_json()).unwrap()
    }

    fn run_project(
        project: GameplayModuleConformanceProject,
        event: GameplayEventEnvelope,
    ) -> asha_gameplay_module_conformance::GameplayModuleConformanceReport {
        run_with(project, event, conformance_composition)
    }

    fn run_with(
        project: GameplayModuleConformanceProject,
        event: GameplayEventEnvelope,
        composition: fn() -> Result<GameplayStaticComposition, GameplayStaticCompositionError>,
    ) -> asha_gameplay_module_conformance::GameplayModuleConformanceReport {
        run_with_inputs(
            project,
            event,
            composition,
            conformance_needs_manifest(),
            conformance_reachable_surfaces(),
        )
    }

    fn run_with_inputs(
        project: GameplayModuleConformanceProject,
        event: GameplayEventEnvelope,
        composition: fn() -> Result<GameplayStaticComposition, GameplayStaticCompositionError>,
        needs_manifest: GameplayModuleConformanceNeedsManifest,
        reachable_surfaces: Vec<GameplayModuleConformanceReachableSurface>,
    ) -> asha_gameplay_module_conformance::GameplayModuleConformanceReport {
        run_gameplay_module_conformance(GameplayModuleConformanceCase {
            project_bundle_json: serde_json::to_string(&project).unwrap(),
            consumer_needs_manifest_json: serde_json::to_string(&needs_manifest).unwrap(),
            reachable_surfaces,
            composition,
            events: vec![event],
        })
        .unwrap()
    }

    #[test]
    fn public_facade_executes_real_downstream_code_and_hashes_behavior() {
        let first = composition(2).observe_session_event(root_event(7));
        let changed = composition(3).observe_session_event(root_event(7));
        assert!(first.accepted(), "{:?}", first.diagnostics);
        assert!(changed.accepted(), "{:?}", changed.diagnostics);
        assert_eq!(first.invocations.len(), 1);
        assert_eq!(first.module_facts.len(), 1);
        assert_ne!(
            first.invocations[0].output_hash,
            changed.invocations[0].output_hash
        );
        assert_ne!(first.receipt_hash, changed.receipt_hash);
    }

    #[test]
    fn downstream_trigger_observer_proposes_visible_follow_up_without_collision_access() {
        let receipt = composition(2).observe_session_event(trigger_entered_event(10, 20));
        assert!(receipt.accepted(), "{:?}", receipt.diagnostics);
        assert_eq!(receipt.invocations.len(), 1);
        assert_eq!(
            receipt.invocations[0].invocation_id,
            "fixture.trigger-enter.observe"
        );
        assert_eq!(receipt.events.len(), 2);
        let proposed = &receipt.events[1];
        assert_eq!(proposed.event, contract("trigger-reaction-proposed"));
        let payload: TriggerReactionProposal =
            serde_json::from_slice(&proposed.canonical_payload).unwrap();
        assert_eq!(payload.action, "door.open");
        assert_eq!(payload.trigger, 10);
        assert_eq!(payload.subject, 20);
        assert_eq!(payload.overlap_read_hash, None);
    }


    #[test]
    fn public_facade_builds_hash_bound_session_configuration() {
        let registry = binding_registry(4);
        assert_eq!(registry.configurations.len(), 1);
        assert_eq!(registry.bindings.len(), 1);
        assert_eq!(
            registry.registry_hash,
            gameplay_module_binding_registry_hash(&registry)
        );
    }

    #[test]
    fn one_command_conformance_covers_bootstrap_invocation_state_and_replay() {
        let project = conformance_project();
        let report = run_gameplay_module_conformance(GameplayModuleConformanceCase {
            project_bundle_json: serde_json::to_string(&project).unwrap(),
            consumer_needs_manifest_json: serde_json::to_string(&conformance_needs_manifest())
                .unwrap(),
            reachable_surfaces: conformance_reachable_surfaces(),
            composition: conformance_composition,
            events: vec![root_event(7), trigger_entered_event(10, 20)],
        })
        .unwrap();
        assert!(report.valid, "{}", report.trace);
        assert_eq!(
            report.module_ids,
            vec!["asha.owner-events", "fixture.pulse.module"]
        );
        assert_eq!(report.reaction_frames.len(), 2);
        assert!(report
            .reaction_frames
            .iter()
            .all(|frame| frame.invocations.len() == 1));
        assert!(report.checks.iter().all(|check| check.passed));
        assert!(report
            .to_pretty_json()
            .unwrap()
            .contains("verificationReplay"));
    }

    #[test]
    fn provider_drift_and_bad_config_fail_before_partial_bootstrap() {
        let mut provider_drift = conformance_project();
        provider_drift.gameplay_module_bindings.configurations[0]
            .module
            .provider_id = "provider.foreign".to_owned();
        provider_drift.gameplay_module_bindings.registry_hash =
            gameplay_module_binding_registry_hash(&provider_drift.gameplay_module_bindings);
        let report = run_project(provider_drift, root_event(7));
        assert!(!report.valid);
        assert!(report.gaps.iter().any(|gap| gap.code == "providerMismatch"));
        assert!(report.initial_state_hash.is_empty());
        assert!(report.final_state_hash.is_empty());

        let mut bad_config = conformance_project();
        bad_config.gameplay_module_bindings.configurations[0].canonical_config =
            b"not-json".to_vec();
        bad_config.gameplay_module_bindings.configurations[0].config_hash =
            gameplay_module_payload_hash(
                &bad_config.gameplay_module_bindings.configurations[0].canonical_config,
            );
        bad_config.gameplay_module_bindings.registry_hash =
            gameplay_module_binding_registry_hash(&bad_config.gameplay_module_bindings);
        let report = run_project(bad_config, root_event(7));
        assert!(!report.valid);
        assert!(report
            .gaps
            .iter()
            .any(|gap| gap.code == "configurationSchemaMismatch"));
        assert!(report.initial_state_hash.is_empty());
        assert!(report.final_state_hash.is_empty());
    }

    #[test]
    fn declared_but_never_invoked_module_cannot_pass() {
        let mut event = root_event(7);
        event.event = contract("unregistered-pulse");
        let report = run_project(conformance_project(), event);
        assert!(!report.valid);
        assert!(report.gaps.iter().any(|gap| gap.code == "unknownEvent"));
        assert!(report
            .checks
            .iter()
            .any(|check| check.id == "actualInvocation" && !check.passed));
    }

    #[test]
    fn valid_but_unmatched_event_cannot_fake_declared_read_delivery() {
        let mut event = root_event(7);
        event.event = StandardGameplayEventKind::TriggerExited.contract();
        let report = run_project(conformance_project(), event);
        assert!(!report.valid);
        assert!(!report.gaps.iter().any(|gap| gap.code == "unknownEvent"));
        assert!(report
            .gaps
            .iter()
            .any(|gap| gap.code == "declaredReadNotDelivered"));
        assert!(report
            .checks
            .iter()
            .any(|check| check.id == "declaredReadDelivery" && !check.passed));
    }

    #[test]
    fn consumer_needs_fail_with_typed_actual_surface_gaps() {
        let cases = [
            (
                "pulse.publish",
                "identity",
                "fixture.pulse.missing.v1",
                "consumerNeedMissingEvent",
            ),
            (
                "pulse.read",
                "provider",
                "provider.missing",
                "consumerNeedProviderMismatch",
            ),
            (
                "pulse.read",
                "fields",
                "missingField",
                "consumerNeedMissingField",
            ),
            (
                "pulse.read",
                "selectors",
                "eventTarget",
                "consumerNeedMissingSelector",
            ),
            (
                "pulse.read",
                "ordering",
                "invented-order",
                "consumerNeedOrderingMismatch",
            ),
            (
                "pulse.proposal",
                "identity",
                "fixture.pulse.missing-proposal.v1",
                "consumerNeedMissingProposal",
            ),
            (
                "pulse.configuration",
                "identity",
                "fixture.pulse.missing-config.v1",
                "consumerNeedMissingBinding",
            ),
            (
                "pulse.configuration",
                "target",
                "invented-scope",
                "consumerNeedBindingTargetMismatch",
            ),
        ];
        for (need_id, field, replacement, expected_code) in cases {
            let mut manifest = conformance_needs_manifest();
            let need = manifest
                .requirements
                .iter_mut()
                .find(|need| need.id == need_id)
                .unwrap();
            match field {
                "identity" => need.identity = replacement.to_owned(),
                "provider" => need.provider = Some(replacement.to_owned()),
                "fields" => need.fields = vec![replacement.to_owned()],
                "selectors" => need.selectors = vec![replacement.to_owned()],
                "ordering" => need.ordering = Some(replacement.to_owned()),
                "target" => need.target.scope = Some(replacement.to_owned()),
                _ => unreachable!(),
            }
            let report = run_with_inputs(
                conformance_project(),
                root_event(7),
                conformance_composition,
                manifest,
                conformance_reachable_surfaces(),
            );
            assert!(
                report.gaps.iter().any(|gap| gap.code == expected_code),
                "expected {expected_code}: {}",
                report.trace
            );
        }

        let report = run_with_inputs(
            conformance_project(),
            root_event(7),
            conformance_composition,
            conformance_needs_manifest(),
            vec![GameplayModuleConformanceReachableSurface::gameplay_module_conformance()],
        );
        assert!(report
            .gaps
            .iter()
            .any(|gap| gap.code == "consumerNeedUnreachableSurface"));
    }

    #[test]
    fn declared_view_that_is_not_deliverable_cannot_pass() {
        let mut project = conformance_project();
        project.declared_reads[0].fields = vec!["undeclared-field".to_owned()];
        let report = run_project(project, root_event(7));
        assert!(!report.valid);
        assert!(report
            .gaps
            .iter()
            .any(|gap| gap.code == "readAssemblyFailed"));
        assert!(report
            .checks
            .iter()
            .any(|check| check.id == "declaredReadDelivery" && !check.passed));
    }

    fn exhausted_composition() -> Result<GameplayStaticComposition, GameplayStaticCompositionError>
    {
        let mut pulse = provider(4);
        pulse.manifest.invocations[0].max_payload_bytes = 1;
        let mut builder = GameplayStaticCompositionBuilder::new();
        builder.include_standard_owner_events();
        builder.add_provider(pulse);
        builder.build()
    }

    #[test]
    fn invocation_budget_exhaustion_cannot_pass() {
        let report = run_with(conformance_project(), root_event(7), exhausted_composition);
        assert!(!report.valid);
        assert!(report
            .gaps
            .iter()
            .any(|gap| gap.code == "invocationOutputBudgetExceeded"));
    }

    fn missing_codec_composition(
    ) -> Result<GameplayStaticComposition, GameplayStaticCompositionError> {
        let mut pulse = provider(4);
        pulse
            .manifest
            .published_events
            .push(GameplayEventSchemaDeclaration {
                event: contract("missing-codec-event"),
                codec_id: "codec.fixture-pulse.missing".to_owned(),
            });
        let mut builder = GameplayStaticCompositionBuilder::new();
        builder.include_standard_owner_events();
        builder.add_provider(pulse);
        builder.build()
    }

    #[test]
    fn missing_codec_is_a_machine_readable_registry_gap() {
        let report = run_with(
            conformance_project(),
            root_event(7),
            missing_codec_composition,
        );
        assert!(!report.valid);
        assert!(report.gaps.iter().any(|gap| gap.code == "missingCodec"));
        assert!(report.initial_state_hash.is_empty());
    }
}
