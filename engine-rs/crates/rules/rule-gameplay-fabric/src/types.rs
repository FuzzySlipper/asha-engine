use protocol_game_extension::{
    GameplayContractRef, GameplayEventEnvelope, GameplayInvocationFamily, GameplayOwnerRef,
    GameplayProposalEnvelope,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrozenGameplayViews {
    pub epoch: u64,
    pub view_hash: String,
}

/// Produces one immutable read-view generation at the start of each wave.
/// Owner routing may change authority between waves, but never the generation
/// already handed to an invocation in the current wave.
pub trait GameplayViewSource {
    fn freeze(&self, root_id: &str, wave: u32) -> FrozenGameplayViews;

    /// Supplies the concrete declared read set for one Observe invocation.
    /// Existing view sources may return `None`; modules that declare read views
    /// should assemble them once from the wave snapshot and return owned data.
    fn freeze_declared_reads(
        &self,
        _module_id: &str,
        _invocation_id: &str,
        _event: &GameplayEventEnvelope,
    ) -> Result<Option<crate::GameplayFrozenReadSet>, crate::GameplayReadAssemblyError> {
        Ok(None)
    }

    /// Supplies the concrete declared read set for one pre-commit decision
    /// invocation. Decision reads are a separate method so implementations
    /// cannot accidentally reinterpret a proposal Workspace as a committed
    /// event.
    fn freeze_declared_decision_reads(
        &self,
        _module_id: &str,
        _invocation_id: &str,
        _moment: &GameplayDecisionMoment,
    ) -> Result<Option<crate::GameplayFrozenReadSet>, crate::GameplayReadAssemblyError> {
        Ok(None)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameplayInvocationCall {
    pub module_id: String,
    pub subscription_id: String,
    pub invocation_id: String,
    pub family: GameplayInvocationFamily,
    pub input: GameplayInvocationInput,
    pub frozen_views: FrozenGameplayViews,
    pub declared_reads: Option<crate::GameplayFrozenReadSet>,
    pub configuration: Option<GameplayInvocationConfiguration>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameplayInvocationConfiguration {
    pub binding_id: String,
    pub configuration_id: String,
    pub scope: crate::GameplayModuleStateScope,
    pub canonical_config: Vec<u8>,
    pub config_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameplayInvocationInput {
    Observe(GameplayEventEnvelope),
    Decision(GameplayDecisionMoment),
}

impl GameplayInvocationInput {
    pub fn observe_event(&self) -> Option<&GameplayEventEnvelope> {
        match self {
            Self::Observe(event) => Some(event),
            Self::Decision(_) => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameplayOperationWorkspace {
    pub contract: GameplayContractRef,
    pub canonical_payload: Vec<u8>,
    pub workspace_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameplayDecisionMoment {
    pub decision_id: String,
    pub operation: GameplayProposalEnvelope,
    pub expected_owner_revision: String,
    pub workspace: GameplayOperationWorkspace,
    pub resume_token: Option<String>,
}

/// Coordinator-issued authority to resume exactly one suspended decision
/// generation. The complete Workspace is returned so callers do not have to
/// reconstruct a transformed pre-commit generation from hashes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameplayDecisionContinuation {
    pub token: String,
    pub decision_id: String,
    pub registry_digest: String,
    pub owner_id: String,
    pub operation_hash: String,
    pub expected_owner_revision: String,
    pub generation: u64,
    pub workspace: GameplayOperationWorkspace,
}

/// Explicit Session-owned continuation state. It is passed to the coordinator
/// rather than hidden in a global or mutable registry.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameplayDecisionContinuations {
    pub(crate) pending: std::collections::BTreeMap<String, GameplayDecisionContinuation>,
    pub(crate) generations: std::collections::BTreeMap<String, u64>,
}

impl GameplayDecisionContinuations {
    pub fn pending(&self, decision_id: &str) -> Option<&GameplayDecisionContinuation> {
        self.pending.get(decision_id)
    }

    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameplayGuardVote {
    Accept,
    Reject,
    Abstain,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameplayWorkspaceTransform {
    pub input_workspace_hash: String,
    pub workspace: GameplayOperationWorkspace,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameplayReactionDisposition {
    Continue,
    Cancel { reason: String },
    Suspend { token: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameplayDecisionOutput {
    Guard(GameplayGuardVote),
    Transform(GameplayWorkspaceTransform),
    React {
        disposition: GameplayReactionDisposition,
        transform: Option<GameplayWorkspaceTransform>,
    },
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GameplayInvocationOutput {
    /// Control fields such as event id, wave, emitter, and causation are
    /// canonicalized by the coordinator. Contract, header, and payload fields
    /// are the module's proposed event.
    pub events: Vec<GameplayEventEnvelope>,
    /// Control fields such as proposal id, wave, emitter, and causation are
    /// canonicalized by the coordinator before owner routing.
    pub proposals: Vec<GameplayProposalEnvelope>,
    /// Candidate facts for the emitting module's registered state owner. They
    /// are validated and recorded by Observe; applying them remains the
    /// module-state authority coordinator's explicit step.
    pub module_facts: Vec<crate::GameplayModuleFact>,
    pub trace_codes: Vec<String>,
    pub decision: Option<GameplayDecisionOutput>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameplayHostError {
    pub code: String,
    pub message: String,
}

pub trait GameplayInvocationHost {
    fn resolve_configuration(
        &self,
        _call: &GameplayInvocationCall,
    ) -> Result<Option<GameplayInvocationConfiguration>, GameplayHostError> {
        Ok(None)
    }

    fn invoke(
        &self,
        call: &GameplayInvocationCall,
    ) -> Result<GameplayInvocationOutput, GameplayHostError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameplayOwnerRoutingCall {
    pub owner: GameplayOwnerRef,
    pub proposal: GameplayProposalEnvelope,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GameplayOwnerRoutingOutput {
    pub accepted: bool,
    pub fact_hashes: Vec<String>,
    pub events: Vec<GameplayEventEnvelope>,
    pub diagnostic_codes: Vec<String>,
}

/// Pre-commit decisions cannot publish post-commit events as part of the
/// atomic owner call. This narrower result makes an unsupported event-producing
/// decision impossible to represent; post-commit proposal routes use
/// [`GameplayOwnerRoutingOutput`] instead.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GameplayDecisionRoutingOutput {
    pub accepted: bool,
    pub fact_hashes: Vec<String>,
    pub diagnostic_codes: Vec<String>,
}

pub trait GameplayProposalRouter {
    fn route(&mut self, call: &GameplayOwnerRoutingCall) -> GameplayOwnerRoutingOutput;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameplayWaveStateHashes {
    pub authority_state_hash: String,
    pub module_state_hash: String,
    pub prefab_state_hash: String,
    pub trigger_state_hash: String,
}

/// Mutable Session transaction port used only at the barrier between Observe
/// waves. Invocations receive immutable methods; routing and fact application
/// happen after every invocation in the current wave has returned.
pub trait GameplayWaveAuthority {
    fn freeze(&self, root_id: &str, wave: u32) -> FrozenGameplayViews;

    fn freeze_declared_reads(
        &self,
        module_id: &str,
        invocation_id: &str,
        event: &GameplayEventEnvelope,
    ) -> Result<Option<crate::GameplayFrozenReadSet>, crate::GameplayReadAssemblyError>;

    fn route(&mut self, call: &GameplayOwnerRoutingCall) -> GameplayOwnerRoutingOutput;

    fn apply_module_facts_atomic(
        &mut self,
        facts: &[crate::GameplayModuleFact],
    ) -> Result<(), GameplayHostError>;

    fn state_hashes(&self) -> GameplayWaveStateHashes;
}

/// Pre-commit owner port. The coordinator checks the owner revision before
/// invocation and immediately before the single atomic route.
pub trait GameplayDecisionOwner {
    fn revision_hash(&self, owner: &GameplayOwnerRef) -> String;
    fn route_precommit(&mut self, call: &GameplayOwnerRoutingCall)
        -> GameplayDecisionRoutingOutput;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameplayRuntimeLimits {
    pub max_waves: u32,
    pub max_events_per_root: u32,
    pub max_proposals_per_root: u32,
    pub max_invocations_per_root: u32,
    pub max_payload_bytes_per_root: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GameplayRuntimeDiagnosticCode {
    UnknownEvent,
    UndeclaredInvocation,
    UndeclaredEvent,
    UndeclaredProposal,
    UndeclaredModuleFact,
    MissingProposalOwner,
    ReadAssemblyFailed,
    HostFailure,
    WaveBudgetExceeded,
    EventBudgetExceeded,
    ProposalBudgetExceeded,
    InvocationBudgetExceeded,
    PayloadBudgetExceeded,
    PayloadCodecRejected,
    InvocationOutputBudgetExceeded,
    SubscriptionDeliveryBudgetExceeded,
    UnexpectedDecisionOutput,
    MissingDecisionOutput,
    GuardRejected,
    WorkspaceContractMismatch,
    WorkspaceHashMismatch,
    ContinuationRequired,
    ContinuationMismatch,
    ContinuationUnavailable,
    StaleDecision,
    ReactionCancelled,
    ReactionSuspended,
    OwnerRejected,
    InvalidOwnerEvent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameplayRuntimeDiagnostic {
    pub code: GameplayRuntimeDiagnosticCode,
    pub path: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameplayInvocationEvidence {
    pub module_id: String,
    pub subscription_id: String,
    pub invocation_id: String,
    pub event_id: String,
    pub wave: u32,
    pub frozen_view_hash: String,
    pub declared_read_set_hash: Option<String>,
    #[serde(default)]
    pub declared_reads: Option<crate::GameplayFrozenReadSet>,
    #[serde(default)]
    pub configuration: Option<GameplayInvocationConfiguration>,
    pub delivery_hash: String,
    pub output_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameplayRoutingEvidence {
    pub registry_digest: String,
    pub proposal_id: String,
    pub proposal_kind: String,
    pub proposal_hash: String,
    pub owner_id: String,
    pub accepted: bool,
    pub fact_hashes: Vec<String>,
    pub diagnostic_codes: Vec<String>,
    pub routing_hash: String,
}

/// Opaque proof that a proposal was resolved and routed through one closed
/// gameplay-fabric registry. Consumers can inspect the evidence but cannot
/// manufacture this receipt outside the routing coordinator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameplayRoutingReceipt {
    pub(crate) evidence: GameplayRoutingEvidence,
    pub(crate) accepted_events: Vec<GameplayEventEnvelope>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameplayWaveBarrierEvidence {
    pub wave: u32,
    pub frozen_view: FrozenGameplayViews,
    pub state_before: GameplayWaveStateHashes,
    pub state_after: GameplayWaveStateHashes,
    pub routing_hashes: Vec<String>,
    pub module_fact_hashes: Vec<String>,
    pub barrier_hash: String,
}

impl GameplayRoutingReceipt {
    pub fn evidence(&self) -> &GameplayRoutingEvidence {
        &self.evidence
    }

    /// Canonically ordered owner events that the caller must enqueue or
    /// explicitly reject before considering this route complete.
    pub fn accepted_events(&self) -> &[GameplayEventEnvelope] {
        &self.accepted_events
    }

    pub fn into_parts(self) -> (GameplayRoutingEvidence, Vec<GameplayEventEnvelope>) {
        (self.evidence, self.accepted_events)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameplayEventEvidence {
    pub event_id: String,
    pub event_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameplayObserveReceipt {
    pub registry_digest: String,
    pub root_id: String,
    pub waves_processed: u32,
    pub wave_views: Vec<FrozenGameplayViews>,
    pub wave_barriers: Vec<GameplayWaveBarrierEvidence>,
    pub events: Vec<GameplayEventEnvelope>,
    pub event_evidence: Vec<GameplayEventEvidence>,
    pub invocations: Vec<GameplayInvocationEvidence>,
    pub routing: Vec<GameplayRoutingEvidence>,
    pub module_facts: Vec<crate::GameplayModuleFact>,
    pub diagnostics: Vec<GameplayRuntimeDiagnostic>,
    pub receipt_hash: String,
}

impl GameplayObserveReceipt {
    pub fn accepted(&self) -> bool {
        self.diagnostics.is_empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GameplayDecisionStatus {
    Accepted,
    Rejected,
    Suspended,
    Stale,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameplayDecisionReceipt {
    pub registry_digest: String,
    pub decision_id: String,
    pub owner_id: String,
    pub initial_workspace_hash: String,
    pub final_workspace_hash: String,
    pub status: GameplayDecisionStatus,
    pub suspension_token: Option<String>,
    pub continuation: Option<GameplayDecisionContinuation>,
    pub invocations: Vec<GameplayInvocationEvidence>,
    pub routing: Option<GameplayRoutingEvidence>,
    pub diagnostics: Vec<GameplayRuntimeDiagnostic>,
    pub receipt_hash: String,
}
