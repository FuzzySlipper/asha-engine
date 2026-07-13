use protocol_game_extension::{
    GameplayContractRef, GameplayExecutionBudget, GameplayModuleManifest, GameplayModuleRef,
    GameplayOwnedSchemaDeclaration, GameplayOwnerRef, GameplayReadSelectorCapability,
    GameplayReadViewKind, GameplayReadViewRequirement,
};
use rule_gameplay_fabric::{
    gameplay_module_payload_hash, verify_reaction_frame, FrozenGameplayViews, GameplayModuleFact,
    GameplayModuleInitialization, GameplayModuleStateError, GameplayModuleStateMigration,
    GameplayModuleStateRegistration, GameplayModuleStateScope, GameplayModuleStateStore,
    GameplayObserveReceipt, GameplayReactionDivergence, GameplayReactionFrame,
    GameplayReactionSourceFact, GameplayTypedModuleStateAdapter, GameplayWaveBarrierEvidence,
    GameplayWaveStateHashes,
};
use std::sync::Arc;
use svc_gameplay_fabric::{
    gameplay_contract, stable_identity, GameplayFabricRegistry, GameplayFabricRegistryBuilder,
    GameplayLinkedProvider, GameplayReadViewProviderRegistration, GameplayStateOwnerRegistration,
};

fn contract(name: &str, version: u32) -> GameplayContractRef {
    gameplay_contract(
        "game.counter",
        name,
        version,
        &format!("fixture:game.counter.{name}.v{version};canonical-json-v1"),
    )
}

fn state_schema() -> GameplayContractRef {
    contract("counter-state", 2)
}

fn fact_schema() -> GameplayContractRef {
    contract("increment-fact", 1)
}

fn view_schema() -> GameplayContractRef {
    contract("counter-view", 1)
}

fn owner() -> GameplayOwnerRef {
    GameplayOwnerRef {
        owner_id: "authority.game-counter".to_owned(),
        provider_id: "provider.game-counter".to_owned(),
    }
}

fn registry() -> GameplayFabricRegistry {
    let module = GameplayModuleManifest {
        module_ref: GameplayModuleRef {
            module_id: "game.counter-module".to_owned(),
            namespace: "game.counter".to_owned(),
            version: "1.0.0".to_owned(),
            sdk_hash: stable_identity(["counter", "sdk"]),
            contract_hash: stable_identity(["counter", "contract"]),
            artifact_hash: stable_identity(["counter", "artifact"]),
            provider_id: "provider.game-counter".to_owned(),
        },
        published_events: Vec::new(),
        subscriptions: Vec::new(),
        invocations: Vec::new(),
        read_views: vec![GameplayReadViewRequirement {
            view: view_schema(),
            provider_id: "provider.game-counter".to_owned(),
            kind: GameplayReadViewKind::ModuleNamed,
            fields: vec!["value".to_owned()],
            selector_capabilities: vec![GameplayReadSelectorCapability::ModuleStateScope],
            max_items: 1,
        }],
        proposal_kinds: Vec::new(),
        state_schemas: vec![GameplayOwnedSchemaDeclaration {
            schema: state_schema(),
            owner: owner(),
        }],
        fact_schemas: vec![GameplayOwnedSchemaDeclaration {
            schema: fact_schema(),
            owner: owner(),
        }],
        ordering: Vec::new(),
        budget: GameplayExecutionBudget {
            max_waves: 4,
            max_events_per_root: 16,
            max_proposals_per_root: 16,
            max_invocations_per_root: 16,
            max_payload_bytes_per_root: 16_384,
        },
        deterministic_requirements: vec!["canonical-state".to_owned()],
        source_hash: stable_identity(["counter", "source"]),
    };
    let provider = GameplayLinkedProvider {
        provider_id: module.module_ref.provider_id.clone(),
        module_id: module.module_ref.module_id.clone(),
        version: module.module_ref.version.clone(),
        contract_hash: module.module_ref.contract_hash.clone(),
        artifact_hash: module.module_ref.artifact_hash.clone(),
        sdk_hash: module.module_ref.sdk_hash.clone(),
        source_hash: module.source_hash.clone(),
    };
    let mut builder = GameplayFabricRegistryBuilder::new();
    builder
        .register_linked_provider(provider)
        .register_state_owner(GameplayStateOwnerRegistration {
            schema: state_schema(),
            owner: owner(),
        })
        .register_state_owner(GameplayStateOwnerRegistration {
            schema: fact_schema(),
            owner: owner(),
        })
        .register_read_view_provider(GameplayReadViewProviderRegistration {
            view: view_schema(),
            provider_id: "provider.game-counter".to_owned(),
            kind: GameplayReadViewKind::ModuleNamed,
            fields: vec!["value".to_owned()],
            selector_capabilities: vec![GameplayReadSelectorCapability::ModuleStateScope],
            max_items: 1,
            ordering: "singleValue".to_owned(),
        })
        .register_module(module);
    builder.build().expect("state registry")
}

