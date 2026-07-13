use core_game_rules::{ReactionWindowId, ValueChannelId};
use protocol_game_extension::{
    GameplayCausationRef, GameplayContractRef, GameplayEmitterRef, GameplayEventEnvelope,
    GameplayEventPhase, GameplayEventSchemaDeclaration, GameplayExecutionBudget,
    GameplayHeaderSelector, GameplayInvocationDescriptor, GameplayInvocationFamily,
    GameplayModuleManifest, GameplayModuleRef, GameplayOrderingConstraint, GameplayOwnerRef,
    GameplayProposalDeclaration, GameplayProposalEnvelope, GameplaySubscriptionDeclaration,
};
use rule_gameplay_fabric::{
    resolve_declared_reactions, FrozenGameplayViews, GameplayDecisionContinuations,
    GameplayDecisionMoment, GameplayDecisionOutput, GameplayDecisionOwner, GameplayDecisionStatus,
    GameplayFabricCoordinator, GameplayGuardVote, GameplayInvocationCall, GameplayInvocationHost,
    GameplayInvocationInput, GameplayInvocationOutput, GameplayOperationWorkspace,
    GameplayOwnerRoutingCall, GameplayOwnerRoutingOutput, GameplayProposalRouter,
    GameplayReactionDisposition, GameplayRuntimeDiagnosticCode, GameplayRuntimeLimits,
    GameplayViewSource, GameplayWorkspaceTransform, ReactionBehavior, ReactionDefinition,
    ReactionResolutionInput, ReactionWindowKind,
};
use std::cell::{Cell, RefCell};
use std::collections::BTreeSet;
use std::rc::Rc;
use svc_gameplay_fabric::{
    GameplayFabricRegistry, GameplayFabricRegistryBuilder, GameplayLinkedProvider,
    GameplayProposalOwnerRegistration, TypedGameplayEventCodec,
};

fn contract(namespace: &str, name: &str) -> GameplayContractRef {
    GameplayContractRef {
        namespace: namespace.to_owned(),
        name: name.to_owned(),
        version: 1,
        schema_hash: format!("sha256:{namespace}.{name}.v1"),
    }
}

fn root_contract() -> GameplayContractRef {
    contract("game.authority", "root-event")
}

fn applied_contract() -> GameplayContractRef {
    contract("game.authority", "proposal-applied")
}

fn proposal_contract() -> GameplayContractRef {
    contract("game.shared", "change-request")
}

fn owner() -> GameplayOwnerRef {
    GameplayOwnerRef {
        owner_id: "authority.game".to_owned(),
        provider_id: "provider.authority".to_owned(),
    }
}

fn budget() -> GameplayExecutionBudget {
    GameplayExecutionBudget {
        max_waves: 8,
        max_events_per_root: 64,
        max_proposals_per_root: 32,
        max_invocations_per_root: 64,
        max_payload_bytes_per_root: 65_536,
    }
}

