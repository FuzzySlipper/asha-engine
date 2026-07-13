use core_entity::{EntityLifecycleCommand, EntitySource, EntityStore};
use core_ids::{EntityId, PrefabId, PrefabInstanceId, PrefabPartId, TagId};
use protocol_game_extension::*;
use protocol_project_bundle::PrefabPartReference;
use rule_gameplay_fabric::*;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
use svc_gameplay_fabric::*;
use svc_serialization::{
    PrefabDefinition, PrefabPart, PrefabPartRoleBinding, PrefabPartSource, PrefabRegistry,
    PrefabRegistryValidationContext, PrefabTransform, ValidatedPrefabRegistry,
    PREFAB_DEFINITION_SCHEMA_VERSION, PREFAB_REGISTRY_SCHEMA_VERSION,
};

fn contract(name: &str) -> GameplayContractRef {
    GameplayContractRef {
        namespace: "game.fixture".to_owned(),
        name: name.to_owned(),
        version: 1,
        schema_hash: format!("sha256:{name}"),
    }
}

fn owner() -> GameplayOwnerRef {
    GameplayOwnerRef {
        owner_id: "authority.fixture".to_owned(),
        provider_id: "provider.fixture".to_owned(),
    }
}

fn requirement(
    name: &str,
    kind: GameplayReadViewKind,
    fields: &[&str],
    selectors: &[GameplayReadSelectorCapability],
    max_items: u32,
) -> GameplayReadViewRequirement {
    GameplayReadViewRequirement {
        view: contract(name),
        provider_id: "provider.fixture".to_owned(),
        kind,
        fields: fields.iter().map(|field| (*field).to_owned()).collect(),
        selector_capabilities: selectors.to_vec(),
        max_items,
    }
}

