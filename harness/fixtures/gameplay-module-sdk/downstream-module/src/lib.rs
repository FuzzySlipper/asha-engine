//! Downstream-shaped proof: this crate imports only the approved public facade.

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

fn schema_descriptor(namespace: &str, name: &str) -> String {
    format!("fixture:{namespace}.{name};canonical-json-v1")
}

fn declaration(event: GameplayContractRef) -> GameplayEventSchemaDeclaration {
    GameplayEventSchemaDeclaration {
        codec_id: gameplay_canonical_codec_id(&event.schema_hash),
        event,
    }
}

fn typed_json_codec<T>(event: GameplayContractRef) -> TypedGameplayEventCodec<T>
where
    T: Serialize + for<'de> Deserialize<'de> + 'static,
{
    let descriptor = schema_descriptor(&event.namespace, &event.name);
    TypedGameplayEventCodec::new(
        declaration(event),
        descriptor,
        |payload: &T| serde_json::to_vec(payload).map_err(|error| error.to_string()),
        |bytes| serde_json::from_slice(bytes).map_err(|error| error.to_string()),
    )
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
    TypedGameplayEventCodec::new(
        GameplayEventSchemaDeclaration {
            codec_id: gameplay_canonical_codec_id(&event.schema_hash),
            event,
        },
        kind.schema_descriptor(),
        |payload| serde_json::to_vec(payload).map_err(|error| error.to_string()),
        |bytes| serde_json::from_slice(bytes).map_err(|error| error.to_string()),
    )
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
                &typed_json_codec::<TriggerReactionProposal>(contract("trigger-reaction-proposed")),
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
            &typed_json_codec::<Pulse>(contract("pulse-result")),
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
}

impl GameplayTypedModuleStateAdapter for PulseStateAdapter {
    type Config = PulseConfiguration;
    type State = u64;
    type Fact = u64;
    type View = u64;

    fn module_id(&self) -> &str {
        "fixture.pulse.module"
    }

    fn state_schema(&self) -> &GameplayContractRef {
        static VALUE: std::sync::OnceLock<GameplayContractRef> = std::sync::OnceLock::new();
        VALUE.get_or_init(|| contract("pulse-state"))
    }

    fn fact_schema(&self) -> &GameplayContractRef {
        static VALUE: std::sync::OnceLock<GameplayContractRef> = std::sync::OnceLock::new();
        VALUE.get_or_init(|| contract("pulse-fact"))
    }

    fn owner(&self) -> &GameplayOwnerRef {
        static VALUE: std::sync::OnceLock<GameplayOwnerRef> = std::sync::OnceLock::new();
        VALUE.get_or_init(|| GameplayOwnerRef {
            owner_id: "authority.fixture-pulse".to_owned(),
            provider_id: "provider.fixture-pulse".to_owned(),
        })
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
        Ok(config.multiplier)
    }

    fn apply_fact(&self, state: &Self::State, fact: &Self::Fact) -> Result<Self::State, String> {
        Ok(state.saturating_add(*fact))
    }

    fn migrate(&self, _from_version: u32, state: &Self::State) -> Result<Self::State, String> {
        Ok(*state)
    }

    fn view_schema(&self) -> Option<&GameplayContractRef> {
        static VALUE: std::sync::OnceLock<GameplayContractRef> = std::sync::OnceLock::new();
        Some(VALUE.get_or_init(|| contract("pulse-state-view")))
    }

    fn project_view(&self, state: &Self::State) -> Result<Self::View, String> {
        Ok(*state)
    }

    fn encode_view(&self, view: &Self::View) -> Result<Vec<u8>, String> {
        serde_json::to_vec(view).map_err(|error| error.to_string())
    }
}