fn module(module_id: &str, namespace: &str, provider_id: &str) -> GameplayModuleManifest {
    GameplayModuleManifest {
        module_ref: GameplayModuleRef {
            module_id: module_id.to_owned(),
            namespace: namespace.to_owned(),
            version: "1.0.0".to_owned(),
            sdk_hash: "sha256:sdk".to_owned(),
            contract_hash: format!("sha256:{module_id}.contract"),
            artifact_hash: format!("sha256:{module_id}.artifact"),
            provider_id: provider_id.to_owned(),
        },
        published_events: Vec::new(),
        subscriptions: Vec::new(),
        invocations: Vec::new(),
        read_views: Vec::new(),
        proposal_kinds: Vec::new(),
        state_schemas: Vec::new(),
        fact_schemas: Vec::new(),
        ordering: Vec::new(),
        budget: budget(),
        deterministic_requirements: vec!["canonical-input-order".to_owned()],
        source_hash: format!("sha256:{module_id}.source"),
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

fn event_declaration(event: GameplayContractRef) -> GameplayEventSchemaDeclaration {
    GameplayEventSchemaDeclaration {
        event,
        codec_id: "asha.bytes-v1".to_owned(),
    }
}

fn codec(declaration: GameplayEventSchemaDeclaration) -> TypedGameplayEventCodec<Vec<u8>> {
    TypedGameplayEventCodec::new(
        declaration,
        |payload| Ok(payload.clone()),
        |bytes| Ok(bytes.to_vec()),
    )
}

fn add_observe(manifest: &mut GameplayModuleManifest, suffix: &str, input: GameplayContractRef) {
    let invocation_id = format!("observe-{suffix}");
    manifest.invocations.push(GameplayInvocationDescriptor {
        invocation_id: invocation_id.clone(),
        family: GameplayInvocationFamily::Observe,
        input_contract: input.clone(),
        output_contract: proposal_contract(),
        read_requirements: Vec::new(),
        max_outputs: 4,
        max_payload_bytes: 4_096,
    });
    manifest
        .subscriptions
        .push(GameplaySubscriptionDeclaration {
            subscription_id: format!("{}.observe-{suffix}", manifest.module_ref.module_id),
            event: input,
            invocation_id,
            selector: GameplayHeaderSelector {
                source: None,
                target: None,
                scope: None,
                required_tags: vec!["gameplay".to_owned()],
            },
            max_deliveries_per_root: 8,
        });
}

fn registry(observer_count: usize, observe_applied: bool) -> GameplayFabricRegistry {
    let mut authority = module(
        "game.authority-module",
        "game.authority",
        "provider.authority",
    );
    authority
        .published_events
        .push(event_declaration(root_contract()));
    authority
        .published_events
        .push(event_declaration(applied_contract()));

    let mut observers = Vec::new();
    for index in 0..observer_count {
        let letter = char::from(b'a' + index as u8);
        let module_id = format!("game.observer-{letter}");
        let namespace = format!("game.observer{letter}");
        let provider_id = format!("provider.observer-{letter}");
        let mut observer = module(&module_id, &namespace, &provider_id);
        add_observe(&mut observer, "root", root_contract());
        if observe_applied {
            add_observe(&mut observer, "applied", applied_contract());
        }
        observer.proposal_kinds.push(GameplayProposalDeclaration {
            proposal: proposal_contract(),
            owner: owner(),
        });
        if index + 1 < observer_count {
            observer.ordering.push(GameplayOrderingConstraint {
                before_module: module_id,
                after_module: format!("game.observer-{}", char::from(letter as u8 + 1)),
            });
        }
        observers.push(observer);
    }

    let mut builder = GameplayFabricRegistryBuilder::new();
    builder
        .register_event_codec(codec(event_declaration(root_contract())))
        .register_event_codec(codec(event_declaration(applied_contract())))
        .register_proposal_owner(GameplayProposalOwnerRegistration {
            proposal: proposal_contract(),
            owner: owner(),
        })
        .register_linked_provider(provider(&authority))
        .register_module(authority);
    for observer in observers.into_iter().rev() {
        builder
            .register_linked_provider(provider(&observer))
            .register_module(observer);
    }
    builder.build().expect("valid Observe registry")
}

fn dummy_event(event: GameplayContractRef) -> GameplayEventEnvelope {
    GameplayEventEnvelope {
        event_id: "module-controlled-id".to_owned(),
        event,
        tick: 999,
        root_sequence: 999,
        wave: 999,
        event_sequence: 999,
        phase: GameplayEventPhase::ScheduledMoment,
        emitter: GameplayEmitterRef::Scheduler {
            scheduler_id: "module-controlled-emitter".to_owned(),
        },
        causation: GameplayCausationRef {
            root_id: "module-controlled-root".to_owned(),
            parent_event_id: None,
            decision_id: None,
        },
        source: None,
        subjects: Vec::new(),
        targets: Vec::new(),
        scope: None,
        tags: vec!["gameplay".to_owned()],
        canonical_payload: vec![7],
        payload_hash: "sha256:payload".to_owned(),
    }
}

fn root_event() -> GameplayEventEnvelope {
    let mut event = dummy_event(root_contract());
    event.event_id = "root-event-1".to_owned();
    event.tick = 41;
    event.root_sequence = 7;
    event.phase = GameplayEventPhase::PostCommit;
    event.emitter = GameplayEmitterRef::Owner {
        owner_id: "authority.game".to_owned(),
    };
    event.causation.root_id = "root-7".to_owned();
    event
}

fn dummy_proposal() -> GameplayProposalEnvelope {
    GameplayProposalEnvelope {
        proposal_id: "module-controlled-id".to_owned(),
        proposal: proposal_contract(),
        tick: 999,
        root_sequence: 999,
        wave: 999,
        proposal_sequence: 999,
        emitter: GameplayEmitterRef::Scheduler {
            scheduler_id: "module-controlled-emitter".to_owned(),
        },
        causation: GameplayCausationRef {
            root_id: "module-controlled-root".to_owned(),
            parent_event_id: None,
            decision_id: None,
        },
        originating_event_id: None,
        source: None,
        targets: Vec::new(),
        canonical_payload: vec![1, 2, 3],
        payload_hash: "sha256:proposal-payload".to_owned(),
    }
}

fn limits(max_waves: u32) -> GameplayRuntimeLimits {
    GameplayRuntimeLimits {
        max_waves,
        max_events_per_root: 64,
        max_proposals_per_root: 32,
        max_invocations_per_root: 64,
        max_payload_bytes_per_root: 65_536,
    }
}

struct StateViews {
    routed: Rc<Cell<u32>>,
}

impl GameplayViewSource for StateViews {
    fn freeze(&self, _root_id: &str, wave: u32) -> FrozenGameplayViews {
        FrozenGameplayViews {
            epoch: u64::from(wave),
            view_hash: format!("state:{}", self.routed.get()),
        }
    }
}

struct ProposalHost {
    routed: Rc<Cell<u32>>,
    calls: RefCell<Vec<GameplayInvocationCall>>,
}

impl GameplayInvocationHost for ProposalHost {
    fn invoke(
        &self,
        call: &GameplayInvocationCall,
    ) -> Result<GameplayInvocationOutput, rule_gameplay_fabric::GameplayHostError> {
        if call
            .input
            .observe_event()
            .is_some_and(|event| event.event == root_contract())
        {
            assert_eq!(
                self.routed.get(),
                0,
                "same-wave output was routed too early"
            );
        }
        self.calls.borrow_mut().push(call.clone());
        Ok(GameplayInvocationOutput {
            events: Vec::new(),
            proposals: vec![dummy_proposal()],
            module_facts: Vec::new(),
            trace_codes: vec!["proposal-created".to_owned()],
            decision: None,
        })
    }
}

struct AcceptingRouter {
    routed: Rc<Cell<u32>>,
}

impl GameplayProposalRouter for AcceptingRouter {
    fn route(&mut self, call: &GameplayOwnerRoutingCall) -> GameplayOwnerRoutingOutput {
        assert_eq!(call.owner, owner());
        assert_eq!(
            call.proposal.emitter,
            GameplayEmitterRef::Module {
                module_id: call.proposal.emitter_module_id().to_owned(),
            }
        );
        self.routed.set(self.routed.get() + 1);
        GameplayOwnerRoutingOutput {
            accepted: true,
            fact_hashes: vec![format!("fact:{}", self.routed.get())],
            events: vec![dummy_event(applied_contract())],
            diagnostic_codes: Vec::new(),
        }
    }
}

trait EmitterModuleId {
    fn emitter_module_id(&self) -> &str;
}

impl EmitterModuleId for GameplayProposalEnvelope {
    fn emitter_module_id(&self) -> &str {
        match &self.emitter {
            GameplayEmitterRef::Module { module_id } => module_id,
            _ => panic!("coordinator did not replace module-controlled proposal emitter"),
        }
    }
}

fn run_buffered_observe() -> (
    rule_gameplay_fabric::GameplayObserveReceipt,
    Vec<GameplayInvocationCall>,
) {
    let registry = registry(2, false);
    let routed = Rc::new(Cell::new(0));
    let views = StateViews {
        routed: Rc::clone(&routed),
    };
    let host = ProposalHost {
        routed: Rc::clone(&routed),
        calls: RefCell::new(Vec::new()),
    };
    let mut router = AcceptingRouter { routed };
    let receipt = GameplayFabricCoordinator::new(&registry, limits(4)).observe(
        root_event(),
        &views,
        &host,
        &mut router,
    );
    let calls = host.calls.into_inner();
    (receipt, calls)
}

#[test]
fn observe_buffers_same_wave_outputs_and_routes_in_validated_module_order() {
    let (receipt, calls) = run_buffered_observe();

    assert!(receipt.accepted(), "{:#?}", receipt.diagnostics);
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].module_id, "game.observer-a");
    assert_eq!(calls[1].module_id, "game.observer-b");
    assert_eq!(calls[0].frozen_views, calls[1].frozen_views);
    assert_eq!(receipt.wave_views[0].view_hash, "state:0");
    assert_eq!(receipt.wave_views[1].view_hash, "state:2");
    assert_eq!(receipt.routing.len(), 2);
    assert_eq!(receipt.events.len(), 3);
    assert!(receipt.events[1..].iter().all(|event| {
        event.wave == 1
            && event.tick == 41
            && event.root_sequence == 7
            && matches!(event.emitter, GameplayEmitterRef::Owner { .. })
    }));
}