fn registry() -> GameplayFabricRegistry {
    let requirements = vec![
        requirement(
            "capability-view",
            GameplayReadViewKind::EntityCapability,
            &["staticCollider"],
            &[
                GameplayReadSelectorCapability::EventTarget,
                GameplayReadSelectorCapability::CollisionCapability,
            ],
            1,
        ),
        requirement(
            "relationship-view",
            GameplayReadViewKind::Relationship,
            &["entity"],
            &[
                GameplayReadSelectorCapability::EventTarget,
                GameplayReadSelectorCapability::Containment,
            ],
            1,
        ),
        requirement(
            "prefab-view",
            GameplayReadViewKind::PrefabPart,
            &["entity", "part", "role"],
            &[GameplayReadSelectorCapability::PrefabPartRole],
            1,
        ),
        requirement(
            "owner-query-view",
            GameplayReadViewKind::OwnerQuery,
            &["entities"],
            &[
                GameplayReadSelectorCapability::OwnerQuery,
                GameplayReadSelectorCapability::EventTarget,
            ],
            4,
        ),
        requirement(
            "trigger-overlap-view",
            GameplayReadViewKind::OwnerQuery,
            &["subjects", "providerRevision", "overlapHash"],
            &[
                GameplayReadSelectorCapability::OwnerQuery,
                GameplayReadSelectorCapability::EventSource,
            ],
            4,
        ),
        requirement(
            "named-view",
            GameplayReadViewKind::ModuleNamed,
            &["value"],
            &[GameplayReadSelectorCapability::ModuleStateScope],
            1,
        ),
    ];
    let module = GameplayModuleManifest {
        module_ref: GameplayModuleRef {
            module_id: "game.fixture-module".to_owned(),
            namespace: "game.fixture".to_owned(),
            version: "1.0.0".to_owned(),
            sdk_hash: "sha256:sdk".to_owned(),
            contract_hash: "sha256:contract".to_owned(),
            artifact_hash: "sha256:artifact".to_owned(),
            provider_id: "provider.fixture".to_owned(),
        },
        published_events: vec![GameplayEventSchemaDeclaration {
            event: contract("source-event"),
            codec_id: "codec.fixture-source".to_owned(),
        }],
        subscriptions: vec![GameplaySubscriptionDeclaration {
            subscription_id: "fixture.source.observe".to_owned(),
            event: contract("source-event"),
            invocation_id: "fixture.observe".to_owned(),
            selector: GameplayHeaderSelector {
                source: None,
                target: None,
                scope: None,
                required_tags: vec![],
            },
            max_deliveries_per_root: 4,
        }],
        invocations: vec![
            GameplayInvocationDescriptor {
                invocation_id: "fixture.observe".to_owned(),
                family: GameplayInvocationFamily::Observe,
                input_contract: contract("source-event"),
                output_contract: contract("observe-output"),
                read_requirements: vec![
                    ("read-owner-query", "owner-query-view"),
                    ("read-target", "capability-view"),
                    ("read-container", "relationship-view"),
                    ("read-prefab-part", "prefab-view"),
                    ("read-module", "named-view"),
                    ("read-trigger-overlaps", "trigger-overlap-view"),
                ]
                .into_iter()
                .map(|(request_id, view)| GameplayInvocationReadRequirement {
                    request_id: request_id.to_owned(),
                    view: contract(view),
                })
                .collect(),
                max_outputs: 4,
                max_payload_bytes: 4_096,
            },
            GameplayInvocationDescriptor {
                invocation_id: "fixture.secondary".to_owned(),
                family: GameplayInvocationFamily::Observe,
                input_contract: contract("source-event"),
                output_contract: contract("observe-output"),
                read_requirements: vec![GameplayInvocationReadRequirement {
                    request_id: "secondary-owner-query".to_owned(),
                    view: contract("owner-query-view"),
                }],
                max_outputs: 1,
                max_payload_bytes: 1_024,
            },
        ],
        read_views: requirements.clone(),
        proposal_kinds: Vec::new(),
        state_schemas: vec![GameplayOwnedSchemaDeclaration {
            schema: contract("counter-state"),
            owner: owner(),
        }],
        fact_schemas: vec![GameplayOwnedSchemaDeclaration {
            schema: contract("counter-fact"),
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
        deterministic_requirements: vec!["frozen-read-wave".to_owned()],
        source_hash: "sha256:source".to_owned(),
    };
    let mut builder = GameplayFabricRegistryBuilder::new();
    builder
        .register_linked_provider(GameplayLinkedProvider {
            provider_id: "provider.fixture".to_owned(),
            module_id: "game.fixture-module".to_owned(),
            version: "1.0.0".to_owned(),
            contract_hash: "sha256:contract".to_owned(),
            artifact_hash: "sha256:artifact".to_owned(),
            sdk_hash: "sha256:sdk".to_owned(),
            source_hash: "sha256:source".to_owned(),
        })
        .register_state_owner(GameplayStateOwnerRegistration {
            schema: contract("counter-state"),
            owner: owner(),
        })
        .register_state_owner(GameplayStateOwnerRegistration {
            schema: contract("counter-fact"),
            owner: owner(),
        })
        .register_event_codec(TypedGameplayEventCodec::new(
            GameplayEventSchemaDeclaration {
                event: contract("source-event"),
                codec_id: "codec.fixture-source".to_owned(),
            },
            |payload: &u64| serde_json::to_vec(payload).map_err(|error| error.to_string()),
            |bytes| serde_json::from_slice(bytes).map_err(|error| error.to_string()),
        ));
    for requirement in requirements {
        builder.register_read_view_provider(GameplayReadViewProviderRegistration {
            view: requirement.view,
            provider_id: requirement.provider_id,
            kind: requirement.kind,
            fields: requirement.fields,
            selector_capabilities: requirement.selector_capabilities,
            max_items: requirement.max_items,
            ordering: "entityIdAscending".to_owned(),
        });
    }
    builder.register_module(module);
    builder.build().expect("fixture registry")
}

struct CounterAdapter;

impl GameplayTypedModuleStateAdapter for CounterAdapter {
    type Config = u64;
    type State = u64;
    type Fact = u64;
    type View = CounterView;

    fn module_id(&self) -> &str {
        "game.fixture-module"
    }

    fn state_schema(&self) -> &GameplayContractRef {
        static VALUE: std::sync::OnceLock<GameplayContractRef> = std::sync::OnceLock::new();
        VALUE.get_or_init(|| contract("counter-state"))
    }

    fn fact_schema(&self) -> &GameplayContractRef {
        static VALUE: std::sync::OnceLock<GameplayContractRef> = std::sync::OnceLock::new();
        VALUE.get_or_init(|| contract("counter-fact"))
    }

    fn owner(&self) -> &GameplayOwnerRef {
        static VALUE: std::sync::OnceLock<GameplayOwnerRef> = std::sync::OnceLock::new();
        VALUE.get_or_init(owner)
    }

    fn decode_config(&self, canonical: &[u8]) -> Result<Self::Config, String> {
        serde_json::from_slice(canonical).map_err(|error| error.to_string())
    }

    fn initialize(&self, config: &Self::Config) -> Result<Self::State, String> {
        Ok(*config)
    }

    fn decode_state(&self, canonical: &[u8]) -> Result<Self::State, String> {
        serde_json::from_slice(canonical).map_err(|error| error.to_string())
    }

    fn encode_state(&self, state: &Self::State) -> Result<Vec<u8>, String> {
        serde_json::to_vec(state).map_err(|error| error.to_string())
    }

    fn decode_fact(&self, canonical: &[u8]) -> Result<Self::Fact, String> {
        serde_json::from_slice(canonical).map_err(|error| error.to_string())
    }

    fn apply_fact(&self, state: &Self::State, fact: &Self::Fact) -> Result<Self::State, String> {
        Ok(state + fact)
    }

    fn migrate(&self, _from_version: u32, state: &Self::State) -> Result<Self::State, String> {
        Ok(*state)
    }

    fn view_schema(&self) -> Option<&GameplayContractRef> {
        static VALUE: std::sync::OnceLock<GameplayContractRef> = std::sync::OnceLock::new();
        Some(VALUE.get_or_init(|| contract("named-view")))
    }

    fn project_view(&self, state: &Self::State) -> Result<Self::View, String> {
        Ok(CounterView { value: *state })
    }

    fn encode_view(&self, view: &Self::View) -> Result<Vec<u8>, String> {
        serde_json::to_vec(view).map_err(|error| error.to_string())
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
struct CounterView {
    value: u64,
}

struct FixtureOwnerQuery;

impl GameplayOwnerQueryProvider for FixtureOwnerQuery {
    fn provider_id(&self) -> &str {
        "provider.fixture"
    }

    fn query(
        &self,
        request: GameplayResolvedOwnerQuery,
    ) -> Result<GameplayOwnerQueryResult, GameplayReadProviderError> {
        let GameplayResolvedOwnerQuery::NearbyEntities { max_items, .. } = request else {
            return Err(GameplayReadProviderError {
                code: "unsupported".to_owned(),
                message: "fixture only handles nearby entities".to_owned(),
            });
        };
        Ok(GameplayOwnerQueryResult::NearbyEntities {
            entities: vec![3, 4].into_iter().take(max_items as usize).collect(),
            provider_revision: 7,
        })
    }
}

struct FixtureViews<'a, 'registry> {
    registry: &'registry GameplayFabricRegistry,
    entities: &'a EntityStore,
    state: &'a GameplayModuleStateStore,
    prefab_registry: &'a ValidatedPrefabRegistry,
    prefab_instances: &'a GameplayPrefabInstanceIndex,
    scopes: &'a GameplayEntityScopeIndex,
    owner_query: &'a FixtureOwnerQuery,
}

impl GameplayViewSource for FixtureViews<'_, '_> {
    fn freeze(&self, _root_id: &str, wave: u32) -> FrozenGameplayViews {
        FrozenGameplayViews {
            epoch: u64::from(wave),
            view_hash: format!("{}:wave:{wave}", self.state.state_hash()),
        }
    }

    fn freeze_declared_reads(
        &self,
        module_id: &str,
        invocation_id: &str,
        event: &GameplayEventEnvelope,
    ) -> Result<Option<GameplayFrozenReadSet>, GameplayReadAssemblyError> {
        let mut plan = plan();
        plan.module_id = module_id.to_owned();
        plan.invocation_id = invocation_id.to_owned();
        plan.event_id = event.event_id.clone();
        plan.wave = event.wave;
        GameplayReadAssembler::new(
            self.registry,
            self.entities,
            self.state,
            self.prefab_registry,
            self.prefab_instances,
            self.scopes,
            vec![self.owner_query],
        )
        .expect("fixture provider composition")
        .assemble(&plan, event)
        .map(Some)
    }
}

#[derive(Default)]
struct ReadHost {
    calls: RefCell<Vec<GameplayInvocationCall>>,
}

impl GameplayInvocationHost for ReadHost {
    fn invoke(
        &self,
        call: &GameplayInvocationCall,
    ) -> Result<GameplayInvocationOutput, GameplayHostError> {
        self.calls.borrow_mut().push(call.clone());
        Ok(GameplayInvocationOutput::default())
    }
}

struct NoopRouter;

impl GameplayProposalRouter for NoopRouter {
    fn route(&mut self, _call: &GameplayOwnerRoutingCall) -> GameplayOwnerRoutingOutput {
        GameplayOwnerRoutingOutput::default()
    }
}

struct RecordedReplayViews<'a> {
    recorded: &'a GameplayVerificationReplayInput,
}

impl GameplayViewSource for RecordedReplayViews<'_> {
    fn freeze(&self, root_id: &str, wave: u32) -> FrozenGameplayViews {
        assert_eq!(root_id, self.recorded.root_id);
        let recorded = self
            .recorded
            .frozen_views
            .get(wave as usize)
            .expect("verification input contains every frozen wave view");
        FrozenGameplayViews {
            epoch: recorded.epoch,
            view_hash: recorded.view_hash.clone(),
        }
    }

    fn freeze_declared_reads(
        &self,
        module_id: &str,
        invocation_id: &str,
        event: &GameplayEventEnvelope,
    ) -> Result<Option<GameplayFrozenReadSet>, GameplayReadAssemblyError> {
        Ok(self
            .recorded
            .frozen_read_sets
            .iter()
            .find(|reads| {
                reads.module_id == module_id
                    && reads.invocation_id == invocation_id
                    && reads.event_id == event.event_id
                    && reads.wave == event.wave
            })
            .cloned())
    }
}

struct RecordedReplayRunner<'a> {
    fixture: &'a Fixture,
}

impl GameplayVerificationReplayRunner for RecordedReplayRunner<'_> {
    fn rerun(
        &self,
        recorded: &GameplayVerificationReplayInput,
    ) -> Result<GameplayReactionFrame, GameplayModuleStateError> {
        if recorded.registry_digest != self.fixture.registry.registry_digest()
            || recorded.module_order != self.fixture.registry.module_order()
        {
            return Err(GameplayModuleStateError::InvalidSnapshot(
                "recorded registry identity does not match the linked registry".to_owned(),
            ));
        }
        let [root_event] = recorded.root_events.as_slice() else {
            return Err(GameplayModuleStateError::InvalidSnapshot(
                "fixture verification expects exactly one recorded root event".to_owned(),
            ));
        };
        let state = state_store(&self.fixture.registry);
        if state.state_hash() != recorded.state_hash_before {
            return Err(GameplayModuleStateError::InvalidSnapshot(
                "recorded pre-state hash does not match restored module state".to_owned(),
            ));
        }
        let host = ReadHost::default();
        let observe = GameplayFabricCoordinator::new(
            &self.fixture.registry,
            GameplayRuntimeLimits {
                max_waves: 2,
                max_events_per_root: 8,
                max_proposals_per_root: 8,
                max_invocations_per_root: 8,
                max_payload_bytes_per_root: 65_536,
            },
        )
        .observe(
            root_event.clone(),
            &RecordedReplayViews { recorded },
            &host,
            &mut NoopRouter,
        );
        let state_hash_after = state.state_hash();
        let final_session_hash = state.final_session_hash("activation.fixture");
        Ok(GameplayReactionFrame::from_observe(
            &self.fixture.registry,
            &observe,
            recorded.source_facts.clone(),
            &observe.module_facts,
            recorded.state_hash_before.clone(),
            state_hash_after,
            final_session_hash,
        ))
    }
}