pub fn provider(multiplier: u64) -> GameplayStaticModuleProvider {
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
        subscriptions: vec![
            GameplaySubscriptionDeclaration {
                subscription_id: "fixture.pulse.observe".to_owned(),
                event: contract("pulse"),
                invocation_id: "fixture.pulse.observe".to_owned(),
                selector: GameplayHeaderSelector {
                    source: None,
                    target: None,
                    scope: None,
                    required_tags: vec![],
                },
                max_deliveries_per_root: 4,
            },
            GameplaySubscriptionDeclaration {
                subscription_id: "fixture.trigger-enter.observe".to_owned(),
                event: StandardGameplayEventKind::TriggerEntered.contract(),
                invocation_id: "fixture.trigger-enter.observe".to_owned(),
                selector: GameplayHeaderSelector {
                    source: None,
                    target: None,
                    scope: None,
                    required_tags: vec![],
                },
                max_deliveries_per_root: 4,
            },
        ],
        invocations: vec![
            GameplayInvocationDescriptor {
                invocation_id: "fixture.pulse.observe".to_owned(),
                family: GameplayInvocationFamily::Observe,
                input_contract: contract("pulse"),
                output_contract: contract("pulse-result"),
                read_requirements: vec![GameplayInvocationReadRequirement {
                    request_id: "pulse-state".to_owned(),
                    view: contract("pulse-state-view"),
                }],
                max_outputs: 2,
                max_payload_bytes: 1_024,
            },
            GameplayInvocationDescriptor {
                invocation_id: "fixture.trigger-enter.observe".to_owned(),
                family: GameplayInvocationFamily::Observe,
                input_contract: StandardGameplayEventKind::TriggerEntered.contract(),
                output_contract: contract("trigger-reaction-proposed"),
                read_requirements: vec![GameplayInvocationReadRequirement {
                    request_id: "current-trigger-overlaps".to_owned(),
                    view: contract("trigger-overlaps-view"),
                }],
                max_outputs: 2,
                max_payload_bytes: 1_024,
            },
        ],
        read_views: vec![
            GameplayReadViewRequirement {
                view: contract("pulse-state-view"),
                provider_id: "provider.fixture-pulse".to_owned(),
                kind: GameplayReadViewKind::ModuleNamed,
                fields: vec!["amount".to_owned()],
                selector_capabilities: vec![GameplayReadSelectorCapability::ModuleStateScope],
                max_items: 1,
            },
            GameplayReadViewRequirement {
                view: contract("trigger-overlaps-view"),
                provider_id: "provider.fixture-trigger-overlaps".to_owned(),
                kind: GameplayReadViewKind::OwnerQuery,
                fields: vec!["trigger".to_owned(), "subjects".to_owned()],
                selector_capabilities: vec![
                    GameplayReadSelectorCapability::EventSource,
                    GameplayReadSelectorCapability::OwnerQuery,
                ],
                max_items: 8,
            },
        ],
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
    build_provenance().apply_to_manifest::<PulseBehavior>(&mut manifest);
    let configuration_metadata = GameplayConfigurationSchemaMetadata {
        module_id: "fixture.pulse.module".to_owned(),
        configuration: contract("configuration"),
        codec_id: "codec.fixture-pulse.configuration".to_owned(),
        fields: vec![GameplayConfigurationFieldMetadata {
            name: "multiplier".to_owned(),
            value_type: "u64".to_owned(),
            required: true,
        }],
    };
    GameplayStaticModuleProvider::linked_from_manifest(
        manifest,
        &build_provenance(),
        PulseBehavior { multiplier },
    )
    .event_codec(json_codec(contract("pulse"), "codec.fixture-pulse.pulse"))
    .event_codec(json_codec(
        contract("pulse-result"),
        "codec.fixture-pulse.result",
    ))
    .event_codec(GameplayEventCodecRegistration::typed(typed_json_codec::<
        TriggerReactionProposal,
    >(contract(
        "trigger-reaction-proposed",
    ))))
    .read_view_provider(GameplayReadViewProviderRegistration {
        view: contract("pulse-state-view"),
        provider_id: "provider.fixture-pulse".to_owned(),
        kind: GameplayReadViewKind::ModuleNamed,
        fields: vec!["amount".to_owned()],
        selector_capabilities: vec![GameplayReadSelectorCapability::ModuleStateScope],
        max_items: 1,
        ordering: "single-module-state".to_owned(),
    })
    .read_view_provider(GameplayReadViewProviderRegistration {
        view: contract("trigger-overlaps-view"),
        provider_id: "provider.fixture-trigger-overlaps".to_owned(),
        kind: GameplayReadViewKind::OwnerQuery,
        fields: vec!["trigger".to_owned(), "subjects".to_owned()],
        selector_capabilities: vec![
            GameplayReadSelectorCapability::EventSource,
            GameplayReadSelectorCapability::OwnerQuery,
        ],
        max_items: 8,
        ordering: "entity-id-ascending".to_owned(),
    })
    .state_owner(GameplayStateOwnerRegistration {
        schema: contract("pulse-state"),
        owner: pulse_owner(),
    })
    .state_owner(GameplayStateOwnerRegistration {
        schema: contract("pulse-fact"),
        owner: pulse_owner(),
    })
    .state_adapter(GameplayModuleStateRegistration::typed(PulseStateAdapter))
    .configuration_schema(configuration_metadata.clone())
    .configuration_codec(GameplayConfigurationCodecRegistration::typed::<
        PulseConfiguration,
    >(configuration_metadata))
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
    let canonical_config =
        serde_json::to_vec(&PulseConfiguration { multiplier }).expect("multiplier serializes");
    let configuration = GameplayModuleConfiguration {
        configuration_id: "fixture.pulse.default".to_owned(),
        module,
        configuration: contract("configuration"),
        codec_id: "codec.fixture-pulse.configuration".to_owned(),
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

pub fn conformance_needs_manifest_json() -> String {
    include_str!("../../../../consumer-needs/manifests/gameplay-module-fixture.json").to_owned()
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

#[cfg(test)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct DecisionWorkspace {
    amount: u64,
    transformed: bool,
}

#[cfg(test)]
struct FixtureDecisionBehavior;

#[cfg(test)]
impl GameplayModuleBehavior for FixtureDecisionBehavior {
    fn invoke(
        &self,
        context: &GameplayModuleContext<'_>,
    ) -> Result<GameplayModuleActions, GameplayModuleError> {
        let mut workspace: DecisionWorkspace = context.decision_workspace()?;
        let mut actions = context.actions();
        match context.invocation_id() {
            "fixture.decision.transform" => {
                if context.read("decision-target-collision").is_none() {
                    return Err(GameplayModuleError {
                        code: "missingDecisionRead".to_owned(),
                        message: "decision target collision read was not delivered".to_owned(),
                    });
                }
                if !workspace.transformed {
                    workspace.amount = workspace.amount.saturating_add(2);
                    workspace.transformed = true;
                }
                actions.transform_workspace_json(
                    decision_contract("workspace"),
                    context
                        .decision_workspace_hash()
                        .expect("decision Workspace hash"),
                    &workspace,
                )?;
            }
            "fixture.decision.react" if context.decision_resume_token().is_none() => {
                actions.react(
                    GameplayReactionDisposition::Suspend {
                        token: "fixture-public-reaction".to_owned(),
                    },
                    None,
                );
            }
            "fixture.decision.react" => {
                actions.react(GameplayReactionDisposition::Continue, None);
            }
            _ => {
                return Err(GameplayModuleError {
                    code: "unexpectedDecisionInvocation".to_owned(),
                    message: context.invocation_id().to_owned(),
                });
            }
        }
        Ok(actions)
    }
}

#[cfg(test)]
fn decision_contract(name: &str) -> GameplayContractRef {
    gameplay_contract(
        "fixture.decision",
        name,
        1,
        &schema_descriptor("fixture.decision", name),
    )
}

#[cfg(test)]
fn decision_owner() -> GameplayOwnerRef {
    GameplayOwnerRef {
        owner_id: "authority.fixture-decision".to_owned(),
        provider_id: "provider.fixture-decision".to_owned(),
    }
}

#[cfg(test)]
fn decision_provider() -> GameplayStaticModuleProvider {
    let proposal = decision_contract("operation");
    let workspace = decision_contract("workspace");
    let view = decision_contract("target-collision-view");
    let owner = decision_owner();
    let mut manifest = GameplayModuleManifest {
        module_ref: GameplayModuleRef {
            module_id: "fixture.decision.module".to_owned(),
            namespace: "fixture.decision".to_owned(),
            version: "1.0.0".to_owned(),
            sdk_hash: "sha256:gameplay-sdk-v1".to_owned(),
            contract_hash: "sha256:fixture-decision-contract".to_owned(),
            artifact_hash: "sha256:fixture-decision-artifact".to_owned(),
            provider_id: "provider.fixture-decision".to_owned(),
        },
        published_events: Vec::new(),
        subscriptions: Vec::new(),
        invocations: vec![
            GameplayInvocationDescriptor {
                invocation_id: "fixture.decision.transform".to_owned(),
                family: GameplayInvocationFamily::Transform,
                input_contract: proposal.clone(),
                output_contract: workspace.clone(),
                read_requirements: vec![GameplayInvocationReadRequirement {
                    request_id: "decision-target-collision".to_owned(),
                    view: view.clone(),
                }],
                max_outputs: 1,
                max_payload_bytes: 4_096,
            },
            GameplayInvocationDescriptor {
                invocation_id: "fixture.decision.react".to_owned(),
                family: GameplayInvocationFamily::React,
                input_contract: proposal.clone(),
                output_contract: workspace,
                read_requirements: Vec::new(),
                max_outputs: 1,
                max_payload_bytes: 4_096,
            },
        ],
        read_views: vec![GameplayReadViewRequirement {
            view: view.clone(),
            provider_id: "provider.fixture-decision".to_owned(),
            kind: GameplayReadViewKind::EntityCapability,
            fields: vec!["staticCollider".to_owned()],
            selector_capabilities: vec![
                GameplayReadSelectorCapability::EventTarget,
                GameplayReadSelectorCapability::CollisionCapability,
            ],
            max_items: 1,
        }],
        proposal_kinds: vec![GameplayProposalDeclaration {
            proposal: proposal.clone(),
            owner: owner.clone(),
        }],
        state_schemas: Vec::new(),
        fact_schemas: Vec::new(),
        ordering: Vec::new(),
        budget: GameplayExecutionBudget {
            max_waves: 4,
            max_events_per_root: 8,
            max_proposals_per_root: 4,
            max_invocations_per_root: 12,
            max_payload_bytes_per_root: 16_384,
        },
        deterministic_requirements: vec!["canonical-json".to_owned()],
        source_hash: "sha256:fixture-decision-source".to_owned(),
    };
    build_provenance().apply_to_manifest::<FixtureDecisionBehavior>(&mut manifest);
    GameplayStaticModuleProvider::linked_from_manifest(
        manifest,
        &build_provenance(),
        FixtureDecisionBehavior,
    )
    .proposal_codec(GameplayEventCodecRegistration::typed(typed_json_codec::<
        DecisionWorkspace,
    >(
        proposal.clone()
    )))
    .proposal_owner(GameplayProposalOwnerRegistration { proposal, owner })
    .read_view_provider(GameplayReadViewProviderRegistration {
        view,
        provider_id: "provider.fixture-decision".to_owned(),
        kind: GameplayReadViewKind::EntityCapability,
        fields: vec!["staticCollider".to_owned()],
        selector_capabilities: vec![
            GameplayReadSelectorCapability::EventTarget,
            GameplayReadSelectorCapability::CollisionCapability,
        ],
        max_items: 1,
        ordering: "entityIdAscending".to_owned(),
    })
}

#[cfg(test)]
fn decision_composition() -> GameplayStaticComposition {
    let mut builder = GameplayStaticCompositionBuilder::new();
    builder.add_provider(decision_provider());
    builder.build().expect("public decision composition")
}

fn json_codec(event: GameplayContractRef, codec_id: &str) -> GameplayEventCodecRegistration {
    let _legacy_codec_label = codec_id;
    GameplayEventCodecRegistration::typed(typed_json_codec::<Pulse>(event))
}

#[cfg(test)]
mod tests {
    use super::*;
    use asha_gameplay_module_conformance::{
        run_gameplay_module_conformance, GameplayModuleConformanceCase,
        GameplayModuleConformanceNeedsManifest, GameplayModuleConformanceProject,
        GameplayModuleConformanceReachableSurface,
    };
    use asha_gameplay_runtime_host::{
        BundleArtifacts, GameplayBindingEntityTargets, GameplayDecisionMoment,
        GameplayDecisionStatus, GameplayOperationWorkspace, GameplayRuntimeDecisionOwner,
        GameplayRuntimeDecisionOwnerOutput, GameplayRuntimeDeclaredReadPlan, GameplayRuntimeHost,
        GameplayRuntimeProjectInput, GameplayRuntimeSchedulerCommand,
        GameplayRuntimeSchedulerDefinition, GameplayRuntimeSpatialEntity,
        GameplayTriggerDefinition, LoadPlan, LoadStep, RuntimeSessionId, SceneId,
        ScheduledActionId, ScheduledActionValidity, TickScheduledActionDraft,
        TriggerReconcileCause, GAMEPLAY_TRIGGER_DEFINITION_SCHEMA_VERSION,
    };

    fn conformance_composition() -> Result<GameplayStaticComposition, GameplayStaticCompositionError>
    {
        Ok(composition(4))
    }

    fn conformance_project() -> GameplayModuleConformanceProject {
        serde_json::from_str(include_str!("../project/gameplay-project.json")).unwrap()
    }

    fn conformance_needs_manifest() -> GameplayModuleConformanceNeedsManifest {
        serde_json::from_str(&conformance_needs_manifest_json()).unwrap()
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

    fn runtime_host_project_input(multiplier: u64) -> GameplayRuntimeProjectInput {
        let plan = LoadPlan {
            steps: vec![
                LoadStep::ValidateVersions {
                    bundle_schema_version: 1,
                    protocol_version: 1,
                },
                LoadStep::LoadAssetLock {
                    artifact: "assets/lock.json".to_owned(),
                    asset_count: 0,
                },
                LoadStep::LoadSceneDocument {
                    artifact: "scene/scene.json".to_owned(),
                    scene: SceneId::new(1),
                },
                LoadStep::BootstrapScene {
                    scene: SceneId::new(1),
                    runtime_session: RuntimeSessionId::new(1),
                },
                LoadStep::ValidateFinalState,
            ],
        };
        let scene = r#"{
  "schemaVersion": 1,
  "id": 1,
  "metadata": { "name": "downstream-host", "authoringFormatVersion": 1 },
  "dependencies": [],
  "nodes": [
    { "id": 1, "parent": null, "childOrder": 0, "label": null, "tags": [], "transform": { "translation": [0, 0, 0], "rotation": [0, 0, 0, 1], "scale": [1, 1, 1] }, "kind": { "kind": "emptyGroup" } }
  ]
}"#;
        let artifacts = BundleArtifacts::new()
            .with_artifact("assets/lock.json", "{ \"entries\": [] }\n")
            .with_artifact("scene/scene.json", scene);
        GameplayRuntimeProjectInput {
            load_plan: plan,
            artifacts,
            composition: composition(multiplier),
            bindings: binding_registry(multiplier),
            entity_targets: GameplayBindingEntityTargets::new(),
            spatial_entities: vec![
                GameplayRuntimeSpatialEntity {
                    entity: EntityId::new(10),
                    translation: [0.0, 0.0, 0.0],
                    half_extents: [0.5, 0.5, 0.5],
                    static_collider: false,
                },
                GameplayRuntimeSpatialEntity {
                    entity: EntityId::new(20),
                    translation: [2.0, 0.0, 0.0],
                    half_extents: [0.5, 0.5, 0.5],
                    static_collider: false,
                },
            ],
            declared_reads: vec![
                GameplayRuntimeDeclaredReadPlan {
                    module_id: "fixture.pulse.module".to_owned(),
                    invocation_id: "fixture.pulse.observe".to_owned(),
                    requests: vec![GameplayReadRequest {
                        request_id: "pulse-state".to_owned(),
                        view: contract("pulse-state-view"),
                        fields: vec!["amount".to_owned()],
                        selector: GameplayReadSelector::ModuleNamed {
                            scope: GameplayModuleStateScope::Session,
                        },
                    }],
                },
                GameplayRuntimeDeclaredReadPlan {
                    module_id: "fixture.pulse.module".to_owned(),
                    invocation_id: "fixture.trigger-enter.observe".to_owned(),
                    requests: vec![GameplayReadRequest {
                        request_id: "current-trigger-overlaps".to_owned(),
                        view: contract("trigger-overlaps-view"),
                        fields: vec!["trigger".to_owned(), "subjects".to_owned()],
                        selector: GameplayReadSelector::OwnerQuery {
                            query: GameplayOwnerQuery::CurrentTriggerOverlaps {
                                trigger: GameplayEventEntityBinding::Source,
                                max_items: 8,
                            },
                        },
                    }],
                },
            ],
            triggers: vec![GameplayTriggerDefinition {
                schema_version: GAMEPLAY_TRIGGER_DEFINITION_SCHEMA_VERSION,
                entity: 10,
                scope: "zone.exit".to_owned(),
                tags: vec!["door".to_owned(), "exit".to_owned()],
            }],
            scheduler: GameplayRuntimeSchedulerDefinition::new(
                GameplayOwnerRef {
                    owner_id: "authority.fixture-scheduler".to_owned(),
                    provider_id: "provider.fixture-scheduler".to_owned(),
                },
                Vec::new(),
                vec![StandardGameplayProposalKind::SetCapabilityActivation.contract()],
            ),
        }
    }

    fn decision_runtime_host_project_input() -> GameplayRuntimeProjectInput {
        let mut input = runtime_host_project_input(4);
        input.composition = decision_composition();
        input.bindings = GameplayModuleBindingRegistryBuilder::new().build();
        input.declared_reads = vec![GameplayRuntimeDeclaredReadPlan {
            module_id: "fixture.decision.module".to_owned(),
            invocation_id: "fixture.decision.transform".to_owned(),
            requests: vec![GameplayReadRequest {
                request_id: "decision-target-collision".to_owned(),
                view: decision_contract("target-collision-view"),
                fields: vec!["staticCollider".to_owned()],
                selector: GameplayReadSelector::Capability {
                    binding: GameplayEventEntityBinding::Target { index: 0 },
                    capability: GameplayCapabilityReadKind::Collision,
                },
            }],
        }];
        input.triggers = Vec::new();
        input.scheduler = GameplayRuntimeSchedulerDefinition::new(
            GameplayOwnerRef {
                owner_id: "authority.fixture-scheduler".to_owned(),
                provider_id: "provider.fixture-scheduler".to_owned(),
            },
            Vec::new(),
            vec![decision_contract("operation")],
        );
        input
    }

    fn public_decision_moment(decision_id: &str) -> GameplayDecisionMoment {
        let payload = serde_json::to_vec(&DecisionWorkspace {
            amount: 4,
            transformed: false,
        })
        .unwrap();
        GameplayDecisionMoment {
            decision_id: decision_id.to_owned(),
            operation: GameplayProposalEnvelope {
                proposal_id: format!("proposal-{decision_id}"),
                proposal: decision_contract("operation"),
                tick: 1,
                root_sequence: 1,
                wave: 0,
                proposal_sequence: 0,
                emitter: GameplayEmitterRef::Owner {
                    owner_id: "fixture.consumer".to_owned(),
                },
                causation: GameplayCausationRef {
                    root_id: decision_id.to_owned(),
                    parent_event_id: None,
                    decision_id: Some(decision_id.to_owned()),
                },
                originating_event_id: None,
                source: Some(GameplayEntityRef {
                    entity: EntityId::new(10),
                }),
                targets: vec![GameplayEntityRef {
                    entity: EntityId::new(20),
                }],
                canonical_payload: payload.clone(),
                payload_hash: gameplay_canonical_payload_hash(&payload),
            },
            expected_owner_revision: "revision:0".to_owned(),
            workspace: GameplayOperationWorkspace::from_payload(
                decision_contract("workspace"),
                payload,
            ),
            resume_token: None,
        }
    }

    #[derive(Default)]
    struct PublicDecisionOwner {
        commits: Vec<Vec<u8>>,
    }

    impl GameplayRuntimeDecisionOwner for PublicDecisionOwner {
        fn revision_hash(&self, owner: &GameplayOwnerRef) -> String {
            assert_eq!(owner, &decision_owner());
            format!("revision:{}", self.commits.len())
        }

        fn route_precommit(
            &mut self,
            owner: &GameplayOwnerRef,
            operation: &GameplayProposalEnvelope,
        ) -> GameplayRuntimeDecisionOwnerOutput {
            assert_eq!(owner, &decision_owner());
            self.commits.push(operation.canonical_payload.clone());
            GameplayRuntimeDecisionOwnerOutput {
                accepted: true,
                fact_hashes: vec![gameplay_module_payload_hash(&operation.canonical_payload)],
                ..GameplayRuntimeDecisionOwnerOutput::default()
            }
        }
    }

    #[test]
    fn approved_public_decision_host_resumes_exactly_once_with_declared_reads() {
        let mut host =
            GameplayRuntimeHost::activate_project(decision_runtime_host_project_input()).unwrap();
        let mut owner = PublicDecisionOwner::default();
        let suspended = host.decide(public_decision_moment("public-decision"), &mut owner);
        assert_eq!(suspended.status, GameplayDecisionStatus::Suspended);
        assert_eq!(suspended.invocations.len(), 2);
        assert!(suspended.invocations[0].declared_read_set_hash.is_some());
        let continuation = suspended.continuation.clone().unwrap();

        let snapshot = host.compose_snapshot().unwrap();
        let mut restored = GameplayRuntimeHost::restore_project(
            decision_runtime_host_project_input(),
            &snapshot.text,
        )
        .unwrap();
        assert_eq!(restored.readout().pending_decision_count, 1);

        let mut missing = public_decision_moment("public-decision");
        missing.workspace = continuation.workspace.clone();
        let missing_receipt = restored.decide(missing, &mut owner);
        assert_eq!(missing_receipt.status, GameplayDecisionStatus::Failed);
        assert!(missing_receipt.invocations.is_empty());

        let mut wrong = public_decision_moment("public-decision");
        wrong.workspace = continuation.workspace.clone();
        wrong.resume_token = Some("not-authorized".to_owned());
        let wrong_receipt = restored.decide(wrong, &mut owner);
        assert_eq!(wrong_receipt.status, GameplayDecisionStatus::Failed);
        assert!(wrong_receipt.invocations.is_empty());

        let mut resumed = public_decision_moment("public-decision");
        resumed.workspace = continuation.workspace.clone();
        resumed.resume_token = Some(continuation.token.clone());
        let accepted = restored.decide(resumed, &mut owner);
        assert_eq!(accepted.status, GameplayDecisionStatus::Accepted);
        assert_eq!(owner.commits.len(), 1);
        let committed: DecisionWorkspace = serde_json::from_slice(&owner.commits[0]).unwrap();
        assert_eq!(committed.amount, 6);
        assert!(committed.transformed);

        let mut replayed = public_decision_moment("public-decision");
        replayed.workspace = continuation.workspace;
        replayed.resume_token = Some(continuation.token);
        let replayed_receipt = restored.decide(replayed, &mut owner);
        assert_eq!(replayed_receipt.status, GameplayDecisionStatus::Failed);
        assert!(replayed_receipt.invocations.is_empty());
        assert_eq!(owner.commits.len(), 1);
        assert_eq!(restored.readout().pending_decision_count, 0);
    }

    fn exercise_runtime_host(host: &mut GameplayRuntimeHost) -> Vec<String> {
        let mut frame_hashes = Vec::new();

        assert!(host
            .reconcile_triggers(1, TriggerReconcileCause::Tick)
            .unwrap()
            .gameplay_events
            .is_empty());
        let movement = host
            .move_actor_and_reconcile(EntityId::new(20), [-2.0, 0.0, 0.0], 2)
            .unwrap();
        let entered = movement.triggers;
        assert_eq!(entered.reactions.len(), 1);
        assert!(
            entered.reactions[0].observe.accepted(),
            "{:?}",
            entered.reactions[0].observe.diagnostics
        );
        let emitted = &entered.reactions[0].observe.events[1];
        assert_eq!(emitted.event, contract("trigger-reaction-proposed"));
        let emitted_payload: TriggerReactionProposal =
            serde_json::from_slice(&emitted.canonical_payload).unwrap();
        assert!(emitted_payload.overlap_read_hash.is_some());
        assert_eq!(entered.reactions[0].frame.routing_receipts.len(), 1);
        assert!(entered.reactions[0].frame.routing_receipts[0].accepted);
        assert_eq!(host.readout().active_overlap_count, 1);
        frame_hashes.push(entered.reactions[0].frame.frame_hash.clone());

        let state_before = host.readout().module_state_hash;
        let pulse = host.observe(root_event(7)).unwrap();
        assert!(pulse.observe.accepted(), "{:?}", pulse.observe.diagnostics);
        assert_eq!(pulse.frame.accepted_module_facts.len(), 1);
        assert_ne!(host.readout().module_state_hash, state_before);
        assert_eq!(host.readout().reaction_frame_count, 2);
        assert_eq!(
            host.readout().last_reaction_frame_hash.as_deref(),
            Some(pulse.frame.frame_hash.as_str())
        );
        frame_hashes.push(pulse.frame.frame_hash);
        frame_hashes
    }

    fn scheduled_collision_deactivation() -> TickScheduledActionDraft {
        let payload = CapabilityActivationGameplayProposal {
            entity: 10,
            capability: "collision".to_owned(),
            action: "deactivate".to_owned(),
        };
        let canonical_payload = serde_json::to_vec(&payload).unwrap();
        TickScheduledActionDraft {
            id: ScheduledActionId::new("fixture.scheduler.disable-trigger-collision"),
            execute_at: 5,
            priority: 0,
            proposal: GameplayProposalEnvelope {
                proposal_id: "draft.scheduler.disable-trigger-collision".to_owned(),
                proposal: StandardGameplayProposalKind::SetCapabilityActivation.contract(),
                tick: 0,
                root_sequence: 5,
                wave: 0,
                proposal_sequence: 0,
                emitter: GameplayEmitterRef::Owner {
                    owner_id: "authority.fixture".to_owned(),
                },
                causation: GameplayCausationRef {
                    root_id: "fixture.scheduler.root".to_owned(),
                    parent_event_id: None,
                    decision_id: None,
                },
                originating_event_id: None,
                source: None,
                targets: vec![GameplayEntityRef {
                    entity: EntityId::new(10),
                }],
                canonical_payload: canonical_payload.clone(),
                payload_hash: gameplay_canonical_payload_hash(&canonical_payload),
            },
            source: GameplayEmitterRef::Owner {
                owner_id: "authority.fixture".to_owned(),
            },
            causation: GameplayCausationRef {
                root_id: "fixture.scheduler.root".to_owned(),
                parent_event_id: None,
                decision_id: None,
            },
        }
    }

    #[test]
    fn public_runtime_host_schedules_routes_reads_and_restores_actions() {
        let mut host =
            GameplayRuntimeHost::activate_project(runtime_host_project_input(4)).unwrap();
        let initial = host.readout();
        let action_id = ScheduledActionId::new("fixture.scheduler.disable-trigger-collision");
        let routed = {
            let mut scheduler = host.scheduler_port();
            let scheduled = scheduler
                .apply(GameplayRuntimeSchedulerCommand::ScheduleTick(
                    scheduled_collision_deactivation(),
                ))
                .unwrap();
            assert_eq!(scheduled.readout.pending_action_count, 1);
            let triggered = scheduler
                .apply(GameplayRuntimeSchedulerCommand::ExecuteTick {
                    action_id: action_id.clone(),
                    tick: 5,
                    validity: ScheduledActionValidity::CURRENT,
                })
                .unwrap();
            assert!(triggered.scheduler.dispatch.is_some());
            assert_eq!(triggered.readout.outstanding_dispatch_count, 1);
            scheduler.route(&action_id).unwrap()
        };
        assert!(routed.routing.accepted);
        assert_eq!(routed.delivered_events.len(), 1);
        assert!(routed.reaction.as_ref().unwrap().observe.accepted());
        assert_eq!(routed.readout.pending_action_count, 0);
        assert_eq!(routed.readout.outstanding_dispatch_count, 0);
        assert_eq!(routed.readout.outstanding_event_delivery_count, 0);
        assert_eq!(routed.readout.fact_count, 4);
        assert_ne!(
            host.readout().scheduler.state_hash,
            initial.scheduler.state_hash
        );
        assert_ne!(
            host.readout().authority_state_hash,
            initial.authority_state_hash
        );
        assert_ne!(host.readout().runtime_host_hash, initial.runtime_host_hash);

        let snapshot = host.compose_snapshot().unwrap();
        let restored =
            GameplayRuntimeHost::restore_project(runtime_host_project_input(4), &snapshot.text)
                .unwrap();
        assert_eq!(restored.readout().scheduler, host.readout().scheduler);
        assert_eq!(
            restored.readout().runtime_host_hash,
            host.readout().runtime_host_hash
        );
    }

    #[test]
    fn approved_public_runtime_host_links_real_downstream_behavior() {
        let mut host =
            GameplayRuntimeHost::activate_project(runtime_host_project_input(4)).unwrap();
        let frame_hashes = exercise_runtime_host(&mut host);
        let runtime_hash = host.readout().runtime_host_hash;
        let snapshot = host.compose_snapshot().unwrap();

        let restored =
            GameplayRuntimeHost::restore_project(runtime_host_project_input(4), &snapshot.text)
                .unwrap();
        assert_eq!(restored.readout().runtime_host_hash, runtime_hash);
        assert_eq!(
            restored
                .reaction_frames()
                .iter()
                .map(|frame| frame.frame_hash.clone())
                .collect::<Vec<_>>(),
            frame_hashes
        );

        let mut replay =
            GameplayRuntimeHost::activate_project(runtime_host_project_input(4)).unwrap();
        assert_eq!(exercise_runtime_host(&mut replay), frame_hashes);
        assert_eq!(replay.readout().runtime_host_hash, runtime_hash);

        let mut changed =
            GameplayRuntimeHost::activate_project(runtime_host_project_input(5)).unwrap();
        let changed_frames = exercise_runtime_host(&mut changed);
        assert_ne!(changed_frames[1], frame_hashes[1]);
        assert_ne!(changed.readout().runtime_host_hash, runtime_hash);
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
