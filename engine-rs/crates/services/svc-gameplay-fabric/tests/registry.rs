use protocol_game_extension::{
    GameplayContractRef, GameplayEventSchemaDeclaration, GameplayExecutionBudget,
    GameplayHeaderSelector, GameplayInvocationDescriptor, GameplayInvocationFamily,
    GameplayInvocationReadRequirement, GameplayModuleManifest, GameplayModuleRef,
    GameplayOrderingConstraint, GameplayOwnerRef, GameplayProposalDeclaration,
    GameplayReadSelectorCapability, GameplayReadViewKind, GameplayReadViewRequirement,
    GameplayRegistryDiagnosticCode, GameplaySubscriptionDeclaration,
};
use serde::{Deserialize, Serialize};
use svc_gameplay_fabric::{
    gameplay_canonical_codec_id, gameplay_contract, stable_identity, GameplayEventFilterDescriptor,
    GameplayEventFilterField, GameplayEventFilterFieldDescriptor, GameplayEventFilterFieldShape,
    GameplayEventFilterValue, GameplayEventFilterValueKind, GameplayFabricRegistry,
    GameplayFabricRegistryBuilder, GameplayLinkedProvider, GameplayProposalOwnerRegistration,
    GameplayReadViewProviderRegistration, TypedGameplayEventCodec,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct DamageApplied {
    amount: u32,
}

fn schema_descriptor(namespace: &str, name: &str) -> String {
    format!("fixture:{namespace}.{name};canonical-json-v1")
}

fn contract(namespace: &str, name: &str, _legacy_hash: &str) -> GameplayContractRef {
    gameplay_contract(namespace, name, 1, &schema_descriptor(namespace, name))
}

fn event_declaration() -> GameplayEventSchemaDeclaration {
    let event = contract("game.combat", "damage-applied", "sha256:event-v1");
    GameplayEventSchemaDeclaration {
        codec_id: gameplay_canonical_codec_id(&event.schema_hash),
        event,
    }
}

fn module(module_id: &str, namespace: &str, provider_id: &str) -> GameplayModuleManifest {
    GameplayModuleManifest {
        module_ref: GameplayModuleRef {
            module_id: module_id.into(),
            namespace: namespace.into(),
            version: "1.0.0".into(),
            sdk_hash: stable_identity(["sdk", module_id]),
            contract_hash: stable_identity(["contract", module_id]),
            artifact_hash: stable_identity(["artifact", module_id]),
            provider_id: provider_id.into(),
        },
        published_events: Vec::new(),
        subscriptions: Vec::new(),
        invocations: Vec::new(),
        read_views: Vec::new(),
        proposal_kinds: Vec::new(),
        state_schemas: Vec::new(),
        fact_schemas: Vec::new(),
        ordering: Vec::new(),
        budget: GameplayExecutionBudget {
            max_waves: 8,
            max_events_per_root: 64,
            max_proposals_per_root: 32,
            max_invocations_per_root: 64,
            max_payload_bytes_per_root: 65_536,
        },
        deterministic_requirements: vec!["canonical-input-order".into()],
        source_hash: stable_identity(["source", module_id]),
    }
}

fn provider(manifest: &GameplayModuleManifest) -> GameplayLinkedProvider {
    GameplayLinkedProvider {
        provider_id: manifest.module_ref.provider_id.clone(),
        module_id: manifest.module_ref.module_id.clone(),
        version: manifest.module_ref.version.clone(),
        contract_hash: manifest.module_ref.contract_hash.clone(),
        artifact_hash: manifest.module_ref.artifact_hash.clone(),
        sdk_hash: manifest.module_ref.sdk_hash.clone(),
        source_hash: manifest.source_hash.clone(),
    }
}

fn codec(declaration: GameplayEventSchemaDeclaration) -> TypedGameplayEventCodec<DamageApplied> {
    let descriptor = schema_descriptor(&declaration.event.namespace, &declaration.event.name);
    TypedGameplayEventCodec::new(
        declaration,
        descriptor,
        |payload| serde_json::to_vec(payload).map_err(|error| error.to_string()),
        |bytes| serde_json::from_slice(bytes).map_err(|error| error.to_string()),
    )
}

fn damage_filter_matches(payload: &DamageApplied, field: &GameplayEventFilterField) -> bool {
    matches!(
        (field.name.as_str(), &field.value),
        ("amount", GameplayEventFilterValue::Integer(amount))
            if u32::try_from(*amount) == Ok(payload.amount)
    )
}

fn valid_pair() -> (GameplayModuleManifest, GameplayModuleManifest) {
    let declaration = event_declaration();
    let mut combat = module("game.combat-rules", "game.combat", "provider.combat");
    combat.published_events.push(declaration.clone());
    combat.ordering.push(GameplayOrderingConstraint {
        before_module: "game.combat-rules".into(),
        after_module: "game.ui-feedback".into(),
    });

    let mut feedback = module(
        "game.ui-feedback",
        "game.presentation-feedback",
        "provider.feedback",
    );
    feedback.invocations.push(GameplayInvocationDescriptor {
        invocation_id: "observe-damage".into(),
        family: GameplayInvocationFamily::Observe,
        input_contract: declaration.event.clone(),
        output_contract: contract("game.presentation-feedback", "damage-cue", "sha256:cue-v1"),
        read_requirements: Vec::new(),
        max_outputs: 4,
        max_payload_bytes: 4_096,
    });
    feedback
        .subscriptions
        .push(GameplaySubscriptionDeclaration {
            subscription_id: "feedback.observe-damage".into(),
            event: declaration.event,
            invocation_id: "observe-damage".into(),
            selector: GameplayHeaderSelector {
                source: None,
                target: None,
                scope: None,
                required_tags: vec!["combat".into()],
            },
            max_deliveries_per_root: 16,
        });
    (combat, feedback)
}

fn build_pair(reverse: bool) -> GameplayFabricRegistry {
    let (combat, feedback) = valid_pair();
    build_manifests(combat, feedback, reverse)
}

fn build_manifests(
    combat: GameplayModuleManifest,
    feedback: GameplayModuleManifest,
    reverse: bool,
) -> GameplayFabricRegistry {
    let combat_provider = provider(&combat);
    let feedback_provider = provider(&feedback);
    let declaration = event_declaration();
    let mut builder = GameplayFabricRegistryBuilder::new();
    if reverse {
        builder
            .register_module(feedback)
            .register_module(combat)
            .register_linked_provider(feedback_provider)
            .register_linked_provider(combat_provider);
    } else {
        builder
            .register_module(combat)
            .register_module(feedback)
            .register_linked_provider(combat_provider)
            .register_linked_provider(feedback_provider);
    }
    builder.register_event_codec(codec(declaration));
    builder.build().expect("valid immutable registry")
}

fn codes(
    error: &svc_gameplay_fabric::GameplayRegistryBuildError,
) -> Vec<GameplayRegistryDiagnosticCode> {
    error
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code)
        .collect()
}