struct Fixture {
    registry: GameplayFabricRegistry,
    entities: EntityStore,
    prefab_registry: ValidatedPrefabRegistry,
    prefab_instances: GameplayPrefabInstanceIndex,
    scopes: GameplayEntityScopeIndex,
}

fn fixture() -> Fixture {
    let mut entities = EntityStore::new();
    for id in 1..=4 {
        entities
            .apply(EntityLifecycleCommand::Create {
                id: EntityId::new(id),
                source: EntitySource::RuntimeCreated { by: None },
                labels: if id == 3 { vec![TagId::new(9)] } else { vec![] },
            })
            .unwrap();
    }
    entities.attach_collision(EntityId::new(2), false);
    entities.attach_containment(EntityId::new(2), EntityId::new(3));

    let prefab_registry = ValidatedPrefabRegistry::new(
        PrefabRegistry {
            schema_version: PREFAB_REGISTRY_SCHEMA_VERSION,
            definitions: vec![PrefabDefinition {
                id: PrefabId::new(10),
                schema_version: PREFAB_DEFINITION_SCHEMA_VERSION,
                display_name: "Fixture".to_owned(),
                parts: vec![PrefabPart {
                    id: PrefabPartId::new(20),
                    namespace: "gameplay/root".to_owned(),
                    display_name: "Gameplay root".to_owned(),
                    parent: None,
                    transform: PrefabTransform::IDENTITY,
                    source: PrefabPartSource::EntityDefinition {
                        stable_id: "fixture.root".to_owned(),
                    },
                }],
                part_roles: vec![PrefabPartRoleBinding {
                    role: "gameplay-root".to_owned(),
                    part: PrefabPartId::new(20),
                }],
                variant: None,
            }],
        },
        &PrefabRegistryValidationContext {
            asset_ids: Default::default(),
            entity_definition_ids: ["fixture.root".to_owned()].into_iter().collect(),
        },
    )
    .expect("valid prefab registry");
    let mut prefab_instances = GameplayPrefabInstanceIndex::default();
    prefab_instances
        .insert(
            PrefabInstanceId::new(30),
            GameplayPrefabInstanceBinding {
                prefab: PrefabId::new(10),
                part_entities: BTreeMap::from([(PrefabPartId::new(20), EntityId::new(4))]),
            },
        )
        .unwrap();
    let mut scopes = GameplayEntityScopeIndex::default();
    scopes.bind("arena", EntityId::new(3));
    Fixture {
        registry: registry(),
        entities,
        prefab_registry,
        prefab_instances,
        scopes,
    }
}