#[test]
fn identical_inputs_and_registry_digest_produce_identical_receipt_hashes() {
    let (first, _) = run_buffered_observe();
    let (second, _) = run_buffered_observe();

    assert_eq!(first.registry_digest, second.registry_digest);
    assert_eq!(first.event_evidence, second.event_evidence);
    assert_eq!(first.invocations, second.invocations);
    assert_eq!(first.routing, second.routing);
    assert_eq!(first.receipt_hash, second.receipt_hash);
}

fn reverse_declared_subscription_registry() -> GameplayFabricRegistry {
    let mut authority = module(
        "game.authority-module",
        "game.authority",
        "provider.authority",
    );
    authority
        .published_events
        .push(event_declaration(root_contract()));
    authority
        .published_events
        .push(event_declaration(applied_contract()));

    let mut observer = module("game.ordered-observer", "game.ordered", "provider.ordered");
    observer.proposal_kinds.push(GameplayProposalDeclaration {
        proposal: proposal_contract(),
        owner: owner(),
    });
    for suffix in ["z", "a"] {
        add_observe(&mut observer, suffix, root_contract());
    }

    let mut builder = GameplayFabricRegistryBuilder::new();
    builder
        .register_event_codec(codec(event_declaration(root_contract())))
        .register_event_codec(codec(event_declaration(applied_contract())))
        .register_proposal_owner(GameplayProposalOwnerRegistration {
            proposal: proposal_contract(),
            owner: owner(),
        })
        .register_linked_provider(provider(&authority))
        .register_linked_provider(provider(&observer))
        .register_module(authority)
        .register_module(observer);
    builder.build().expect("valid ordered registry")
}