fn rejected(
    result: Result<GameplayFabricRegistry, svc_gameplay_fabric::GameplayRegistryBuildError>,
) -> svc_gameplay_fabric::GameplayRegistryBuildError {
    match result {
        Ok(_) => panic!("registry construction unexpectedly succeeded"),
        Err(error) => error,
    }
}

#[test]
fn typed_codecs_and_two_namespaces_produce_order_independent_topology() {
    let forward = build_pair(false);
    let reverse = build_pair(true);

    assert_eq!(forward.registry_digest(), reverse.registry_digest());
    assert_eq!(forward.topology_dump(), reverse.topology_dump());
    assert_eq!(forward.readout(), reverse.readout());
    assert_eq!(forward.readout().module_ids.len(), 2);
    assert_eq!(
        forward.module_order(),
        &[
            "game.combat-rules".to_owned(),
            "game.ui-feedback".to_owned()
        ]
    );

    let event = event_declaration().event;
    assert!(forward.event_is_declared(&event));
    assert!(forward.module_publishes_event("game.combat-rules", &event));
    assert!(!forward.module_publishes_event("game.ui-feedback", &event));
    let payload = DamageApplied { amount: 17 };
    let encoded = forward.encode_event(&event, &payload).expect("encode");
    let decoded: DamageApplied = forward.decode_event(&event, &encoded).expect("decode");
    assert_eq!(decoded, payload);

    let (mut budget_changed, feedback) = valid_pair();
    budget_changed.budget.max_events_per_root += 1;
    let changed = build_manifests(budget_changed, feedback, false);
    assert_ne!(forward.registry_digest(), changed.registry_digest());
}