fn state_store(_registry: &GameplayFabricRegistry) -> GameplayModuleStateStore {
    let mut state = GameplayModuleStateStore::new(
        Rc::new(registry()),
        vec![GameplayModuleStateRegistration::typed(CounterAdapter)],
    )
    .unwrap();
    let config = serde_json::to_vec(&5_u64).unwrap();
    state
        .initialize_atomic(vec![GameplayModuleInitialization {
            initialization_id: "fixture-init".to_owned(),
            module_id: "game.fixture-module".to_owned(),
            state_schema: contract("counter-state"),
            scope: GameplayModuleStateScope::Session,
            config_hash: gameplay_module_payload_hash(&config),
            canonical_config: config,
        }])
        .unwrap();
    state
}

fn event() -> GameplayEventEnvelope {
    GameplayEventEnvelope {
        event_id: "event-1".to_owned(),
        event: contract("source-event"),
        tick: 9,
        root_sequence: 1,
        wave: 2,
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
        scope: Some("arena".to_owned()),
        tags: vec![],
        canonical_payload: vec![],
        payload_hash: gameplay_payload_hash(&[]),
    }
}

fn plan() -> GameplayReadPlan {
    GameplayReadPlan {
        module_id: "game.fixture-module".to_owned(),
        invocation_id: "fixture.observe".to_owned(),
        event_id: "event-1".to_owned(),
        wave: 2,
        requests: vec![
            GameplayReadRequest {
                request_id: "read-owner-query".to_owned(),
                view: contract("owner-query-view"),
                fields: vec!["entities".to_owned()],
                selector: GameplayReadSelector::OwnerQuery {
                    query: GameplayOwnerQuery::NearbyEntities {
                        anchor: GameplayEventEntityBinding::Target { index: 0 },
                        radius_millimeters: 5_000,
                        required_tags: vec![],
                        max_items: 2,
                    },
                },
            },
            GameplayReadRequest {
                request_id: "read-target".to_owned(),
                view: contract("capability-view"),
                fields: vec!["staticCollider".to_owned()],
                selector: GameplayReadSelector::Capability {
                    binding: GameplayEventEntityBinding::Target { index: 0 },
                    capability: GameplayCapabilityReadKind::Collision,
                },
            },
            GameplayReadRequest {
                request_id: "read-container".to_owned(),
                view: contract("relationship-view"),
                fields: vec!["entity".to_owned()],
                selector: GameplayReadSelector::Related {
                    binding: GameplayEventEntityBinding::Target { index: 0 },
                    relationship: GameplayRelationshipReadKind::Containment,
                },
            },
            GameplayReadRequest {
                request_id: "read-prefab-part".to_owned(),
                view: contract("prefab-view"),
                fields: vec!["role".to_owned(), "entity".to_owned()],
                selector: GameplayReadSelector::PrefabPart {
                    instance: PrefabInstanceId::new(30),
                    reference: PrefabPartReference {
                        prefab: PrefabId::new(10),
                        role: "gameplay-root".to_owned(),
                    },
                },
            },
            GameplayReadRequest {
                request_id: "read-module".to_owned(),
                view: contract("named-view"),
                fields: vec!["value".to_owned()],
                selector: GameplayReadSelector::ModuleNamed {
                    scope: GameplayModuleStateScope::Session,
                },
            },
        ],
    }
}