#[test]
fn scheduled_moments_use_stable_subscription_ids_not_manifest_insertion_order() {
    let registry = reverse_declared_subscription_registry();
    let routed = Rc::new(Cell::new(0));
    let views = StateViews {
        routed: Rc::clone(&routed),
    };
    let host = ProposalHost {
        routed: Rc::clone(&routed),
        calls: RefCell::new(Vec::new()),
    };
    let mut router = AcceptingRouter { routed };
    let mut scheduled = root_event();
    scheduled.phase = GameplayEventPhase::ScheduledMoment;
    let receipt = GameplayFabricCoordinator::new(&registry, limits(4)).observe(
        scheduled,
        &views,
        &host,
        &mut router,
    );

    assert!(receipt.accepted(), "{:#?}", receipt.diagnostics);
    let calls = host.calls.into_inner();
    assert_eq!(calls[0].subscription_id, "game.ordered-observer.observe-a");
    assert_eq!(calls[1].subscription_id, "game.ordered-observer.observe-z");
    assert!(calls.iter().all(|call| {
        call.input
            .observe_event()
            .is_some_and(|event| event.phase == GameplayEventPhase::ScheduledMoment)
    }));
}

struct UndeclaredEventHost;

impl GameplayInvocationHost for UndeclaredEventHost {
    fn invoke(
        &self,
        _call: &GameplayInvocationCall,
    ) -> Result<GameplayInvocationOutput, rule_gameplay_fabric::GameplayHostError> {
        Ok(GameplayInvocationOutput {
            events: vec![dummy_event(contract("game.unknown", "event"))],
            proposals: Vec::new(),
            module_facts: Vec::new(),
            trace_codes: Vec::new(),
            decision: None,
        })
    }
}

struct RejectUnexpectedRoute;

impl GameplayProposalRouter for RejectUnexpectedRoute {
    fn route(&mut self, _call: &GameplayOwnerRoutingCall) -> GameplayOwnerRoutingOutput {
        panic!("invalid invocation output must not reach authority routing")
    }
}

#[test]
fn undeclared_outputs_fail_the_wave_before_owner_routing() {
    let registry = registry(1, false);
    let routed = Rc::new(Cell::new(0));
    let views = StateViews { routed };
    let mut router = RejectUnexpectedRoute;
    let receipt = GameplayFabricCoordinator::new(&registry, limits(4)).observe(
        root_event(),
        &views,
        &UndeclaredEventHost,
        &mut router,
    );

    assert!(!receipt.accepted());
    assert_eq!(receipt.events.len(), 1);
    assert!(receipt
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == GameplayRuntimeDiagnosticCode::UndeclaredEvent));
}