#[test]
fn semantic_identity_ignores_provenance_but_binds_declared_compatibility() {
    let baseline = build_pair(false);

    let (mut source_changed, feedback) = valid_pair();
    source_changed.source_hash = stable_identity(["source", "rebuilt"]);
    source_changed.module_ref.artifact_hash = stable_identity(["artifact", "rebuilt"]);
    let rebuilt = build_manifests(source_changed, feedback, false);
    assert_eq!(
        baseline.semantic_compatibility_digest(),
        rebuilt.semantic_compatibility_digest()
    );
    assert_ne!(
        baseline.artifact_provenance_digest(),
        rebuilt.artifact_provenance_digest()
    );

    let (mut behavior_changed, feedback) = valid_pair();
    behavior_changed.module_ref.version = "2.0.0".to_owned();
    let incompatible = build_manifests(behavior_changed, feedback, false);
    assert_ne!(
        baseline.semantic_compatibility_digest(),
        incompatible.semantic_compatibility_digest()
    );
}

#[test]
fn codec_admission_rejects_noncanonical_unknown_and_wrong_hash_payloads() {
    let registry = build_pair(false);
    let event = event_declaration().event;
    let noncanonical = br#"{ "amount": 17 }"#;
    assert!(matches!(
        registry.admit_payload(
            &event,
            noncanonical,
            &svc_gameplay_fabric::gameplay_canonical_payload_hash(noncanonical),
        ),
        Err(svc_gameplay_fabric::GameplayCodecError::NonCanonical { .. })
    ));
    let canonical = serde_json::to_vec(&DamageApplied { amount: 17 }).unwrap();
    assert!(matches!(
        registry.encode_event(&event, &17_u64),
        Err(svc_gameplay_fabric::GameplayCodecError::WrongPayloadType { .. })
    ));
    assert!(matches!(
        registry.admit_payload(&event, &canonical, "fnv1a64:0000000000000000"),
        Err(svc_gameplay_fabric::GameplayCodecError::PayloadHashMismatch { .. })
    ));
    assert!(matches!(
        registry.admit_payload(
            &contract("game.missing", "event", "unused"),
            &canonical,
            &svc_gameplay_fabric::gameplay_canonical_payload_hash(&canonical),
        ),
        Err(svc_gameplay_fabric::GameplayCodecError::UnknownContract { .. })
    ));
}