struct CounterAdapter {
    state_schema: GameplayContractRef,
}

impl GameplayTypedModuleStateAdapter for CounterAdapter {
    type Config = u64;
    type State = u64;
    type Fact = u64;
    type View = serde_json::Value;

    fn module_id(&self) -> &str {
        "game.counter-module"
    }

    fn state_schema(&self) -> &GameplayContractRef {
        &self.state_schema
    }

    fn fact_schema(&self) -> &GameplayContractRef {
        static VALUE: std::sync::OnceLock<GameplayContractRef> = std::sync::OnceLock::new();
        VALUE.get_or_init(fact_schema)
    }

    fn owner(&self) -> &GameplayOwnerRef {
        static VALUE: std::sync::OnceLock<GameplayOwnerRef> = std::sync::OnceLock::new();
        VALUE.get_or_init(owner)
    }

    fn decode_config(&self, canonical_config: &[u8]) -> Result<Self::Config, String> {
        serde_json::from_slice(canonical_config).map_err(|error| error.to_string())
    }

    fn decode_state(&self, canonical_state: &[u8]) -> Result<Self::State, String> {
        serde_json::from_slice(canonical_state).map_err(|error| error.to_string())
    }

    fn decode_fact(&self, canonical_fact: &[u8]) -> Result<Self::Fact, String> {
        serde_json::from_slice(canonical_fact).map_err(|error| error.to_string())
    }

    fn encode_state(&self, state: &Self::State) -> Result<Vec<u8>, String> {
        serde_json::to_vec(state).map_err(|error| error.to_string())
    }

    fn initialize(&self, config: &Self::Config) -> Result<Self::State, String> {
        Ok(*config)
    }

    fn apply_fact(
        &self,
        state: &Self::State,
        increment: &Self::Fact,
    ) -> Result<Self::State, String> {
        Ok(state.saturating_add(*increment))
    }

    fn migrate(&self, from_version: u32, state: &Self::State) -> Result<Self::State, String> {
        if from_version != 1 {
            return Err("unsupported source version".to_owned());
        }
        Ok(state.saturating_add(100))
    }

    fn view_schema(&self) -> Option<&GameplayContractRef> {
        static VALUE: std::sync::OnceLock<GameplayContractRef> = std::sync::OnceLock::new();
        Some(VALUE.get_or_init(view_schema))
    }

    fn project_view(&self, state: &Self::State) -> Result<Self::View, String> {
        Ok(serde_json::json!({ "value": state }))
    }

    fn encode_view(&self, view: &Self::View) -> Result<Vec<u8>, String> {
        serde_json::to_vec(view).map_err(|error| error.to_string())
    }
}

fn adapters() -> Vec<GameplayModuleStateRegistration> {
    vec![GameplayModuleStateRegistration::typed(CounterAdapter {
        state_schema: state_schema(),
    })]
}

fn counter_value(store: &GameplayModuleStateStore, scope: &GameplayModuleStateScope) -> u64 {
    let view = store.named_view(&state_schema(), scope).unwrap();
    serde_json::from_slice::<serde_json::Value>(&view.canonical_payload).unwrap()["value"]
        .as_u64()
        .unwrap()
}

fn initialization(
    id: &str,
    scope: GameplayModuleStateScope,
    value: &[u8],
) -> GameplayModuleInitialization {
    GameplayModuleInitialization {
        initialization_id: id.to_owned(),
        module_id: "game.counter-module".to_owned(),
        state_schema: state_schema(),
        scope,
        canonical_config: value.to_vec(),
        config_hash: gameplay_module_payload_hash(value),
    }
}