#[test]
fn downstream_read_plan_is_typed_bounded_frozen_and_stable() {
    let fixture = fixture();
    let state = state_store(&fixture.registry);
    let owner_query = FixtureOwnerQuery;
    let assembler = GameplayReadAssembler::new(
        &fixture.registry,
        &fixture.entities,
        &state,
        &fixture.prefab_registry,
        &fixture.prefab_instances,
        &fixture.scopes,
        vec![&owner_query],
    )
    .unwrap();

    let first = assembler.assemble(&plan(), &event()).unwrap();
    let second = assembler.assemble(&plan(), &event()).unwrap();
    assert_eq!(first, second);
    assert_eq!(
        first
            .reads
            .iter()
            .map(|read| read.request_id.as_str())
            .collect::<Vec<_>>(),
        vec![
            "read-container",
            "read-module",
            "read-owner-query",
            "read-prefab-part",
            "read-target",
        ]
    );
    let target = first
        .reads
        .iter()
        .find(|read| read.request_id == "read-target")
        .unwrap();
    let GameplayReadValue::Capability { readout } = &target.value else {
        panic!("expected typed capability readout")
    };
    assert_eq!(readout.entity, 2);
    assert_eq!(readout.presence, "active");
    assert_eq!(
        readout.fields["staticCollider"],
        GameplayScalarReadValue::Boolean(false)
    );
    let named = first
        .reads
        .iter()
        .find(|read| read.request_id == "read-module")
        .unwrap();
    assert_eq!(
        named.decode_named_view::<CounterView>().unwrap(),
        CounterView { value: 5 }
    );
    let readout = assembler.read_plan_readout(&plan()).unwrap();
    assert_eq!(readout.entries.len(), 5);
    assert!(readout
        .entries
        .iter()
        .all(|entry| entry.provider_hash.starts_with("fnv1a64:")));
}

