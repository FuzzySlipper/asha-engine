use crate::observe::{
    delivery_hash, diagnostic_code, gameplay_proposal_hash, routing_hash, semantic_output_hash,
    stable_hash,
};
use crate::{
    FrozenGameplayViews, GameplayDecisionContinuation, GameplayDecisionContinuations,
    GameplayDecisionMoment, GameplayDecisionOutput, GameplayDecisionOwner, GameplayDecisionReceipt,
    GameplayDecisionStatus, GameplayFabricCoordinator, GameplayGuardVote, GameplayInvocationCall,
    GameplayInvocationEvidence, GameplayInvocationHost, GameplayInvocationInput,
    GameplayOperationWorkspace, GameplayOwnerRoutingCall, GameplayRoutingEvidence,
    GameplayRuntimeDiagnostic, GameplayRuntimeDiagnosticCode, GameplayViewSource,
    GameplayWorkspaceTransform,
};
use protocol_game_extension::GameplayInvocationFamily;

impl GameplayOperationWorkspace {
    pub fn from_payload(
        contract: protocol_game_extension::GameplayContractRef,
        canonical_payload: Vec<u8>,
    ) -> Self {
        let workspace_hash = gameplay_payload_hash(&canonical_payload);
        Self {
            contract,
            canonical_payload,
            workspace_hash,
        }
    }
}

