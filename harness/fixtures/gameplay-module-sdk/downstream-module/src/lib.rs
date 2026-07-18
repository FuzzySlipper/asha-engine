//! Public-facade provider regression: this crate imports only approved surfaces.

use asha_gameplay_module_sdk::*;
use asha_runtime_session_composition::{
    BootstrapResolutionContext, BundleArtifacts, EngineBridge, GameplayBindingEntityTargets,
    GameplayRuntimePrefabBootstrap, GameplayRuntimePrefabCatalog, GameplayRuntimePrefabPlacement,
    GameplayRuntimePrefabPlacementOrigin, GameplayRuntimePrefabTransform,
    GameplayRuntimeProjectInput, GameplayRuntimeSchedulerDefinition, GameplayRuntimeSpatialEntity,
    GameplayTriggerDefinition, LoadPlan, LoadStep, RuntimeSessionId, SceneId,
    StaticRuntimeSessionBuilder, StaticRuntimeSessionCompositionError,
    GAMEPLAY_TRIGGER_DEFINITION_SCHEMA_VERSION,
};
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

/// Consumer-owned native provider root. Building this crate as its declared
/// `cdylib` links the concrete downstream modules and returns the one bounded
/// RuntimeBridge authority cell that transport glue exposes.
pub fn build_native_runtime_session(
    input: GameplayRuntimeProjectInput,
) -> Result<EngineBridge, StaticRuntimeSessionCompositionError> {
    StaticRuntimeSessionBuilder::activate_project(input)?.build()
}

pub fn primary_fire_runtime_host_project_input() -> GameplayRuntimeProjectInput {
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
  "schemaVersion": 4,
  "id": 1,
  "metadata": { "name": "primary-fire-provider", "authoringFormatVersion": 4 },
  "dependencies": [],
  "nodes": [
    { "id": 10, "parent": null, "childOrder": 0, "label": "Primary fire trigger", "tags": [], "transform": { "translation": [0, 0, 0], "rotation": [0, 0, 0, 1], "scale": [1, 1, 1] }, "kind": { "kind": "entityInstance", "instance": { "instanceId": "fixture.primary-fire.trigger", "reference": { "kind": "entityDefinition", "stableId": "fixture/primary-fire-trigger" }, "spawnMarkerId": null } } }
  ]
}"#;
    let artifacts = BundleArtifacts::new()
        .with_artifact("assets/lock.json", "{ \"entries\": [] }\n")
        .with_artifact("scene/scene.json", scene);
    GameplayRuntimeProjectInput {
        load_plan: plan,
        artifacts,
        bootstrap_resolution: BootstrapResolutionContext {
            entity_definition_ids: ["fixture/primary-fire-trigger".to_owned()]
                .into_iter()
                .collect(),
            ..Default::default()
        },
        composition: primary_fire_composition(),
        composition_requirement: None,
        bindings: GameplayModuleBindingRegistryBuilder::new().build(),
        entity_targets: GameplayBindingEntityTargets::new(),
        spatial_entities: vec![GameplayRuntimeSpatialEntity {
            entity: EntityId::new(10),
            translation: [0.0, 0.0, 0.0],
            half_extents: [0.5, 0.5, 0.5],
            static_collider: false,
        }],
        declared_reads: primary_fire_topology().declared_reads().to_vec(),
        triggers: vec![GameplayTriggerDefinition {
            schema_version: GAMEPLAY_TRIGGER_DEFINITION_SCHEMA_VERSION,
            scene_instance_id: "fixture.primary-fire.trigger".to_owned(),
            scope: "fixture.primary-fire".to_owned(),
            tags: vec!["fixture".to_owned()],
        }],
        scheduler: GameplayRuntimeSchedulerDefinition::new(
            GameplayOwnerRef {
                owner_id: "authority.fixture-scheduler".to_owned(),
                provider_id: "provider.fixture-scheduler".to_owned(),
            },
            Vec::new(),
            vec![StandardGameplayProposalKind::ResolvePrimaryFire.contract()],
        ),
    }
}

/// Public downstream fixture that composes a decision Transform with a
/// stateful observer. It is used by the native provider regression to demonstrate
/// that one generated RuntimeBridge reaches both authority and module-owned
/// projection without a sidecar host.
pub fn composed_runtime_host_project_input(multiplier: u64) -> GameplayRuntimeProjectInput {
    let mut input = primary_fire_runtime_host_project_input();
    input.composition = composed_static_composition(multiplier);
    input.bindings = binding_registry(multiplier);
    input.declared_reads = primary_fire_topology().declared_reads().to_vec();
    input
        .declared_reads
        .extend_from_slice(pulse_topology().declared_reads());
    input.scheduler = GameplayRuntimeSchedulerDefinition::new(
        GameplayOwnerRef {
            owner_id: "authority.fixture-scheduler".to_owned(),
            provider_id: "provider.fixture-scheduler".to_owned(),
        },
        Vec::new(),
        vec![
            StandardGameplayProposalKind::ResolvePrimaryFire.contract(),
            StandardGameplayProposalKind::SetCapabilityActivation.contract(),
        ],
    );
    input
}

pub fn composed_runtime_prefab_bootstrap() -> GameplayRuntimePrefabBootstrap {
    GameplayRuntimePrefabBootstrap {
        registry_json: r#"{
  "schemaVersion": 1,
  "definitions": [{
    "id": 70,
    "schemaVersion": 1,
    "displayName": "Composed interaction target",
    "parts": [{
      "id": 1,
      "namespace": "body",
      "displayName": "Target",
      "parent": null,
      "transform": { "translation": [0, 0, 0], "rotation": [0, 0, 0, 1], "scale": [1, 1, 1] },
      "source": { "kind": "entityDefinition", "stableId": "fixture.composed.interaction-target" }
    }],
    "partRoles": [{ "role": "interaction/target", "part": 1 }],
    "variant": null
  }]
}"#
        .to_owned(),
        catalog: GameplayRuntimePrefabCatalog {
            asset_ids: Vec::new(),
            entity_definition_ids: vec!["fixture.composed.interaction-target".to_owned()],
        },
        placements: vec![GameplayRuntimePrefabPlacement {
            command_id: "place-composed-interaction-target".to_owned(),
            scene_instance_id: "fixture.composed.interaction-target".to_owned(),
            origin: GameplayRuntimePrefabPlacementOrigin::Authored,
            instance: 700,
            prefab: 70,
            seed: 11,
            transform: GameplayRuntimePrefabTransform::IDENTITY,
            overrides: Vec::new(),
        }],
    }
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
fn decision_topology() -> GameplayDerivedModuleTopology {
    let proposal = decision_contract("operation");
    let workspace = decision_contract("workspace");
    let transform = GameplayModuleInvocationTopology::decision(
        "fixture.decision.transform",
        GameplayInvocationFamily::Transform,
        proposal.clone(),
        workspace.clone(),
        1,
        4_096,
    )
    .read(GameplayModuleReadTopology {
        request: GameplayReadRequest {
            request_id: "decision-target-collision".to_owned(),
            view: decision_contract("target-collision-view"),
            fields: vec!["staticCollider".to_owned()],
            selector: GameplayReadSelector::Capability {
                binding: GameplayEventEntityBinding::Target { index: 0 },
                capability: GameplayCapabilityReadKind::Collision,
            },
        },
        provider_id: "provider.fixture-decision".to_owned(),
        kind: GameplayReadViewKind::EntityCapability,
        selector_capabilities: vec![
            GameplayReadSelectorCapability::EventTarget,
            GameplayReadSelectorCapability::CollisionCapability,
        ],
        max_items: 1,
        ordering: "entityIdAscending".to_owned(),
    });
    let react = GameplayModuleInvocationTopology::decision(
        "fixture.decision.react",
        GameplayInvocationFamily::React,
        proposal,
        workspace,
        1,
        4_096,
    );
    GameplayDerivedModuleTopology::derive("fixture.decision.module", vec![transform, react])
        .expect("decision topology is unambiguous")
}

