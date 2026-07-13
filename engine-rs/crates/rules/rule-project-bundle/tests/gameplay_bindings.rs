use core_entity::{Aabb, EntityLifecycleCommand, EntitySource, EntityTransform, TransformCommand};
use core_ids::{
    EntityId, PrefabId, PrefabInstanceId, PrefabPartId, RuntimeSessionId, SceneId, SceneNodeId,
};
use core_math::Vec3;
use core_scene::{encode, SceneMetadata, SceneNode, SceneNodeKind, SceneTree};
use gameplay_module_sdk::*;
use protocol_game_extension::{
    GameplayModuleBinding, GameplayModuleBindingDiagnosticCode, GameplayModuleBindingOverride,
    GameplayModuleBindingRegistry, GameplayModuleBindingTarget, GameplayModuleConfiguration,
    PrefabPartReference as ProtocolPrefabPartReference, GAMEPLAY_MODULE_BINDING_SCHEMA_VERSION,
};
use rule_project_bundle::*;
use svc_serialization::{
    LoadPlan, LoadStep, PrefabDefinition, PrefabInstanceRecord, PrefabPart, PrefabPartReference,
    PrefabPartRoleBinding, PrefabPartSource, PrefabRegistry, PrefabRegistryValidationContext,
    PrefabTransform, ValidatedPrefabRegistry, PREFAB_DEFINITION_SCHEMA_VERSION,
    PREFAB_REGISTRY_SCHEMA_VERSION,
};

const MODULE_ID: &str = "game.binding-fixture.module";
const NAMESPACE: &str = "game.binding-fixture";

fn contract(name: &str) -> GameplayContractRef {
    GameplayContractRef {
        namespace: NAMESPACE.to_owned(),
        name: name.to_owned(),
        version: 1,
        schema_hash: format!("sha256:{NAMESPACE}.{name}"),
    }
}

fn owner() -> GameplayOwnerRef {
    GameplayOwnerRef {
        owner_id: "authority.binding-fixture".to_owned(),
        provider_id: "provider.binding-fixture".to_owned(),
    }
}

fn module_ref() -> GameplayModuleRef {
    GameplayModuleRef {
        module_id: MODULE_ID.to_owned(),
        namespace: NAMESPACE.to_owned(),
        version: "1.0.0".to_owned(),
        sdk_hash: "sha256:sdk-v1".to_owned(),
        contract_hash: "sha256:binding-contract".to_owned(),
        artifact_hash: "sha256:binding-artifact".to_owned(),
        provider_id: owner().provider_id,
    }
}

struct FixtureBehavior;

impl GameplayModuleBehavior for FixtureBehavior {
    fn invoke(
        &self,
        context: &GameplayModuleContext<'_>,
    ) -> Result<GameplayModuleActions, GameplayModuleError> {
        let value: u64 = context.event_payload()?;
        let configuration: CounterConfiguration = context.configuration()?;
        let mut actions = context.actions();
        actions.emit_json(
            contract("result"),
            &value.saturating_mul(configuration.multiplier),
            context.source(),
            Vec::new(),
            context.target(0).into_iter().collect(),
        )?;
        Ok(actions)
    }
}

struct CounterAdapter;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CounterConfiguration {
    multiplier: u64,
}

impl GameplayTypedModuleStateAdapter for CounterAdapter {
    type Config = CounterConfiguration;
    type State = u64;
    type Fact = u64;
    type View = u64;

    fn module_id(&self) -> &str {
        MODULE_ID
    }

    fn state_schema(&self) -> &GameplayContractRef {
        static VALUE: std::sync::OnceLock<GameplayContractRef> = std::sync::OnceLock::new();
        VALUE.get_or_init(|| contract("state"))
    }

    fn fact_schema(&self) -> &GameplayContractRef {
        static VALUE: std::sync::OnceLock<GameplayContractRef> = std::sync::OnceLock::new();
        VALUE.get_or_init(|| contract("fact"))
    }

