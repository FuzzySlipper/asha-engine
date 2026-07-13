use crate::types::{
    FrozenGameplayViews, GameplayEventEvidence, GameplayHostError, GameplayInvocationCall,
    GameplayInvocationEvidence, GameplayInvocationHost, GameplayInvocationOutput,
    GameplayObserveReceipt, GameplayOwnerRoutingCall, GameplayProposalRouter,
    GameplayRoutingEvidence, GameplayRoutingReceipt, GameplayRuntimeDiagnostic,
    GameplayRuntimeDiagnosticCode, GameplayRuntimeLimits, GameplayViewSource,
};
use protocol_game_extension::{
    GameplayCausationRef, GameplayEmitterRef, GameplayEventEnvelope, GameplayEventPhase,
    GameplayInvocationDescriptor, GameplayInvocationFamily, GameplayModuleManifest,
    GameplayProposalEnvelope, GameplaySubscriptionDeclaration,
};
use std::collections::BTreeMap;
use svc_gameplay_fabric::GameplayFabricRegistry;

/// Coordinates bounded post-commit Observe waves over one immutable Session
/// registry. Invocation implementations remain statically composed behind one
/// host port; this type is not a second handler registry.
pub struct GameplayFabricCoordinator<'registry> {
    pub(crate) registry: &'registry GameplayFabricRegistry,
    pub(crate) limits: GameplayRuntimeLimits,
}

impl<'registry> GameplayFabricCoordinator<'registry> {
    pub fn new(registry: &'registry GameplayFabricRegistry, limits: GameplayRuntimeLimits) -> Self {
        Self { registry, limits }
    }

    /// Routes one proposal at an explicit authority boundary and returns an
    /// opaque receipt proving that owner resolution came from this registry.
    pub fn route_proposal(
        &self,
        proposal: GameplayProposalEnvelope,
        router: &mut dyn GameplayProposalRouter,
    ) -> Result<GameplayRoutingReceipt, GameplayRuntimeDiagnostic> {
        let Some(owner) = self.registry.proposal_owner(&proposal.proposal).cloned() else {
            return Err(GameplayRuntimeDiagnostic {
                code: GameplayRuntimeDiagnosticCode::MissingProposalOwner,
                path: "proposal.proposal".to_owned(),
                message: format!(
                    "proposal `{}` has no owner in the closed registry",
                    proposal.proposal.key()
                ),
            });
        };
        let call = GameplayOwnerRoutingCall { owner, proposal };
        let mut output = router.route(&call);
        output.fact_hashes.sort();
        output.diagnostic_codes.sort();
        let proposal_hash = gameplay_proposal_hash(&call.proposal);
        Ok(GameplayRoutingReceipt {
            evidence: GameplayRoutingEvidence {
                proposal_id: call.proposal.proposal_id.clone(),
                proposal_kind: call.proposal.proposal.key(),
                proposal_hash: proposal_hash.clone(),
                owner_id: call.owner.owner_id.clone(),
                accepted: output.accepted,
                fact_hashes: output.fact_hashes.clone(),
                diagnostic_codes: output.diagnostic_codes.clone(),
                routing_hash: routing_hash(&proposal_hash, &call.owner.owner_id, &output),
            },
        })
    }

    pub fn observe(
        &self,
        mut root_event: GameplayEventEnvelope,
        views: &dyn GameplayViewSource,
        host: &dyn GameplayInvocationHost,
        router: &mut dyn GameplayProposalRouter,
    ) -> GameplayObserveReceipt {
        root_event.wave = 0;
        root_event.event_sequence = 0;
        canonicalize_headers(&mut root_event);

        let root_id = root_event.causation.root_id.clone();
        let mut state = ObserveState::new(self.registry, self.limits, root_id, root_event);
        if !state.registry.event_is_declared(&state.events[0].event) {
            state.diagnostic(
                GameplayRuntimeDiagnosticCode::UnknownEvent,
                "root.event",
                format!(
                    "root event `{}` is not declared",
                    state.events[0].event.key()
                ),
            );
            return state.finish();
        }
        if !state.charge_initial_payload() {
            return state.finish();
        }

        let mut wave_events = vec![state.events[0].clone()];
        while !wave_events.is_empty() {
            let wave = state.waves_processed;
            if wave >= state.limits.max_waves {
                state.diagnostic(
                    GameplayRuntimeDiagnosticCode::WaveBudgetExceeded,
                    format!("waves[{wave}]"),
                    format!("Observe cascade exceeded {} waves", state.limits.max_waves),
                );
                break;
            }

            wave_events.sort_by(|left, right| {
                (left.event_sequence, left.event_id.as_str())
                    .cmp(&(right.event_sequence, right.event_id.as_str()))
            });
            let frozen_views = views.freeze(&state.root_id, wave);
            state.wave_views.push(frozen_views.clone());

            let pending =
                self.invoke_wave(wave, &wave_events, views, &frozen_views, host, &mut state);
            state.waves_processed += 1;
            if !state.diagnostics.is_empty() {
                break;
            }

            wave_events = self.route_wave(wave, pending, router, &mut state);
            if !state.diagnostics.is_empty() {
                break;
            }
        }
        state.finish()
    }