#[test]
fn cascade_exhaustion_is_a_typed_failure_instead_of_truncation_or_recursion() {
    let registry = registry(1, true);
    let routed = Rc::new(Cell::new(0));
    let views = StateViews {
        routed: Rc::clone(&routed),
    };
    let host = ProposalHost {
        routed: Rc::new(Cell::new(0)),
        calls: RefCell::new(Vec::new()),
    };
    let mut router = AcceptingRouter { routed };
    let receipt = GameplayFabricCoordinator::new(&registry, limits(2)).observe(
        root_event(),
        &views,
        &host,
        &mut router,
    );

    assert!(!receipt.accepted());
    assert_eq!(receipt.waves_processed, 2);
    assert!(receipt
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == GameplayRuntimeDiagnosticCode::WaveBudgetExceeded));
}

fn workspace_contract() -> GameplayContractRef {
    contract("game.shared", "operation-workspace")
}

fn decision_registry() -> GameplayFabricRegistry {
    let authority = module(
        "game.authority-module",
        "game.authority",
        "provider.authority",
    );
    let mut participant = module(
        "game.decision-participant",
        "game.decision",
        "provider.decision",
    );
    participant
        .proposal_kinds
        .push(GameplayProposalDeclaration {
            proposal: proposal_contract(),
            owner: owner(),
        });
    for family in [
        GameplayInvocationFamily::Guard,
        GameplayInvocationFamily::Transform,
        GameplayInvocationFamily::React,
    ] {
        participant.invocations.push(GameplayInvocationDescriptor {
            invocation_id: format!("{}-operation", family.as_str()),
            family,
            input_contract: proposal_contract(),
            output_contract: workspace_contract(),
            read_requirements: Vec::new(),
            max_outputs: 1,
            max_payload_bytes: 4_096,
        });
    }

    let mut builder = GameplayFabricRegistryBuilder::new();
    builder
        .register_proposal_owner(GameplayProposalOwnerRegistration {
            proposal: proposal_contract(),
            owner: owner(),
        })
        .register_linked_provider(provider(&authority))
        .register_linked_provider(provider(&participant))
        .register_module(authority)
        .register_module(participant);
    builder.build().expect("valid decision registry")
}

fn decision_moment(expected_owner_revision: &str) -> GameplayDecisionMoment {
    let workspace = GameplayOperationWorkspace::from_payload(workspace_contract(), vec![1]);
    let mut operation = dummy_proposal();
    operation.proposal_id = "operation-1".to_owned();
    operation.canonical_payload = workspace.canonical_payload.clone();
    operation.payload_hash = workspace.workspace_hash.clone();
    GameplayDecisionMoment {
        decision_id: "decision-1".to_owned(),
        operation,
        expected_owner_revision: expected_owner_revision.to_owned(),
        workspace,
        resume_token: None,
    }
}

#[derive(Clone, Copy)]
enum ReactBehavior {
    Continue,
    Suspend,
}

struct DecisionHost {
    reject_guard: bool,
    react: ReactBehavior,
    calls: RefCell<Vec<GameplayInvocationFamily>>,
}

impl GameplayInvocationHost for DecisionHost {
    fn invoke(
        &self,
        call: &GameplayInvocationCall,
    ) -> Result<GameplayInvocationOutput, rule_gameplay_fabric::GameplayHostError> {
        self.calls.borrow_mut().push(call.family);
        let GameplayInvocationInput::Decision(moment) = &call.input else {
            panic!("decision host received Observe input")
        };
        let decision = match call.family {
            GameplayInvocationFamily::Guard => {
                GameplayDecisionOutput::Guard(if self.reject_guard {
                    GameplayGuardVote::Reject
                } else {
                    GameplayGuardVote::Accept
                })
            }
            GameplayInvocationFamily::Transform => {
                GameplayDecisionOutput::Transform(GameplayWorkspaceTransform {
                    input_workspace_hash: moment.workspace.workspace_hash.clone(),
                    workspace: GameplayOperationWorkspace::from_payload(
                        workspace_contract(),
                        vec![2],
                    ),
                })
            }
            GameplayInvocationFamily::React => GameplayDecisionOutput::React {
                disposition: match self.react {
                    ReactBehavior::Continue => GameplayReactionDisposition::Continue,
                    ReactBehavior::Suspend => GameplayReactionDisposition::Suspend {
                        token: "reaction-token-1".to_owned(),
                    },
                },
                transform: None,
            },
            GameplayInvocationFamily::Observe => panic!("unexpected Observe family"),
        };
        Ok(GameplayInvocationOutput {
            events: Vec::new(),
            proposals: Vec::new(),
            module_facts: Vec::new(),
            trace_codes: vec![call.family.as_str().to_owned()],
            decision: Some(decision),
        })
    }
}

struct DecisionOwner {
    revision: RefCell<String>,
    commits: Cell<u32>,
}