/// Hash convention for operation workspaces owned by this coordinator.
pub fn gameplay_payload_hash(payload: &[u8]) -> String {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in (payload.len() as u64)
        .to_le_bytes()
        .into_iter()
        .chain(payload.iter().copied())
    {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("fnv1a64:{hash:016x}")
}

impl GameplayDecisionReceipt {
    pub fn accepted(&self) -> bool {
        self.status == GameplayDecisionStatus::Accepted
    }
}

impl GameplayFabricCoordinator<'_> {
    /// Runs Guard, Transform, and React in their fixed transaction stages,
    /// then routes exactly one final operation to its registered owner.
    pub fn decide(
        &self,
        mut moment: GameplayDecisionMoment,
        continuations: &mut GameplayDecisionContinuations,
        views: &dyn GameplayViewSource,
        host: &dyn GameplayInvocationHost,
        owner_port: &mut dyn GameplayDecisionOwner,
    ) -> GameplayDecisionReceipt {
        let Some(owner) = self
            .registry
            .proposal_owner(&moment.operation.proposal)
            .cloned()
        else {
            return DecisionState::failed_without_owner(self, moment);
        };
        let mut state = DecisionState::new(self, &moment, owner.owner_id.clone());
        if !state.validate_workspace(&moment.workspace, "workspace") {
            return state.finish();
        }
        if let Err(error) =
            continuations.authorize(&moment, self.registry.registry_digest(), &owner.owner_id)
        {
            state.diagnostic(error.code, "resumeToken", error.message);
            return state.finish();
        }
        if owner_port.revision_hash(&owner) != moment.expected_owner_revision {
            state.stale("owner revision changed before decision invocation");
            return state.finish();
        }

        let stages = [
            GameplayInvocationFamily::Guard,
            GameplayInvocationFamily::Transform,
            GameplayInvocationFamily::React,
        ];
        for (stage_index, family) in stages.into_iter().enumerate() {
            let frozen_views = views.freeze(&moment.decision_id, stage_index as u32);
            let continue_stage = self.invoke_decision_stage(
                family,
                &mut moment,
                &frozen_views,
                views,
                host,
                &mut state,
            );
            if !continue_stage || !state.diagnostics.is_empty() {
                if state.status == GameplayDecisionStatus::Suspended {
                    let seed = state
                        .suspension_seed
                        .take()
                        .expect("Suspended status carries a continuation seed");
                    let continuation = continuations.issue(
                        &moment,
                        self.registry.registry_digest(),
                        &owner.owner_id,
                        &seed,
                    );
                    state.suspension_token = Some(continuation.token.clone());
                    state.continuation = Some(continuation);
                }
                return state.finish();
            }
        }

        if owner_port.revision_hash(&owner) != moment.expected_owner_revision {
            state.stale("owner revision changed before atomic commit");
            return state.finish();
        }

        moment.operation.canonical_payload = moment.workspace.canonical_payload.clone();
        moment.operation.payload_hash = moment.workspace.workspace_hash.clone();
        let call = GameplayOwnerRoutingCall {
            owner: owner.clone(),
            proposal: moment.operation.clone(),
        };
        let mut output = owner_port.route_precommit(&call);
        output.fact_hashes.sort();
        output.diagnostic_codes.sort();
        let proposal_hash = gameplay_proposal_hash(&call.proposal);
        state.routing = Some(GameplayRoutingEvidence {
            proposal_id: call.proposal.proposal_id.clone(),
            proposal_kind: call.proposal.proposal.key(),
            proposal_hash: proposal_hash.clone(),
            owner_id: owner.owner_id,
            accepted: output.accepted,
            fact_hashes: output.fact_hashes.clone(),
            diagnostic_codes: output.diagnostic_codes.clone(),
            routing_hash: routing_hash(&proposal_hash, &call.owner.owner_id, &output),
        });
        if output.accepted {
            state.status = GameplayDecisionStatus::Accepted;
        } else {
            state.status = GameplayDecisionStatus::Rejected;
            state.diagnostic(
                GameplayRuntimeDiagnosticCode::OwnerRejected,
                "owner",
                if output.diagnostic_codes.is_empty() {
                    "authority owner rejected the operation".to_owned()
                } else {
                    format!(
                        "authority owner rejected the operation: {}",
                        output.diagnostic_codes.join(",")
                    )
                },
            );
        }
        state.final_workspace_hash = moment.workspace.workspace_hash;
        state.finish()
    }

    fn invoke_decision_stage(
        &self,
        family: GameplayInvocationFamily,
        moment: &mut GameplayDecisionMoment,
        frozen_views: &FrozenGameplayViews,
        views: &dyn GameplayViewSource,
        host: &dyn GameplayInvocationHost,
        state: &mut DecisionState<'_>,
    ) -> bool {
        let operation_contract = moment.operation.proposal.clone();
        for module_id in self.registry.module_order() {
            let manifest = self
                .registry
                .module(module_id)
                .expect("module order only contains registry modules");
            let mut invocations = manifest
                .invocations
                .iter()
                .filter(|invocation| {
                    invocation.family == family && invocation.input_contract == operation_contract
                })
                .collect::<Vec<_>>();
            invocations.sort_by(|left, right| left.invocation_id.cmp(&right.invocation_id));
            for invocation in invocations {
                if state.invocations.len() as u32 >= self.limits.max_invocations_per_root
                    || state.module_invocations(module_id)
                        >= manifest.budget.max_invocations_per_root
                {
                    state.diagnostic(
                        GameplayRuntimeDiagnosticCode::InvocationBudgetExceeded,
                        format!(
                            "modules.{module_id}.invocations.{}",
                            invocation.invocation_id
                        ),
                        "pre-commit invocation budget exceeded",
                    );
                    continue;
                }
                if invocation.output_contract != moment.workspace.contract {
                    state.diagnostic(
                        GameplayRuntimeDiagnosticCode::WorkspaceContractMismatch,
                        format!(
                            "modules.{module_id}.invocations.{}.outputContract",
                            invocation.invocation_id
                        ),
                        "invocation output contract does not match the operation Workspace",
                    );
                    continue;
                }

                state.increment_module_invocations(module_id);
                let declared_reads = match views.freeze_declared_decision_reads(
                    module_id,
                    &invocation.invocation_id,
                    moment,
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
                    subscription_id: format!("decision:{}", moment.decision_id),
                    invocation_id: invocation.invocation_id.clone(),
                    family,
                    input: GameplayInvocationInput::Decision(moment.clone()),
                    frozen_views: frozen_views.clone(),
                    declared_reads,
                    configuration: None,
                };
                call.configuration = match host.resolve_configuration(&call) {
                    Ok(configuration) => configuration,
                    Err(error) => {
                        state.diagnostic(
                            GameplayRuntimeDiagnosticCode::HostFailure,
                            format!(
                                "modules.{module_id}.invocations.{}",
                                invocation.invocation_id
                            ),
                            format!("{}: {}", error.code, error.message),
                        );
                        continue;
                    }
                };
                let delivery_hash = delivery_hash(self.registry.registry_digest(), &call);
                let output = match host.invoke(&call) {
                    Ok(output) => output,
                    Err(error) => {
                        state.diagnostic(
                            GameplayRuntimeDiagnosticCode::HostFailure,
                            format!(
                                "modules.{module_id}.invocations.{}",
                                invocation.invocation_id
                            ),
                            format!("{}: {}", error.code, error.message),
                        );
                        continue;
                    }
                };
                let output_hash = semantic_output_hash(&output);
                state.invocations.push(GameplayInvocationEvidence {
                    module_id: module_id.clone(),
                    subscription_id: format!("decision:{}", moment.decision_id),
                    invocation_id: invocation.invocation_id.clone(),
                    event_id: moment.decision_id.clone(),
                    wave: family_stage(family),
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
                if !output.events.is_empty()
                    || !output.proposals.is_empty()
                    || !output.module_facts.is_empty()
                {
                    state.diagnostic(
                        GameplayRuntimeDiagnosticCode::UnexpectedDecisionOutput,
                        format!(
                            "modules.{module_id}.invocations.{}",
                            invocation.invocation_id
                        ),
                        "pre-commit invocations cannot emit post-commit events or proposals",
                    );
                    continue;
                }
                if !state.charge_payload(invocation.max_payload_bytes, &moment.workspace) {
                    continue;
                }
                let Some(decision) = output.decision else {
                    state.diagnostic(
                        GameplayRuntimeDiagnosticCode::MissingDecisionOutput,
                        format!(
                            "modules.{module_id}.invocations.{}",
                            invocation.invocation_id
                        ),
                        "pre-commit invocation returned no decision",
                    );
                    continue;
                };
                if !state.apply_decision(family, decision, moment) {
                    return false;
                }
            }
        }
        true
    }
}

struct DecisionState<'registry> {
    coordinator: &'registry GameplayFabricCoordinator<'registry>,
    decision_id: String,
    owner_id: String,
    initial_workspace_hash: String,
    final_workspace_hash: String,
    status: GameplayDecisionStatus,
    suspension_token: Option<String>,
    suspension_seed: Option<String>,
    continuation: Option<GameplayDecisionContinuation>,
    invocations: Vec<GameplayInvocationEvidence>,
    routing: Option<GameplayRoutingEvidence>,
    diagnostics: Vec<GameplayRuntimeDiagnostic>,
    module_invocations: std::collections::BTreeMap<String, u32>,
    payload_bytes: u64,
}

impl<'registry> DecisionState<'registry> {
    fn new(
        coordinator: &'registry GameplayFabricCoordinator<'registry>,
        moment: &GameplayDecisionMoment,
        owner_id: String,
    ) -> Self {
        Self {
            coordinator,
            decision_id: moment.decision_id.clone(),
            owner_id,
            initial_workspace_hash: moment.workspace.workspace_hash.clone(),
            final_workspace_hash: moment.workspace.workspace_hash.clone(),
            status: GameplayDecisionStatus::Failed,
            suspension_token: None,
            suspension_seed: None,
            continuation: None,
            invocations: Vec::new(),
            routing: None,
            diagnostics: Vec::new(),
            module_invocations: std::collections::BTreeMap::new(),
            payload_bytes: moment.workspace.canonical_payload.len() as u64,
        }
    }

    fn failed_without_owner(
        coordinator: &'registry GameplayFabricCoordinator<'registry>,
        moment: GameplayDecisionMoment,
    ) -> GameplayDecisionReceipt {
        let mut state = Self::new(coordinator, &moment, "unresolved".to_owned());
        state.diagnostic(
            GameplayRuntimeDiagnosticCode::MissingProposalOwner,
            "operation.proposal",
            format!(
                "proposal `{}` has no registered owner",
                moment.operation.proposal.key()
            ),
        );
        state.finish()
    }

    fn module_invocations(&self, module_id: &str) -> u32 {
        self.module_invocations.get(module_id).copied().unwrap_or(0)
    }

    fn increment_module_invocations(&mut self, module_id: &str) {
        *self
            .module_invocations
            .entry(module_id.to_owned())
            .or_default() += 1;
    }

    fn charge_payload(
        &mut self,
        invocation_limit: u32,
        workspace: &GameplayOperationWorkspace,
    ) -> bool {
        let bytes = workspace.canonical_payload.len() as u64;
        self.payload_bytes = self.payload_bytes.saturating_add(bytes);
        if bytes > u64::from(invocation_limit)
            || self.payload_bytes > u64::from(self.coordinator.limits.max_payload_bytes_per_root)
        {
            self.diagnostic(
                GameplayRuntimeDiagnosticCode::PayloadBudgetExceeded,
                "workspace.canonicalPayload",
                "pre-commit Workspace payload budget exceeded",
            );
            return false;
        }
        true
    }

    fn validate_workspace(&mut self, workspace: &GameplayOperationWorkspace, path: &str) -> bool {
        let actual_hash = gameplay_payload_hash(&workspace.canonical_payload);
        if actual_hash != workspace.workspace_hash {
            self.diagnostic(
                GameplayRuntimeDiagnosticCode::WorkspaceHashMismatch,
                format!("{path}.workspaceHash"),
                format!(
                    "Workspace hash `{}` does not match canonical payload `{actual_hash}`",
                    workspace.workspace_hash
                ),
            );
            return false;
        }
        true
    }

    fn apply_decision(
        &mut self,
        family: GameplayInvocationFamily,
        decision: GameplayDecisionOutput,
        moment: &mut GameplayDecisionMoment,
    ) -> bool {
        match (family, decision) {
            (GameplayInvocationFamily::Guard, GameplayDecisionOutput::Guard(vote)) => {
                if vote == GameplayGuardVote::Reject {
                    self.status = GameplayDecisionStatus::Rejected;
                    self.diagnostic(
                        GameplayRuntimeDiagnosticCode::GuardRejected,
                        "guard",
                        "a declared Guard rejected the pending operation",
                    );
                }
                true
            }
            (GameplayInvocationFamily::Transform, GameplayDecisionOutput::Transform(transform)) => {
                self.apply_transform(transform, moment)
            }
            (
                GameplayInvocationFamily::React,
                GameplayDecisionOutput::React {
                    disposition,
                    transform,
                },
            ) => {
                if let Some(transform) = transform {
                    if !self.apply_transform(transform, moment) {
                        return false;
                    }
                }
                match disposition {
                    crate::GameplayReactionDisposition::Continue => true,
                    crate::GameplayReactionDisposition::Cancel { reason } => {
                        self.status = GameplayDecisionStatus::Rejected;
                        self.diagnostic(
                            GameplayRuntimeDiagnosticCode::ReactionCancelled,
                            "react",
                            reason,
                        );
                        false
                    }
                    crate::GameplayReactionDisposition::Suspend { token } => {
                        self.status = GameplayDecisionStatus::Suspended;
                        self.suspension_seed = Some(token);
                        self.diagnostic(
                            GameplayRuntimeDiagnosticCode::ReactionSuspended,
                            "react",
                            "reaction suspended; coordinator continuation issued",
                        );
                        false
                    }
                }
            }
            (_, _) => {
                self.diagnostic(
                    GameplayRuntimeDiagnosticCode::UnexpectedDecisionOutput,
                    "decision",
                    "invocation returned a decision for a different family",
                );
                false
            }
        }
    }

    fn apply_transform(
        &mut self,
        transform: GameplayWorkspaceTransform,
        moment: &mut GameplayDecisionMoment,
    ) -> bool {
        if transform.input_workspace_hash != moment.workspace.workspace_hash {
            self.diagnostic(
                GameplayRuntimeDiagnosticCode::WorkspaceHashMismatch,
                "transform.inputWorkspaceHash",
                "Transform targeted a stale Workspace generation",
            );
            return false;
        }
        if transform.workspace.contract != moment.workspace.contract {
            self.diagnostic(
                GameplayRuntimeDiagnosticCode::WorkspaceContractMismatch,
                "transform.workspace.contract",
                "Transform changed the operation Workspace contract",
            );
            return false;
        }
        if !self.validate_workspace(&transform.workspace, "transform.workspace") {
            return false;
        }
        self.final_workspace_hash = transform.workspace.workspace_hash.clone();
        moment.workspace = transform.workspace;
        true
    }

    fn stale(&mut self, message: &str) {
        self.status = GameplayDecisionStatus::Stale;
        self.diagnostic(
            GameplayRuntimeDiagnosticCode::StaleDecision,
            "expectedOwnerRevision",
            message,
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

    fn finish(mut self) -> GameplayDecisionReceipt {
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
        let mut parts = vec![
            self.coordinator.registry.registry_digest().to_owned(),
            self.decision_id.clone(),
            self.owner_id.clone(),
            self.initial_workspace_hash.clone(),
            self.final_workspace_hash.clone(),
            format!("{:?}", self.status),
            self.suspension_token
                .clone()
                .unwrap_or_else(|| "-".to_owned()),
        ];
        for invocation in &self.invocations {
            parts.extend([
                invocation.module_id.clone(),
                invocation.invocation_id.clone(),
                invocation.delivery_hash.clone(),
                invocation.output_hash.clone(),
            ]);
        }
        if let Some(routing) = &self.routing {
            parts.extend([
                routing.proposal_hash.clone(),
                routing.owner_id.clone(),
                routing.accepted.to_string(),
                routing.routing_hash.clone(),
            ]);
        }
        for diagnostic in &self.diagnostics {
            parts.extend([
                diagnostic_code(diagnostic.code).to_owned(),
                diagnostic.path.clone(),
                diagnostic.message.clone(),
            ]);
        }
        let receipt_hash = stable_hash(parts.iter().map(String::as_str));
        GameplayDecisionReceipt {
            registry_digest: self.coordinator.registry.registry_digest().to_owned(),
            decision_id: self.decision_id,
            owner_id: self.owner_id,
            initial_workspace_hash: self.initial_workspace_hash,
            final_workspace_hash: self.final_workspace_hash,
            status: self.status,
            suspension_token: self.suspension_token,
            continuation: self.continuation,
            invocations: self.invocations,
            routing: self.routing,
            diagnostics: self.diagnostics,
            receipt_hash,
        }
    }
}

struct ContinuationAuthorizationError {
    code: GameplayRuntimeDiagnosticCode,
    message: String,
}

impl GameplayDecisionContinuations {
    fn authorize(
        &mut self,
        moment: &GameplayDecisionMoment,
        registry_digest: &str,
        owner_id: &str,
    ) -> Result<(), ContinuationAuthorizationError> {
        let pending = self.pending.get(&moment.decision_id);
        let Some(token) = moment.resume_token.as_deref() else {
            if pending.is_some() {
                return Err(ContinuationAuthorizationError {
                    code: GameplayRuntimeDiagnosticCode::ContinuationRequired,
                    message:
                        "suspended decision requires its coordinator-issued continuation token"
                            .to_owned(),
                });
            }
            return Ok(());
        };
        let Some(continuation) = pending else {
            return Err(ContinuationAuthorizationError {
                code: GameplayRuntimeDiagnosticCode::ContinuationUnavailable,
                message: "continuation token is unknown, already consumed, or belongs to another decision"
                    .to_owned(),
            });
        };
        let binding_matches = continuation.token == token
            && continuation.decision_id == moment.decision_id
            && continuation.registry_digest == registry_digest
            && continuation.owner_id == owner_id
            && continuation.operation_hash == gameplay_proposal_hash(&moment.operation)
            && continuation.expected_owner_revision == moment.expected_owner_revision
            && continuation.workspace.contract == moment.workspace.contract
            && continuation.workspace.workspace_hash == moment.workspace.workspace_hash;
        if !binding_matches {
            return Err(ContinuationAuthorizationError {
                code: GameplayRuntimeDiagnosticCode::ContinuationMismatch,
                message: "continuation token does not match this decision, Workspace generation, registry, owner, or expected revision"
                    .to_owned(),
            });
        }
        self.pending.remove(&moment.decision_id);
        Ok(())
    }

    fn issue(
        &mut self,
        moment: &GameplayDecisionMoment,
        registry_digest: &str,
        owner_id: &str,
        suspension_seed: &str,
    ) -> GameplayDecisionContinuation {
        let generation = self
            .generations
            .entry(moment.decision_id.clone())
            .and_modify(|value| *value = value.saturating_add(1))
            .or_insert(1);
        let generation_text = generation.to_string();
        let operation_hash = gameplay_proposal_hash(&moment.operation);
        let token = stable_hash([
            registry_digest,
            moment.decision_id.as_str(),
            owner_id,
            operation_hash.as_str(),
            moment.expected_owner_revision.as_str(),
            moment.workspace.contract.key().as_str(),
            moment.workspace.workspace_hash.as_str(),
            suspension_seed,
            generation_text.as_str(),
        ]);
        let continuation = GameplayDecisionContinuation {
            token,
            decision_id: moment.decision_id.clone(),
            registry_digest: registry_digest.to_owned(),
            owner_id: owner_id.to_owned(),
            operation_hash,
            expected_owner_revision: moment.expected_owner_revision.clone(),
            generation: *generation,
            workspace: moment.workspace.clone(),
        };
        self.pending
            .insert(moment.decision_id.clone(), continuation.clone());
        continuation
    }
}

fn family_stage(family: GameplayInvocationFamily) -> u32 {
    match family {
        GameplayInvocationFamily::Guard => 0,
        GameplayInvocationFamily::Transform => 1,
        GameplayInvocationFamily::React => 2,
        GameplayInvocationFamily::Observe => 3,
    }
}
