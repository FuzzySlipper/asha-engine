use protocol_game_extension::{
    GameplayCausationRef, GameplayContractRef, GameplayEmitterRef, GameplayEntityRef,
    GameplayEventEnvelope, GameplayEventPhase, GameplayProposalEnvelope,
};
use rule_gameplay_fabric::{
    gameplay_module_payload_hash, GameplayDecisionOutput, GameplayGuardVote, GameplayHostError,
    GameplayInvocationCall, GameplayInvocationInput, GameplayInvocationOutput, GameplayModuleFact,
    GameplayModuleStateScope, GameplayOperationWorkspace, GameplayReactionDisposition,
    GameplayWorkspaceTransform,
};
use serde::{de::DeserializeOwned, Serialize};
use svc_gameplay_fabric::{gameplay_canonical_payload_hash, TypedGameplayEventCodec};

/// Read-only, handler-height view of one coordinator invocation. Raw Session
/// stores and authority owners are intentionally absent.
pub struct GameplayModuleContext<'call> {
    call: &'call GameplayInvocationCall,
}

impl<'call> GameplayModuleContext<'call> {
    pub(crate) fn new(call: &'call GameplayInvocationCall) -> Self {
        Self { call }
    }

    pub fn module_id(&self) -> &str {
        &self.call.module_id
    }

    pub fn invocation_id(&self) -> &str {
        &self.call.invocation_id
    }

    pub fn configuration<T: DeserializeOwned>(&self) -> Result<T, GameplayModuleError> {
        let configuration =
            self.call
                .configuration
                .as_ref()
                .ok_or_else(|| GameplayModuleError {
                    code: "missingInvocationConfiguration".to_owned(),
                    message: format!(
                        "invocation `{}` has no resolved authored configuration",
                        self.call.invocation_id
                    ),
                })?;
        serde_json::from_slice(&configuration.canonical_config).map_err(|error| {
            GameplayModuleError {
                code: "configurationDecodeFailed".to_owned(),
                message: error.to_string(),
            }
        })
    }

    pub fn configuration_scope(&self) -> Option<&GameplayModuleStateScope> {
        self.call
            .configuration
            .as_ref()
            .map(|configuration| &configuration.scope)
    }

    pub fn event_contract(&self) -> Option<&GameplayContractRef> {
        self.event().map(|event| &event.event)
    }

    pub fn tick(&self) -> Option<u64> {
        match &self.call.input {
            GameplayInvocationInput::Observe(event) => Some(event.tick),
            GameplayInvocationInput::Decision(moment) => Some(moment.operation.tick),
        }
    }

    pub fn event_payload<T: DeserializeOwned>(&self) -> Result<T, GameplayModuleError> {
        let event = self.event().ok_or_else(|| GameplayModuleError {
            code: "notObserveInvocation".to_owned(),
            message: "typed event payload requested during a decision invocation".to_owned(),
        })?;
        serde_json::from_slice(&event.canonical_payload).map_err(|error| GameplayModuleError {
            code: "eventDecodeFailed".to_owned(),
            message: error.to_string(),
        })
    }

    pub fn decision_workspace<T: DeserializeOwned>(&self) -> Result<T, GameplayModuleError> {
        let GameplayInvocationInput::Decision(moment) = &self.call.input else {
            return Err(GameplayModuleError {
                code: "notDecisionInvocation".to_owned(),
                message: "decision Workspace requested during an Observe invocation".to_owned(),
            });
        };
        serde_json::from_slice(&moment.workspace.canonical_payload).map_err(|error| {
            GameplayModuleError {
                code: "workspaceDecodeFailed".to_owned(),
                message: error.to_string(),
            }
        })
    }

    pub fn decision_workspace_contract(&self) -> Option<&GameplayContractRef> {
        let GameplayInvocationInput::Decision(moment) = &self.call.input else {
            return None;
        };
        Some(&moment.workspace.contract)
    }

    pub fn decision_workspace_hash(&self) -> Option<&str> {
        let GameplayInvocationInput::Decision(moment) = &self.call.input else {
            return None;
        };
        Some(&moment.workspace.workspace_hash)
    }