fn fact(
    id: &str,
    scope: GameplayModuleStateScope,
    revision: u64,
    value: &[u8],
) -> GameplayModuleFact {
    GameplayModuleFact {
        fact_id: id.to_owned(),
        module_id: "game.counter-module".to_owned(),
        fact_schema: fact_schema(),
        state_schema: state_schema(),
        scope,
        expected_revision: revision,
        canonical_payload: value.to_vec(),
        payload_hash: gameplay_module_payload_hash(value),
    }
}

fn migration(
    id: &str,
    scope: GameplayModuleStateScope,
    value: &[u8],
) -> GameplayModuleStateMigration {
    GameplayModuleStateMigration {
        migration_id: id.to_owned(),
        module_id: "game.counter-module".to_owned(),
        from_state_schema: contract("counter-state", 1),
        to_state_schema: state_schema(),
        scope,
        source_revision: 4,
        canonical_state: value.to_vec(),
        state_hash: gameplay_module_payload_hash(value),
        initialized_from: "legacy/project-default".to_owned(),
    }
}

#[test]
fn typed_fact_updates_owned_state_and_rejects_foreign_or_stale_writes() {
    let registry = Arc::new(registry());
    assert!(matches!(
        GameplayModuleStateStore::new(
            registry.clone(),
            vec![GameplayModuleStateRegistration::typed(CounterAdapter {
                state_schema: contract("unregistered-counter-state", 1),
            })],
        ),
        Err(GameplayModuleStateError::UndeclaredState)
    ));
    let scope = GameplayModuleStateScope::Session;
    let mut store = GameplayModuleStateStore::new(registry, adapters()).unwrap();
    store
        .initialize_atomic(vec![initialization("project/default", scope.clone(), b"4")])
        .unwrap();
    let receipt = store
        .apply_fact(fact("fact.increment", scope.clone(), 0, b"3"))
        .unwrap();
    assert_ne!(receipt.before_hash, receipt.after_hash);
    assert_eq!(receipt.record_revision, 1);
    assert_eq!(counter_value(&store, &scope), 7);
    let view = store.named_view(&state_schema(), &scope).unwrap();
    assert_eq!(view.canonical_payload, br#"{"value":7}"#);
    assert_eq!(view.provider_id, "provider.game-counter");

    let before = store.state_hash();
    let mut foreign = fact("fact.foreign", scope.clone(), 1, b"1");
    foreign.module_id = "game.other-module".to_owned();
    assert_eq!(
        store.apply_fact(foreign),
        Err(GameplayModuleStateError::ForeignModule)
    );
    assert_eq!(
        store.apply_fact(fact("fact.stale", scope, 0, b"1")),
        Err(GameplayModuleStateError::StaleRevision)
    );
    assert_eq!(store.state_hash(), before);
}

#[test]
fn initialization_is_atomic_and_entity_facets_are_separate() {
    let registry = Arc::new(registry());
    let session = GameplayModuleStateScope::Session;
    let entity = GameplayModuleStateScope::Entity { entity: 42 };
    let mut store = GameplayModuleStateStore::new(registry, adapters()).unwrap();
    let invalid = initialization("entity/42", entity.clone(), b"not-json");
    assert!(matches!(
        store.initialize_atomic(vec![
            initialization("session", session.clone(), b"1"),
            invalid
        ]),
        Err(GameplayModuleStateError::AdapterRejected(_))
    ));
    assert!(store.record(&state_schema(), &session).is_none());
    store
        .initialize_atomic(vec![
            initialization("session", session.clone(), b"1"),
            initialization("entity/42", entity.clone(), b"9"),
        ])
        .unwrap();
    assert_eq!(counter_value(&store, &session), 1);
    assert_eq!(counter_value(&store, &entity), 9);
}

#[test]
fn snapshot_playback_and_migration_preserve_authority_evidence() {
    let registry = Arc::new(registry());
    let scope = GameplayModuleStateScope::Session;
    let init = initialization("session", scope.clone(), b"2");
    let accepted = fact("fact.one", scope.clone(), 0, b"5");
    let mut store = GameplayModuleStateStore::playback(
        registry.clone(),
        adapters(),
        vec![init.clone()],
        std::slice::from_ref(&accepted),
    )
    .unwrap();
    let snapshot = store.encode_snapshot().unwrap();
    let restored =
        GameplayModuleStateStore::decode_snapshot(registry.clone(), adapters(), &snapshot).unwrap();
    assert_eq!(restored.state_hash(), store.state_hash());
    assert_eq!(restored.accepted_facts(), &[accepted]);
    let mut invalid_fact_evidence: serde_json::Value = serde_json::from_slice(&snapshot).unwrap();
    invalid_fact_evidence["acceptedFacts"][0]["payloadHash"] = serde_json::json!("tampered");
    assert!(matches!(
        GameplayModuleStateStore::decode_snapshot(
            registry.clone(),
            adapters(),
            &serde_json::to_vec(&invalid_fact_evidence).unwrap(),
        ),
        Err(GameplayModuleStateError::InvalidSnapshot(_))
    ));

    let authority_snapshot = br#"{"tick":7,"entityHash":"entity:7"}"#;
    let session_snapshot = store
        .encode_session_snapshot(authority_snapshot, "authority:7")
        .unwrap();
    let restored_session = GameplayModuleStateStore::decode_session_snapshot(
        registry.clone(),
        adapters(),
        &session_snapshot,
    )
    .unwrap();
    assert_eq!(restored_session.authority_snapshot, authority_snapshot);
    assert_eq!(restored_session.authority_state_hash, "authority:7");
    assert_eq!(
        restored_session.module_state.state_hash(),
        store.state_hash()
    );
    assert_eq!(
        restored_session.final_session_hash,
        store.final_session_hash("authority:7")
    );

    let mut corrupted: serde_json::Value = serde_json::from_slice(&session_snapshot).unwrap();
    corrupted["authoritySnapshot"][0] = serde_json::json!(0);
    let corrupted = serde_json::to_vec(&corrupted).unwrap();
    assert!(matches!(
        GameplayModuleStateStore::decode_session_snapshot(registry.clone(), adapters(), &corrupted),
        Err(GameplayModuleStateError::InvalidSnapshot(_))
    ));

    store.migrate_record(&state_schema(), &scope, 1, 1).unwrap();
    assert_eq!(counter_value(&store, &scope), 107);
    assert_ne!(
        store.final_session_hash("authority:7"),
        restored_session.final_session_hash
    );
    let before = store.state_hash();
    assert!(matches!(
        store.migrate_record(&state_schema(), &scope, 99, 2),
        Err(GameplayModuleStateError::AdapterRejected(_))
    ));
    assert_eq!(store.state_hash(), before);
    let readout = store.readouts();
    assert_eq!(readout[0].revision, 2);
    assert_eq!(readout[0].initialized_from, "session");
    assert_eq!(
        readout[0].state_hash,
        store.record(&state_schema(), &scope).unwrap().state_hash
    );

    let mut migrated = GameplayModuleStateStore::new(registry.clone(), adapters()).unwrap();
    migrated
        .migrate_atomic(vec![migration("counter-v1-v2", scope.clone(), b"7")])
        .unwrap();
    assert_eq!(counter_value(&migrated, &scope), 107);
    assert_eq!(migrated.readouts()[0].revision, 5);
    assert_eq!(
        migrated.readouts()[0].initialized_from,
        "legacy/project-default; migrated-by:counter-v1-v2"
    );

    let mut atomic_failure = GameplayModuleStateStore::new(registry, adapters()).unwrap();
    let invalid = migration(
        "bad",
        GameplayModuleStateScope::Entity { entity: 9 },
        b"bad",
    );
    assert!(matches!(
        atomic_failure.migrate_atomic(vec![
            migration("would-succeed", scope.clone(), b"1"),
            invalid,
        ]),
        Err(GameplayModuleStateError::AdapterRejected(_))
    ));
    assert!(atomic_failure.readouts().is_empty());
}

#[test]
fn reaction_frame_verification_classifies_code_event_fact_and_post_state_drift() {
    let registry = Arc::new(registry());
    let observe = GameplayObserveReceipt {
        registry_digest: registry.registry_digest().to_owned(),
        root_id: String::new(),
        waves_processed: 0,
        wave_views: Vec::new(),
        wave_barriers: Vec::new(),
        events: Vec::new(),
        event_evidence: Vec::new(),
        invocations: Vec::new(),
        routing: Vec::new(),
        module_facts: Vec::new(),
        diagnostics: Vec::new(),
        receipt_hash: "receipt:observe".to_owned(),
    };
    let accepted = fact("fact.one", GameplayModuleStateScope::Session, 0, b"1");
    let expected = GameplayReactionFrame::from_observe(
        registry.as_ref(),
        &observe,
        vec![GameplayReactionSourceFact::new(
            "authority.combat".to_owned(),
            "combat.hit.v1".to_owned(),
            b"source:1".to_vec(),
        )],
        std::slice::from_ref(&accepted),
        "state:before".to_owned(),
        "state:after".to_owned(),
        "session:after".to_owned(),
    );
    assert!(verify_reaction_frame(&expected, &expected).is_empty());

    let played = GameplayModuleStateStore::playback_frame(
        registry,
        adapters(),
        vec![initialization(
            "session",
            GameplayModuleStateScope::Session,
            b"0",
        )],
        &expected,
    )
    .unwrap();
    assert_eq!(
        counter_value(&played, &GameplayModuleStateScope::Session),
        1
    );

    let cases: Vec<(GameplayReactionFrame, GameplayReactionDivergence)> = vec![
        (
            {
                let mut frame = expected.clone();
                frame.module_artifacts[0].push_str("-changed");
                frame
            },
            GameplayReactionDivergence::RegistryOrCode,
        ),
        (
            {
                let mut frame = expected.clone();
                frame.registry_digest.push_str("-schema-changed");
                frame
            },
            GameplayReactionDivergence::RegistryOrCode,
        ),
        (
            {
                let mut frame = expected.clone();
                frame.module_order.reverse();
                frame.module_order.push("game.other-module".to_owned());
                frame
            },
            GameplayReactionDivergence::RegistryOrCode,
        ),
        (
            {
                let mut frame = expected.clone();
                frame
                    .delivered_event_hashes
                    .push("event:changed".to_owned());
                frame
            },
            GameplayReactionDivergence::Events,
        ),
        (
            {
                let mut frame = expected.clone();
                frame
                    .routed_proposal_hashes
                    .push("proposal:changed".to_owned());
                frame
            },
            GameplayReactionDivergence::ProposalsOrRouting,
        ),
        (
            {
                let mut frame = expected.clone();
                frame
                    .accepted_module_fact_hashes
                    .push("fact:changed".to_owned());
                frame
            },
            GameplayReactionDivergence::ModuleFacts,
        ),
        (
            {
                let mut frame = expected.clone();
                frame
                    .frozen_views
                    .push(rule_gameplay_fabric::GameplayReactionViewEvidence {
                        epoch: 0,
                        view_hash: "view:before".to_owned(),
                    });
                frame.frozen_view_hashes.push("view:before".to_owned());
                frame.wave_barriers.push(GameplayWaveBarrierEvidence {
                    wave: 0,
                    frozen_view: FrozenGameplayViews {
                        epoch: 0,
                        view_hash: "view:before".to_owned(),
                    },
                    state_before: GameplayWaveStateHashes {
                        authority_state_hash: "authority:before".to_owned(),
                        module_state_hash: "state:before".to_owned(),
                        prefab_state_hash: "prefab:before".to_owned(),
                        trigger_state_hash: "trigger:before".to_owned(),
                    },
                    state_after: GameplayWaveStateHashes {
                        authority_state_hash: "authority:after".to_owned(),
                        module_state_hash: "state:after".to_owned(),
                        prefab_state_hash: "prefab:after".to_owned(),
                        trigger_state_hash: "trigger:after".to_owned(),
                    },
                    routing_hashes: Vec::new(),
                    module_fact_hashes: Vec::new(),
                    barrier_hash: "tampered-barrier".to_owned(),
                });
                frame
            },
            GameplayReactionDivergence::Views,
        ),
        (
            {
                let mut frame = expected.clone();
                frame.state_hash_after = "state:changed".to_owned();
                frame
            },
            GameplayReactionDivergence::State,
        ),
    ];
    for (mut changed, expected_divergence) in cases {
        changed.frame_hash = changed.canonical_hash();
        assert_eq!(
            verify_reaction_frame(&expected, &changed),
            vec![expected_divergence]
        );
    }
}