    fn owner(&self) -> &GameplayOwnerRef {
        static VALUE: std::sync::OnceLock<GameplayOwnerRef> = std::sync::OnceLock::new();
        VALUE.get_or_init(owner)
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
}

fn composition() -> GameplayStaticComposition {
    let owner = owner();
    let manifest = GameplayModuleManifest {
        module_ref: module_ref(),
        published_events: vec![
            GameplayEventSchemaDeclaration {
                event: contract("root"),
                codec_id: "codec.binding-fixture.root".to_owned(),
            },
            GameplayEventSchemaDeclaration {
                event: contract("result"),
                codec_id: "codec.binding-fixture.result".to_owned(),
            },
        ],
        subscriptions: vec![GameplaySubscriptionDeclaration {
            subscription_id: "binding-fixture.observe".to_owned(),
            event: contract("root"),
            invocation_id: "binding-fixture.observe".to_owned(),
            selector: GameplayHeaderSelector {
                source: None,
                target: None,
                scope: None,
                required_tags: Vec::new(),
            },
            max_deliveries_per_root: 2,
        }],
        invocations: vec![GameplayInvocationDescriptor {
            invocation_id: "binding-fixture.observe".to_owned(),
            family: GameplayInvocationFamily::Observe,
            input_contract: contract("root"),
            output_contract: contract("result"),
            read_requirements: Vec::new(),
            max_outputs: 1,
            max_payload_bytes: 1_024,
        }],
        read_views: Vec::new(),
        proposal_kinds: Vec::new(),
        state_schemas: vec![GameplayOwnedSchemaDeclaration {
            schema: contract("state"),
            owner: owner.clone(),
        }],
        fact_schemas: vec![GameplayOwnedSchemaDeclaration {
            schema: contract("fact"),
            owner: owner.clone(),
        }],
        ordering: Vec::new(),
        budget: GameplayExecutionBudget {
            max_waves: 2,
            max_events_per_root: 1,
            max_proposals_per_root: 1,
            max_invocations_per_root: 1,
            max_payload_bytes_per_root: 1_024,
        },
        deterministic_requirements: vec!["canonical-json".to_owned()],
        source_hash: "sha256:binding-source".to_owned(),
    };
    let configuration_metadata = GameplayConfigurationSchemaMetadata {
        module_id: MODULE_ID.to_owned(),
        configuration: contract("configuration"),
        codec_id: "codec.binding-fixture.configuration".to_owned(),
        fields: vec![GameplayConfigurationFieldMetadata {
            name: "multiplier".to_owned(),
            value_type: "u64".to_owned(),
            required: true,
        }],
    };
    let provider = GameplayStaticModuleProvider::linked_from_manifest(manifest, FixtureBehavior)
        .event_codec(json_u64_codec(
            contract("root"),
            "codec.binding-fixture.root",
        ))
        .event_codec(json_u64_codec(
            contract("result"),
            "codec.binding-fixture.result",
        ))
        .state_owner(GameplayStateOwnerRegistration {
            schema: contract("state"),
            owner: owner.clone(),
        })
        .state_owner(GameplayStateOwnerRegistration {
            schema: contract("fact"),
            owner,
        })
        .state_adapter(GameplayModuleStateRegistration::typed(CounterAdapter))
        .configuration_schema(configuration_metadata.clone())
        .configuration_codec(GameplayConfigurationCodecRegistration::typed::<
            CounterConfiguration,
        >(configuration_metadata));
    let mut builder = GameplayStaticCompositionBuilder::new();
    builder.include_standard_owner_events();
    builder.add_provider(provider);
    builder.build().unwrap()
}

fn json_u64_codec(event: GameplayContractRef, codec_id: &str) -> GameplayEventCodecRegistration {
    GameplayEventCodecRegistration::typed(TypedGameplayEventCodec::new(
        GameplayEventSchemaDeclaration {
            event,
            codec_id: codec_id.to_owned(),
        },
        |value: &u64| serde_json::to_vec(value).map_err(|error| error.to_string()),
        |bytes| serde_json::from_slice(bytes).map_err(|error| error.to_string()),
    ))
}

fn root_event(value: u64) -> GameplayEventEnvelope {
    root_event_for(value, None, "binding-root")
}

fn root_event_for(value: u64, target: Option<EntityId>, event_id: &str) -> GameplayEventEnvelope {
    let canonical_payload = serde_json::to_vec(&value).unwrap();
    GameplayEventEnvelope {
        event_id: event_id.to_owned(),
        event: contract("root"),
        tick: 0,
        root_sequence: 0,
        wave: 0,
        event_sequence: 0,
        phase: GameplayEventPhase::PostCommit,
        emitter: GameplayEmitterRef::Owner {
            owner_id: "authority.binding-test".to_owned(),
        },
        causation: GameplayCausationRef {
            root_id: event_id.to_owned(),
            parent_event_id: None,
            decision_id: None,
        },
        source: None,
        subjects: Vec::new(),
        targets: target
            .map(|entity| GameplayEntityRef { entity })
            .into_iter()
            .collect(),
        scope: None,
        tags: Vec::new(),
        payload_hash: gameplay_module_payload_hash(&canonical_payload),
        canonical_payload,
    }
}

fn loaded_bundle() -> ProjectBundleLoadResult {
    let scene = SceneTree {
        id: SceneId::new(90),
        schema_version: 1,
        metadata: SceneMetadata {
            name: Some("binding-stage".into()),
            authoring_format_version: 1,
        },
        dependencies: vec![],
        roots: vec![SceneNode::leaf(
            SceneNodeId::new(1),
            SceneNodeKind::EmptyGroup,
        )],
    };
    let plan = LoadPlan {
        steps: vec![
            LoadStep::ValidateVersions {
                bundle_schema_version: 1,
                protocol_version: 1,
            },
            LoadStep::LoadAssetLock {
                artifact: "assets/lock.json".into(),
                asset_count: 0,
            },
            LoadStep::LoadSceneDocument {
                artifact: "scene/scene.json".into(),
                scene: SceneId::new(90),
            },
            LoadStep::BootstrapScene {
                scene: SceneId::new(90),
                runtime_session: RuntimeSessionId::new(5),
            },
            LoadStep::ValidateFinalState,
        ],
    };
    let artifacts = BundleArtifacts::new()
        .with_artifact("assets/lock.json", "{ \"entries\": [] }\n")
        .with_artifact("scene/scene.json", encode(&scene.to_flat()));
    let mut bundle = execute_load_plan(&plan, &artifacts).unwrap();
    let prefab = PrefabDefinition {
        id: PrefabId::new(10),
        schema_version: PREFAB_DEFINITION_SCHEMA_VERSION,
        display_name: "Turret".to_owned(),
        parts: vec![PrefabPart {
            id: PrefabPartId::new(1),
            namespace: "weapon/muzzle".to_owned(),
            display_name: "Muzzle".to_owned(),
            parent: None,
            transform: PrefabTransform::IDENTITY,
            source: PrefabPartSource::EntityDefinition {
                stable_id: "weapon.muzzle".to_owned(),
            },
        }],
        part_roles: vec![PrefabPartRoleBinding {
            role: "weapon/muzzle".to_owned(),
            part: PrefabPartId::new(1),
        }],
        variant: None,
    };
    let context = PrefabRegistryValidationContext {
        entity_definition_ids: ["weapon.muzzle".to_owned()].into_iter().collect(),
        ..Default::default()
    };
    let registry = ValidatedPrefabRegistry::new(
        PrefabRegistry {
            schema_version: PREFAB_REGISTRY_SCHEMA_VERSION,
            definitions: vec![prefab],
        },
        &context,
    )
    .unwrap();
    let catalog = PrefabInstantiationCatalog::from(&context);
    let entities = bundle.runtime_entities.get_or_insert_default();
    bundle
        .prefab_instances
        .instantiate(
            entities,
            &registry,
            &catalog,
            InstantiatePrefabCommand {
                command_id: "place-turret".to_owned(),
                origin: PrefabPlacementOrigin::Authored,
                record: PrefabInstanceRecord {
                    instance: PrefabInstanceId::new(20),
                    prefab: PrefabId::new(10),
                    seed: 4,
                    transform: PrefabTransform::IDENTITY,
                    overrides: Vec::new(),
                },
            },
        )
        .unwrap();
    bundle
        .prefab_instances
        .instantiate(
            entities,
            &registry,
            &catalog,
            InstantiatePrefabCommand {
                command_id: "place-second-turret".to_owned(),
                origin: PrefabPlacementOrigin::Authored,
                record: PrefabInstanceRecord {
                    instance: PrefabInstanceId::new(21),
                    prefab: PrefabId::new(10),
                    seed: 5,
                    transform: PrefabTransform::IDENTITY,
                    overrides: Vec::new(),
                },
            },
        )
        .unwrap();
    assert!(bundle
        .prefab_instances
        .resolve_part(
            PrefabInstanceId::new(20),
            &PrefabPartReference {
                prefab: PrefabId::new(10),
                role: "weapon/muzzle".to_owned(),
            },
        )
        .is_some());
    bundle
}

fn configuration(id: &str, value: u64) -> GameplayModuleConfiguration {
    let canonical_config = serde_json::to_vec(&CounterConfiguration { multiplier: value }).unwrap();
    GameplayModuleConfiguration {
        configuration_id: id.to_owned(),
        module: module_ref(),
        configuration: contract("configuration"),
        codec_id: "codec.binding-fixture.configuration".to_owned(),
        config_hash: gameplay_module_payload_hash(&canonical_config),
        canonical_config,
    }
}

fn bindings() -> GameplayModuleBindingRegistry {
    let mut registry = GameplayModuleBindingRegistry {
        schema_version: GAMEPLAY_MODULE_BINDING_SCHEMA_VERSION,
        configurations: vec![configuration("default", 4), configuration("turret-20", 9)],
        bindings: vec![
            GameplayModuleBinding {
                binding_id: "session-counter".to_owned(),
                module_id: MODULE_ID.to_owned(),
                configuration_id: "default".to_owned(),
                state_schema: contract("state"),
                target: GameplayModuleBindingTarget::Session,
                required_reads: Vec::new(),
                output_contracts: vec![contract("result")],
                enabled: true,
            },
            GameplayModuleBinding {
                binding_id: "muzzle-counter".to_owned(),
                module_id: MODULE_ID.to_owned(),
                configuration_id: "default".to_owned(),
                state_schema: contract("state"),
                target: GameplayModuleBindingTarget::PrefabPart {
                    part: ProtocolPrefabPartReference {
                        prefab: PrefabId::new(10),
                        role: "weapon/muzzle".to_owned(),
                    },
                },
                required_reads: Vec::new(),
                output_contracts: vec![contract("result")],
                enabled: true,
            },
        ],
        overrides: vec![GameplayModuleBindingOverride {
            binding_id: "muzzle-counter".to_owned(),
            prefab_instance: PrefabInstanceId::new(20),
            configuration_id: Some("turret-20".to_owned()),
            enabled: None,
        }],
        registry_hash: String::new(),
    };
    registry.registry_hash = gameplay_module_binding_registry_hash(&registry);
    registry
}

#[test]
fn bindings_activate_atomic_facets_and_round_trip_against_project_bundle_authority() {
    let bundle = loaded_bundle();
    let bindings = bindings();
    let session = GameplayBoundProjectBundleSession::activate(
        bundle.clone(),
        composition(),
        bindings.clone(),
        &GameplayBindingEntityTargets::new(),
    )
    .unwrap();
    assert_eq!(session.activation.readouts.len(), 3);
    let execution = session.observe_session_event(root_event(6));
    assert!(execution.accepted(), "{:?}", execution.diagnostics);
    assert_eq!(execution.invocations.len(), 1);
    assert_eq!(execution.events.len(), 2);
    let session_record = session
        .module_state
        .record(&contract("state"), &GameplayModuleStateScope::Session)
        .unwrap();
    assert_eq!(
        session_record.state_hash,
        gameplay_module_payload_hash(&serde_json::to_vec(&4_u64).unwrap())
    );
    let muzzle = bundle
        .prefab_instances
        .resolve_part(
            PrefabInstanceId::new(20),
            &PrefabPartReference {
                prefab: PrefabId::new(10),
                role: "weapon/muzzle".to_owned(),
            },
        )
        .unwrap();
    let muzzle_record = session
        .module_state
        .record(
            &contract("state"),
            &GameplayModuleStateScope::Entity {
                entity: muzzle.entity.raw(),
            },
        )
        .unwrap();
    assert_eq!(
        muzzle_record.state_hash,
        gameplay_module_payload_hash(&serde_json::to_vec(&9_u64).unwrap())
    );
    let second_muzzle = bundle
        .prefab_instances
        .resolve_part(
            PrefabInstanceId::new(21),
            &PrefabPartReference {
                prefab: PrefabId::new(10),
                role: "weapon/muzzle".to_owned(),
            },
        )
        .unwrap();
    let second_record = session
        .module_state
        .record(
            &contract("state"),
            &GameplayModuleStateScope::Entity {
                entity: second_muzzle.entity.raw(),
            },
        )
        .unwrap();
    assert_eq!(
        second_record.state_hash,
        gameplay_module_payload_hash(&serde_json::to_vec(&4_u64).unwrap())
    );
    let overridden = session.observe_session_event(root_event_for(
        6,
        Some(muzzle.entity),
        "binding-root-overridden",
    ));
    let base = session.observe_session_event(root_event_for(
        6,
        Some(second_muzzle.entity),
        "binding-root-base",
    ));
    assert_eq!(
        serde_json::from_slice::<u64>(&overridden.events[1].canonical_payload).unwrap(),
        54
    );
    assert_eq!(
        serde_json::from_slice::<u64>(&base.events[1].canonical_payload).unwrap(),
        24
    );
    assert_eq!(
        overridden.invocations[0]
            .configuration
            .as_ref()
            .unwrap()
            .configuration_id,
        "turret-20"
    );
    assert_eq!(
        base.invocations[0]
            .configuration
            .as_ref()
            .unwrap()
            .configuration_id,
        "default"
    );
    let before_hash = session.module_state.state_hash();
    let artifact = session.compose_gameplay_session_snapshot().unwrap();
    let restored = GameplayBoundProjectBundleSession::restore(
        bundle,
        composition(),
        bindings,
        &GameplayBindingEntityTargets::new(),
        &artifact.text,
    )
    .unwrap();
    assert_eq!(restored.module_state.state_hash(), before_hash);
    assert_eq!(restored.activation, session.activation);
}

#[test]
fn runtime_session_reconciles_persists_and_restores_trigger_overlap_authority() {
    use rule_trigger_volume::{
        KinematicTriggerDefinition, TriggerOverlapFactKind, TriggerReconcileCause,
    };

    let mut bundle = loaded_bundle();
    let entities = bundle.runtime_entities.get_or_insert_default();
    for id in [100, 101] {
        entities
            .apply(EntityLifecycleCommand::Create {
                id: EntityId::new(id),
                source: EntitySource::RuntimeCreated { by: None },
                labels: Vec::new(),
            })
            .unwrap();
        entities.attach_transform(EntityId::new(id), EntityTransform::IDENTITY);
        entities.attach_bounds(
            EntityId::new(id),
            Aabb::new(Vec3::splat(-0.5), Vec3::splat(0.5)),
        );
        entities.attach_collision(EntityId::new(id), id == 100);
    }

    let registry = bindings();
    let mut session = GameplayBoundProjectBundleSession::activate(
        bundle.clone(),
        composition(),
        registry.clone(),
        &GameplayBindingEntityTargets::new(),
    )
    .unwrap();
    session
        .install_trigger_definitions([KinematicTriggerDefinition::new(
            EntityId::new(100),
            "zone.runtime-session",
            ["door", "zone"],
        )])
        .unwrap();
    let entered = session
        .reconcile_triggers(8, TriggerReconcileCause::Spawn)
        .unwrap();
    assert_eq!(entered.collision.facts.len(), 1);
    assert_eq!(
        entered.collision.facts[0].kind,
        TriggerOverlapFactKind::Enter
    );
    assert_eq!(entered.gameplay_events.len(), 1);
    assert_eq!(
        entered.gameplay_events[0].event,
        StandardGameplayEventKind::TriggerEntered.contract()
    );
    assert!(entered.reactions[0].accepted());

    let before_hash = entered.collision.overlap_hash;
    let artifact = session.compose_gameplay_session_snapshot().unwrap();
    let mut restored = GameplayBoundProjectBundleSession::restore(
        bundle,
        composition(),
        registry,
        &GameplayBindingEntityTargets::new(),
        &artifact.text,
    )
    .unwrap();
    let unchanged = restored
        .reconcile_triggers(9, TriggerReconcileCause::Restore)
        .unwrap();
    assert!(unchanged.collision.facts.is_empty());
    assert_eq!(unchanged.collision.overlap_hash, before_hash);

    restored
        .bundle
        .runtime_entities
        .as_mut()
        .unwrap()
        .apply_transform(TransformCommand::Set {
            id: EntityId::new(101),
            transform: EntityTransform::at(Vec3::new(5.0, 0.0, 0.0)),
        })
        .unwrap();
    let exited = restored
        .reconcile_triggers(10, TriggerReconcileCause::Teleport)
        .unwrap();
    assert_eq!(exited.collision.facts.len(), 1);
    assert_eq!(exited.collision.facts[0].kind, TriggerOverlapFactKind::Exit);
    assert!(exited.collision.active_overlaps.is_empty());
}

#[test]
fn provider_drift_and_corrupt_snapshots_fail_closed() {
    let bundle = loaded_bundle();
    let mut drifted = bindings();
    drifted.configurations[0].module.artifact_hash = "sha256:other-code".to_owned();
    drifted.registry_hash = gameplay_module_binding_registry_hash(&drifted);
    let error = GameplayBoundProjectBundleSession::activate(
        bundle.clone(),
        composition(),
        drifted,
        &GameplayBindingEntityTargets::new(),
    )
    .err()
    .unwrap();
    assert!(matches!(
        error,
        GameplayBindingActivationError::Invalid { diagnostics }
            if diagnostics.iter().any(|item| item.code == GameplayModuleBindingDiagnosticCode::ProviderMismatch)
    ));

    let bindings = bindings();
    let session = GameplayBoundProjectBundleSession::activate(
        bundle.clone(),
        composition(),
        bindings.clone(),
        &GameplayBindingEntityTargets::new(),
    )
    .unwrap();
    let artifact = session.compose_gameplay_session_snapshot().unwrap();
    let mut stored: serde_json::Value = serde_json::from_str(&artifact.text).unwrap();
    stored["snapshotHash"] = serde_json::json!("tampered");
    assert!(matches!(
        GameplayBoundProjectBundleSession::restore(
            bundle,
            composition(),
            bindings,
            &GameplayBindingEntityTargets::new(),
            &serde_json::to_string(&stored).unwrap(),
        ),
        Err(GameplayBindingActivationError::Snapshot(_))
    ));
}

fn assert_binding_diagnostic(
    bundle: &ProjectBundleLoadResult,
    mut bindings: GameplayModuleBindingRegistry,
    code: GameplayModuleBindingDiagnosticCode,
) {
    bindings.registry_hash = gameplay_module_binding_registry_hash(&bindings);
    let error = GameplayBoundProjectBundleSession::activate(
        bundle.clone(),
        composition(),
        bindings,
        &GameplayBindingEntityTargets::new(),
    )
    .err()
    .expect("invalid binding must not construct a Session");
    assert!(matches!(
        error,
        GameplayBindingActivationError::Invalid { diagnostics }
            if diagnostics.iter().any(|item| item.code == code)
    ));
}

#[test]
fn stale_contracts_foreign_modules_bad_roles_reads_outputs_and_overrides_reject() {
    let bundle = loaded_bundle();

    let mut stale_configuration = bindings();
    stale_configuration.configurations[0].configuration = contract("old-configuration");
    assert_binding_diagnostic(
        &bundle,
        stale_configuration,
        GameplayModuleBindingDiagnosticCode::ConfigurationSchemaMismatch,
    );

    let mut foreign_module = bindings();
    foreign_module.bindings[0].module_id = "game.foreign.module".to_owned();
    assert_binding_diagnostic(
        &bundle,
        foreign_module,
        GameplayModuleBindingDiagnosticCode::ModuleMismatch,
    );

    let mut bad_role = bindings();
    bad_role.bindings[1].target = GameplayModuleBindingTarget::PrefabPart {
        part: ProtocolPrefabPartReference {
            prefab: PrefabId::new(10),
            role: "missing/role".to_owned(),
        },
    };
    assert_binding_diagnostic(
        &bundle,
        bad_role,
        GameplayModuleBindingDiagnosticCode::UnresolvedTarget,
    );

    let mut undeclared_read = bindings();
    undeclared_read.bindings[0]
        .required_reads
        .push(GameplayReadViewRequirement {
            view: contract("private-view"),
            provider_id: "provider.binding-fixture".to_owned(),
            kind: GameplayReadViewKind::ModuleNamed,
            fields: vec!["value".to_owned()],
            selector_capabilities: vec![GameplayReadSelectorCapability::ModuleStateScope],
            max_items: 1,
        });
    assert_binding_diagnostic(
        &bundle,
        undeclared_read,
        GameplayModuleBindingDiagnosticCode::ReadContractMismatch,
    );

    let mut undeclared_output = bindings();
    undeclared_output.bindings[0].output_contracts = vec![contract("private-output")];
    assert_binding_diagnostic(
        &bundle,
        undeclared_output,
        GameplayModuleBindingDiagnosticCode::OutputContractMismatch,
    );

    let mut malformed_override = bindings();
    malformed_override.overrides[0].prefab_instance = PrefabInstanceId::new(999);
    assert_binding_diagnostic(
        &bundle,
        malformed_override,
        GameplayModuleBindingDiagnosticCode::InvalidOverride,
    );

    let mut malformed_config = bindings();
    malformed_config.configurations[0].canonical_config = b"not-json".to_vec();
    malformed_config.configurations[0].config_hash =
        gameplay_module_payload_hash(&malformed_config.configurations[0].canonical_config);
    assert_binding_diagnostic(
        &bundle,
        malformed_config,
        GameplayModuleBindingDiagnosticCode::ConfigurationSchemaMismatch,
    );

    for malformed in [
        serde_json::json!({}),
        serde_json::json!({"multiplier": "nine"}),
        serde_json::json!({"multiplier": 9, "unownedField": true}),
    ] {
        let mut bindings = bindings();
        bindings.configurations[0].canonical_config = serde_json::to_vec(&malformed).unwrap();
        bindings.configurations[0].config_hash =
            gameplay_module_payload_hash(&bindings.configurations[0].canonical_config);
        assert_binding_diagnostic(
            &bundle,
            bindings,
            GameplayModuleBindingDiagnosticCode::ConfigurationSchemaMismatch,
        );
    }
}