    /// Returns the coordinator-issued continuation token only while resuming
    /// a suspended pre-commit decision. Modules can use this to distinguish
    /// the initial reaction window from its authorized continuation without
    /// treating the token as module-owned authority.
    pub fn decision_resume_token(&self) -> Option<&str> {
        let GameplayInvocationInput::Decision(moment) = &self.call.input else {
            return None;
        };
        moment.resume_token.as_deref()
    }

    pub fn source(&self) -> Option<u64> {
        self.event()
            .and_then(|event| event.source.as_ref())
            .map(|source| source.entity.raw())
    }

    pub fn target(&self, index: usize) -> Option<u64> {
        self.event()
            .and_then(|event| event.targets.get(index))
            .map(|target| target.entity.raw())
    }

    pub fn read(&self, request_id: &str) -> Option<&rule_gameplay_fabric::GameplayFrozenRead> {
        self.call
            .declared_reads
            .as_ref()?
            .reads
            .iter()
            .find(|read| read.request_id == request_id)
    }

    pub fn named_view<T: DeserializeOwned>(
        &self,
        request_id: &str,
    ) -> Result<T, GameplayModuleError> {
        self.read(request_id)
            .ok_or_else(|| GameplayModuleError {
                code: "missingDeclaredRead".to_owned(),
                message: format!("declared read `{request_id}` was not delivered"),
            })?
            .decode_named_view()
            .map_err(|error| GameplayModuleError {
                code: "namedViewDecodeFailed".to_owned(),
                message: format!("{error:?}"),
            })
    }

    pub fn actions(&self) -> GameplayModuleActions {
        GameplayModuleActions::new(self)
    }

    fn event(&self) -> Option<&GameplayEventEnvelope> {
        self.call.input.observe_event()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameplayModuleError {
    pub code: String,
    pub message: String,
}

impl From<GameplayModuleError> for GameplayHostError {
    fn from(error: GameplayModuleError) -> Self {
        Self {
            code: error.code,
            message: error.message,
        }
    }
}

pub trait GameplayModuleBehavior {
    fn invoke(
        &self,
        context: &GameplayModuleContext<'_>,
    ) -> Result<GameplayModuleActions, GameplayModuleError>;
}

/// Boring authoring helper. It fills canonical JSON bytes, payload hashes, and
/// placeholder chronology; the coordinator replaces chronology/emitter fields
/// with its authoritative root/wave sequence before routing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameplayModuleActions {
    events: Vec<GameplayEventEnvelope>,
    proposals: Vec<GameplayProposalEnvelope>,
    module_facts: Vec<GameplayModuleFact>,
    trace_codes: Vec<String>,
    decision: Option<GameplayDecisionOutput>,
    module_id: String,
    tick: u64,
    root_sequence: u64,
    wave: u32,
    causation: GameplayCausationRef,
}

impl GameplayModuleActions {
    fn new(context: &GameplayModuleContext<'_>) -> Self {
        let (tick, root_sequence, wave, causation) = match &context.call.input {
            GameplayInvocationInput::Observe(event) => (
                event.tick,
                event.root_sequence,
                event.wave,
                event.causation.clone(),
            ),
            GameplayInvocationInput::Decision(moment) => (
                moment.operation.tick,
                moment.operation.root_sequence,
                moment.operation.wave,
                moment.operation.causation.clone(),
            ),
        };
        Self {
            events: Vec::new(),
            proposals: Vec::new(),
            module_facts: Vec::new(),
            trace_codes: Vec::new(),
            decision: None,
            module_id: context.module_id().to_owned(),
            tick,
            root_sequence,
            wave,
            causation,
        }
    }

    pub fn emit<T: 'static>(
        &mut self,
        codec: &TypedGameplayEventCodec<T>,
        payload: &T,
        source: Option<u64>,
        subjects: Vec<u64>,
        targets: Vec<u64>,
    ) -> Result<&mut Self, GameplayModuleError> {
        let canonical_payload = codec.encode(payload).map_err(|error| GameplayModuleError {
            code: "eventEncodeFailed".to_owned(),
            message: error.to_string(),
        })?;
        let ordinal = self.events.len();
        self.events.push(GameplayEventEnvelope {
            event_id: format!("candidate-event-{ordinal}"),
            event: codec.contract().clone(),
            tick: self.tick,
            root_sequence: self.root_sequence,
            wave: self.wave,
            event_sequence: ordinal as u32,
            phase: GameplayEventPhase::PostCommit,
            emitter: GameplayEmitterRef::Module {
                module_id: self.module_id.clone(),
            },
            causation: self.causation.clone(),
            source: source.map(entity_ref),
            subjects: subjects.into_iter().map(entity_ref).collect(),
            targets: targets.into_iter().map(entity_ref).collect(),
            scope: None,
            tags: Vec::new(),
            payload_hash: gameplay_canonical_payload_hash(&canonical_payload),
            canonical_payload,
        });
        Ok(self)
    }