    fn invoke_wave(
        &self,
        wave: u32,
        events: &[GameplayEventEnvelope],
        views: &dyn GameplayViewSource,
        frozen_views: &FrozenGameplayViews,
        host: &dyn GameplayInvocationHost,
        state: &mut ObserveState<'_>,
    ) -> Vec<PendingInvocationOutput> {
        let mut pending = Vec::new();
        for event in events {
            for module_id in self.registry.module_order() {
                let manifest = self
                    .registry
                    .module(module_id)
                    .expect("module order only contains registry modules");
                let mut subscriptions = manifest.subscriptions.iter().collect::<Vec<_>>();
                subscriptions
                    .sort_by(|left, right| left.subscription_id.cmp(&right.subscription_id));
                for subscription in subscriptions {
                    if subscription.event != event.event || !selector_matches(subscription, event) {
                        continue;
                    }
                    let Some(invocation) = manifest
                        .invocations
                        .iter()
                        .find(|candidate| candidate.invocation_id == subscription.invocation_id)
                    else {
                        state.diagnostic(
                            GameplayRuntimeDiagnosticCode::UndeclaredInvocation,
                            format!(
                                "modules.{module_id}.subscriptions.{}",
                                subscription.subscription_id
                            ),
                            format!(
                                "invocation `{}` is not declared",
                                subscription.invocation_id
                            ),
                        );
                        continue;
                    };
                    if invocation.family != GameplayInvocationFamily::Observe {
                        state.diagnostic(
                            GameplayRuntimeDiagnosticCode::UndeclaredInvocation,
                            format!(
                                "modules.{module_id}.invocations.{}",
                                invocation.invocation_id
                            ),
                            "an Observe delivery cannot invoke another invocation family",
                        );
                        continue;
                    }
                    if !state.charge_delivery(manifest, subscription, invocation, wave) {
                        continue;
                    }

                    let declared_reads = match views.freeze_declared_reads(
                        module_id,
                        &invocation.invocation_id,
                        event,
                    ) {
                        Ok(reads) => reads,
                        Err(error) => {
                            let details = error
                                .diagnostics
                                .iter()
                                .map(|diagnostic| {
                                    format!(
                                        "{:?}:{}:{}",
                                        diagnostic.code, diagnostic.request_id, diagnostic.message
                                    )
                                })
                                .collect::<Vec<_>>()
                                .join("; ");
                            state.diagnostic(
                                GameplayRuntimeDiagnosticCode::ReadAssemblyFailed,
                                format!(
                                    "modules.{module_id}.invocations.{}.reads",
                                    invocation.invocation_id
                                ),
                                details,
                            );
                            continue;
                        }
                    };

                    let mut call = GameplayInvocationCall {
                        module_id: module_id.clone(),
                        subscription_id: subscription.subscription_id.clone(),
                        invocation_id: invocation.invocation_id.clone(),
                        family: GameplayInvocationFamily::Observe,
                        input: crate::GameplayInvocationInput::Observe(event.clone()),
                        frozen_views: frozen_views.clone(),
                        declared_reads,
                        configuration: None,
                    };
                    call.configuration = match host.resolve_configuration(&call) {
                        Ok(configuration) => configuration,
                        Err(error) => {
                            state.host_failure(module_id, invocation, error);
                            continue;
                        }
                    };
                    let delivery_hash = delivery_hash(self.registry.registry_digest(), &call);
                    match host.invoke(&call) {
                        Ok(output) => {
                            let output_hash = semantic_output_hash(&output);
                            state.invocations.push(GameplayInvocationEvidence {
                                module_id: module_id.clone(),
                                subscription_id: subscription.subscription_id.clone(),
                                invocation_id: invocation.invocation_id.clone(),
                                event_id: event.event_id.clone(),
                                wave,
                                frozen_view_hash: frozen_views.view_hash.clone(),
                                declared_read_set_hash: call
                                    .declared_reads
                                    .as_ref()
                                    .map(|reads| reads.read_set_hash.clone()),
                                declared_reads: call.declared_reads.clone(),
                                configuration: call.configuration.clone(),
                                delivery_hash,
                                output_hash,
                            });
                            if state.validate_output(manifest, invocation, &output) {
                                pending.push(PendingInvocationOutput {
                                    module_id: module_id.clone(),
                                    parent_event_id: event.event_id.clone(),
                                    output,
                                });
                            }
                        }
                        Err(error) => state.host_failure(module_id, invocation, error),
                    }
                }
            }
        }
        pending
    }