#[test]
fn provider_owned_filter_descriptor_validates_shape_and_matches_typed_payload() {
    let (combat, feedback) = valid_pair();
    let event = event_declaration().event;
    let mut builder = GameplayFabricRegistryBuilder::new();
    builder
        .register_linked_provider(provider(&combat))
        .register_linked_provider(provider(&feedback))
        .register_event_codec(codec(event_declaration()).with_filter(
            GameplayEventFilterDescriptor {
                fields: vec![GameplayEventFilterFieldDescriptor {
                    name: "amount".to_owned(),
                    value_kind: GameplayEventFilterValueKind::Integer,
                    required: true,
                }],
            },
            damage_filter_matches,
        ))
        .register_module(combat)
        .register_module(feedback);
    let registry = builder.build().expect("typed filter registry");

    for fields in [
        Vec::new(),
        vec![GameplayEventFilterFieldShape {
            name: "damage".to_owned(),
            value_kind: GameplayEventFilterValueKind::Integer,
        }],
        vec![GameplayEventFilterFieldShape {
            name: "amount".to_owned(),
            value_kind: GameplayEventFilterValueKind::Text,
        }],
    ] {
        assert!(matches!(
            registry.validate_event_filter_shape(&event, &fields),
            Err(svc_gameplay_fabric::GameplayCodecError::InvalidFilter { .. })
        ));
    }

    let payload = serde_json::to_vec(&DamageApplied { amount: 17 }).expect("payload");
    let matching = vec![GameplayEventFilterField {
        name: "amount".to_owned(),
        value: GameplayEventFilterValue::Integer(17),
    }];
    let different = vec![GameplayEventFilterField {
        name: "amount".to_owned(),
        value: GameplayEventFilterValue::Integer(19),
    }];
    assert!(!registry
        .validate_event_filter_shape(
            &event,
            &matching
                .iter()
                .map(GameplayEventFilterField::shape)
                .collect::<Vec<_>>(),
        )
        .expect("provider filter identity")
        .is_empty());
    assert!(registry
        .matches_event_filter(&event, &payload, &matching)
        .expect("typed filter match"));
    assert!(!registry
        .matches_event_filter(&event, &payload, &different)
        .expect("typed filter mismatch"));
}

#[test]
fn codec_descriptor_and_placeholder_identities_fail_closed() {
    let (combat, feedback) = valid_pair();
    let mut bad_codec = GameplayFabricRegistryBuilder::new();
    bad_codec
        .register_linked_provider(provider(&combat))
        .register_linked_provider(provider(&feedback))
        .register_event_codec(TypedGameplayEventCodec::new(
            event_declaration(),
            "wrong-schema-descriptor",
            |payload: &DamageApplied| {
                serde_json::to_vec(payload).map_err(|error| error.to_string())
            },
            |bytes| serde_json::from_slice(bytes).map_err(|error| error.to_string()),
        ))
        .register_module(combat)
        .register_module(feedback);
    assert!(codes(&rejected(bad_codec.build()))
        .contains(&GameplayRegistryDiagnosticCode::SchemaHashMismatch));

    let (mut combat, feedback) = valid_pair();
    combat.module_ref.sdk_hash = "sha256:name-v1".to_owned();
    let mut placeholder = GameplayFabricRegistryBuilder::new();
    placeholder
        .register_linked_provider(provider(&combat))
        .register_linked_provider(provider(&feedback))
        .register_event_codec(codec(event_declaration()))
        .register_module(combat)
        .register_module(feedback);
    assert!(codes(&rejected(placeholder.build()))
        .contains(&GameplayRegistryDiagnosticCode::InvalidIdentifier));
}

#[test]
fn registry_digest_binds_actual_invocation_read_topology() {
    let baseline = build_pair(false);
    let (combat, mut feedback) = valid_pair();
    let view = contract(
        "game.presentation-feedback",
        "target-view",
        "legacy-target-view",
    );
    feedback.read_views.push(GameplayReadViewRequirement {
        view: view.clone(),
        provider_id: "provider.feedback".into(),
        kind: GameplayReadViewKind::EntityCapability,
        fields: vec!["lifecycle".into()],
        selector_capabilities: vec![GameplayReadSelectorCapability::LifecycleCapability],
        max_items: 1,
    });
    feedback.invocations[0]
        .read_requirements
        .push(GameplayInvocationReadRequirement {
            request_id: "target-lifecycle".into(),
            view: view.clone(),
        });
    let mut builder = GameplayFabricRegistryBuilder::new();
    builder
        .register_linked_provider(provider(&combat))
        .register_linked_provider(provider(&feedback))
        .register_event_codec(codec(event_declaration()))
        .register_read_view_provider(GameplayReadViewProviderRegistration {
            view,
            provider_id: "provider.feedback".into(),
            kind: GameplayReadViewKind::EntityCapability,
            fields: vec!["lifecycle".into()],
            selector_capabilities: vec![GameplayReadSelectorCapability::LifecycleCapability],
            max_items: 1,
            ordering: "singleValue".into(),
        })
        .register_module(combat)
        .register_module(feedback);
    let changed = builder.build().expect("valid read topology");
    assert_ne!(baseline.registry_digest(), changed.registry_digest());
    assert!(changed
        .topology_dump()
        .contains("reads=target-lifecycle=game.presentation-feedback.target-view.v1@fnv1a64:"));
}