    pub fn propose<T: 'static>(
        &mut self,
        codec: &TypedGameplayEventCodec<T>,
        payload: &T,
        source: Option<u64>,
        targets: Vec<u64>,
    ) -> Result<&mut Self, GameplayModuleError> {
        let canonical_payload = codec.encode(payload).map_err(|error| GameplayModuleError {
            code: "proposalEncodeFailed".to_owned(),
            message: error.to_string(),
        })?;
        let ordinal = self.proposals.len();
        self.proposals.push(GameplayProposalEnvelope {
            proposal_id: format!("candidate-proposal-{ordinal}"),
            proposal: codec.contract().clone(),
            tick: self.tick,
            root_sequence: self.root_sequence,
            wave: self.wave,
            proposal_sequence: ordinal as u32,
            emitter: GameplayEmitterRef::Module {
                module_id: self.module_id.clone(),
            },
            causation: self.causation.clone(),
            originating_event_id: None,
            source: source.map(entity_ref),
            targets: targets.into_iter().map(entity_ref).collect(),
            payload_hash: gameplay_canonical_payload_hash(&canonical_payload),
            canonical_payload,
        });
        Ok(self)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn record_local_fact_json<T: Serialize>(
        &mut self,
        fact_schema: GameplayContractRef,
        state_schema: GameplayContractRef,
        scope: GameplayModuleStateScope,
        expected_revision: u64,
        payload: &T,
    ) -> Result<&mut Self, GameplayModuleError> {
        let canonical_payload = encode(payload)?;
        let ordinal = self.module_facts.len();
        self.module_facts.push(GameplayModuleFact {
            fact_id: format!(
                "{}/fact/{}/{ordinal}",
                self.causation.root_id, self.module_id
            ),
            module_id: self.module_id.clone(),
            fact_schema,
            state_schema,
            scope,
            expected_revision,
            payload_hash: gameplay_module_payload_hash(&canonical_payload),
            canonical_payload,
        });
        Ok(self)
    }

    pub fn trace(&mut self, code: impl Into<String>) -> &mut Self {
        self.trace_codes.push(code.into());
        self
    }

    pub fn guard(&mut self, vote: GameplayGuardVote) -> &mut Self {
        self.decision = Some(GameplayDecisionOutput::Guard(vote));
        self
    }

    pub fn transform_workspace_json<T: Serialize>(
        &mut self,
        contract: GameplayContractRef,
        input_workspace_hash: impl Into<String>,
        payload: &T,
    ) -> Result<&mut Self, GameplayModuleError> {
        let canonical_payload = encode(payload)?;
        self.decision = Some(GameplayDecisionOutput::Transform(
            GameplayWorkspaceTransform {
                input_workspace_hash: input_workspace_hash.into(),
                workspace: GameplayOperationWorkspace::from_payload(contract, canonical_payload),
            },
        ));
        Ok(self)
    }

    pub fn react(
        &mut self,
        disposition: GameplayReactionDisposition,
        transform: Option<GameplayWorkspaceTransform>,
    ) -> &mut Self {
        self.decision = Some(GameplayDecisionOutput::React {
            disposition,
            transform,
        });
        self
    }

    pub(crate) fn finish(self) -> GameplayInvocationOutput {
        GameplayInvocationOutput {
            events: self.events,
            proposals: self.proposals,
            module_facts: self.module_facts,
            trace_codes: self.trace_codes,
            decision: self.decision,
        }
    }
}

fn encode<T: Serialize>(payload: &T) -> Result<Vec<u8>, GameplayModuleError> {
    serde_json::to_vec(payload).map_err(|error| GameplayModuleError {
        code: "payloadEncodeFailed".to_owned(),
        message: error.to_string(),
    })
}

fn entity_ref(raw: u64) -> GameplayEntityRef {
    GameplayEntityRef {
        entity: core_ids::EntityId::new(raw),
    }
}