    fn route_wave(
        &self,
        wave: u32,
        pending: Vec<PendingInvocationOutput>,
        router: &mut dyn GameplayProposalRouter,
        state: &mut ObserveState<'_>,
    ) -> Vec<GameplayEventEnvelope> {
        let next_wave = wave + 1;
        let mut next_events = Vec::new();
        let mut proposal_queue = Vec::new();

        for pending_output in pending {
            state
                .module_facts
                .extend(pending_output.output.module_facts);
            for event in pending_output.output.events {
                let normalized = state.normalize_module_event(
                    event,
                    &pending_output.module_id,
                    &pending_output.parent_event_id,
                    next_wave,
                );
                next_events.push(normalized);
            }
            for proposal in pending_output.output.proposals {
                let normalized = state.normalize_proposal(
                    proposal,
                    &pending_output.module_id,
                    &pending_output.parent_event_id,
                    wave,
                );
                proposal_queue.push(normalized);
            }
        }

        for proposal in proposal_queue {
            let Some(owner) = self.registry.proposal_owner(&proposal.proposal).cloned() else {
                state.diagnostic(
                    GameplayRuntimeDiagnosticCode::MissingProposalOwner,
                    format!("proposals.{}", proposal.proposal_id),
                    format!(
                        "proposal `{}` has no registered owner",
                        proposal.proposal.key()
                    ),
                );
                continue;
            };
            let call = GameplayOwnerRoutingCall {
                owner: owner.clone(),
                proposal: proposal.clone(),
            };
            let mut output = router.route(&call);
            output.fact_hashes.sort();
            output.diagnostic_codes.sort();
            let proposal_hash = gameplay_proposal_hash(&proposal);
            let routing_hash = routing_hash(&proposal_hash, &owner.owner_id, &output);
            state.routing.push(GameplayRoutingEvidence {
                proposal_id: proposal.proposal_id.clone(),
                proposal_kind: proposal.proposal.key(),
                proposal_hash,
                owner_id: owner.owner_id.clone(),
                accepted: output.accepted,
                fact_hashes: output.fact_hashes.clone(),
                diagnostic_codes: output.diagnostic_codes.clone(),
                routing_hash,
            });
            if !output.accepted {
                continue;
            }
            for event in output.events {
                if !self.registry.event_is_declared(&event.event) {
                    state.diagnostic(
                        GameplayRuntimeDiagnosticCode::UndeclaredEvent,
                        format!("proposals.{}.events", proposal.proposal_id),
                        format!("owner emitted undeclared event `{}`", event.event.key()),
                    );
                    continue;
                }
                let normalized = state.normalize_owner_event(
                    event,
                    &owner.owner_id,
                    proposal.originating_event_id.as_deref(),
                    next_wave,
                );
                if state.charge_owner_event(&normalized) {
                    next_events.push(normalized);
                }
            }
        }
        next_events
    }
}

struct PendingInvocationOutput {
    module_id: String,
    parent_event_id: String,
    output: GameplayInvocationOutput,
}

#[derive(Default)]
struct ModuleUsage {
    invocations: u32,
    events: u32,
    proposals: u32,
    payload_bytes: u64,
}

struct ObserveState<'registry> {
    registry: &'registry GameplayFabricRegistry,
    limits: GameplayRuntimeLimits,
    root_id: String,
    tick: u64,
    root_sequence: u64,
    waves_processed: u32,
    total_events: u32,
    total_proposals: u32,
    total_invocations: u32,
    total_payload_bytes: u64,
    next_event_sequence: BTreeMap<u32, u32>,
    next_proposal_sequence: u32,
    subscription_deliveries: BTreeMap<String, u32>,
    module_usage: BTreeMap<String, ModuleUsage>,
    wave_views: Vec<FrozenGameplayViews>,
    events: Vec<GameplayEventEnvelope>,
    event_evidence: Vec<GameplayEventEvidence>,
    invocations: Vec<GameplayInvocationEvidence>,
    routing: Vec<GameplayRoutingEvidence>,
    module_facts: Vec<crate::GameplayModuleFact>,
    diagnostics: Vec<GameplayRuntimeDiagnostic>,
}