#[test]
fn duplicate_event_kind_and_schema_conflict_reject_registry_construction() {
    let (combat, mut feedback) = valid_pair();
    let mut conflicting = event_declaration();
    conflicting.event.schema_hash = "sha256:different-schema".into();
    feedback.published_events.push(conflicting);

    let mut builder = GameplayFabricRegistryBuilder::new();
    builder
        .register_linked_provider(provider(&combat))
        .register_linked_provider(provider(&feedback))
        .register_event_codec(codec(event_declaration()))
        .register_module(combat)
        .register_module(feedback);
    let error = rejected(builder.build());

    assert!(codes(&error).contains(&GameplayRegistryDiagnosticCode::SchemaHashMismatch));
}

#[test]
fn missing_codec_and_owner_cardinality_reject_registry_construction() {
    let (mut combat, feedback) = valid_pair();
    let proposal = GameplayProposalDeclaration {
        proposal: contract("game.combat", "damage-proposal", "sha256:proposal-v1"),
        owner: GameplayOwnerRef {
            owner_id: "authority.combat".into(),
            provider_id: "provider.combat".into(),
        },
    };
    combat.proposal_kinds.push(proposal.clone());
    let mut builder = GameplayFabricRegistryBuilder::new();
    builder
        .register_linked_provider(provider(&combat))
        .register_linked_provider(provider(&feedback))
        .register_module(combat.clone())
        .register_module(feedback.clone());
    let missing = rejected(builder.build());
    assert!(codes(&missing).contains(&GameplayRegistryDiagnosticCode::MissingCodec));
    assert!(codes(&missing).contains(&GameplayRegistryDiagnosticCode::MissingProposalOwner));

    let owner = GameplayProposalOwnerRegistration {
        proposal: proposal.proposal,
        owner: proposal.owner,
    };
    let mut valid = GameplayFabricRegistryBuilder::new();
    valid
        .register_linked_provider(provider(&combat))
        .register_linked_provider(provider(&feedback))
        .register_event_codec(codec(event_declaration()))
        .register_event_codec(codec(GameplayEventSchemaDeclaration {
            codec_id: gameplay_canonical_codec_id(&owner.proposal.schema_hash),
            event: owner.proposal.clone(),
        }))
        .register_proposal_owner(owner.clone())
        .register_module(combat.clone())
        .register_module(feedback.clone());
    let registry = valid.build().expect("single exact owner is retained");
    assert_eq!(registry.proposal_owner(&owner.proposal), Some(&owner.owner));
    assert!(registry.module_declares_proposal("game.combat-rules", &owner.proposal));

    let mut duplicate = GameplayFabricRegistryBuilder::new();
    duplicate
        .register_linked_provider(provider(&combat))
        .register_linked_provider(provider(&feedback))
        .register_event_codec(codec(event_declaration()))
        .register_event_codec(codec(GameplayEventSchemaDeclaration {
            codec_id: gameplay_canonical_codec_id(&owner.proposal.schema_hash),
            event: owner.proposal.clone(),
        }))
        .register_proposal_owner(owner.clone())
        .register_proposal_owner(owner)
        .register_module(combat)
        .register_module(feedback);
    let error = rejected(duplicate.build());
    assert!(codes(&error).contains(&GameplayRegistryDiagnosticCode::MultipleProposalOwners));
}