impl GameplayDecisionOwner for DecisionOwner {
    fn revision_hash(&self, expected_owner: &GameplayOwnerRef) -> String {
        assert_eq!(expected_owner, &owner());
        self.revision.borrow().clone()
    }

    fn route_precommit(&mut self, call: &GameplayOwnerRoutingCall) -> GameplayOwnerRoutingOutput {
        assert_eq!(call.owner, owner());
        assert_eq!(call.proposal.canonical_payload, vec![2]);
        self.commits.set(self.commits.get() + 1);
        *self.revision.borrow_mut() = format!("revision-{}", self.commits.get() + 1);
        GameplayOwnerRoutingOutput {
            accepted: true,
            fact_hashes: vec!["fact:operation-accepted".to_owned()],
            events: Vec::new(),
            diagnostic_codes: Vec::new(),
        }
    }
}

fn decision_views() -> StateViews {
    StateViews {
        routed: Rc::new(Cell::new(0)),
    }
}

#[test]
fn precommit_runs_guard_transform_react_then_commits_once() {
    let registry = decision_registry();
    let host = DecisionHost {
        reject_guard: false,
        react: ReactBehavior::Continue,
        calls: RefCell::new(Vec::new()),
    };
    let mut owner_port = DecisionOwner {
        revision: RefCell::new("revision-1".to_owned()),
        commits: Cell::new(0),
    };
    let mut continuations = GameplayDecisionContinuations::default();
    let receipt = GameplayFabricCoordinator::new(&registry, limits(4)).decide(
        decision_moment("revision-1"),
        &mut continuations,
        &decision_views(),
        &host,
        &mut owner_port,
    );

    assert!(receipt.accepted(), "{:#?}", receipt.diagnostics);
    assert_eq!(owner_port.commits.get(), 1);
    assert_eq!(
        host.calls.into_inner(),
        vec![
            GameplayInvocationFamily::Guard,
            GameplayInvocationFamily::Transform,
            GameplayInvocationFamily::React,
        ]
    );
    assert_eq!(
        receipt.final_workspace_hash,
        GameplayOperationWorkspace::from_payload(workspace_contract(), vec![2]).workspace_hash
    );
    assert!(receipt.routing.is_some());
}

#[test]
fn guard_rejection_leaves_authority_untouched() {
    let registry = decision_registry();
    let host = DecisionHost {
        reject_guard: true,
        react: ReactBehavior::Continue,
        calls: RefCell::new(Vec::new()),
    };
    let mut owner_port = DecisionOwner {
        revision: RefCell::new("revision-1".to_owned()),
        commits: Cell::new(0),
    };
    let mut continuations = GameplayDecisionContinuations::default();
    let receipt = GameplayFabricCoordinator::new(&registry, limits(4)).decide(
        decision_moment("revision-1"),
        &mut continuations,
        &decision_views(),
        &host,
        &mut owner_port,
    );

    assert_eq!(receipt.status, GameplayDecisionStatus::Rejected);
    assert_eq!(owner_port.commits.get(), 0);
    assert!(receipt.routing.is_none());
    assert_eq!(
        host.calls.into_inner(),
        vec![GameplayInvocationFamily::Guard]
    );
}

#[test]
fn suspended_reaction_is_explicit_and_does_not_commit() {
    let registry = decision_registry();
    let host = DecisionHost {
        reject_guard: false,
        react: ReactBehavior::Suspend,
        calls: RefCell::new(Vec::new()),
    };
    let mut owner_port = DecisionOwner {
        revision: RefCell::new("revision-1".to_owned()),
        commits: Cell::new(0),
    };
    let mut continuations = GameplayDecisionContinuations::default();
    let receipt = GameplayFabricCoordinator::new(&registry, limits(4)).decide(
        decision_moment("revision-1"),
        &mut continuations,
        &decision_views(),
        &host,
        &mut owner_port,
    );

    assert_eq!(receipt.status, GameplayDecisionStatus::Suspended);
    let continuation = receipt.continuation.as_ref().expect("continuation record");
    assert_eq!(
        receipt.suspension_token.as_deref(),
        Some(continuation.token.as_str())
    );
    assert_ne!(continuation.token, "reaction-token-1");
    assert_eq!(continuation.decision_id, "decision-1");
    assert_eq!(continuation.expected_owner_revision, "revision-1");
    assert_eq!(continuation.generation, 1);
    assert_eq!(continuations.pending("decision-1"), Some(continuation));
    assert_eq!(owner_port.commits.get(), 0);
    assert!(receipt.routing.is_none());
}

struct RejectUnexpectedInvocation;