#[test]
fn current_trigger_overlaps_flow_through_declared_owner_query_boundary() {
    use core_entity::{Aabb, EntityTransform};
    use core_math::Vec3;
    use rule_trigger_volume::{
        KinematicTriggerDefinition, TriggerReconcileCause, TriggerVolumeRule,
    };

    let mut fixture = fixture();
    fixture
        .entities
        .attach_transform(EntityId::new(1), EntityTransform::IDENTITY);
    fixture.entities.attach_bounds(
        EntityId::new(1),
        Aabb::new(Vec3::splat(-1.0), Vec3::splat(1.0)),
    );
    fixture.entities.attach_collision(EntityId::new(1), true);
    fixture
        .entities
        .attach_transform(EntityId::new(2), EntityTransform::IDENTITY);
    fixture.entities.attach_bounds(
        EntityId::new(2),
        Aabb::new(Vec3::splat(-0.5), Vec3::splat(0.5)),
    );

    let mut triggers = TriggerVolumeRule::new([KinematicTriggerDefinition::new(
        EntityId::new(1),
        "zone.fixture",
        ["door"],
    )])
    .unwrap();
    triggers.reconcile(&fixture.entities, 9, TriggerReconcileCause::Tick);
    let provider = GameplayTriggerOverlapQueryProvider::new("provider.fixture", &triggers);
    let state = state_store(&fixture.registry);
    let assembler = GameplayReadAssembler::new(
        &fixture.registry,
        &fixture.entities,
        &state,
        &fixture.prefab_registry,
        &fixture.prefab_instances,
        &fixture.scopes,
        vec![&provider],
    )
    .unwrap();
    let overlap_plan = GameplayReadPlan {
        module_id: "game.fixture-module".to_owned(),
        invocation_id: "fixture.observe".to_owned(),
        event_id: "event-1".to_owned(),
        wave: 2,
        requests: vec![GameplayReadRequest {
            request_id: "read-trigger-overlaps".to_owned(),
            view: contract("trigger-overlap-view"),
            fields: vec![
                "subjects".to_owned(),
                "providerRevision".to_owned(),
                "overlapHash".to_owned(),
            ],
            selector: GameplayReadSelector::OwnerQuery {
                query: GameplayOwnerQuery::CurrentTriggerOverlaps {
                    trigger: GameplayEventEntityBinding::Source,
                    max_items: 4,
                },
            },
        }],
    };
    let reads = assembler.assemble(&overlap_plan, &event()).unwrap();
    let GameplayReadValue::OwnerQuery {
        result:
            GameplayOwnerQueryResult::CurrentTriggerOverlaps {
                trigger,
                subjects,
                provider_revision,
                overlap_hash,
            },
    } = &reads.reads[0].value
    else {
        panic!("expected current-trigger-overlaps read");
    };
    assert_eq!(*trigger, 1);
    assert_eq!(subjects, &vec![2]);
    assert_eq!(*provider_revision, 1);
    assert!(overlap_hash.starts_with("fnv1a64:"));
}

#[test]
fn coordinator_delivers_declared_reads_and_binds_them_into_delivery_evidence() {
    let fixture = fixture();
    let state = state_store(&fixture.registry);
    let owner_query = FixtureOwnerQuery;
    let views = FixtureViews {
        registry: &fixture.registry,
        entities: &fixture.entities,
        state: &state,
        prefab_registry: &fixture.prefab_registry,
        prefab_instances: &fixture.prefab_instances,
        scopes: &fixture.scopes,
        owner_query: &owner_query,
    };
    let host = ReadHost::default();
    let receipt = GameplayFabricCoordinator::new(
        &fixture.registry,
        GameplayRuntimeLimits {
            max_waves: 2,
            max_events_per_root: 8,
            max_proposals_per_root: 8,
            max_invocations_per_root: 8,
            max_payload_bytes_per_root: 65_536,
        },
    )
    .observe(event(), &views, &host, &mut NoopRouter);
    assert!(receipt.accepted(), "{:?}", receipt.diagnostics);
    let calls = host.calls.borrow();
    assert_eq!(calls.len(), 1);
    let reads = calls[0]
        .declared_reads
        .as_ref()
        .expect("declared reads reach the invocation host");
    assert_eq!(reads.reads.len(), 5);
    assert_eq!(
        receipt.invocations[0].frozen_view_hash,
        views.freeze("root-1", 0).view_hash
    );
    assert!(receipt.invocations[0].delivery_hash.starts_with("fnv1a64:"));
}