#[cfg(test)]
fn decision_provider() -> GameplayStaticModuleProvider {
    let proposal = decision_contract("operation");
    let owner = decision_owner();
    let topology = decision_topology();
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
        invocations: Vec::new(),
        read_views: Vec::new(),
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
    topology
        .apply_to_manifest(&mut manifest)
        .expect("decision topology belongs to its manifest");
    build_provenance().apply_to_manifest::<FixtureDecisionBehavior>(&mut manifest);
    GameplayStaticModuleProvider::linked_from_manifest(
        manifest,
        &build_provenance(),
        FixtureDecisionBehavior,
    )
    .proposal_codec(gameplay_serde_json_codec_registration::<DecisionWorkspace>(
        proposal.clone(),
        schema_descriptor("fixture.decision", "operation"),
    ))
    .proposal_owner(GameplayProposalOwnerRegistration { proposal, owner })
    .derived_topology(&topology)
}

#[cfg(test)]
fn decision_composition() -> GameplayStaticComposition {
    let mut builder = GameplayStaticCompositionBuilder::new();
    builder.add_provider(decision_provider());
    builder.build().expect("public decision composition")
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
    use asha_gameplay_runtime_host::GameplayRuntimeHost;
    use asha_runtime_session_composition::{
        BundleArtifacts, ComposedGameplayOwner, ComposedGameplayOwnerCheckpoint,
        ComposedGameplayOwnerOutput, EnemyDirectNavAuthoritySource, EnemyDirectNavMovementRequest,
        EngineConfig, FlatSceneDocumentDto, FpsBootstrapResolutionRegistry,
        FpsBridgeBoundsCapability, FpsBridgeHealth, FpsBridgePolicyBinding, FpsBridgeRole,
        FpsBridgeStoredEntityDefinition, FpsBridgeTransformCapability, FpsBridgeWeaponMount,
        FpsPrimaryFireRequest, FpsRuntimeSessionLoadRequest, FpsRuntimeSessionRestartRequest,
        GameplayBindingEntityTargets, GameplayDecisionMoment, GameplayDecisionStatus,
        GameplayModuleViewRequest, GameplayModuleViewScope, GameplayOperationWorkspace,
        GameplayPrefabPartInteractionRequest, GameplayRuntimeDecisionOwner,
        GameplayRuntimeDecisionOwnerOutput, GameplayRuntimeSchedulerCommand,
        GameplayRuntimeSchedulerDefinition, GameplayRuntimeSpatialEntity,
        GameplayTriggerDefinition, LoadPlan, LoadStep, ProjectBundleLoadRequest, RuntimeBridge,
        RuntimeSessionId, SceneEntityInstanceDto, SceneEntityReferenceDto, SceneId,
        SceneMetadataDto, SceneNodeId, SceneNodeKindDto, SceneNodeRecordDto, SceneTransformDto,
        ScheduledActionId, ScheduledActionValidity, StaticRuntimeSessionBuilder,
        TickScheduledActionDraft, TriggerReconcileCause, Vec3,
        GAMEPLAY_TRIGGER_DEFINITION_SCHEMA_VERSION,
    };
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

    fn conformance_composition() -> Result<GameplayStaticComposition, GameplayStaticCompositionError>
    {
        Ok(composition(4))
    }

    struct ProviderLifetimeBehavior {
        multiplier: u64,
        drops: Arc<AtomicUsize>,
    }

    impl GameplayModuleBehavior for ProviderLifetimeBehavior {
        fn invoke(
            &self,
            context: &GameplayModuleContext<'_>,
        ) -> Result<GameplayModuleActions, GameplayModuleError> {
            PulseBehavior {
                multiplier: self.multiplier,
            }
            .invoke(context)
        }
    }

    impl Drop for ProviderLifetimeBehavior {
        fn drop(&mut self) {
            self.drops.fetch_add(1, Ordering::SeqCst);
        }
    }

    fn instrumented_runtime_host_project_input(
        multiplier: u64,
        drops: Arc<AtomicUsize>,
    ) -> GameplayRuntimeProjectInput {
        let provider =
            provider_with_behavior(multiplier, ProviderLifetimeBehavior { multiplier, drops });
        let module = provider.manifest.module_ref.clone();
        let mut composition = GameplayStaticCompositionBuilder::new();
        composition.include_standard_owner_events();
        composition.add_provider(provider);
        let mut input = runtime_host_project_input(multiplier);
        input.composition = composition.build().expect("instrumented composition");
        input.bindings = binding_registry_for_module(module, multiplier);
        input
    }

    #[test]
    fn authored_topology_is_the_manifest_and_runtime_read_plan_source() {
        let topology = pulse_topology();
        let provider = provider(4);
        assert_eq!(provider.manifest.subscriptions, topology.subscriptions());
        assert_eq!(provider.manifest.invocations, topology.invocations());
        assert_eq!(provider.manifest.read_views, topology.read_views());
        assert_eq!(
            runtime_host_project_input(4).declared_reads,
            topology.declared_reads()
        );
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
  "schemaVersion": 4,
  "id": 1,
  "metadata": { "name": "downstream-host", "authoringFormatVersion": 4 },
  "dependencies": [],
  "nodes": [
    { "id": 10, "parent": null, "childOrder": 0, "label": "Exit trigger", "tags": [], "transform": { "translation": [0, 0, 0], "rotation": [0, 0, 0, 1], "scale": [1, 1, 1] }, "kind": { "kind": "entityInstance", "instance": { "instanceId": "fixture.exit.trigger", "reference": { "kind": "entityDefinition", "stableId": "fixture/exit-trigger" }, "spawnMarkerId": null } } },
    { "id": 20, "parent": null, "childOrder": 1, "label": "Moving subject", "tags": [], "transform": { "translation": [2, 0, 0], "rotation": [0, 0, 0, 1], "scale": [1, 1, 1] }, "kind": { "kind": "entityInstance", "instance": { "instanceId": "fixture.moving.subject", "reference": { "kind": "entityDefinition", "stableId": "fixture/moving-subject" }, "spawnMarkerId": null } } }
  ]
}"#;
        let artifacts = BundleArtifacts::new()
            .with_artifact("assets/lock.json", "{ \"entries\": [] }\n")
            .with_artifact("scene/scene.json", scene);
        GameplayRuntimeProjectInput {
            load_plan: plan,
            artifacts,
            bootstrap_resolution: BootstrapResolutionContext {
                entity_definition_ids: [
                    "fixture/exit-trigger".to_owned(),
                    "fixture/moving-subject".to_owned(),
                ]
                .into_iter()
                .collect(),
                ..Default::default()
            },
            composition: composition(multiplier),
            composition_requirement: None,
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
            declared_reads: pulse_topology().declared_reads().to_vec(),
            triggers: vec![GameplayTriggerDefinition {
                schema_version: GAMEPLAY_TRIGGER_DEFINITION_SCHEMA_VERSION,
                scene_instance_id: "fixture.exit.trigger".to_owned(),
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

    fn composed_fps_load_request() -> FpsRuntimeSessionLoadRequest {
        FpsRuntimeSessionLoadRequest {
            project_bundle: "downstream-composed-cell".to_owned(),
            scene_document: composed_fps_scene_document(),
            bootstrap_resolution_registry: FpsBootstrapResolutionRegistry {
                schema_version: 1,
                entity_definition_ids: vec![
                    "actor/composed-player".to_owned(),
                    "actor/composed-enemy".to_owned(),
                ],
                prefab_ids: Vec::new(),
                generator_presets: Vec::new(),
                catalog_ids: Vec::new(),
            },
            definitions: vec![
                FpsBridgeStoredEntityDefinition {
                    entity: 101,
                    stable_id: "actor/composed-player".to_owned(),
                    display_name: "Composed Player".to_owned(),
                    source_path: "catalogs/player.entity.json".to_owned(),
                    tags: vec!["player".to_owned()],
                    role: FpsBridgeRole::Player,
                    transform: Some(FpsBridgeTransformCapability {
                        translation: [2.5, 1.5, 1.5],
                        rotation: [0.0, 0.0, 0.0, 1.0],
                        scale: [1.0, 1.0, 1.0],
                    }),
                    bounds: Some(FpsBridgeBoundsCapability {
                        min: [2.2, 1.0, 1.0],
                        max: [2.8, 2.0, 2.0],
                    }),
                    render_visible: Some(true),
                    static_collider: Some(false),
                    health: Some(FpsBridgeHealth {
                        current: 88,
                        max: 88,
                    }),
                    weapon: Some(FpsBridgeWeaponMount {
                        weapon_id: "weapon.composed.primary".to_owned(),
                        damage: 75,
                        range_units: 16,
                        ammo: 3,
                        cooldown_ticks_after_fire: 4,
                    }),
                    policy_binding: None,
                },
                FpsBridgeStoredEntityDefinition {
                    entity: 777,
                    stable_id: "actor/composed-enemy".to_owned(),
                    display_name: "Composed Enemy".to_owned(),
                    source_path: "catalogs/enemy.entity.json".to_owned(),
                    tags: vec!["enemy".to_owned()],
                    role: FpsBridgeRole::Enemy,
                    transform: Some(FpsBridgeTransformCapability {
                        translation: [2.5, 1.5, 5.2],
                        rotation: [0.0, 0.0, 0.0, 1.0],
                        scale: [1.0, 1.0, 1.0],
                    }),
                    bounds: Some(FpsBridgeBoundsCapability {
                        min: [2.2, 1.0, 5.0],
                        max: [2.8, 2.0, 5.8],
                    }),
                    render_visible: Some(true),
                    static_collider: Some(false),
                    health: Some(FpsBridgeHealth {
                        current: 150,
                        max: 150,
                    }),
                    weapon: None,
                    policy_binding: Some(FpsBridgePolicyBinding {
                        binding_id: "binding.composed-enemy.v0".to_owned(),
                        policy_id: "policy.composed-enemy.v0".to_owned(),
                        view_kind: "runtime_session.nav_policy_view.v0".to_owned(),
                        view_version: "v0".to_owned(),
                        allowed_intents: vec!["runtime.intent.move_direct_nav.v0".to_owned()],
                        runtime_moment: "runtime.tick.enemy_policy.v0".to_owned(),
                    }),
                },
            ],
            game_rule_modules: Vec::new(),
        }
    }

    fn composed_fps_scene_document() -> FlatSceneDocumentDto {
        let instance = |id: u64, stable_id: &str, translation: [f32; 3]| SceneNodeRecordDto {
            id: SceneNodeId::new(id),
            parent: None,
            child_order: id as u32,
            label: Some(stable_id.to_owned()),
            tags: Vec::new(),
            transform: SceneTransformDto {
                translation,
                rotation: [0.0, 0.0, 0.0, 1.0],
                scale: [1.0, 1.0, 1.0],
            },
            kind: SceneNodeKindDto::EntityInstance {
                instance: SceneEntityInstanceDto {
                    instance_id: format!("instance.{id}"),
                    reference: SceneEntityReferenceDto::EntityDefinition {
                        stable_id: stable_id.to_owned(),
                    },
                    spawn_marker_id: None,
                },
            },
        };
        FlatSceneDocumentDto {
            schema_version: 3,
            id: SceneId::new(9002),
            metadata: SceneMetadataDto {
                name: Some("Downstream composed FPS fixture".to_owned()),
                authoring_format_version: 3,
            },
            dependencies: Vec::new(),
            nodes: vec![
                instance(101, "actor/composed-player", [2.5, 1.5, 1.5]),
                instance(777, "actor/composed-enemy", [2.5, 1.5, 5.2]),
            ],
        }
    }

    struct CorruptPrimaryFireIdentityBehavior;

    impl GameplayModuleBehavior for CorruptPrimaryFireIdentityBehavior {
        fn invoke(
            &self,
            context: &GameplayModuleContext<'_>,
        ) -> Result<GameplayModuleActions, GameplayModuleError> {
            let mut workspace: PrimaryFireGameplayDecisionWorkspace =
                context.decision_workspace()?;
            workspace.shooter = workspace.shooter.saturating_add(1);
            let mut actions = context.actions();
            actions.transform_workspace_json(
                StandardGameplayProposalKind::ResolvePrimaryFire.contract(),
                context
                    .decision_workspace_hash()
                    .expect("corrupt fixture receives a Workspace hash"),
                &workspace,
            )?;
            Ok(actions)
        }
    }

    fn corrupt_primary_fire_project_input() -> GameplayRuntimeProjectInput {
        let mut input = primary_fire_runtime_host_project_input();
        let mut composition = GameplayStaticCompositionBuilder::new();
        composition.include_standard_owner_events();
        composition.add_provider(primary_fire_provider_with_behavior(
            CorruptPrimaryFireIdentityBehavior,
        ));
        input.composition = composition.build().expect("corrupt fixture composes");
        input
    }

    fn initialized_primary_fire_bridge(input: GameplayRuntimeProjectInput) -> EngineBridge {
        let mut bridge = StaticRuntimeSessionBuilder::activate_project(input)
            .unwrap()
            .build()
            .unwrap();
        bridge.initialize_engine(EngineConfig { seed: 41 }).unwrap();
        bridge
            .load_project_bundle(ProjectBundleLoadRequest {
                bundle_schema_version: 1,
                protocol_version: 1,
                scene_id: 1,
            })
            .unwrap();
        bridge
    }

    #[test]
    fn composed_primary_fire_transform_commits_once_and_preserves_far_range_damage() {
        let mut close = initialized_primary_fire_bridge(primary_fire_runtime_host_project_input());
        close
            .load_fps_runtime_session(composed_fps_load_request())
            .unwrap();
        let close_result = close
            .apply_fps_primary_fire(FpsPrimaryFireRequest {
                tick: 9,
                origin: [2.5, 1.5, 1.5],
                direction: [0.0, 0.0, 1.0],
                shooter_role: None,
                target_role: None,
            })
            .unwrap();
        assert_eq!(close_result.target_health_before.unwrap().current, 150);
        assert_eq!(close_result.target_health_after.unwrap().current, 0);
        assert_eq!(close_result.workspace_trace.len(), 3);
        let close_readout = close.read_composed_runtime_session().unwrap();
        assert_eq!(close_readout.gameplay.decision_receipt_count, 1);

        let mut far_request = composed_fps_load_request();
        far_request.definitions[1].transform = Some(FpsBridgeTransformCapability {
            translation: [2.5, 1.5, 8.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            scale: [1.0, 1.0, 1.0],
        });
        far_request.definitions[1].bounds = Some(FpsBridgeBoundsCapability {
            min: [2.2, 1.0, 7.8],
            max: [2.8, 2.0, 8.6],
        });
        far_request.scene_document.nodes[1].transform.translation = [2.5, 1.5, 8.0];
        let mut far = initialized_primary_fire_bridge(primary_fire_runtime_host_project_input());
        far.load_fps_runtime_session(far_request).unwrap();
        let far_result = far
            .apply_fps_primary_fire(FpsPrimaryFireRequest {
                tick: 9,
                origin: [2.5, 1.5, 1.5],
                direction: [0.0, 0.0, 1.0],
                shooter_role: None,
                target_role: None,
            })
            .unwrap();
        assert_eq!(far_result.target_health_before.unwrap().current, 150);
        assert_eq!(far_result.target_health_after.unwrap().current, 75);
        assert_eq!(
            far.read_composed_runtime_session()
                .unwrap()
                .gameplay
                .decision_receipt_count,
            1,
        );
    }

    #[test]
    fn rejected_primary_fire_transform_restores_authority_and_fabric_evidence() {
        let mut bridge = initialized_primary_fire_bridge(corrupt_primary_fire_project_input());
        bridge
            .load_fps_runtime_session(composed_fps_load_request())
            .unwrap();
        let before = bridge.read_composed_runtime_session().unwrap();
        let error = bridge
            .apply_fps_primary_fire(FpsPrimaryFireRequest {
                tick: 9,
                origin: [2.5, 1.5, 1.5],
                direction: [0.0, 0.0, 1.0],
                shooter_role: None,
                target_role: None,
            })
            .expect_err("the combat owner rejects a transformed shooter identity");
        assert!(error.message.contains("not accepted"), "{error}");
        assert_eq!(bridge.read_composed_runtime_session().unwrap(), before);
    }

    #[test]
    fn public_bridge_reads_module_view_and_routes_prefab_interaction_once() {
        let discovery_host = GameplayRuntimeHost::activate_project_with_prefabs(
            composed_runtime_host_project_input(4),
            composed_runtime_prefab_bootstrap(),
        )
        .unwrap();
        let target = discovery_host.prefab_readout().instances[0].roles[0].entity;
        assert_eq!(target, 4_102_412_266_368_810);

        let mut bridge = StaticRuntimeSessionBuilder::activate_project_with_prefabs(
            composed_runtime_host_project_input(4),
            composed_runtime_prefab_bootstrap(),
        )
        .unwrap()
        .build()
        .unwrap();
        bridge.initialize_engine(EngineConfig { seed: 41 }).unwrap();
        bridge
            .load_project_bundle(ProjectBundleLoadRequest {
                bundle_schema_version: 1,
                protocol_version: 1,
                scene_id: 1,
            })
            .unwrap();
        bridge
            .load_fps_runtime_session(composed_fps_load_request())
            .unwrap();

        let before = bridge.read_composed_runtime_session().unwrap();
        let view = bridge
            .read_gameplay_module_view(GameplayModuleViewRequest {
                view: pulse_state_view_contract(),
                scope: GameplayModuleViewScope::Session,
                expected_runtime_session_hash: before.runtime_session_hash.clone(),
            })
            .unwrap();
        assert_eq!(
            serde_json::from_slice::<u64>(&view.canonical_payload).unwrap(),
            4
        );
        assert_eq!(view.runtime_session_hash, before.runtime_session_hash);
        assert_eq!(bridge.read_composed_runtime_session().unwrap(), before);

        let request = GameplayPrefabPartInteractionRequest {
            actor: 101,
            instance: 700,
            role: "interaction/target".to_owned(),
            expected_target: target,
            tick: 12,
            expected_runtime_session_hash: before.runtime_session_hash.clone(),
        };
        let receipt = bridge
            .apply_gameplay_prefab_part_interaction(request.clone())
            .unwrap();
        assert_eq!(receipt.target, target);
        assert_ne!(receipt.runtime_session_hash, before.runtime_session_hash);
        let after = bridge.read_composed_runtime_session().unwrap();
        assert_eq!(receipt.runtime_session_hash, after.runtime_session_hash);
        assert_eq!(
            after.gameplay.reaction_frame_count,
            before.gameplay.reaction_frame_count + 1
        );
        let updated_view = bridge
            .read_gameplay_module_view(GameplayModuleViewRequest {
                view: pulse_state_view_contract(),
                scope: GameplayModuleViewScope::Session,
                expected_runtime_session_hash: after.runtime_session_hash.clone(),
            })
            .unwrap();
        assert_eq!(
            serde_json::from_slice::<u64>(&updated_view.canonical_payload).unwrap(),
            5
        );
        assert_eq!(updated_view.revision, view.revision + 1);

        let stale = bridge
            .apply_gameplay_prefab_part_interaction(request)
            .expect_err("a stale generation cannot repeat the interaction");
        assert!(stale.message.contains("expected RuntimeSession"), "{stale}");
        assert_eq!(bridge.read_composed_runtime_session().unwrap(), after);

        let wrong_target = bridge
            .apply_gameplay_prefab_part_interaction(GameplayPrefabPartInteractionRequest {
                actor: 101,
                instance: 700,
                role: "interaction/target".to_owned(),
                expected_target: target.saturating_add(1),
                tick: 13,
                expected_runtime_session_hash: after.runtime_session_hash.clone(),
            })
            .expect_err("a caller cannot substitute the resolved prefab target");
        assert!(
            wrong_target.message.contains("target mismatch"),
            "{wrong_target}"
        );
        assert_eq!(bridge.read_composed_runtime_session().unwrap(), after);
    }

    #[test]
    fn public_static_builder_composes_one_bridge_cell_and_restores_it() {
        let mut first = build_native_runtime_session(runtime_host_project_input(4)).unwrap();
        let initial = first.read_composed_runtime_session().unwrap();

        first.initialize_engine(EngineConfig { seed: 41 }).unwrap();
        first
            .load_project_bundle(ProjectBundleLoadRequest {
                bundle_schema_version: 1,
                protocol_version: 1,
                scene_id: 1,
            })
            .unwrap();
        first
            .load_fps_runtime_session(composed_fps_load_request())
            .unwrap();
        let loaded = first.read_composed_runtime_session().unwrap();
        assert_ne!(loaded.entity_authority_hash, initial.entity_authority_hash);
        assert_eq!(loaded.fps_session_epoch, 1);

        let frames_before = loaded.gameplay.reaction_frame_count;
        first
            .apply_fps_primary_fire(FpsPrimaryFireRequest {
                tick: 9,
                origin: [2.5, 1.5, 1.5],
                direction: [0.0, 0.0, 1.0],
                shooter_role: None,
                target_role: None,
            })
            .unwrap();
        let reacted = first.read_composed_runtime_session().unwrap();
        assert!(reacted.gameplay.reaction_frame_count > frames_before);
        assert_ne!(reacted.runtime_session_hash, loaded.runtime_session_hash);

        let moved = first
            .apply_enemy_direct_nav_movement(EnemyDirectNavMovementRequest {
                entity: 777,
                seed_position: Vec3::new(2.5, 1.5, 5.2),
                target: Vec3::new(0.0, 0.0, 0.0),
                max_step_units: 16.0,
            })
            .unwrap();
        assert_eq!(
            moved.authority_source,
            EnemyDirectNavAuthoritySource::RustEntityStore,
        );
        assert!(moved.reached);
        let trigger_reacted = first.read_composed_runtime_session().unwrap();
        assert!(
            trigger_reacted.gameplay.reaction_frame_count > reacted.gameplay.reaction_frame_count
        );
        assert_eq!(trigger_reacted.gameplay.active_overlap_count, 1);
        assert_ne!(
            trigger_reacted.entity_authority_hash,
            reacted.entity_authority_hash,
        );

        let checkpoint = first.checkpoint_composed_runtime_session().unwrap();
        let mut restored = StaticRuntimeSessionBuilder::restore_project(
            runtime_host_project_input(4),
            &checkpoint,
        )
        .unwrap()
        .build()
        .unwrap();
        assert_eq!(
            restored.read_composed_runtime_session().unwrap(),
            *checkpoint.readout(),
        );

        let mut isolated =
            StaticRuntimeSessionBuilder::activate_project(runtime_host_project_input(4))
                .unwrap()
                .build()
                .unwrap();
        let isolated_readout = isolated.read_composed_runtime_session().unwrap();
        assert_eq!(isolated_readout, initial);
        assert_ne!(
            checkpoint.readout().runtime_session_hash,
            isolated_readout.runtime_session_hash,
        );
    }

    #[test]
    fn public_static_runtime_provider_lifecycle_releases_each_isolated_cell() {
        let first_drops = Arc::new(AtomicUsize::new(0));
        let second_drops = Arc::new(AtomicUsize::new(0));
        let mut first = StaticRuntimeSessionBuilder::activate_project(
            instrumented_runtime_host_project_input(4, first_drops.clone()),
        )
        .unwrap()
        .build()
        .unwrap();
        let mut second = StaticRuntimeSessionBuilder::activate_project(
            instrumented_runtime_host_project_input(5, second_drops.clone()),
        )
        .unwrap()
        .build()
        .unwrap();
        let second_before = second.read_composed_runtime_session().unwrap();

        first.initialize_engine(EngineConfig { seed: 51 }).unwrap();
        first
            .load_project_bundle(ProjectBundleLoadRequest {
                bundle_schema_version: 1,
                protocol_version: 1,
                scene_id: 1,
            })
            .unwrap();
        first
            .load_fps_runtime_session(composed_fps_load_request())
            .unwrap();
        let loaded = first.read_composed_runtime_session().unwrap();
        assert_eq!(loaded.fps_session_epoch, 1);
        assert_eq!(
            second.read_composed_runtime_session().unwrap(),
            second_before
        );

        let restarted = first
            .restart_fps_runtime_session(FpsRuntimeSessionRestartRequest { expected_epoch: 1 })
            .unwrap();
        assert_eq!(restarted.session_epoch, 2);
        let after_restart = first.read_composed_runtime_session().unwrap();
        assert_eq!(after_restart.fps_session_epoch, 2);
        assert_eq!(
            after_restart.gameplay.gameplay_registry_digest,
            loaded.gameplay.gameplay_registry_digest,
        );
        assert_eq!(
            second.read_composed_runtime_session().unwrap(),
            second_before
        );

        let switched_status = first
            .load_project_bundle(ProjectBundleLoadRequest {
                bundle_schema_version: 1,
                protocol_version: 1,
                scene_id: 2,
            })
            .unwrap();
        assert_eq!(switched_status.loaded_project_bundle, Some(2));
        assert_eq!(
            first
                .get_project_bundle_composition_status()
                .unwrap()
                .loaded_project_bundle,
            Some(2),
        );
        let switched = first.read_composed_runtime_session().unwrap();
        assert_eq!(
            switched.gameplay.gameplay_registry_digest,
            after_restart.gameplay.gameplay_registry_digest,
        );
        assert_eq!(first_drops.load(Ordering::SeqCst), 0);
        assert_eq!(
            second.read_composed_runtime_session().unwrap(),
            second_before
        );

        first.unload_project_bundle().unwrap();
        assert_eq!(first_drops.load(Ordering::SeqCst), 1);
        assert!(first.read_composed_runtime_session().is_err());
        assert_eq!(
            first
                .get_project_bundle_composition_status()
                .unwrap()
                .loaded_project_bundle,
            None,
        );
        drop(first);
        assert_eq!(first_drops.load(Ordering::SeqCst), 1);

        second.unload_project_bundle().unwrap();
        assert_eq!(second_drops.load(Ordering::SeqCst), 1);
        assert!(second.read_composed_runtime_session().is_err());
        drop(second);
        assert_eq!(second_drops.load(Ordering::SeqCst), 1);

        let evidence = serde_json::json!({
            "schemaVersion": 1,
            "phase": "provider-lifecycle",
            "session": ["provider-session:A-to-B", "isolated-session"],
            "waveOrAction": "load-A/restart/same-cell-switch-B/unload/explicit-close",
            "registryDigest": switched.gameplay.gameplay_registry_digest,
            "evidenceHashes": [
                loaded.runtime_session_hash,
                after_restart.runtime_session_hash,
                second_before.runtime_session_hash,
                switched.runtime_session_hash,
            ],
            "projectBundleSequence": [1, 2, null],
            "explicitCloseObserved": true,
            "resourceReleaseCounts": {
                "providerSession": first_drops.load(Ordering::SeqCst),
                "isolatedSession": second_drops.load(Ordering::SeqCst),
            },
        });
        eprintln!("ASHA_GAMEPLAY_RUNTIME_HOST_EVIDENCE={evidence}");
    }

    #[test]
    fn composed_bridge_runs_decision_transform_and_exactly_once_resume() {
        let mut bridge =
            StaticRuntimeSessionBuilder::activate_project(decision_runtime_host_project_input())
                .unwrap()
                .build()
                .unwrap();
        let mut owner = PublicDecisionOwner::default();
        let suspended = bridge
            .decide_composed_gameplay(public_decision_moment("composed-decision"), &mut owner)
            .unwrap();
        assert_eq!(suspended.status, GameplayDecisionStatus::Suspended);
        assert!(suspended.invocations[0].declared_read_set_hash.is_some());
        let continuation = suspended.continuation.unwrap();

        let mut resumed = public_decision_moment("composed-decision");
        resumed.workspace = continuation.workspace.clone();
        resumed.resume_token = Some(continuation.token.clone());
        let accepted = bridge
            .decide_composed_gameplay(resumed, &mut owner)
            .unwrap();
        assert_eq!(accepted.status, GameplayDecisionStatus::Accepted);
        assert_eq!(owner.commits.len(), 1);
        let committed: DecisionWorkspace = serde_json::from_slice(&owner.commits[0]).unwrap();
        assert_eq!(committed.amount, 6);
        assert!(committed.transformed);

        let before_replay = bridge.read_composed_runtime_session().unwrap();
        let mut replayed = public_decision_moment("composed-decision");
        replayed.workspace = continuation.workspace;
        replayed.resume_token = Some(continuation.token);
        let replayed_receipt = bridge
            .decide_composed_gameplay(replayed, &mut owner)
            .unwrap();
        assert_eq!(replayed_receipt.status, GameplayDecisionStatus::Failed);
        assert!(replayed_receipt.invocations.is_empty());
        assert_eq!(owner.commits.len(), 1);
        assert_eq!(
            bridge.read_composed_runtime_session().unwrap(),
            before_replay,
        );
    }

    #[test]
    fn composed_rulebench_owner_is_atomic_checkpointed_and_replay_bound() {
        let mut bridge =
            StaticRuntimeSessionBuilder::activate_project(rulebench_combat_project_input())
                .unwrap()
                .with_gameplay_owner(RulebenchCombatOwner::new(false))
                .unwrap()
                .build()
                .unwrap();
        let initial = bridge.read_composed_runtime_session().unwrap();
        let initial_owner = initial.gameplay_owner.clone().unwrap();

        let suspended = bridge
            .transact_composed_gameplay_owner(public_decision_moment("rulebench-combat"))
            .unwrap();
        assert_eq!(suspended.decision.status, GameplayDecisionStatus::Suspended);
        assert!(suspended.reaction_frame_hashes.is_empty());
        assert_eq!(suspended.gameplay_owner, initial_owner);
        let continuation = suspended.decision.continuation.clone().unwrap();

        let mut resumed = public_decision_moment("rulebench-combat");
        resumed.workspace = continuation.workspace.clone();
        resumed.resume_token = Some(continuation.token.clone());
        let accepted = bridge.transact_composed_gameplay_owner(resumed).unwrap();
        assert_eq!(accepted.decision.status, GameplayDecisionStatus::Accepted);
        assert_eq!(accepted.reaction_frame_hashes.len(), 1);
        assert_eq!(
            accepted.reaction_event_keys,
            vec![contract("pulse").key(), contract("pulse-result").key()],
        );
        assert_ne!(accepted.gameplay_owner.state_hash, initial_owner.state_hash);
        assert_ne!(
            accepted.gameplay_owner.replay_hash,
            initial_owner.replay_hash
        );
        let accepted_readout = bridge.read_composed_runtime_session().unwrap();
        assert_eq!(
            accepted.runtime_session_hash,
            accepted_readout.runtime_session_hash
        );
        assert_eq!(accepted_readout.gameplay.pending_decision_count, 0);
        assert_eq!(accepted_readout.gameplay.reaction_frame_count, 1);

        let module_view = bridge
            .read_gameplay_module_view(GameplayModuleViewRequest {
                view: pulse_state_view_contract(),
                scope: GameplayModuleViewScope::Session,
                expected_runtime_session_hash: accepted.runtime_session_hash.clone(),
            })
            .unwrap();
        assert!(serde_json::from_slice::<u64>(&module_view.canonical_payload).unwrap() > 4);

        let before_replay = bridge.read_composed_runtime_session().unwrap();
        let mut replayed = public_decision_moment("rulebench-combat");
        replayed.workspace = continuation.workspace;
        replayed.resume_token = Some(continuation.token);
        let replayed = bridge.transact_composed_gameplay_owner(replayed).unwrap();
        assert_eq!(replayed.decision.status, GameplayDecisionStatus::Failed);
        assert_eq!(
            bridge.read_composed_runtime_session().unwrap(),
            before_replay
        );

        let stale = bridge
            .transact_composed_gameplay_owner(public_decision_moment("rulebench-stale"))
            .unwrap();
        assert_eq!(stale.decision.status, GameplayDecisionStatus::Stale);
        assert_eq!(
            bridge.read_composed_runtime_session().unwrap(),
            before_replay
        );

        let checkpoint = bridge.checkpoint_composed_runtime_session().unwrap();
        assert_eq!(
            checkpoint.gameplay_owner_checkpoint().unwrap().state_hash(),
            accepted_readout.gameplay_owner.as_ref().unwrap().state_hash,
        );
        let mut restored = StaticRuntimeSessionBuilder::restore_project(
            rulebench_combat_project_input(),
            &checkpoint,
        )
        .unwrap()
        .with_gameplay_owner(RulebenchCombatOwner::new(false))
        .unwrap()
        .build()
        .unwrap();
        assert_eq!(
            restored.read_composed_runtime_session().unwrap(),
            *checkpoint.readout()
        );

        let mismatch = StaticRuntimeSessionBuilder::restore_project(
            rulebench_combat_project_input(),
            &checkpoint,
        )
        .unwrap()
        .with_gameplay_owner(RulebenchCombatOwner::mismatch_after_restore());
        assert!(
            mismatch.is_err(),
            "replay-mismatched owner restore must fail"
        );
    }

    #[test]
    fn composed_rulebench_owner_rolls_back_owner_module_and_continuation_on_fact_rejection() {
        let mut bridge =
            StaticRuntimeSessionBuilder::activate_project(rulebench_combat_project_input())
                .unwrap()
                .with_gameplay_owner(RulebenchCombatOwner::new(true))
                .unwrap()
                .build()
                .unwrap();
        let suspended = bridge
            .transact_composed_gameplay_owner(public_decision_moment("rulebench-reject"))
            .unwrap();
        let continuation = suspended.decision.continuation.unwrap();
        let before_resume = bridge.read_composed_runtime_session().unwrap();
        let mut resumed = public_decision_moment("rulebench-reject");
        resumed.workspace = continuation.workspace;
        resumed.resume_token = Some(continuation.token);
        let error = bridge
            .transact_composed_gameplay_owner(resumed)
            .expect_err("malformed resolved fact must reject the whole transaction");
        assert!(error.message.contains("owner facts"), "{error}");
        assert_eq!(
            bridge.read_composed_runtime_session().unwrap(),
            before_resume
        );
    }

    #[test]
    fn composed_owner_restores_when_suspended_post_decision_checkpoint_fails() {
        let mut bridge =
            StaticRuntimeSessionBuilder::activate_project(rulebench_combat_project_input())
                .unwrap()
                .with_gameplay_owner(AdversarialCheckpointOwner::fail_suspended_checkpoint())
                .unwrap()
                .build()
                .unwrap();
        let before = bridge.read_composed_runtime_session().unwrap();

        let error = bridge
            .transact_composed_gameplay_owner(public_decision_moment("checkpoint-failure"))
            .expect_err("post-decision checkpoint failure must reject atomically");
        assert!(error.message.contains("checkpoint"), "{error}");
        assert_eq!(bridge.read_composed_runtime_session().unwrap(), before);
    }

    #[test]
    fn composed_owner_rejects_transaction_checkpoint_identity_drift() {
        let mut bridge =
            StaticRuntimeSessionBuilder::activate_project(rulebench_combat_project_input())
                .unwrap()
                .with_gameplay_owner(AdversarialCheckpointOwner::drift_transaction_checkpoint())
                .unwrap()
                .build()
                .unwrap();

        let suspended = bridge
            .transact_composed_gameplay_owner(public_decision_moment("identity-drift"))
            .unwrap();
        let continuation = suspended.decision.continuation.unwrap();
        let mut resumed = public_decision_moment("identity-drift");
        resumed.workspace = continuation.workspace;
        resumed.resume_token = Some(continuation.token);
        let error = bridge
            .transact_composed_gameplay_owner(resumed)
            .expect_err("transaction checkpoint identity drift must fail closed");
        assert!(error.message.contains("identity"), "{error}");
        assert_eq!(
            bridge
                .read_composed_runtime_session()
                .unwrap()
                .gameplay_owner
                .unwrap()
                .owner,
            decision_owner(),
        );
    }

    #[test]
    fn composed_owner_rejects_identity_mutation_during_commit() {
        let mut bridge =
            StaticRuntimeSessionBuilder::activate_project(rulebench_combat_project_input())
                .unwrap()
                .with_gameplay_owner(AdversarialCheckpointOwner::mutate_identity_during_commit())
                .unwrap()
                .build()
                .unwrap();
        let suspended = bridge
            .transact_composed_gameplay_owner(public_decision_moment("identity-mutation"))
            .unwrap();
        let before = bridge.read_composed_runtime_session().unwrap();
        let continuation = suspended.decision.continuation.unwrap();
        let mut resumed = public_decision_moment("identity-mutation");
        resumed.workspace = continuation.workspace;
        resumed.resume_token = Some(continuation.token);

        let error = bridge
            .transact_composed_gameplay_owner(resumed)
            .expect_err("owner identity mutation during commit must fail closed");
        assert!(error.message.contains("identity"), "{error}");
        assert_eq!(bridge.read_composed_runtime_session().unwrap(), before);
    }

    #[test]
    fn composed_bridge_restores_and_finishes_interrupted_scheduler_delivery() {
        let mut bridge =
            StaticRuntimeSessionBuilder::activate_project(runtime_host_project_input(4))
                .unwrap()
                .build()
                .unwrap();
        let action_id = ScheduledActionId::new("fixture.scheduler.disable-trigger-collision");
        bridge
            .apply_composed_gameplay_scheduler_command(
                GameplayRuntimeSchedulerCommand::ScheduleTick(scheduled_collision_deactivation()),
            )
            .unwrap();
        let dispatched = bridge
            .apply_composed_gameplay_scheduler_command(
                GameplayRuntimeSchedulerCommand::ExecuteTick {
                    action_id: action_id.clone(),
                    tick: 5,
                    validity: ScheduledActionValidity::CURRENT,
                },
            )
            .unwrap();
        assert_eq!(dispatched.readout.outstanding_dispatch_count, 1);

        let checkpoint = bridge.checkpoint_composed_runtime_session().unwrap();
        let mut restored = StaticRuntimeSessionBuilder::restore_project(
            runtime_host_project_input(4),
            &checkpoint,
        )
        .unwrap()
        .build()
        .unwrap();
        assert_eq!(
            restored
                .read_composed_runtime_session()
                .unwrap()
                .gameplay
                .scheduler
                .outstanding_dispatch_count,
            1,
        );
        let routed = restored
            .route_composed_gameplay_scheduled_action(&action_id)
            .unwrap();
        assert!(routed.routing.accepted);
        assert_eq!(routed.readout.outstanding_dispatch_count, 0);
        assert_eq!(routed.readout.outstanding_event_delivery_count, 0);
        assert_eq!(routed.readout.pending_action_count, 0);
        assert!(routed.reaction.unwrap().observe.accepted());
    }

    fn decision_runtime_host_project_input() -> GameplayRuntimeProjectInput {
        let mut input = runtime_host_project_input(4);
        input.composition = decision_composition();
        input.bindings = GameplayModuleBindingRegistryBuilder::new().build();
        input.declared_reads = decision_topology().declared_reads().to_vec();
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

    fn rulebench_combat_project_input() -> GameplayRuntimeProjectInput {
        let mut input = runtime_host_project_input(4);
        let mut composition = GameplayStaticCompositionBuilder::new();
        composition.include_standard_owner_events();
        composition.add_provider(provider(4));
        composition.add_provider(decision_provider());
        input.composition = composition.build().expect("Rulebench-shaped composition");
        input
            .declared_reads
            .extend(decision_topology().declared_reads().iter().cloned());
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

    #[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
    #[serde(rename_all = "camelCase", deny_unknown_fields)]
    struct RulebenchCombatState {
        revision: u64,
        committed_amounts: Vec<u64>,
        replay_steps: Vec<String>,
    }

    struct RulebenchCombatOwner {
        owner: GameplayOwnerRef,
        state: RulebenchCombatState,
        reject_fact_delivery: bool,
        mismatch_after_restore: bool,
    }

    impl RulebenchCombatOwner {
        fn new(reject_fact_delivery: bool) -> Self {
            Self {
                owner: decision_owner(),
                state: RulebenchCombatState::default(),
                reject_fact_delivery,
                mismatch_after_restore: false,
            }
        }

        fn mismatch_after_restore() -> Self {
            Self {
                mismatch_after_restore: true,
                ..Self::new(false)
            }
        }

        fn reaction_fact(
            &self,
            operation: &GameplayProposalEnvelope,
            sequence: u32,
            phase: &str,
            amount: u64,
        ) -> GameplayEventEnvelope {
            let mut event = root_event(amount);
            if phase == "resolved" {
                event.event = contract("pulse-result");
            }
            event.event_id = format!("{}/reaction/{phase}", operation.causation.root_id);
            event.tick = operation.tick;
            event.root_sequence = operation.root_sequence;
            event.wave = operation.wave.saturating_add(1);
            event.event_sequence = sequence;
            event.emitter = GameplayEmitterRef::Owner {
                owner_id: self.owner.owner_id.clone(),
            };
            event.causation = GameplayCausationRef {
                root_id: operation.causation.root_id.clone(),
                parent_event_id: None,
                decision_id: operation.causation.decision_id.clone(),
            };
            event.source = operation.source.clone();
            event.targets = operation.targets.clone();
            event.tags = vec![format!("reaction-{phase}")];
            event
        }
    }

    impl ComposedGameplayOwner for RulebenchCombatOwner {
        fn owner(&self) -> &GameplayOwnerRef {
            &self.owner
        }

        fn revision_hash(&self) -> String {
            format!("revision:{}", self.state.revision)
        }

        fn checkpoint(&self) -> Result<ComposedGameplayOwnerCheckpoint, String> {
            let canonical_state =
                serde_json::to_vec(&self.state).map_err(|error| error.to_string())?;
            let replay_hash = gameplay_module_payload_hash(
                &serde_json::to_vec(&self.state.replay_steps).map_err(|error| error.to_string())?,
            );
            ComposedGameplayOwnerCheckpoint::new(self.owner.clone(), canonical_state, replay_hash)
        }

        fn restore(&mut self, checkpoint: &ComposedGameplayOwnerCheckpoint) -> Result<(), String> {
            if checkpoint.owner() != &self.owner {
                return Err("combat owner identity mismatch".to_owned());
            }
            self.state = serde_json::from_slice(checkpoint.canonical_state())
                .map_err(|error| error.to_string())?;
            if self.mismatch_after_restore {
                self.state.replay_steps.push("restore-mismatch".to_owned());
            }
            Ok(())
        }

        fn route_precommit(
            &mut self,
            operation: &GameplayProposalEnvelope,
        ) -> ComposedGameplayOwnerOutput {
            let workspace: DecisionWorkspace = serde_json::from_slice(&operation.canonical_payload)
                .expect("typed combat Workspace");
            self.state.revision = self.state.revision.saturating_add(1);
            self.state.committed_amounts.push(workspace.amount);
            self.state
                .replay_steps
                .extend(["reaction-opened".to_owned(), "reaction-resolved".to_owned()]);
            let mut events = vec![
                self.reaction_fact(operation, 0, "opened", 1),
                self.reaction_fact(operation, 1, "resolved", workspace.amount),
            ];
            if self.reject_fact_delivery {
                events[1].canonical_payload.push(0);
            }
            ComposedGameplayOwnerOutput {
                accepted: true,
                fact_hashes: vec![operation.payload_hash.clone()],
                diagnostic_codes: Vec::new(),
                events,
            }
        }
    }

    struct AdversarialCheckpointOwner {
        owner: GameplayOwnerRef,
        state: std::cell::Cell<u64>,
        checkpoint_calls: std::cell::Cell<u32>,
        fail_on_call: Option<u32>,
        drift_on_call: Option<u32>,
        mutate_identity_on_route: bool,
    }

    impl AdversarialCheckpointOwner {
        fn fail_suspended_checkpoint() -> Self {
            Self {
                owner: decision_owner(),
                state: std::cell::Cell::new(0),
                checkpoint_calls: std::cell::Cell::new(0),
                fail_on_call: Some(4),
                drift_on_call: None,
                mutate_identity_on_route: false,
            }
        }

        fn drift_transaction_checkpoint() -> Self {
            Self {
                owner: decision_owner(),
                state: std::cell::Cell::new(0),
                checkpoint_calls: std::cell::Cell::new(0),
                fail_on_call: None,
                drift_on_call: Some(4),
                mutate_identity_on_route: false,
            }
        }

        fn mutate_identity_during_commit() -> Self {
            Self {
                owner: decision_owner(),
                state: std::cell::Cell::new(0),
                checkpoint_calls: std::cell::Cell::new(0),
                fail_on_call: None,
                drift_on_call: None,
                mutate_identity_on_route: true,
            }
        }

        fn checkpoint_for(
            &self,
            owner: GameplayOwnerRef,
        ) -> Result<ComposedGameplayOwnerCheckpoint, String> {
            let canonical_state = serde_json::to_vec(&self.state.get()).unwrap();
            ComposedGameplayOwnerCheckpoint::new(
                owner,
                canonical_state,
                format!("replay:{}", self.state.get()),
            )
        }
    }

    impl ComposedGameplayOwner for AdversarialCheckpointOwner {
        fn owner(&self) -> &GameplayOwnerRef {
            &self.owner
        }

        fn revision_hash(&self) -> String {
            "revision:0".to_owned()
        }

        fn checkpoint(&self) -> Result<ComposedGameplayOwnerCheckpoint, String> {
            let call = self.checkpoint_calls.get().saturating_add(1);
            self.checkpoint_calls.set(call);
            if self.fail_on_call == Some(call) {
                self.state.set(99);
                return Err("adversarial post-decision checkpoint failure".to_owned());
            }
            let owner = if self.drift_on_call == Some(call) {
                GameplayOwnerRef {
                    owner_id: "authority.foreign".to_owned(),
                    provider_id: "provider.foreign".to_owned(),
                }
            } else {
                self.owner.clone()
            };
            self.checkpoint_for(owner)
        }

        fn restore(&mut self, checkpoint: &ComposedGameplayOwnerCheckpoint) -> Result<(), String> {
            self.owner = checkpoint.owner().clone();
            self.state.set(
                serde_json::from_slice(checkpoint.canonical_state())
                    .map_err(|error| error.to_string())?,
            );
            Ok(())
        }

        fn route_precommit(
            &mut self,
            _operation: &GameplayProposalEnvelope,
        ) -> ComposedGameplayOwnerOutput {
            if self.mutate_identity_on_route {
                self.owner = GameplayOwnerRef {
                    owner_id: "authority.foreign".to_owned(),
                    provider_id: "provider.foreign".to_owned(),
                };
            }
            ComposedGameplayOwnerOutput {
                accepted: true,
                ..ComposedGameplayOwnerOutput::default()
            }
        }
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