#[test]
fn missing_compiled_provider_rejects_registry_construction() {
    let (combat, feedback) = valid_pair();
    let mut builder = GameplayFabricRegistryBuilder::new();
    builder
        .register_event_codec(codec(event_declaration()))
        .register_module(combat)
        .register_module(feedback);

    let error = rejected(builder.build());
    assert!(codes(&error).contains(&GameplayRegistryDiagnosticCode::MissingProvider));
}

#[test]
fn foreign_namespace_and_missing_invocation_reject_registry_construction() {
    let (combat, mut feedback) = valid_pair();
    let foreign_event = contract("game.combat", "foreign-cue", "sha256:foreign-v1");
    feedback
        .published_events
        .push(GameplayEventSchemaDeclaration {
            codec_id: gameplay_canonical_codec_id(&foreign_event.schema_hash),
            event: foreign_event,
        });
    feedback.subscriptions[0].invocation_id = "not-registered".into();
    let foreign = feedback.published_events[0].clone();

    let mut builder = GameplayFabricRegistryBuilder::new();
    builder
        .register_linked_provider(provider(&combat))
        .register_linked_provider(provider(&feedback))
        .register_event_codec(codec(event_declaration()))
        .register_event_codec(codec(foreign))
        .register_module(combat)
        .register_module(feedback);
    let error = rejected(builder.build());
    let diagnostic_codes = codes(&error);
    assert!(diagnostic_codes.contains(&GameplayRegistryDiagnosticCode::ForeignNamespaceWrite));
    assert!(diagnostic_codes.contains(&GameplayRegistryDiagnosticCode::MissingInvocation));
}

#[test]
fn subscription_input_schema_must_match_the_published_event() {
    let (combat, mut feedback) = valid_pair();
    feedback.invocations[0].input_contract = contract(
        "game.presentation-feedback",
        "different-input",
        "sha256:different-input",
    );

    let mut builder = GameplayFabricRegistryBuilder::new();
    builder
        .register_linked_provider(provider(&combat))
        .register_linked_provider(provider(&feedback))
        .register_event_codec(codec(event_declaration()))
        .register_module(combat)
        .register_module(feedback);
    let error = rejected(builder.build());
    assert!(codes(&error).contains(&GameplayRegistryDiagnosticCode::InvalidSubscriptionInvocation));
}

#[test]
fn ordering_cycle_rejects_registry_construction() {
    let (combat, mut feedback) = valid_pair();
    feedback.ordering.push(GameplayOrderingConstraint {
        before_module: "game.ui-feedback".into(),
        after_module: "game.combat-rules".into(),
    });

    let mut builder = GameplayFabricRegistryBuilder::new();
    builder
        .register_linked_provider(provider(&combat))
        .register_linked_provider(provider(&feedback))
        .register_event_codec(codec(event_declaration()))
        .register_module(combat)
        .register_module(feedback);
    let error = rejected(builder.build());
    assert!(codes(&error).contains(&GameplayRegistryDiagnosticCode::OrderingCycle));
}

fn read_metadata_error(
    requirement: GameplayReadViewRequirement,
    registration: Option<GameplayReadViewProviderRegistration>,
) -> svc_gameplay_fabric::GameplayRegistryBuildError {
    let (combat, mut feedback) = valid_pair();
    feedback.read_views.push(requirement);
    let mut builder = GameplayFabricRegistryBuilder::new();
    builder
        .register_linked_provider(provider(&combat))
        .register_linked_provider(provider(&feedback))
        .register_event_codec(codec(event_declaration()))
        .register_module(combat)
        .register_module(feedback);
    if let Some(registration) = registration {
        builder.register_read_view_provider(registration);
    }
    rejected(builder.build())
}