#[test]
fn verification_replay_uses_serialized_root_events_views_and_frozen_read_values() {
    let fixture = fixture();
    let state = state_store(&fixture.registry);
    let owner_query = FixtureOwnerQuery;
    let views = FixtureViews {
        registry: &fixture.registry,
        entities: &fixture.entities,
        state: &state,
        prefab_registry: &fixture.prefab_registry,
        prefab_instances: &fixture.prefab_instances,
        scopes: &fixture.scopes,
        owner_query: &owner_query,
    };
    let root_event = event();
    let observe = GameplayFabricCoordinator::new(
        &fixture.registry,
        GameplayRuntimeLimits {
            max_waves: 2,
            max_events_per_root: 8,
            max_proposals_per_root: 8,
            max_invocations_per_root: 8,
            max_payload_bytes_per_root: 65_536,
        },
    )
    .observe(
        root_event.clone(),
        &views,
        &ReadHost::default(),
        &mut NoopRouter,
    );
    let expected = GameplayReactionFrame::from_observe(
        &fixture.registry,
        &observe,
        Vec::new(),
        &observe.module_facts,
        state.state_hash(),
        state.state_hash(),
        state.final_session_hash("activation.fixture"),
    );
    let encoded = serde_json::to_vec(&expected).unwrap();
    let restored: GameplayReactionFrame = serde_json::from_slice(&encoded).unwrap();
    let recorded_reads = restored.invocations[0]
        .declared_reads
        .as_ref()
        .expect("durable frame retains canonical frozen read values");
    assert_eq!(recorded_reads.reads.len(), 5);
    assert!(recorded_reads.nested_hashes_are_valid());

    let verification =
        run_verification_replay(&restored, &RecordedReplayRunner { fixture: &fixture }).unwrap();
    assert!(verification.divergences.is_empty());
    assert_eq!(verification.actual_frame_hash, restored.frame_hash);

    let mut tampered = restored.clone();
    {
        let reads = tampered.invocations[0].declared_reads.as_mut().unwrap();
        reads.reads[0].value = GameplayReadValue::Missing {
            reason: "tampered".to_owned(),
        };
    }
    tampered.frame_hash = tampered.canonical_hash();
    assert!(
        verify_reaction_frame(&restored, &tampered).contains(&GameplayReactionDivergence::Views)
    );

    {
        let reads = tampered.invocations[0].declared_reads.as_mut().unwrap();
        let read = &mut reads.reads[0];
        read.value_hash = read.canonical_value_hash();
        assert!(!reads.nested_hashes_are_valid());
    }
    tampered.frame_hash = tampered.canonical_hash();
    assert!(
        verify_reaction_frame(&restored, &tampered).contains(&GameplayReactionDivergence::Views)
    );
}