impl GameplayInvocationHost for RejectUnexpectedInvocation {
    fn invoke(
        &self,
        _call: &GameplayInvocationCall,
    ) -> Result<GameplayInvocationOutput, rule_gameplay_fabric::GameplayHostError> {
        panic!("stale decision must fail before module invocation")
    }
}

#[test]
fn stale_suspended_resume_fails_before_invocation_or_commit() {
    let registry = decision_registry();
    let suspending_host = DecisionHost {
        reject_guard: false,
        react: ReactBehavior::Suspend,
        calls: RefCell::new(Vec::new()),
    };
    let mut continuations = GameplayDecisionContinuations::default();
    let mut owner_port = DecisionOwner {
        revision: RefCell::new("revision-1".to_owned()),
        commits: Cell::new(0),
    };
    let suspended = GameplayFabricCoordinator::new(&registry, limits(4)).decide(
        decision_moment("revision-1"),
        &mut continuations,
        &decision_views(),
        &suspending_host,
        &mut owner_port,
    );
    let continuation = suspended.continuation.expect("continuation record");
    *owner_port.revision.borrow_mut() = "revision-2".to_owned();
    let mut resumed = decision_moment("revision-1");
    resumed.workspace = continuation.workspace;
    resumed.resume_token = Some(continuation.token);
    let receipt = GameplayFabricCoordinator::new(&registry, limits(4)).decide(
        resumed,
        &mut continuations,
        &decision_views(),
        &RejectUnexpectedInvocation,
        &mut owner_port,
    );

    assert_eq!(receipt.status, GameplayDecisionStatus::Stale);
    assert_eq!(owner_port.commits.get(), 0);
    assert!(receipt.routing.is_none());
}

#[test]
fn suspended_continuation_rejects_missing_wrong_and_replayed_tokens() {
    let registry = decision_registry();
    let suspending_host = DecisionHost {
        reject_guard: false,
        react: ReactBehavior::Suspend,
        calls: RefCell::new(Vec::new()),
    };
    let mut owner_port = DecisionOwner {
        revision: RefCell::new("revision-1".to_owned()),
        commits: Cell::new(0),
    };
    let mut continuations = GameplayDecisionContinuations::default();
    let suspended = GameplayFabricCoordinator::new(&registry, limits(4)).decide(
        decision_moment("revision-1"),
        &mut continuations,
        &decision_views(),
        &suspending_host,
        &mut owner_port,
    );
    let continuation = suspended.continuation.expect("continuation record");

    let mut missing = decision_moment("revision-1");
    missing.workspace = continuation.workspace.clone();
    let missing_receipt = GameplayFabricCoordinator::new(&registry, limits(4)).decide(
        missing,
        &mut continuations,
        &decision_views(),
        &RejectUnexpectedInvocation,
        &mut owner_port,
    );
    assert_eq!(missing_receipt.status, GameplayDecisionStatus::Failed);
    assert!(missing_receipt
        .diagnostics
        .iter()
        .any(|item| { item.code == GameplayRuntimeDiagnosticCode::ContinuationRequired }));

    let mut wrong = decision_moment("revision-1");
    wrong.workspace = continuation.workspace.clone();
    wrong.resume_token = Some("wrong-token".to_owned());
    let wrong_receipt = GameplayFabricCoordinator::new(&registry, limits(4)).decide(
        wrong,
        &mut continuations,
        &decision_views(),
        &RejectUnexpectedInvocation,
        &mut owner_port,
    );
    assert_eq!(wrong_receipt.status, GameplayDecisionStatus::Failed);
    assert!(wrong_receipt
        .diagnostics
        .iter()
        .any(|item| { item.code == GameplayRuntimeDiagnosticCode::ContinuationMismatch }));

    let continuing_host = DecisionHost {
        reject_guard: false,
        react: ReactBehavior::Continue,
        calls: RefCell::new(Vec::new()),
    };
    let mut correct = decision_moment("revision-1");
    correct.workspace = continuation.workspace.clone();
    correct.resume_token = Some(continuation.token.clone());
    let accepted = GameplayFabricCoordinator::new(&registry, limits(4)).decide(
        correct,
        &mut continuations,
        &decision_views(),
        &continuing_host,
        &mut owner_port,
    );
    assert!(accepted.accepted(), "{:#?}", accepted.diagnostics);
    assert_eq!(owner_port.commits.get(), 1);

    let mut replayed = decision_moment("revision-1");
    replayed.workspace = continuation.workspace;
    replayed.resume_token = Some(continuation.token);
    let replayed_receipt = GameplayFabricCoordinator::new(&registry, limits(4)).decide(
        replayed,
        &mut continuations,
        &decision_views(),
        &RejectUnexpectedInvocation,
        &mut owner_port,
    );
    assert_eq!(replayed_receipt.status, GameplayDecisionStatus::Failed);
    assert!(replayed_receipt
        .diagnostics
        .iter()
        .any(|item| { item.code == GameplayRuntimeDiagnosticCode::ContinuationUnavailable }));
    assert_eq!(owner_port.commits.get(), 1);
}

