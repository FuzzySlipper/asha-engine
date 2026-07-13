//! Replayable shared gameplay-action scheduling authority.
//!
//! Matching is read-only. Queue mutation happens only through explicit
//! commands. The product runtime scopes possession of this authority behind a
//! host-borrowing scheduler port, so an Observe participant can collect matches
//! in one frozen wave and route trigger proposals at the next explicit boundary.

use protocol_game_extension::{
    GameplayCausationRef, GameplayContractRef, GameplayEmitterRef, GameplayEventEnvelope,
    GameplayHeaderSelector, GameplayOwnerRef, GameplayProposalEnvelope,
};
use rule_gameplay_fabric::{
    gameplay_proposal_hash, verify_gameplay_routing_evidence, GameplayRoutingEvidence,
    GameplayRoutingReceipt,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

const SNAPSHOT_SCHEMA_VERSION: u32 = 3;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ScheduledActionId(pub String);

impl ScheduledActionId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameplayEventCondition {
    pub event: GameplayContractRef,
    pub selector: GameplayHeaderSelector,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TickScheduledActionDraft {
    pub id: ScheduledActionId,
    pub execute_at: u64,
    pub priority: i32,
    pub proposal: GameplayProposalEnvelope,
    pub source: GameplayEmitterRef,
    pub causation: GameplayCausationRef,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventConditionedActionDraft {
    pub id: ScheduledActionId,
    pub condition: GameplayEventCondition,
    pub priority: i32,
    pub proposal: GameplayProposalEnvelope,
    pub timeout_at: Option<u64>,
    pub source: GameplayEmitterRef,
    pub causation: GameplayCausationRef,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ScheduledGameplayAction {
    Tick {
        id: ScheduledActionId,
        execute_at: u64,
        priority: i32,
        insertion_sequence: u64,
        proposal: GameplayProposalEnvelope,
        source: GameplayEmitterRef,
        causation: GameplayCausationRef,
    },
    EventConditioned {
        id: ScheduledActionId,
        condition: GameplayEventCondition,
        priority: i32,
        insertion_sequence: u64,
        proposal: GameplayProposalEnvelope,
        timeout_at: Option<u64>,
        source: GameplayEmitterRef,
        causation: GameplayCausationRef,
    },
}

impl ScheduledGameplayAction {
    pub fn id(&self) -> &ScheduledActionId {
        match self {
            Self::Tick { id, .. } | Self::EventConditioned { id, .. } => id,
        }
    }

    fn priority(&self) -> i32 {
        match self {
            Self::Tick { priority, .. } | Self::EventConditioned { priority, .. } => *priority,
        }
    }

    fn insertion_sequence(&self) -> u64 {
        match self {
            Self::Tick {
                insertion_sequence, ..
            }
            | Self::EventConditioned {
                insertion_sequence, ..
            } => *insertion_sequence,
        }
    }

    fn proposal(&self) -> &GameplayProposalEnvelope {
        match self {
            Self::Tick { proposal, .. } | Self::EventConditioned { proposal, .. } => proposal,
        }
    }

    fn causation(&self) -> &GameplayCausationRef {
        match self {
            Self::Tick { causation, .. } | Self::EventConditioned { causation, .. } => causation,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduledActionValidity {
    pub targets_present: bool,
    pub causation_current: bool,
}

impl ScheduledActionValidity {
    pub const CURRENT: Self = Self {
        targets_present: true,
        causation_current: true,
    };
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameplaySchedulerCommand {
    ScheduleTick(TickScheduledActionDraft),
    ScheduleEventConditioned(EventConditionedActionDraft),
    ExecuteTick {
        action_id: ScheduledActionId,
        tick: u64,
        validity: ScheduledActionValidity,
    },
    TriggerEvent {
        action_id: ScheduledActionId,
        event: GameplayEventEnvelope,
        validity: ScheduledActionValidity,
    },
    Timeout {
        action_id: ScheduledActionId,
        tick: u64,
    },
    Cancel {
        action_id: ScheduledActionId,
        reason: String,
    },
    RecordRouting {
        action_id: ScheduledActionId,
        receipt: GameplayRoutingReceipt,
    },
    CompleteEventDelivery {
        action_id: ScheduledActionId,
        routing_hash: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ScheduledActionRejectionReason {
    MissingTarget,
    StaleCausation,
    OwnerRejected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum GameplaySchedulerFact {
    Scheduled {
        action: Box<ScheduledGameplayAction>,
    },
    Triggered {
        action_id: ScheduledActionId,
        tick: u64,
        triggering_event_id: Option<String>,
        dispatch: Box<GameplayScheduledDispatch>,
    },
    TimedOut {
        action_id: ScheduledActionId,
        tick: u64,
    },
    Cancelled {
        action_id: ScheduledActionId,
        reason: String,
    },
    Rejected {
        action_id: ScheduledActionId,
        reason: ScheduledActionRejectionReason,
    },
    RoutingAccepted {
        action_id: ScheduledActionId,
        routing: GameplayRoutingEvidence,
        events: Vec<GameplayEventEnvelope>,
    },
    RoutingRejected {
        action_id: ScheduledActionId,
        routing: GameplayRoutingEvidence,
    },
    EventDeliveryCompleted {
        action_id: ScheduledActionId,
        routing_hash: String,
        event_ids: Vec<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameplayScheduledDispatch {
    pub action_id: ScheduledActionId,
    pub proposal: GameplayProposalEnvelope,
    pub proposal_hash: String,
    pub priority: i32,
    pub insertion_sequence: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameplayScheduledEventDelivery {
    pub action_id: ScheduledActionId,
    pub routing: GameplayRoutingEvidence,
    pub events: Vec<GameplayEventEnvelope>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameplaySchedulerReceipt {
    pub fact: GameplaySchedulerFact,
    pub dispatch: Option<GameplayScheduledDispatch>,
    pub state_hash_before: String,
    pub state_hash_after: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameplaySchedulerError {
    InvalidActionId,
    UndeclaredEvent,
    UndeclaredProposal,
    DuplicateAction,
    UnknownAction,
    WrongActionKind,
    NotReady,
    EventDoesNotMatch,
    TimeoutNotReached,
    RoutingMismatch,
    InvalidSnapshot(String),
}

impl core::fmt::Display for GameplaySchedulerError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidActionId => formatter.write_str("scheduled action id is invalid"),
            Self::UndeclaredEvent => formatter.write_str("scheduled event contract is undeclared"),
            Self::UndeclaredProposal => {
                formatter.write_str("scheduled proposal contract is undeclared")
            }
            Self::DuplicateAction => formatter.write_str("scheduled action id already exists"),
            Self::UnknownAction => formatter.write_str("scheduled action does not exist"),
            Self::WrongActionKind => {
                formatter.write_str("scheduled action has the wrong trigger kind")
            }
            Self::NotReady => formatter.write_str("scheduled action is not ready"),
            Self::EventDoesNotMatch => {
                formatter.write_str("event does not match scheduled condition")
            }
            Self::TimeoutNotReached => {
                formatter.write_str("scheduled action timeout has not been reached")
            }
            Self::RoutingMismatch => {
                formatter.write_str("routing outcome does not match a triggered proposal")
            }
            Self::InvalidSnapshot(message) => {
                write!(formatter, "invalid scheduler snapshot: {message}")
            }
        }
    }
}

impl std::error::Error for GameplaySchedulerError {}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct GameplaySchedulerSnapshot {
    schema_version: u32,
    owner: GameplayOwnerRef,
    declared_events: BTreeSet<GameplayContractRef>,
    declared_proposals: BTreeSet<GameplayContractRef>,
    next_insertion_sequence: u64,
    pending: Vec<ScheduledGameplayAction>,
    awaiting_routing: BTreeMap<ScheduledActionId, GameplayScheduledDispatch>,
    awaiting_event_delivery: BTreeMap<ScheduledActionId, GameplayScheduledEventDelivery>,
    retired_ids: BTreeSet<ScheduledActionId>,
    facts: Vec<GameplaySchedulerFact>,
    state_hash: String,
}

#[derive(Debug, Clone)]
pub struct GameplayActionScheduler {
    owner: GameplayOwnerRef,
    declared_events: BTreeSet<GameplayContractRef>,
    declared_proposals: BTreeSet<GameplayContractRef>,
    next_insertion_sequence: u64,
    pending: BTreeMap<ScheduledActionId, ScheduledGameplayAction>,
    awaiting_routing: BTreeMap<ScheduledActionId, GameplayScheduledDispatch>,
    awaiting_event_delivery: BTreeMap<ScheduledActionId, GameplayScheduledEventDelivery>,
    retired_ids: BTreeSet<ScheduledActionId>,
    facts: Vec<GameplaySchedulerFact>,
}

impl GameplayActionScheduler {
    pub fn new(owner: GameplayOwnerRef) -> Self {
        Self::with_contracts(owner, BTreeSet::new(), BTreeSet::new())
    }

    pub fn with_contracts(
        owner: GameplayOwnerRef,
        declared_events: BTreeSet<GameplayContractRef>,
        declared_proposals: BTreeSet<GameplayContractRef>,
    ) -> Self {
        Self {
            owner,
            declared_events,
            declared_proposals,
            next_insertion_sequence: 0,
            pending: BTreeMap::new(),
            awaiting_routing: BTreeMap::new(),
            awaiting_event_delivery: BTreeMap::new(),
            retired_ids: BTreeSet::new(),
            facts: Vec::new(),
        }
    }

    pub fn owner(&self) -> &GameplayOwnerRef {
        &self.owner
    }

    pub fn declared_events(&self) -> &BTreeSet<GameplayContractRef> {
        &self.declared_events
    }

    pub fn declared_proposals(&self) -> &BTreeSet<GameplayContractRef> {
        &self.declared_proposals
    }

    pub fn pending_len(&self) -> usize {
        self.pending.len()
    }

    pub fn pending_actions(&self) -> Vec<&ScheduledGameplayAction> {
        self.pending.values().collect()
    }

    pub fn facts(&self) -> &[GameplaySchedulerFact] {
        &self.facts
    }

    /// Canonical owner inputs that were triggered but not yet acknowledged by
    /// a closed-registry routing receipt. This is the interruption/reload
    /// recovery surface.
    pub fn outstanding_dispatches(&self) -> Vec<&GameplayScheduledDispatch> {
        let mut dispatches = self.awaiting_routing.values().collect::<Vec<_>>();
        dispatches.sort_by(|left, right| {
            (
                left.priority,
                left.action_id.as_str(),
                left.insertion_sequence,
            )
                .cmp(&(
                    right.priority,
                    right.action_id.as_str(),
                    right.insertion_sequence,
                ))
        });
        dispatches
    }

    /// Accepted, registry-routed owner events that have not yet been delivered
    /// into the gameplay fabric. These survive snapshot/replay so retry never
    /// reroutes the already-applied authority proposal.
    pub fn outstanding_event_deliveries(&self) -> Vec<&GameplayScheduledEventDelivery> {
        self.awaiting_event_delivery.values().collect()
    }

    pub fn due_action_ids(&self, tick: u64) -> Vec<ScheduledActionId> {
        let mut due = self
            .pending
            .values()
            .filter_map(|action| match action {
                ScheduledGameplayAction::Tick { id, execute_at, .. } if *execute_at <= tick => {
                    Some((action_order_key(action, *execute_at), id.clone()))
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        due.sort_by(|left, right| left.0.cmp(&right.0));
        due.into_iter().map(|(_, id)| id).collect()
    }

    pub fn matching_action_ids(&self, event: &GameplayEventEnvelope) -> Vec<ScheduledActionId> {
        let mut matching = self
            .pending
            .values()
            .filter_map(|action| match action {
                ScheduledGameplayAction::EventConditioned {
                    id,
                    condition,
                    priority,
                    insertion_sequence,
                    timeout_at,
                    ..
                } if timeout_at.is_none_or(|timeout| event.tick < timeout)
                    && condition.event == event.event
                    && selector_matches(&condition.selector, event) =>
                {
                    Some(((*priority, id.as_str(), *insertion_sequence), id.clone()))
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        matching.sort_by(|left, right| left.0.cmp(&right.0));
        matching.into_iter().map(|(_, id)| id).collect()
    }

    pub fn timed_out_action_ids(&self, tick: u64) -> Vec<ScheduledActionId> {
        let mut timed_out = self
            .pending
            .values()
            .filter_map(|action| match action {
                ScheduledGameplayAction::EventConditioned {
                    id,
                    timeout_at: Some(timeout_at),
                    priority,
                    insertion_sequence,
                    ..
                } if *timeout_at <= tick => Some((
                    (*timeout_at, *priority, id.as_str(), *insertion_sequence),
                    id.clone(),
                )),
                _ => None,
            })
            .collect::<Vec<_>>();
        timed_out.sort_by(|left, right| left.0.cmp(&right.0));
        timed_out.into_iter().map(|(_, id)| id).collect()
    }

    pub fn apply(
        &mut self,
        command: GameplaySchedulerCommand,
    ) -> Result<GameplaySchedulerReceipt, GameplaySchedulerError> {
        let state_hash_before = self.state_hash();
        let (fact, dispatch) = match command {
            GameplaySchedulerCommand::ScheduleTick(draft) => {
                self.require_proposal(&draft.proposal.proposal)?;
                let sequence = self.claim_action_id(&draft.id)?;
                let action = ScheduledGameplayAction::Tick {
                    id: draft.id.clone(),
                    execute_at: draft.execute_at,
                    priority: draft.priority,
                    insertion_sequence: sequence,
                    proposal: draft.proposal,
                    source: draft.source,
                    causation: draft.causation,
                };
                self.pending.insert(draft.id, action.clone());
                (
                    GameplaySchedulerFact::Scheduled {
                        action: Box::new(action),
                    },
                    None,
                )
            }
            GameplaySchedulerCommand::ScheduleEventConditioned(draft) => {
                self.require_proposal(&draft.proposal.proposal)?;
                if !self.declared_events.contains(&draft.condition.event) {
                    return Err(GameplaySchedulerError::UndeclaredEvent);
                }
                let sequence = self.claim_action_id(&draft.id)?;
                let action = ScheduledGameplayAction::EventConditioned {
                    id: draft.id.clone(),
                    condition: draft.condition,
                    priority: draft.priority,
                    insertion_sequence: sequence,
                    proposal: draft.proposal,
                    timeout_at: draft.timeout_at,
                    source: draft.source,
                    causation: draft.causation,
                };
                self.pending.insert(draft.id, action.clone());
                (
                    GameplaySchedulerFact::Scheduled {
                        action: Box::new(action),
                    },
                    None,
                )
            }
            GameplaySchedulerCommand::ExecuteTick {
                action_id,
                tick,
                validity,
            } => self.execute_tick(&action_id, tick, validity)?,
            GameplaySchedulerCommand::TriggerEvent {
                action_id,
                event,
                validity,
            } => self.trigger_event(&action_id, &event, validity)?,
            GameplaySchedulerCommand::Timeout { action_id, tick } => {
                let action = self
                    .pending
                    .get(&action_id)
                    .ok_or(GameplaySchedulerError::UnknownAction)?;
                let ScheduledGameplayAction::EventConditioned {
                    timeout_at: Some(timeout_at),
                    ..
                } = action
                else {
                    return Err(GameplaySchedulerError::WrongActionKind);
                };
                if tick < *timeout_at {
                    return Err(GameplaySchedulerError::TimeoutNotReached);
                }
                self.pending.remove(&action_id);
                self.retired_ids.insert(action_id.clone());
                (GameplaySchedulerFact::TimedOut { action_id, tick }, None)
            }
            GameplaySchedulerCommand::Cancel { action_id, reason } => {
                self.pending
                    .remove(&action_id)
                    .ok_or(GameplaySchedulerError::UnknownAction)?;
                self.retired_ids.insert(action_id.clone());
                (GameplaySchedulerFact::Cancelled { action_id, reason }, None)
            }
            GameplaySchedulerCommand::RecordRouting { action_id, receipt } => {
                let dispatch = self
                    .awaiting_routing
                    .get(&action_id)
                    .ok_or(GameplaySchedulerError::UnknownAction)?;
                let evidence = receipt.evidence();
                if evidence.proposal_id != dispatch.proposal.proposal_id
                    || evidence.proposal_kind != dispatch.proposal.proposal.key()
                    || evidence.proposal_hash != dispatch.proposal_hash
                {
                    return Err(GameplaySchedulerError::RoutingMismatch);
                }
                let (evidence, events) = receipt.into_parts();
                self.awaiting_routing.remove(&action_id);
                let fact = if evidence.accepted {
                    if !events.is_empty() {
                        self.awaiting_event_delivery.insert(
                            action_id.clone(),
                            GameplayScheduledEventDelivery {
                                action_id: action_id.clone(),
                                routing: evidence.clone(),
                                events: events.clone(),
                            },
                        );
                    }
                    GameplaySchedulerFact::RoutingAccepted {
                        action_id,
                        routing: evidence,
                        events,
                    }
                } else {
                    GameplaySchedulerFact::RoutingRejected {
                        action_id,
                        routing: evidence,
                    }
                };
                (fact, None)
            }
            GameplaySchedulerCommand::CompleteEventDelivery {
                action_id,
                routing_hash,
            } => {
                let delivery = self
                    .awaiting_event_delivery
                    .get(&action_id)
                    .ok_or(GameplaySchedulerError::UnknownAction)?;
                if delivery.routing.routing_hash != routing_hash {
                    return Err(GameplaySchedulerError::RoutingMismatch);
                }
                let event_ids = delivery
                    .events
                    .iter()
                    .map(|event| event.event_id.clone())
                    .collect();
                self.awaiting_event_delivery.remove(&action_id);
                (
                    GameplaySchedulerFact::EventDeliveryCompleted {
                        action_id,
                        routing_hash,
                        event_ids,
                    },
                    None,
                )
            }
        };
        self.facts.push(fact.clone());
        Ok(GameplaySchedulerReceipt {
            fact,
            dispatch,
            state_hash_before,
            state_hash_after: self.state_hash(),
        })
    }

    pub fn state_hash(&self) -> String {
        let pending = self.pending.values().collect::<Vec<_>>();
        stable_json_hash(&(
            &self.owner,
            &self.declared_events,
            &self.declared_proposals,
            self.next_insertion_sequence,
            pending,
            &self.awaiting_routing,
            &self.awaiting_event_delivery,
            &self.retired_ids,
            &self.facts,
        ))
    }

    pub fn encode_snapshot(&self) -> Result<Vec<u8>, GameplaySchedulerError> {
        let snapshot = GameplaySchedulerSnapshot {
            schema_version: SNAPSHOT_SCHEMA_VERSION,
            owner: self.owner.clone(),
            declared_events: self.declared_events.clone(),
            declared_proposals: self.declared_proposals.clone(),
            next_insertion_sequence: self.next_insertion_sequence,
            pending: self.pending.values().cloned().collect(),
            awaiting_routing: self.awaiting_routing.clone(),
            awaiting_event_delivery: self.awaiting_event_delivery.clone(),
            retired_ids: self.retired_ids.clone(),
            facts: self.facts.clone(),
            state_hash: self.state_hash(),
        };
        serde_json::to_vec(&snapshot)
            .map_err(|error| GameplaySchedulerError::InvalidSnapshot(error.to_string()))
    }

    pub fn decode_snapshot(bytes: &[u8]) -> Result<Self, GameplaySchedulerError> {
        let snapshot: GameplaySchedulerSnapshot = serde_json::from_slice(bytes)
            .map_err(|error| GameplaySchedulerError::InvalidSnapshot(error.to_string()))?;
        if snapshot.schema_version != SNAPSHOT_SCHEMA_VERSION {
            return Err(GameplaySchedulerError::InvalidSnapshot(format!(
                "unsupported schema version {}",
                snapshot.schema_version
            )));
        }
        let mut pending = BTreeMap::new();
        for action in snapshot.pending {
            if pending.insert(action.id().clone(), action).is_some() {
                return Err(GameplaySchedulerError::InvalidSnapshot(
                    "duplicate pending action id".to_owned(),
                ));
            }
        }
        let replayed = Self::replay(
            snapshot.owner.clone(),
            snapshot.declared_events.clone(),
            snapshot.declared_proposals.clone(),
            &snapshot.facts,
        )
        .map_err(|error| GameplaySchedulerError::InvalidSnapshot(error.to_string()))?;
        if replayed.next_insertion_sequence != snapshot.next_insertion_sequence
            || replayed.pending != pending
            || replayed.awaiting_routing != snapshot.awaiting_routing
            || replayed.awaiting_event_delivery != snapshot.awaiting_event_delivery
            || replayed.retired_ids != snapshot.retired_ids
        {
            return Err(GameplaySchedulerError::InvalidSnapshot(
                "snapshot queue does not agree with replayed facts".to_owned(),
            ));
        }
        let scheduler = Self {
            owner: snapshot.owner,
            declared_events: snapshot.declared_events,
            declared_proposals: snapshot.declared_proposals,
            next_insertion_sequence: snapshot.next_insertion_sequence,
            pending,
            awaiting_routing: snapshot.awaiting_routing,
            awaiting_event_delivery: snapshot.awaiting_event_delivery,
            retired_ids: snapshot.retired_ids,
            facts: snapshot.facts,
        };
        if scheduler.state_hash() != snapshot.state_hash {
            return Err(GameplaySchedulerError::InvalidSnapshot(
                "state hash mismatch".to_owned(),
            ));
        }
        Ok(scheduler)
    }

    pub fn replay(
        owner: GameplayOwnerRef,
        declared_events: BTreeSet<GameplayContractRef>,
        declared_proposals: BTreeSet<GameplayContractRef>,
        facts: &[GameplaySchedulerFact],
    ) -> Result<Self, GameplaySchedulerError> {
        let mut scheduler = Self::with_contracts(owner, declared_events, declared_proposals);
        for fact in facts {
            match fact {
                GameplaySchedulerFact::Scheduled { action } => {
                    scheduler.require_proposal(&action.proposal().proposal)?;
                    if let ScheduledGameplayAction::EventConditioned { condition, .. } =
                        action.as_ref()
                    {
                        if !scheduler.declared_events.contains(&condition.event) {
                            return Err(GameplaySchedulerError::UndeclaredEvent);
                        }
                    }
                    if scheduler.retired_ids.contains(action.id())
                        || scheduler.awaiting_routing.contains_key(action.id())
                    {
                        return Err(GameplaySchedulerError::DuplicateAction);
                    }
                    if scheduler
                        .pending
                        .insert(action.id().clone(), action.as_ref().clone())
                        .is_some()
                    {
                        return Err(GameplaySchedulerError::DuplicateAction);
                    }
                    scheduler.next_insertion_sequence = scheduler
                        .next_insertion_sequence
                        .max(action.insertion_sequence().saturating_add(1));
                }
                GameplaySchedulerFact::Triggered {
                    action_id,
                    dispatch,
                    ..
                } => {
                    scheduler
                        .pending
                        .remove(action_id)
                        .ok_or(GameplaySchedulerError::UnknownAction)?;
                    scheduler.retired_ids.insert(action_id.clone());
                    if dispatch.action_id != *action_id
                        || dispatch.proposal_hash != gameplay_proposal_hash(&dispatch.proposal)
                    {
                        return Err(GameplaySchedulerError::RoutingMismatch);
                    }
                    scheduler
                        .awaiting_routing
                        .insert(action_id.clone(), dispatch.as_ref().clone());
                }
                GameplaySchedulerFact::TimedOut { action_id, .. }
                | GameplaySchedulerFact::Cancelled { action_id, .. }
                | GameplaySchedulerFact::Rejected { action_id, .. } => {
                    scheduler
                        .pending
                        .remove(action_id)
                        .ok_or(GameplaySchedulerError::UnknownAction)?;
                    scheduler.retired_ids.insert(action_id.clone());
                }
                GameplaySchedulerFact::RoutingAccepted {
                    action_id,
                    routing,
                    events,
                } => {
                    let expected = scheduler
                        .awaiting_routing
                        .remove(action_id)
                        .ok_or(GameplaySchedulerError::UnknownAction)?;
                    if expected.proposal_hash != routing.proposal_hash
                        || !routing.accepted
                        || !verify_gameplay_routing_evidence(routing, events)
                    {
                        return Err(GameplaySchedulerError::RoutingMismatch);
                    }
                    if !events.is_empty() {
                        scheduler.awaiting_event_delivery.insert(
                            action_id.clone(),
                            GameplayScheduledEventDelivery {
                                action_id: action_id.clone(),
                                routing: routing.clone(),
                                events: events.clone(),
                            },
                        );
                    }
                }
                GameplaySchedulerFact::RoutingRejected { action_id, routing } => {
                    let expected = scheduler
                        .awaiting_routing
                        .remove(action_id)
                        .ok_or(GameplaySchedulerError::UnknownAction)?;
                    if expected.proposal_hash != routing.proposal_hash
                        || routing.accepted
                        || !verify_gameplay_routing_evidence(routing, &[])
                    {
                        return Err(GameplaySchedulerError::RoutingMismatch);
                    }
                }
                GameplaySchedulerFact::EventDeliveryCompleted {
                    action_id,
                    routing_hash,
                    event_ids,
                } => {
                    let delivery = scheduler
                        .awaiting_event_delivery
                        .remove(action_id)
                        .ok_or(GameplaySchedulerError::UnknownAction)?;
                    let expected_event_ids = delivery
                        .events
                        .iter()
                        .map(|event| event.event_id.as_str())
                        .collect::<Vec<_>>();
                    if delivery.routing.routing_hash != *routing_hash
                        || !expected_event_ids
                            .iter()
                            .copied()
                            .eq(event_ids.iter().map(String::as_str))
                    {
                        return Err(GameplaySchedulerError::RoutingMismatch);
                    }
                }
            }
            scheduler.facts.push(fact.clone());
        }
        Ok(scheduler)
    }

    fn claim_action_id(&mut self, id: &ScheduledActionId) -> Result<u64, GameplaySchedulerError> {
        if !valid_action_id(id.as_str()) {
            return Err(GameplaySchedulerError::InvalidActionId);
        }
        if self.pending.contains_key(id)
            || self.awaiting_routing.contains_key(id)
            || self.awaiting_event_delivery.contains_key(id)
            || self.retired_ids.contains(id)
        {
            return Err(GameplaySchedulerError::DuplicateAction);
        }
        let sequence = self.next_insertion_sequence;
        self.next_insertion_sequence = self.next_insertion_sequence.saturating_add(1);
        Ok(sequence)
    }

    fn require_proposal(
        &self,
        proposal: &GameplayContractRef,
    ) -> Result<(), GameplaySchedulerError> {
        if self.declared_proposals.contains(proposal) {
            Ok(())
        } else {
            Err(GameplaySchedulerError::UndeclaredProposal)
        }
    }

    fn execute_tick(
        &mut self,
        action_id: &ScheduledActionId,
        tick: u64,
        validity: ScheduledActionValidity,
    ) -> Result<(GameplaySchedulerFact, Option<GameplayScheduledDispatch>), GameplaySchedulerError>
    {
        let action = self
            .pending
            .get(action_id)
            .ok_or(GameplaySchedulerError::UnknownAction)?;
        let ScheduledGameplayAction::Tick { execute_at, .. } = action else {
            return Err(GameplaySchedulerError::WrongActionKind);
        };
        if tick < *execute_at {
            return Err(GameplaySchedulerError::NotReady);
        }
        self.finish_ready(action_id, tick, None, validity)
    }

    fn trigger_event(
        &mut self,
        action_id: &ScheduledActionId,
        event: &GameplayEventEnvelope,
        validity: ScheduledActionValidity,
    ) -> Result<(GameplaySchedulerFact, Option<GameplayScheduledDispatch>), GameplaySchedulerError>
    {
        let action = self
            .pending
            .get(action_id)
            .ok_or(GameplaySchedulerError::UnknownAction)?;
        let ScheduledGameplayAction::EventConditioned {
            condition,
            timeout_at,
            ..
        } = action
        else {
            return Err(GameplaySchedulerError::WrongActionKind);
        };
        if timeout_at.is_some_and(|timeout| event.tick >= timeout)
            || condition.event != event.event
            || !selector_matches(&condition.selector, event)
        {
            return Err(GameplaySchedulerError::EventDoesNotMatch);
        }
        self.finish_ready(action_id, event.tick, Some(event), validity)
    }

    fn finish_ready(
        &mut self,
        action_id: &ScheduledActionId,
        tick: u64,
        triggering_event: Option<&GameplayEventEnvelope>,
        validity: ScheduledActionValidity,
    ) -> Result<(GameplaySchedulerFact, Option<GameplayScheduledDispatch>), GameplaySchedulerError>
    {
        let action = self
            .pending
            .remove(action_id)
            .ok_or(GameplaySchedulerError::UnknownAction)?;
        self.retired_ids.insert(action_id.clone());
        if !validity.targets_present {
            return Ok((
                GameplaySchedulerFact::Rejected {
                    action_id: action_id.clone(),
                    reason: ScheduledActionRejectionReason::MissingTarget,
                },
                None,
            ));
        }
        if !validity.causation_current {
            return Ok((
                GameplaySchedulerFact::Rejected {
                    action_id: action_id.clone(),
                    reason: ScheduledActionRejectionReason::StaleCausation,
                },
                None,
            ));
        }

        let mut proposal = action.proposal().clone();
        proposal.proposal_id = format!(
            "scheduler/{}/{}",
            action_id.as_str(),
            action.insertion_sequence()
        );
        proposal.tick = tick;
        proposal.wave = triggering_event.map_or(0, |event| event.wave.saturating_add(1));
        if let Some(event) = triggering_event {
            proposal.root_sequence = event.root_sequence;
        }
        proposal.proposal_sequence = action.insertion_sequence().min(u64::from(u32::MAX)) as u32;
        proposal.emitter = GameplayEmitterRef::Scheduler {
            scheduler_id: self.owner.owner_id.clone(),
        };
        proposal.causation = GameplayCausationRef {
            root_id: triggering_event.map_or_else(
                || action.causation().root_id.clone(),
                |event| event.causation.root_id.clone(),
            ),
            parent_event_id: triggering_event.map(|event| event.event_id.clone()),
            decision_id: triggering_event.map_or_else(
                || action.causation().decision_id.clone(),
                |event| event.causation.decision_id.clone(),
            ),
        };
        proposal.originating_event_id = triggering_event.map(|event| event.event_id.clone());
        let proposal_hash = gameplay_proposal_hash(&proposal);
        let dispatch = GameplayScheduledDispatch {
            action_id: action_id.clone(),
            proposal,
            proposal_hash,
            priority: action.priority(),
            insertion_sequence: action.insertion_sequence(),
        };
        self.awaiting_routing
            .insert(action_id.clone(), dispatch.clone());
        Ok((
            GameplaySchedulerFact::Triggered {
                action_id: action_id.clone(),
                tick,
                triggering_event_id: triggering_event.map(|event| event.event_id.clone()),
                dispatch: Box::new(dispatch.clone()),
            },
            Some(dispatch),
        ))
    }
}

fn action_order_key(
    action: &ScheduledGameplayAction,
    execution_tick: u64,
) -> (u64, i32, &str, u64) {
    (
        execution_tick,
        action.priority(),
        action.id().as_str(),
        action.insertion_sequence(),
    )
}

fn selector_matches(selector: &GameplayHeaderSelector, event: &GameplayEventEnvelope) -> bool {
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

fn valid_action_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"._-".contains(&byte)
        })
}

fn stable_json_hash(value: &impl Serialize) -> String {
    let bytes = serde_json::to_vec(value).expect("scheduler authority values are serializable");
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in (bytes.len() as u64).to_le_bytes().into_iter().chain(bytes) {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("fnv1a64:{hash:016x}")
}

pub fn validate_scheduled_contracts(
    scheduler: &GameplayActionScheduler,
    declared_events: &BTreeSet<GameplayContractRef>,
    declared_proposals: &BTreeSet<GameplayContractRef>,
) -> Result<(), GameplaySchedulerError> {
    for action in scheduler.pending.values() {
        if !declared_proposals.contains(&action.proposal().proposal) {
            return Err(GameplaySchedulerError::InvalidSnapshot(format!(
                "undeclared proposal `{}`",
                action.proposal().proposal.key()
            )));
        }
        if let ScheduledGameplayAction::EventConditioned { condition, .. } = action {
            if !declared_events.contains(&condition.event) {
                return Err(GameplaySchedulerError::InvalidSnapshot(format!(
                    "undeclared event `{}`",
                    condition.event.key()
                )));
            }
        }
    }
    for dispatch in scheduler.awaiting_routing.values() {
        if !declared_proposals.contains(&dispatch.proposal.proposal) {
            return Err(GameplaySchedulerError::InvalidSnapshot(format!(
                "undeclared outstanding proposal `{}`",
                dispatch.proposal.proposal.key()
            )));
        }
    }
    Ok(())
}