#[test]
fn bad_fields_quotas_stale_ids_and_missing_query_provider_fail_without_mutation() {
    let fixture = fixture();
    let state = state_store(&fixture.registry);
    let entity_hash = fixture.entities.hash();
    let state_hash = state.state_hash();
    let owner_query = FixtureOwnerQuery;
    let assembler = GameplayReadAssembler::new(
        &fixture.registry,
        &fixture.entities,
        &state,
        &fixture.prefab_registry,
        &fixture.prefab_instances,
        &fixture.scopes,
        vec![&owner_query],
    )
    .unwrap();

    let mut invalid_field = plan();
    invalid_field.requests[1].fields = vec!["rawStore".to_owned()];
    assert_eq!(
        assembler
            .assemble(&invalid_field, &event())
            .unwrap_err()
            .diagnostics[0]
            .code,
        GameplayReadDiagnosticCode::MissingField
    );

    let mut undeclared = plan();
    undeclared.requests[1].view = contract("private-store-view");
    assert_eq!(
        assembler
            .assemble(&undeclared, &event())
            .unwrap_err()
            .diagnostics[0]
            .code,
        GameplayReadDiagnosticCode::UndeclaredRead
    );

    let mut cross_invocation = plan();
    cross_invocation.requests = vec![GameplayReadRequest {
        request_id: "secondary-owner-query".to_owned(),
        view: contract("owner-query-view"),
        fields: vec!["entities".to_owned()],
        selector: GameplayReadSelector::OwnerQuery {
            query: GameplayOwnerQuery::NearbyEntities {
                anchor: GameplayEventEntityBinding::Target { index: 0 },
                radius_millimeters: 5_000,
                required_tags: Vec::new(),
                max_items: 2,
            },
        },
    }];
    assert_eq!(
        assembler
            .assemble(&cross_invocation, &event())
            .unwrap_err()
            .diagnostics[0]
            .code,
        GameplayReadDiagnosticCode::UndeclaredRead
    );

    let mut unsupported_capability = plan();
    let GameplayReadSelector::Capability { capability, .. } =
        &mut unsupported_capability.requests[1].selector
    else {
        unreachable!()
    };
    *capability = GameplayCapabilityReadKind::Transform;
    assert_eq!(
        assembler
            .assemble(&unsupported_capability, &event())
            .unwrap_err()
            .diagnostics[0]
            .code,
        GameplayReadDiagnosticCode::UnsupportedSelector
    );

    let mut over_budget = plan();
    let GameplayReadSelector::OwnerQuery {
        query: GameplayOwnerQuery::NearbyEntities { max_items, .. },
    } = &mut over_budget.requests[0].selector
    else {
        unreachable!()
    };
    *max_items = 5;
    assert_eq!(
        assembler
            .assemble(&over_budget, &event())
            .unwrap_err()
            .diagnostics[0]
            .code,
        GameplayReadDiagnosticCode::QuotaExceeded
    );

    let mut foreign_prefab = plan();
    let GameplayReadSelector::PrefabPart { reference, .. } =
        &mut foreign_prefab.requests[3].selector
    else {
        unreachable!()
    };
    reference.prefab = PrefabId::new(11);
    assert_eq!(
        assembler
            .assemble(&foreign_prefab, &event())
            .unwrap_err()
            .diagnostics[0]
            .code,
        GameplayReadDiagnosticCode::MissingPrefab
    );

    let mut stale = event();
    stale.targets[0].entity = EntityId::new(999);
    assert_eq!(
        assembler.assemble(&plan(), &stale).unwrap_err().diagnostics[0].code,
        GameplayReadDiagnosticCode::StaleIdentity
    );

    let no_query = GameplayReadAssembler::new(
        &fixture.registry,
        &fixture.entities,
        &state,
        &fixture.prefab_registry,
        &fixture.prefab_instances,
        &fixture.scopes,
        vec![],
    )
    .unwrap();
    assert_eq!(
        no_query
            .assemble(&plan(), &event())
            .unwrap_err()
            .diagnostics[0]
            .code,
        GameplayReadDiagnosticCode::MissingOwnerQueryProvider
    );
    assert_eq!(fixture.entities.hash(), entity_hash);
    assert_eq!(state.state_hash(), state_hash);
}

#[test]
fn frozen_named_view_does_not_observe_later_same_wave_state() {
    let fixture = fixture();
    let mut state = state_store(&fixture.registry);
    let owner_query = FixtureOwnerQuery;
    let named_plan = GameplayReadPlan {
        requests: vec![plan().requests.pop().unwrap()],
        ..plan()
    };
    let before = GameplayReadAssembler::new(
        &fixture.registry,
        &fixture.entities,
        &state,
        &fixture.prefab_registry,
        &fixture.prefab_instances,
        &fixture.scopes,
        vec![&owner_query],
    )
    .unwrap()
    .assemble(&named_plan, &event())
    .unwrap();
    let fact = serde_json::to_vec(&3_u64).unwrap();
    state
        .apply_fact(GameplayModuleFact {
            fact_id: "later-same-wave".to_owned(),
            module_id: "game.fixture-module".to_owned(),
            fact_schema: contract("counter-fact"),
            state_schema: contract("counter-state"),
            scope: GameplayModuleStateScope::Session,
            expected_revision: 0,
            payload_hash: gameplay_module_payload_hash(&fact),
            canonical_payload: fact,
        })
        .unwrap();
    let after = GameplayReadAssembler::new(
        &fixture.registry,
        &fixture.entities,
        &state,
        &fixture.prefab_registry,
        &fixture.prefab_instances,
        &fixture.scopes,
        vec![&owner_query],
    )
    .unwrap()
    .assemble(&named_plan, &event())
    .unwrap();
    assert_eq!(
        before.reads[0].decode_named_view::<CounterView>().unwrap(),
        CounterView { value: 5 }
    );
    assert_eq!(
        after.reads[0].decode_named_view::<CounterView>().unwrap(),
        CounterView { value: 8 }
    );
    assert_ne!(before.read_set_hash, after.read_set_hash);
}