#[test]
fn needs_validation_distinguishes_provider_kind_selector_and_field_gaps() {
    let view = contract(
        "game.presentation-feedback",
        "target-view",
        "sha256:target-view",
    );
    let requirement = GameplayReadViewRequirement {
        view: view.clone(),
        provider_id: "provider.feedback".into(),
        kind: GameplayReadViewKind::EntityCapability,
        fields: vec!["lifecycle".into()],
        selector_capabilities: vec![
            GameplayReadSelectorCapability::EventTarget,
            GameplayReadSelectorCapability::LifecycleCapability,
        ],
        max_items: 1,
    };
    assert!(codes(&read_metadata_error(requirement.clone(), None))
        .contains(&GameplayRegistryDiagnosticCode::MissingReadViewProvider));

    let provider = GameplayReadViewProviderRegistration {
        view,
        provider_id: "provider.feedback".into(),
        kind: GameplayReadViewKind::EntityCapability,
        fields: vec!["lifecycle".into()],
        selector_capabilities: vec![
            GameplayReadSelectorCapability::EventTarget,
            GameplayReadSelectorCapability::LifecycleCapability,
        ],
        max_items: 1,
        ordering: "singleValue".into(),
    };
    let mut wrong_provider = provider.clone();
    wrong_provider.provider_id = "provider.another".into();
    assert!(codes(&read_metadata_error(
        requirement.clone(),
        Some(wrong_provider)
    ))
    .contains(&GameplayRegistryDiagnosticCode::ReadViewProviderMismatch));

    let mut wrong_kind = provider.clone();
    wrong_kind.kind = GameplayReadViewKind::Relationship;
    assert!(
        codes(&read_metadata_error(requirement.clone(), Some(wrong_kind)))
            .contains(&GameplayRegistryDiagnosticCode::ReadViewKindMismatch)
    );

    let mut missing_selector = provider.clone();
    missing_selector.selector_capabilities.clear();
    assert!(codes(&read_metadata_error(
        requirement.clone(),
        Some(missing_selector)
    ))
    .contains(&GameplayRegistryDiagnosticCode::MissingReadViewSelector));

    let mut missing_field = provider;
    missing_field.fields.clear();
    assert!(
        codes(&read_metadata_error(requirement, Some(missing_field)))
            .contains(&GameplayRegistryDiagnosticCode::MissingReadViewField)
    );
}

#[test]
fn invocation_read_requirements_reject_duplicate_ids_and_module_undeclared_views() {
    let (combat, mut feedback) = valid_pair();
    let view = contract(
        "game.presentation-feedback",
        "target-view",
        "sha256:target-view",
    );
    feedback.read_views.push(GameplayReadViewRequirement {
        view: view.clone(),
        provider_id: "provider.feedback".into(),
        kind: GameplayReadViewKind::EntityCapability,
        fields: vec!["lifecycle".into()],
        selector_capabilities: vec![GameplayReadSelectorCapability::LifecycleCapability],
        max_items: 1,
    });
    feedback.invocations[0].read_requirements = vec![
        GameplayInvocationReadRequirement {
            request_id: "target".into(),
            view: view.clone(),
        },
        GameplayInvocationReadRequirement {
            request_id: "target".into(),
            view: contract(
                "game.presentation-feedback",
                "private-view",
                "sha256:private-view",
            ),
        },
    ];
    let mut builder = GameplayFabricRegistryBuilder::new();
    builder
        .register_linked_provider(provider(&combat))
        .register_linked_provider(provider(&feedback))
        .register_event_codec(codec(event_declaration()))
        .register_read_view_provider(GameplayReadViewProviderRegistration {
            view,
            provider_id: "provider.feedback".into(),
            kind: GameplayReadViewKind::EntityCapability,
            fields: vec!["lifecycle".into()],
            selector_capabilities: vec![GameplayReadSelectorCapability::LifecycleCapability],
            max_items: 1,
            ordering: "singleValue".into(),
        })
        .register_module(combat)
        .register_module(feedback);
    let error = rejected(builder.build());
    let diagnostic_codes = codes(&error);
    assert!(diagnostic_codes.contains(&GameplayRegistryDiagnosticCode::DuplicateInvocationRead));
    assert!(diagnostic_codes.contains(&GameplayRegistryDiagnosticCode::MissingInvocationReadView));
}