struct ExistingReactionHost {
    observed_order: RefCell<Vec<String>>,
}

impl GameplayInvocationHost for ExistingReactionHost {
    fn invoke(
        &self,
        call: &GameplayInvocationCall,
    ) -> Result<GameplayInvocationOutput, rule_gameplay_fabric::GameplayHostError> {
        let GameplayInvocationInput::Decision(moment) = &call.input else {
            panic!("reaction host received Observe input")
        };
        let decision = match call.family {
            GameplayInvocationFamily::Guard => {
                GameplayDecisionOutput::Guard(GameplayGuardVote::Accept)
            }
            GameplayInvocationFamily::Transform => {
                GameplayDecisionOutput::Transform(GameplayWorkspaceTransform {
                    input_workspace_hash: moment.workspace.workspace_hash.clone(),
                    workspace: GameplayOperationWorkspace::from_payload(
                        workspace_contract(),
                        vec![2],
                    ),
                })
            }
            GameplayInvocationFamily::React => {
                let health = ValueChannelId::parse("value.health").expect("channel id");
                let definitions = vec![
                    ReactionDefinition::new(
                        ReactionWindowId::parse("reaction.z").expect("reaction id"),
                        ReactionWindowKind::PendingValueDelta,
                        ReactionBehavior::ModifyPendingDelta {
                            channel: health.clone(),
                            amount: 1,
                        },
                    )
                    .with_priority(1),
                    ReactionDefinition::new(
                        ReactionWindowId::parse("reaction.a").expect("reaction id"),
                        ReactionWindowKind::PendingValueDelta,
                        ReactionBehavior::EmitTrace {
                            code: "reaction.trace".to_owned(),
                            message: "stable tie breaker".to_owned(),
                        },
                    )
                    .with_priority(1),
                    ReactionDefinition::new(
                        ReactionWindowId::parse("reaction.high").expect("reaction id"),
                        ReactionWindowKind::PendingValueDelta,
                        ReactionBehavior::ModifyPendingDelta {
                            channel: health.clone(),
                            amount: 3,
                        },
                    )
                    .with_priority(10),
                ];
                let resolution = resolve_declared_reactions(
                    &definitions,
                    &ReactionResolutionInput {
                        window: ReactionWindowKind::PendingValueDelta,
                        channel: Some(health.clone()),
                        pending_delta: -10,
                        declared_reads: BTreeSet::from([health]),
                        allowed_effect_ops: BTreeSet::new(),
                        allowed_modifiers: BTreeSet::new(),
                    },
                );
                assert_eq!(resolution.pending_delta, -6);
                *self.observed_order.borrow_mut() = resolution
                    .trace
                    .iter()
                    .map(|entry| entry.refs[0].1.clone())
                    .collect();
                GameplayDecisionOutput::React {
                    disposition: GameplayReactionDisposition::Continue,
                    transform: None,
                }
            }
            GameplayInvocationFamily::Observe => panic!("unexpected Observe family"),
        };
        Ok(GameplayInvocationOutput {
            events: Vec::new(),
            proposals: Vec::new(),
            module_facts: Vec::new(),
            trace_codes: Vec::new(),
            decision: Some(decision),
        })
    }
}

#[test]
fn react_invocation_preserves_existing_priority_then_stable_id_resolution() {
    let registry = decision_registry();
    let host = ExistingReactionHost {
        observed_order: RefCell::new(Vec::new()),
    };
    let mut owner_port = DecisionOwner {
        revision: RefCell::new("revision-1".to_owned()),
        commits: Cell::new(0),
    };
    let mut continuations = GameplayDecisionContinuations::default();
    let receipt = GameplayFabricCoordinator::new(&registry, limits(4)).decide(
        decision_moment("revision-1"),
        &mut continuations,
        &decision_views(),
        &host,
        &mut owner_port,
    );

    assert!(receipt.accepted(), "{:#?}", receipt.diagnostics);
    assert_eq!(
        host.observed_order.into_inner(),
        vec!["3".to_owned(), "reaction.a".to_owned(), "1".to_owned()]
    );
}