impl<'registry> ObserveState<'registry> {
    fn new(
        registry: &'registry GameplayFabricRegistry,
        limits: GameplayRuntimeLimits,
        root_id: String,
        root_event: GameplayEventEnvelope,
    ) -> Self {
        let tick = root_event.tick;
        let root_sequence = root_event.root_sequence;
        let root_evidence = GameplayEventEvidence {
            event_id: root_event.event_id.clone(),
            event_hash: event_hash(&root_event),
        };
        Self {
            registry,
            limits,
            root_id,
            tick,
            root_sequence,
            waves_processed: 0,
            total_events: 1,
            total_proposals: 0,
            total_invocations: 0,
            total_payload_bytes: 0,
            next_event_sequence: BTreeMap::new(),
            next_proposal_sequence: 0,
            subscription_deliveries: BTreeMap::new(),
            module_usage: BTreeMap::new(),
            wave_views: Vec::new(),
            events: vec![root_event],
            event_evidence: vec![root_evidence],
            invocations: Vec::new(),
            routing: Vec::new(),
            module_facts: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    fn charge_initial_payload(&mut self) -> bool {
        if self.total_events > self.limits.max_events_per_root {
            self.diagnostic(
                GameplayRuntimeDiagnosticCode::EventBudgetExceeded,
                "root",
                "root event exceeds the Session event budget",
            );
            return false;
        }
        self.total_payload_bytes = self.events[0].canonical_payload.len() as u64;
        if self.total_payload_bytes > u64::from(self.limits.max_payload_bytes_per_root) {
            self.diagnostic(
                GameplayRuntimeDiagnosticCode::PayloadBudgetExceeded,
                "root.canonicalPayload",
                "root payload exceeds the Session payload budget",
            );
            return false;
        }
        true
    }

    fn charge_delivery(
        &mut self,
        manifest: &GameplayModuleManifest,
        subscription: &GameplaySubscriptionDeclaration,
        invocation: &GameplayInvocationDescriptor,
        wave: u32,
    ) -> bool {
        let module_id = manifest.module_ref.module_id.as_str();
        let deliveries = self
            .subscription_deliveries
            .entry(subscription.subscription_id.clone())
            .or_default();
        *deliveries += 1;
        if *deliveries > subscription.max_deliveries_per_root {
            self.diagnostic(
                GameplayRuntimeDiagnosticCode::SubscriptionDeliveryBudgetExceeded,
                format!(
                    "modules.{module_id}.subscriptions.{}",
                    subscription.subscription_id
                ),
                "subscription delivery budget exceeded",
            );
            return false;
        }
        if wave >= manifest.budget.max_waves {
            self.diagnostic(
                GameplayRuntimeDiagnosticCode::WaveBudgetExceeded,
                format!(
                    "modules.{module_id}.invocations.{}",
                    invocation.invocation_id
                ),
                "module wave budget exceeded",
            );
            return false;
        }
        self.total_invocations += 1;
        let usage = self.module_usage.entry(module_id.to_owned()).or_default();
        usage.invocations += 1;
        if self.total_invocations > self.limits.max_invocations_per_root
            || usage.invocations > manifest.budget.max_invocations_per_root
        {
            self.diagnostic(
                GameplayRuntimeDiagnosticCode::InvocationBudgetExceeded,
                format!(
                    "modules.{module_id}.invocations.{}",
                    invocation.invocation_id
                ),
                "invocation budget exceeded",
            );
            return false;
        }
        true
    }

    fn validate_output(
        &mut self,
        manifest: &GameplayModuleManifest,
        invocation: &GameplayInvocationDescriptor,
        output: &GameplayInvocationOutput,
    ) -> bool {
        let module_id = manifest.module_ref.module_id.as_str();
        if output.decision.is_some() {
            self.diagnostic(
                GameplayRuntimeDiagnosticCode::UnexpectedDecisionOutput,
                format!(
                    "modules.{module_id}.invocations.{}",
                    invocation.invocation_id
                ),
                "an Observe invocation cannot return a pre-commit decision",
            );
        }
        let output_count = output
            .events
            .len()
            .saturating_add(output.proposals.len())
            .saturating_add(output.module_facts.len());
        if output_count > invocation.max_outputs as usize {
            self.diagnostic(
                GameplayRuntimeDiagnosticCode::InvocationOutputBudgetExceeded,
                format!(
                    "modules.{module_id}.invocations.{}",
                    invocation.invocation_id
                ),
                "invocation output count exceeds its declared budget",
            );
        }
        for event in &output.events {
            if !self
                .registry
                .module_publishes_event(module_id, &event.event)
            {
                self.diagnostic(
                    GameplayRuntimeDiagnosticCode::UndeclaredEvent,
                    format!(
                        "modules.{module_id}.invocations.{}.events",
                        invocation.invocation_id
                    ),
                    format!("module emitted undeclared event `{}`", event.event.key()),
                );
            }
        }
        for proposal in &output.proposals {
            if !self
                .registry
                .module_declares_proposal(module_id, &proposal.proposal)
            {
                self.diagnostic(
                    GameplayRuntimeDiagnosticCode::UndeclaredProposal,
                    format!(
                        "modules.{module_id}.invocations.{}.proposals",
                        invocation.invocation_id
                    ),
                    format!(
                        "module emitted undeclared proposal `{}`",
                        proposal.proposal.key()
                    ),
                );
            }
        }
        for fact in &output.module_facts {
            if fact.module_id != module_id
                || !self
                    .registry
                    .module_declares_fact(module_id, &fact.fact_schema)
                || !self
                    .registry
                    .module_declares_state(module_id, &fact.state_schema)
                || crate::gameplay_module_payload_hash(&fact.canonical_payload) != fact.payload_hash
            {
                self.diagnostic(
                    GameplayRuntimeDiagnosticCode::UndeclaredModuleFact,
                    format!(
                        "modules.{module_id}.invocations.{}.moduleFacts",
                        invocation.invocation_id
                    ),
                    format!("module emitted invalid fact `{}`", fact.fact_id),
                );
            }
        }

        let payload_bytes = output
            .events
            .iter()
            .map(|event| event.canonical_payload.len() as u64)
            .chain(
                output
                    .proposals
                    .iter()
                    .map(|proposal| proposal.canonical_payload.len() as u64),
            )
            .chain(
                output
                    .module_facts
                    .iter()
                    .map(|fact| fact.canonical_payload.len() as u64),
            )
            .sum::<u64>();
        if payload_bytes > u64::from(invocation.max_payload_bytes) {
            self.diagnostic(
                GameplayRuntimeDiagnosticCode::InvocationOutputBudgetExceeded,
                format!(
                    "modules.{module_id}.invocations.{}",
                    invocation.invocation_id
                ),
                "invocation output payload exceeds its declared budget",
            );
        }

        let (module_events_exceeded, module_proposals_exceeded, module_payload_exceeded) = {
            let usage = self.module_usage.entry(module_id.to_owned()).or_default();
            usage.events = usage.events.saturating_add(output.events.len() as u32);
            usage.proposals = usage
                .proposals
                .saturating_add(output.proposals.len() as u32);
            usage.payload_bytes = usage.payload_bytes.saturating_add(payload_bytes);
            (
                usage.events > manifest.budget.max_events_per_root,
                usage.proposals > manifest.budget.max_proposals_per_root,
                usage.payload_bytes > u64::from(manifest.budget.max_payload_bytes_per_root),
            )
        };
        self.total_events = self.total_events.saturating_add(output.events.len() as u32);
        self.total_proposals = self
            .total_proposals
            .saturating_add(output.proposals.len() as u32);
        self.total_payload_bytes = self.total_payload_bytes.saturating_add(payload_bytes);

        if module_events_exceeded || self.total_events > self.limits.max_events_per_root {
            self.diagnostic(
                GameplayRuntimeDiagnosticCode::EventBudgetExceeded,
                format!("modules.{module_id}.budget.maxEventsPerRoot"),
                "event budget exceeded",
            );
        }
        if module_proposals_exceeded || self.total_proposals > self.limits.max_proposals_per_root {
            self.diagnostic(
                GameplayRuntimeDiagnosticCode::ProposalBudgetExceeded,
                format!("modules.{module_id}.budget.maxProposalsPerRoot"),
                "proposal budget exceeded",
            );
        }
        if module_payload_exceeded
            || self.total_payload_bytes > u64::from(self.limits.max_payload_bytes_per_root)
        {
            self.diagnostic(
                GameplayRuntimeDiagnosticCode::PayloadBudgetExceeded,
                format!("modules.{module_id}.budget.maxPayloadBytesPerRoot"),
                "payload budget exceeded",
            );
        }
        self.diagnostics.is_empty()
    }

    fn normalize_module_event(
        &mut self,
        mut event: GameplayEventEnvelope,
        module_id: &str,
        parent_event_id: &str,
        wave: u32,
    ) -> GameplayEventEnvelope {
        let sequence = self.next_event_sequence(wave);
        event.event_id = format!("{}/event/{wave}/{sequence}", self.root_id);
        event.tick = self.tick;
        event.root_sequence = self.root_sequence;
        event.wave = wave;
        event.event_sequence = sequence;
        event.phase = GameplayEventPhase::PostCommit;
        event.emitter = GameplayEmitterRef::Module {
            module_id: module_id.to_owned(),
        };
        event.causation = GameplayCausationRef {
            root_id: self.root_id.clone(),
            parent_event_id: Some(parent_event_id.to_owned()),
            decision_id: None,
        };
        canonicalize_headers(&mut event);
        self.record_event(&event);
        event
    }

    fn normalize_owner_event(
        &mut self,
        mut event: GameplayEventEnvelope,
        owner_id: &str,
        parent_event_id: Option<&str>,
        wave: u32,
    ) -> GameplayEventEnvelope {
        let sequence = self.next_event_sequence(wave);
        event.event_id = format!("{}/event/{wave}/{sequence}", self.root_id);
        event.tick = self.tick;
        event.root_sequence = self.root_sequence;
        event.wave = wave;
        event.event_sequence = sequence;
        event.phase = GameplayEventPhase::PostCommit;
        event.emitter = GameplayEmitterRef::Owner {
            owner_id: owner_id.to_owned(),
        };
        event.causation = GameplayCausationRef {
            root_id: self.root_id.clone(),
            parent_event_id: parent_event_id.map(str::to_owned),
            decision_id: None,
        };
        canonicalize_headers(&mut event);
        event
    }

    fn normalize_proposal(
        &mut self,
        mut proposal: GameplayProposalEnvelope,
        module_id: &str,
        parent_event_id: &str,
        wave: u32,
    ) -> GameplayProposalEnvelope {
        let sequence = self.next_proposal_sequence;
        self.next_proposal_sequence += 1;
        proposal.proposal_id = format!("{}/proposal/{sequence}", self.root_id);
        proposal.tick = self.tick;
        proposal.root_sequence = self.root_sequence;
        proposal.wave = wave;
        proposal.proposal_sequence = sequence;
        proposal.emitter = GameplayEmitterRef::Module {
            module_id: module_id.to_owned(),
        };
        proposal.causation = GameplayCausationRef {
            root_id: self.root_id.clone(),
            parent_event_id: Some(parent_event_id.to_owned()),
            decision_id: None,
        };
        proposal.originating_event_id = Some(parent_event_id.to_owned());
        proposal.targets.sort_by_key(|target| target.entity.raw());
        proposal
    }

    fn charge_owner_event(&mut self, event: &GameplayEventEnvelope) -> bool {
        self.total_events += 1;
        self.total_payload_bytes = self
            .total_payload_bytes
            .saturating_add(event.canonical_payload.len() as u64);
        if self.total_events > self.limits.max_events_per_root {
            self.diagnostic(
                GameplayRuntimeDiagnosticCode::EventBudgetExceeded,
                format!("events.{}", event.event_id),
                "owner event exceeded the Session event budget",
            );
            return false;
        }
        if self.total_payload_bytes > u64::from(self.limits.max_payload_bytes_per_root) {
            self.diagnostic(
                GameplayRuntimeDiagnosticCode::PayloadBudgetExceeded,
                format!("events.{}.canonicalPayload", event.event_id),
                "owner event exceeded the Session payload budget",
            );
            return false;
        }
        self.record_event(event);
        true
    }

    fn record_event(&mut self, event: &GameplayEventEnvelope) {
        self.event_evidence.push(GameplayEventEvidence {
            event_id: event.event_id.clone(),
            event_hash: event_hash(event),
        });
        self.events.push(event.clone());
    }

    fn next_event_sequence(&mut self, wave: u32) -> u32 {
        let sequence = self.next_event_sequence.entry(wave).or_default();
        let value = *sequence;
        *sequence += 1;
        value
    }

    fn host_failure(
        &mut self,
        module_id: &str,
        invocation: &GameplayInvocationDescriptor,
        error: GameplayHostError,
    ) {
        self.diagnostic(
            GameplayRuntimeDiagnosticCode::HostFailure,
            format!(
                "modules.{module_id}.invocations.{}",
                invocation.invocation_id
            ),
            format!("{}: {}", error.code, error.message),
        );
    }

    fn diagnostic(
        &mut self,
        code: GameplayRuntimeDiagnosticCode,
        path: impl Into<String>,
        message: impl Into<String>,
    ) {
        self.diagnostics.push(GameplayRuntimeDiagnostic {
            code,
            path: path.into(),
            message: message.into(),
        });
    }

    fn finish(mut self) -> GameplayObserveReceipt {
        self.diagnostics.sort_by(|left, right| {
            (
                diagnostic_code(left.code),
                left.path.as_str(),
                left.message.as_str(),
            )
                .cmp(&(
                    diagnostic_code(right.code),
                    right.path.as_str(),
                    right.message.as_str(),
                ))
        });
        let receipt_hash = receipt_hash(&self);
        GameplayObserveReceipt {
            registry_digest: self.registry.registry_digest().to_owned(),
            root_id: self.root_id,
            waves_processed: self.waves_processed,
            wave_views: self.wave_views,
            events: self.events,
            event_evidence: self.event_evidence,
            invocations: self.invocations,
            routing: self.routing,
            module_facts: self.module_facts,
            diagnostics: self.diagnostics,
            receipt_hash,
        }
    }
}

fn selector_matches(
    subscription: &GameplaySubscriptionDeclaration,
    event: &GameplayEventEnvelope,
) -> bool {
    let selector = &subscription.selector;
    selector
        .source
        .as_ref()
        .is_none_or(|source| event.source.as_ref() == Some(source))
        && selector
            .target
            .as_ref()
            .is_none_or(|target| event.targets.contains(target))
        && selector
            .scope
            .as_ref()
            .is_none_or(|scope| event.scope.as_ref() == Some(scope))
        && selector
            .required_tags
            .iter()
            .all(|tag| event.tags.contains(tag))
}

fn canonicalize_headers(event: &mut GameplayEventEnvelope) {
    event.subjects.sort_by_key(|subject| subject.entity.raw());
    event.targets.sort_by_key(|target| target.entity.raw());
    event.tags.sort();
    event.tags.dedup();
}

pub(crate) fn delivery_hash(registry_digest: &str, call: &GameplayInvocationCall) -> String {
    let input_hash = match &call.input {
        crate::GameplayInvocationInput::Observe(event) => event_hash(event),
        crate::GameplayInvocationInput::Decision(moment) => stable_hash([
            moment.decision_id.as_str(),
            gameplay_proposal_hash(&moment.operation).as_str(),
            moment.workspace.contract.key().as_str(),
            moment.workspace.workspace_hash.as_str(),
            moment.expected_owner_revision.as_str(),
            moment.resume_token.as_deref().unwrap_or("-"),
        ]),
    };
    stable_hash([
        registry_digest,
        call.module_id.as_str(),
        call.subscription_id.as_str(),
        call.invocation_id.as_str(),
        call.family.as_str(),
        input_hash.as_str(),
        &call.frozen_views.epoch.to_string(),
        call.frozen_views.view_hash.as_str(),
        call.declared_reads
            .as_ref()
            .map(|reads| reads.read_set_hash.as_str())
            .unwrap_or("-"),
        call.configuration
            .as_ref()
            .map(|configuration| {
                crate::gameplay_module_payload_hash(
                    &serde_json::to_vec(configuration)
                        .expect("invocation configuration evidence serializes"),
                )
            })
            .as_deref()
            .unwrap_or("-"),
    ])
}

pub(crate) fn semantic_output_hash(output: &GameplayInvocationOutput) -> String {
    let mut parts = vec![format!("events:{}", output.events.len())];
    parts.extend(output.events.iter().map(semantic_event_hash));
    parts.push(format!("proposals:{}", output.proposals.len()));
    parts.extend(output.proposals.iter().map(semantic_proposal_hash));
    parts.push(format!("moduleFacts:{}", output.module_facts.len()));
    parts.extend(output.module_facts.iter().map(|fact| {
        stable_hash([
            fact.fact_id.as_str(),
            fact.module_id.as_str(),
            fact.fact_schema.key().as_str(),
            fact.state_schema.key().as_str(),
            fact.payload_hash.as_str(),
            &fact.expected_revision.to_string(),
        ])
    }));
    parts.push(format!("traces:{}", output.trace_codes.len()));
    parts.extend(output.trace_codes.iter().cloned());
    parts.push(format!("decision:{:?}", output.decision));
    stable_hash(parts.iter().map(String::as_str))
}

fn semantic_event_hash(event: &GameplayEventEnvelope) -> String {
    let mut hasher = StableHasher::new();
    hasher.field(&event.event.key());
    hasher.field(&event.event.schema_hash);
    feed_event_headers(&mut hasher, event);
    hasher.bytes(&event.canonical_payload);
    hasher.field(&event.payload_hash);
    hasher.finish()
}

fn semantic_proposal_hash(proposal: &GameplayProposalEnvelope) -> String {
    let mut hasher = StableHasher::new();
    hasher.field(&proposal.proposal.key());
    hasher.field(&proposal.proposal.schema_hash);
    feed_optional_entity(&mut hasher, proposal.source.as_ref());
    feed_entities(&mut hasher, &proposal.targets);
    hasher.bytes(&proposal.canonical_payload);
    hasher.field(&proposal.payload_hash);
    hasher.finish()
}

pub(crate) fn event_hash(event: &GameplayEventEnvelope) -> String {
    let mut hasher = StableHasher::new();
    hasher.field(&event.event_id);
    hasher.field(&event.event.key());
    hasher.field(&event.event.schema_hash);
    hasher.number(event.tick);
    hasher.number(event.root_sequence);
    hasher.number(u64::from(event.wave));
    hasher.number(u64::from(event.event_sequence));
    hasher.field(event.phase.as_str());
    feed_emitter(&mut hasher, &event.emitter);
    feed_causation(&mut hasher, &event.causation);
    feed_event_headers(&mut hasher, event);
    hasher.bytes(&event.canonical_payload);
    hasher.field(&event.payload_hash);
    hasher.finish()
}

pub fn gameplay_proposal_hash(proposal: &GameplayProposalEnvelope) -> String {
    let mut hasher = StableHasher::new();
    hasher.field(&proposal.proposal_id);
    hasher.field(&proposal.proposal.key());
    hasher.field(&proposal.proposal.schema_hash);
    hasher.number(proposal.tick);
    hasher.number(proposal.root_sequence);
    hasher.number(u64::from(proposal.wave));
    hasher.number(u64::from(proposal.proposal_sequence));
    feed_emitter(&mut hasher, &proposal.emitter);
    feed_causation(&mut hasher, &proposal.causation);
    hasher.optional_field(proposal.originating_event_id.as_deref());
    feed_optional_entity(&mut hasher, proposal.source.as_ref());
    feed_entities(&mut hasher, &proposal.targets);
    hasher.bytes(&proposal.canonical_payload);
    hasher.field(&proposal.payload_hash);
    hasher.finish()
}

pub(crate) fn routing_hash(
    proposal_hash: &str,
    owner_id: &str,
    output: &crate::GameplayOwnerRoutingOutput,
) -> String {
    let mut parts = vec![
        proposal_hash.to_owned(),
        owner_id.to_owned(),
        output.accepted.to_string(),
    ];
    parts.extend(output.fact_hashes.iter().cloned());
    parts.extend(output.events.iter().map(semantic_event_hash));
    parts.extend(output.diagnostic_codes.iter().cloned());
    stable_hash(parts.iter().map(String::as_str))
}

fn receipt_hash(state: &ObserveState<'_>) -> String {
    let mut parts = vec![
        state.registry.registry_digest().to_owned(),
        state.root_id.clone(),
        state.waves_processed.to_string(),
    ];
    for view in &state.wave_views {
        parts.push(view.epoch.to_string());
        parts.push(view.view_hash.clone());
    }
    parts.extend(
        state
            .event_evidence
            .iter()
            .flat_map(|event| [event.event_id.clone(), event.event_hash.clone()]),
    );
    for invocation in &state.invocations {
        parts.extend([
            invocation.module_id.clone(),
            invocation.subscription_id.clone(),
            invocation.invocation_id.clone(),
            invocation.event_id.clone(),
            invocation.wave.to_string(),
            invocation.frozen_view_hash.clone(),
            invocation.delivery_hash.clone(),
            invocation.output_hash.clone(),
        ]);
    }
    for routing in &state.routing {
        parts.extend([
            routing.proposal_id.clone(),
            routing.proposal_kind.clone(),
            routing.proposal_hash.clone(),
            routing.owner_id.clone(),
            routing.accepted.to_string(),
            routing.routing_hash.clone(),
        ]);
        parts.extend(routing.fact_hashes.iter().cloned());
    }
    for fact in &state.module_facts {
        parts.extend([
            fact.fact_id.clone(),
            fact.module_id.clone(),
            fact.fact_schema.key(),
            fact.state_schema.key(),
            fact.expected_revision.to_string(),
            fact.payload_hash.clone(),
        ]);
    }
    for diagnostic in &state.diagnostics {
        parts.extend([
            diagnostic_code(diagnostic.code).to_owned(),
            diagnostic.path.clone(),
            diagnostic.message.clone(),
        ]);
    }
    stable_hash(parts.iter().map(String::as_str))
}

fn feed_event_headers(hasher: &mut StableHasher, event: &GameplayEventEnvelope) {
    feed_optional_entity(hasher, event.source.as_ref());
    feed_entities(hasher, &event.subjects);
    feed_entities(hasher, &event.targets);
    hasher.optional_field(event.scope.as_deref());
    for tag in &event.tags {
        hasher.field(tag);
    }
}

fn feed_entities(
    hasher: &mut StableHasher,
    entities: &[protocol_game_extension::GameplayEntityRef],
) {
    hasher.number(entities.len() as u64);
    for entity in entities {
        hasher.number(entity.entity.raw());
    }
}

fn feed_optional_entity(
    hasher: &mut StableHasher,
    entity: Option<&protocol_game_extension::GameplayEntityRef>,
) {
    match entity {
        Some(entity) => {
            hasher.field("some");
            hasher.number(entity.entity.raw());
        }
        None => hasher.field("none"),
    }
}

fn feed_emitter(hasher: &mut StableHasher, emitter: &GameplayEmitterRef) {
    match emitter {
        GameplayEmitterRef::Owner { owner_id } => {
            hasher.field("owner");
            hasher.field(owner_id);
        }
        GameplayEmitterRef::Module { module_id } => {
            hasher.field("module");
            hasher.field(module_id);
        }
        GameplayEmitterRef::Scheduler { scheduler_id } => {
            hasher.field("scheduler");
            hasher.field(scheduler_id);
        }
    }
}

fn feed_causation(hasher: &mut StableHasher, causation: &GameplayCausationRef) {
    hasher.field(&causation.root_id);
    hasher.optional_field(causation.parent_event_id.as_deref());
    hasher.optional_field(causation.decision_id.as_deref());
}

pub(crate) fn diagnostic_code(code: GameplayRuntimeDiagnosticCode) -> &'static str {
    match code {
        GameplayRuntimeDiagnosticCode::UnknownEvent => "unknownEvent",
        GameplayRuntimeDiagnosticCode::UndeclaredInvocation => "undeclaredInvocation",
        GameplayRuntimeDiagnosticCode::UndeclaredEvent => "undeclaredEvent",
        GameplayRuntimeDiagnosticCode::UndeclaredProposal => "undeclaredProposal",
        GameplayRuntimeDiagnosticCode::UndeclaredModuleFact => "undeclaredModuleFact",
        GameplayRuntimeDiagnosticCode::MissingProposalOwner => "missingProposalOwner",
        GameplayRuntimeDiagnosticCode::ReadAssemblyFailed => "readAssemblyFailed",
        GameplayRuntimeDiagnosticCode::HostFailure => "hostFailure",
        GameplayRuntimeDiagnosticCode::WaveBudgetExceeded => "waveBudgetExceeded",
        GameplayRuntimeDiagnosticCode::EventBudgetExceeded => "eventBudgetExceeded",
        GameplayRuntimeDiagnosticCode::ProposalBudgetExceeded => "proposalBudgetExceeded",
        GameplayRuntimeDiagnosticCode::InvocationBudgetExceeded => "invocationBudgetExceeded",
        GameplayRuntimeDiagnosticCode::PayloadBudgetExceeded => "payloadBudgetExceeded",
        GameplayRuntimeDiagnosticCode::InvocationOutputBudgetExceeded => {
            "invocationOutputBudgetExceeded"
        }
        GameplayRuntimeDiagnosticCode::SubscriptionDeliveryBudgetExceeded => {
            "subscriptionDeliveryBudgetExceeded"
        }
        GameplayRuntimeDiagnosticCode::UnexpectedDecisionOutput => "unexpectedDecisionOutput",
        GameplayRuntimeDiagnosticCode::MissingDecisionOutput => "missingDecisionOutput",
        GameplayRuntimeDiagnosticCode::GuardRejected => "guardRejected",
        GameplayRuntimeDiagnosticCode::WorkspaceContractMismatch => "workspaceContractMismatch",
        GameplayRuntimeDiagnosticCode::WorkspaceHashMismatch => "workspaceHashMismatch",
        GameplayRuntimeDiagnosticCode::ContinuationRequired => "continuationRequired",
        GameplayRuntimeDiagnosticCode::ContinuationMismatch => "continuationMismatch",
        GameplayRuntimeDiagnosticCode::ContinuationUnavailable => "continuationUnavailable",
        GameplayRuntimeDiagnosticCode::StaleDecision => "staleDecision",
        GameplayRuntimeDiagnosticCode::ReactionCancelled => "reactionCancelled",
        GameplayRuntimeDiagnosticCode::ReactionSuspended => "reactionSuspended",
        GameplayRuntimeDiagnosticCode::OwnerRejected => "ownerRejected",
    }
}

pub(crate) fn stable_hash<'a>(parts: impl IntoIterator<Item = &'a str>) -> String {
    let mut hasher = StableHasher::new();
    for part in parts {
        hasher.field(part);
    }
    hasher.finish()
}

struct StableHasher(u64);

impl StableHasher {
    fn new() -> Self {
        Self(0xcbf2_9ce4_8422_2325)
    }

    fn field(&mut self, value: &str) {
        self.bytes(value.as_bytes());
    }

    fn optional_field(&mut self, value: Option<&str>) {
        match value {
            Some(value) => {
                self.field("some");
                self.field(value);
            }
            None => self.field("none"),
        }
    }

    fn number(&mut self, value: u64) {
        self.bytes(&value.to_le_bytes());
    }

    fn bytes(&mut self, value: &[u8]) {
        for byte in (value.len() as u64)
            .to_le_bytes()
            .into_iter()
            .chain(value.iter().copied())
        {
            self.0 ^= u64::from(byte);
            self.0 = self.0.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }

    fn finish(self) -> String {
        format!("fnv1a64:{:016x}", self.0)
    }
}
