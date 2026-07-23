//! Public composition of replayable gameplay-action scheduling into the host.

use std::collections::BTreeSet;

use core_entity::EntityStore;
use protocol_game_extension::{GameplayContractRef, GameplayEventEnvelope, GameplayOwnerRef};
use rule_gameplay_fabric::{
    direct_authority_routing_receipt, GameplayFabricCoordinator, GameplayOwnerRoutingOutput,
    GameplayReactionSourceFact, GameplayRoutingEvidence,
};
use serde::{Deserialize, Serialize};

pub use rule_scheduler::{
    EventConditionedActionDraft, GameplayEventCondition, GameplayScheduledDispatch,
    GameplayScheduledEventDelivery, GameplaySchedulerError, GameplaySchedulerFact,
    GameplaySchedulerReceipt, ScheduledActionId, ScheduledActionRejectionReason,
    ScheduledActionValidity, ScheduledGameplayAction, TickScheduledActionDraft,
};
use rule_scheduler::{GameplayActionScheduler, GameplaySchedulerCommand};

use crate::{
    authority_verbs::DIRECT_AUTHORITY_OWNER_ID, limits_from_registry, GameplayRuntimeHost,
    GameplayRuntimeHostError, RuntimeSessionOwnerRouter,
};

const MAX_SCHEDULER_READOUT_ITEMS: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GameplayRuntimeSchedulerDefinition {
    pub owner: GameplayOwnerRef,
    pub declared_events: Vec<GameplayContractRef>,
    pub declared_proposals: Vec<GameplayContractRef>,
}

impl GameplayRuntimeSchedulerDefinition {
    pub fn new(
        owner: GameplayOwnerRef,
        declared_events: Vec<GameplayContractRef>,
        declared_proposals: Vec<GameplayContractRef>,
    ) -> Self {
        Self {
            owner,
            declared_events,
            declared_proposals,
        }
    }

    pub(crate) fn build(&self) -> GameplayActionScheduler {
        GameplayActionScheduler::with_contracts(
            self.owner.clone(),
            self.declared_events.iter().cloned().collect(),
            self.declared_proposals.iter().cloned().collect(),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameplayRuntimeSchedulerReadout {
    pub owner_id: String,
    pub state_hash: String,
    pub pending_action_count: u32,
    pub outstanding_dispatch_count: u32,
    pub outstanding_event_delivery_count: u32,
    pub fact_count: u32,
    pub pending_actions: Vec<ScheduledGameplayAction>,
    pub outstanding_dispatches: Vec<GameplayScheduledDispatch>,
    pub outstanding_event_deliveries: Vec<GameplayScheduledEventDelivery>,
    pub truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameplayRuntimeSchedulerCommandReceipt {
    pub scheduler: GameplaySchedulerReceipt,
    pub readout: GameplayRuntimeSchedulerReadout,
}

/// Product-facing scheduler mutations available through a scoped
/// [`GameplayRuntimeSchedulerPort`]. Closed-registry routing and delivery
/// acknowledgement are deliberately absent; the host performs those steps as
/// one recoverable operation through [`GameplayRuntimeSchedulerPort::route`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameplayRuntimeSchedulerCommand {
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
}

impl GameplayRuntimeSchedulerCommand {
    fn into_core(self) -> GameplaySchedulerCommand {
        match self {
            Self::ScheduleTick(draft) => GameplaySchedulerCommand::ScheduleTick(draft),
            Self::ScheduleEventConditioned(draft) => {
                GameplaySchedulerCommand::ScheduleEventConditioned(draft)
            }
            Self::ExecuteTick {
                action_id,
                tick,
                validity,
            } => GameplaySchedulerCommand::ExecuteTick {
                action_id,
                tick,
                validity,
            },
            Self::TriggerEvent {
                action_id,
                event,
                validity,
            } => GameplaySchedulerCommand::TriggerEvent {
                action_id,
                event,
                validity,
            },
            Self::Timeout { action_id, tick } => {
                GameplaySchedulerCommand::Timeout { action_id, tick }
            }
            Self::Cancel { action_id, reason } => {
                GameplaySchedulerCommand::Cancel { action_id, reason }
            }
        }
    }
}

/// Lexically scoped authority for scheduler mutation and routing.
///
/// The port cannot be cloned or serialized and borrows exactly one live host.
/// Possession, granted by the trusted Rust composition/transport adapter, is
/// the authorization boundary. There is no caller-supplied owner string or
/// bearer token to forge, replay, redirect to another Session, or carry across
/// restore.
///
/// The borrow cannot be promoted into persistent or replayable authority:
///
/// ```compile_fail
/// # use gameplay_runtime_host::GameplayRuntimeSchedulerPort;
/// fn persist(
///     port: GameplayRuntimeSchedulerPort<'_>,
/// ) -> GameplayRuntimeSchedulerPort<'static> {
///     port
/// }
/// ```
///
/// The port is deliberately non-cloneable:
///
/// ```compile_fail
/// # use gameplay_runtime_host::GameplayRuntimeSchedulerPort;
/// fn duplicate(port: GameplayRuntimeSchedulerPort<'_>) {
///     let _copy = port.clone();
/// }
/// ```
#[must_use = "scheduler authority is exercised through the scoped port"]
pub struct GameplayRuntimeSchedulerPort<'host> {
    host: &'host mut GameplayRuntimeHost,
}

impl GameplayRuntimeSchedulerPort<'_> {
    pub fn apply(
        &mut self,
        command: GameplayRuntimeSchedulerCommand,
    ) -> Result<GameplayRuntimeSchedulerCommandReceipt, GameplayRuntimeHostError> {
        self.host.apply_scheduler_command(command.into_core())
    }

    pub fn route(
        &mut self,
        action_id: &ScheduledActionId,
    ) -> Result<GameplayRuntimeSchedulerRoutingReceipt, GameplayRuntimeHostError> {
        self.host.route_scheduled_action(action_id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameplayRuntimeSchedulerRoutingReceipt {
    pub routing: GameplayRoutingEvidence,
    pub fact: GameplaySchedulerFact,
    pub delivery_fact: Option<GameplaySchedulerFact>,
    pub delivered_events: Vec<protocol_game_extension::GameplayEventEnvelope>,
    pub reaction: Option<crate::GameplayRuntimeReactionReceipt>,
    pub readout: GameplayRuntimeSchedulerReadout,
}

impl GameplayRuntimeHost {
    /// Borrow scheduler authority for this live host instance. The configured
    /// scheduler owner remains routing/evidence identity; it is not caller
    /// authentication.
    pub fn scheduler_port(&mut self) -> GameplayRuntimeSchedulerPort<'_> {
        GameplayRuntimeSchedulerPort { host: self }
    }

    /// Apply one command after the public scoped port has granted access.
    /// Triggering commands retain the complete proposal in recoverable
    /// scheduler state until the same port routes it.
    fn apply_scheduler_command(
        &mut self,
        command: GameplaySchedulerCommand,
    ) -> Result<GameplayRuntimeSchedulerCommandReceipt, GameplayRuntimeHostError> {
        validate_scheduler_command_codecs(self.session.registry(), &command)?;
        let scheduler = self.scheduler.apply(command)?;
        Ok(GameplayRuntimeSchedulerCommandReceipt {
            scheduler,
            readout: self.scheduler_readout(),
        })
    }

    /// Route one recoverable scheduled dispatch through the same closed fabric
    /// registry and concrete RuntimeSession owner router used by module output.
    pub(crate) fn route_scheduled_action(
        &mut self,
        action_id: &ScheduledActionId,
    ) -> Result<GameplayRuntimeSchedulerRoutingReceipt, GameplayRuntimeHostError> {
        if let Some(delivery) = self
            .scheduler
            .outstanding_event_deliveries()
            .into_iter()
            .find(|delivery| &delivery.action_id == action_id)
            .cloned()
        {
            let routing_fact = self
                .scheduler
                .facts()
                .iter()
                .rev()
                .find(|fact| {
                    matches!(
                        fact,
                        GameplaySchedulerFact::RoutingAccepted {
                            action_id: routed_action_id,
                            ..
                        } if routed_action_id.as_str() == action_id.as_str()
                    )
                })
                .cloned()
                .ok_or(GameplaySchedulerError::RoutingMismatch)?;
            return self.deliver_scheduled_events(delivery, routing_fact);
        }
        let dispatch = self
            .scheduler
            .outstanding_dispatches()
            .into_iter()
            .find(|dispatch| &dispatch.action_id == action_id)
            .cloned()
            .ok_or(GameplaySchedulerError::UnknownAction)?;
        if dispatch.proposal.proposal == crate::authored_behavior::authored_program_step_contract()
        {
            return self.route_authored_program_action(action_id, dispatch);
        }
        let mut entities = self
            .session
            .bundle
            .runtime_entities
            .take()
            .ok_or(GameplayRuntimeHostError::MissingEntityAuthority)?;
        let authority_before = entities.snapshot_durable();
        let scheduler_before = self.scheduler.clone();
        let reaction_frame_count_before = self.reaction_frames.len();
        let routing = GameplayFabricCoordinator::new(
            self.session.registry(),
            limits_from_registry(self.session.registry()),
        )
        .route_proposal(
            dispatch.proposal,
            &mut RuntimeSessionOwnerRouter {
                entities: &mut entities,
            },
        )
        .map_err(GameplayRuntimeHostError::SchedulerRouting);
        let routing = match routing {
            Ok(routing) => routing,
            Err(error) => {
                self.session.bundle.runtime_entities =
                    Some(EntityStore::from_snapshot(authority_before));
                return Err(error);
            }
        };
        let routing_evidence = routing.evidence().clone();
        let recorded = self
            .scheduler
            .apply(GameplaySchedulerCommand::RecordRouting {
                action_id: action_id.clone(),
                receipt: routing,
            });
        let recorded = match recorded {
            Ok(recorded) => recorded,
            Err(error) => {
                self.session.bundle.runtime_entities =
                    Some(EntityStore::from_snapshot(authority_before));
                return Err(error.into());
            }
        };
        self.session.bundle.runtime_entities = Some(entities);
        let GameplaySchedulerReceipt { fact, .. } = recorded;
        if let Some(delivery) = self
            .scheduler
            .outstanding_event_deliveries()
            .into_iter()
            .find(|delivery| &delivery.action_id == action_id)
            .cloned()
        {
            let delivered = self.deliver_scheduled_events(delivery, fact.clone());
            if delivered.is_err() {
                self.session.bundle.runtime_entities =
                    Some(EntityStore::from_snapshot(authority_before));
                self.scheduler = scheduler_before;
                self.reaction_frames.truncate(reaction_frame_count_before);
            }
            return delivered;
        }
        Ok(GameplayRuntimeSchedulerRoutingReceipt {
            routing: routing_evidence,
            fact,
            delivery_fact: None,
            delivered_events: Vec::new(),
            reaction: None,
            readout: self.scheduler_readout(),
        })
    }

    fn route_authored_program_action(
        &mut self,
        action_id: &ScheduledActionId,
        dispatch: GameplayScheduledDispatch,
    ) -> Result<GameplayRuntimeSchedulerRoutingReceipt, GameplayRuntimeHostError> {
        let mut entities = self
            .session
            .bundle
            .runtime_entities
            .take()
            .ok_or(GameplayRuntimeHostError::MissingEntityAuthority)?;
        let entity_checkpoint = entities.clone();
        let scheduler_checkpoint = self.scheduler.clone();
        let authored_checkpoint = self.authored_program.clone();
        let reaction_frame_count_before = self.reaction_frames.len();

        let execution = match self.authored_program.as_mut() {
            Some(program) => {
                program.execute_continuation(&dispatch.proposal, &mut entities, &mut self.scheduler)
            }
            None => Err("authored program is unavailable".to_owned()),
        };
        let output = match execution {
            Ok(execution) => GameplayOwnerRoutingOutput {
                accepted: true,
                fact_hashes: execution
                    .facts
                    .into_iter()
                    .map(|fact| fact.fact_hash)
                    .collect(),
                events: execution.events,
                ..GameplayOwnerRoutingOutput::default()
            },
            Err(code) => {
                entities = entity_checkpoint.clone();
                self.scheduler = scheduler_checkpoint.clone();
                self.authored_program = authored_checkpoint.clone();
                GameplayOwnerRoutingOutput {
                    accepted: false,
                    diagnostic_codes: vec![code],
                    ..GameplayOwnerRoutingOutput::default()
                }
            }
        };
        let receipt =
            direct_authority_routing_receipt(&dispatch.proposal, DIRECT_AUTHORITY_OWNER_ID, output);
        let routing_evidence = receipt.evidence().clone();
        let recorded = self
            .scheduler
            .apply(GameplaySchedulerCommand::RecordRouting {
                action_id: action_id.clone(),
                receipt,
            });
        let recorded = match recorded {
            Ok(recorded) => recorded,
            Err(error) => {
                self.session.bundle.runtime_entities = Some(entity_checkpoint);
                self.scheduler = scheduler_checkpoint;
                self.authored_program = authored_checkpoint;
                return Err(error.into());
            }
        };
        self.session.bundle.runtime_entities = Some(entities);
        let GameplaySchedulerReceipt { fact, .. } = recorded;
        if let Some(delivery) = self
            .scheduler
            .outstanding_event_deliveries()
            .into_iter()
            .find(|delivery| &delivery.action_id == action_id)
            .cloned()
        {
            let delivered = self.deliver_scheduled_events(delivery, fact.clone());
            if delivered.is_err() {
                self.session.bundle.runtime_entities = Some(entity_checkpoint);
                self.scheduler = scheduler_checkpoint;
                self.authored_program = authored_checkpoint;
                self.reaction_frames.truncate(reaction_frame_count_before);
            }
            return delivered;
        }
        Ok(GameplayRuntimeSchedulerRoutingReceipt {
            routing: routing_evidence,
            fact,
            delivery_fact: None,
            delivered_events: Vec::new(),
            reaction: None,
            readout: self.scheduler_readout(),
        })
    }

    fn deliver_scheduled_events(
        &mut self,
        delivery: GameplayScheduledEventDelivery,
        routing_fact: GameplaySchedulerFact,
    ) -> Result<GameplayRuntimeSchedulerRoutingReceipt, GameplayRuntimeHostError> {
        let scheduler_before = self.scheduler.clone();
        let reaction_frame_count_before = self.reaction_frames.len();
        let authority_before = self
            .session
            .bundle
            .runtime_entities
            .as_ref()
            .ok_or(GameplayRuntimeHostError::MissingEntityAuthority)?
            .snapshot_durable();
        let completed = self
            .scheduler
            .apply(GameplaySchedulerCommand::CompleteEventDelivery {
                action_id: delivery.action_id.clone(),
                routing_hash: delivery.routing.routing_hash.clone(),
            })?;
        let source_fact = GameplayReactionSourceFact::new(
            delivery.routing.owner_id.clone(),
            "gameplayOwnerRouting".to_owned(),
            serde_json::to_vec(&delivery.routing)
                .expect("routing evidence serializes for reaction replay"),
        );
        let reaction = self
            .observe_routed_events_with_source_facts(delivery.events.clone(), vec![source_fact]);
        let reaction = match reaction {
            Ok(reaction) if reaction.observe.accepted() => reaction,
            Ok(reaction) => {
                let error = reaction
                    .observe
                    .diagnostics
                    .first()
                    .cloned()
                    .ok_or(GameplaySchedulerError::RoutingMismatch)?;
                self.session.bundle.runtime_entities =
                    Some(EntityStore::from_snapshot(authority_before));
                self.scheduler = scheduler_before;
                self.reaction_frames.truncate(reaction_frame_count_before);
                return Err(GameplayRuntimeHostError::SchedulerRouting(error));
            }
            Err(error) => {
                self.session.bundle.runtime_entities =
                    Some(EntityStore::from_snapshot(authority_before));
                self.scheduler = scheduler_before;
                self.reaction_frames.truncate(reaction_frame_count_before);
                return Err(error);
            }
        };
        Ok(GameplayRuntimeSchedulerRoutingReceipt {
            routing: delivery.routing,
            fact: routing_fact,
            delivery_fact: Some(completed.fact),
            delivered_events: delivery.events,
            reaction: Some(reaction),
            readout: self.scheduler_readout(),
        })
    }

    pub fn scheduler_readout(&self) -> GameplayRuntimeSchedulerReadout {
        let pending_action_count = self.scheduler.pending_len();
        let outstanding_dispatch_count = self.scheduler.outstanding_dispatches().len();
        let outstanding_event_delivery_count = self.scheduler.outstanding_event_deliveries().len();
        let pending_actions = self
            .scheduler
            .pending_actions()
            .into_iter()
            .take(MAX_SCHEDULER_READOUT_ITEMS)
            .cloned()
            .collect();
        let outstanding_dispatches = self
            .scheduler
            .outstanding_dispatches()
            .into_iter()
            .take(MAX_SCHEDULER_READOUT_ITEMS)
            .cloned()
            .collect();
        let outstanding_event_deliveries = self
            .scheduler
            .outstanding_event_deliveries()
            .into_iter()
            .take(MAX_SCHEDULER_READOUT_ITEMS)
            .cloned()
            .collect();
        GameplayRuntimeSchedulerReadout {
            owner_id: self.scheduler.owner().owner_id.clone(),
            state_hash: self.scheduler.state_hash(),
            pending_action_count: u32::try_from(pending_action_count).unwrap_or(u32::MAX),
            outstanding_dispatch_count: u32::try_from(outstanding_dispatch_count)
                .unwrap_or(u32::MAX),
            outstanding_event_delivery_count: u32::try_from(outstanding_event_delivery_count)
                .unwrap_or(u32::MAX),
            fact_count: u32::try_from(self.scheduler.facts().len()).unwrap_or(u32::MAX),
            pending_actions,
            outstanding_dispatches,
            outstanding_event_deliveries,
            truncated: pending_action_count > MAX_SCHEDULER_READOUT_ITEMS
                || outstanding_dispatch_count > MAX_SCHEDULER_READOUT_ITEMS
                || outstanding_event_delivery_count > MAX_SCHEDULER_READOUT_ITEMS,
        }
    }
}

fn validate_scheduler_command_codecs(
    registry: &svc_gameplay_fabric::GameplayFabricRegistry,
    command: &GameplaySchedulerCommand,
) -> Result<(), GameplayRuntimeHostError> {
    match command {
        GameplaySchedulerCommand::ScheduleTick(draft) => registry.admit_proposal(&draft.proposal),
        GameplaySchedulerCommand::ScheduleEventConditioned(draft) => {
            registry.admit_proposal(&draft.proposal)
        }
        GameplaySchedulerCommand::TriggerEvent { event, .. } => registry.admit_event(event),
        GameplaySchedulerCommand::RecordRouting { receipt, .. } => {
            for event in receipt.accepted_events() {
                registry
                    .admit_event(event)
                    .map_err(|error| GameplayRuntimeHostError::Codec(error.to_string()))?;
            }
            return Ok(());
        }
        GameplaySchedulerCommand::ExecuteTick { .. }
        | GameplaySchedulerCommand::Timeout { .. }
        | GameplaySchedulerCommand::Cancel { .. }
        | GameplaySchedulerCommand::CompleteEventDelivery { .. } => return Ok(()),
    }
    .map_err(|error| GameplayRuntimeHostError::Codec(error.to_string()))
}

pub(crate) fn validate_replayed_scheduler_codecs(
    registry: &svc_gameplay_fabric::GameplayFabricRegistry,
    scheduler: &GameplayActionScheduler,
    allow_authored_program: bool,
) -> Result<(), GameplayRuntimeHostError> {
    for fact in scheduler.facts() {
        match fact {
            GameplaySchedulerFact::Scheduled { action } => {
                if allow_authored_program
                    && scheduled_action_proposal(action).proposal
                        == crate::authored_behavior::authored_program_step_contract()
                {
                    continue;
                }
                registry
                    .admit_proposal(scheduled_action_proposal(action))
                    .map_err(|error| {
                        GameplayRuntimeHostError::Snapshot(format!(
                            "replayed scheduler proposal failed codec admission: {error}"
                        ))
                    })?;
            }
            GameplaySchedulerFact::Triggered { dispatch, .. } => {
                if allow_authored_program
                    && dispatch.proposal.proposal
                        == crate::authored_behavior::authored_program_step_contract()
                {
                    continue;
                }
                registry
                    .admit_proposal(&dispatch.proposal)
                    .map_err(|error| {
                        GameplayRuntimeHostError::Snapshot(format!(
                            "replayed scheduler dispatch failed codec admission: {error}"
                        ))
                    })?;
            }
            GameplaySchedulerFact::RoutingAccepted { events, .. } => {
                for event in events {
                    registry.admit_event(event).map_err(|error| {
                        GameplayRuntimeHostError::Snapshot(format!(
                            "replayed scheduler owner event failed codec admission: {error}"
                        ))
                    })?;
                }
            }
            GameplaySchedulerFact::TimedOut { .. }
            | GameplaySchedulerFact::Cancelled { .. }
            | GameplaySchedulerFact::Rejected { .. }
            | GameplaySchedulerFact::RoutingRejected { .. }
            | GameplaySchedulerFact::EventDeliveryCompleted { .. } => {}
        }
    }
    Ok(())
}

fn scheduled_action_proposal(
    action: &ScheduledGameplayAction,
) -> &protocol_game_extension::GameplayProposalEnvelope {
    match action {
        ScheduledGameplayAction::Tick { proposal, .. }
        | ScheduledGameplayAction::EventConditioned { proposal, .. } => proposal,
    }
}

pub(crate) fn validate_scheduler_definition(
    registry: &svc_gameplay_fabric::GameplayFabricRegistry,
    definition: &GameplayRuntimeSchedulerDefinition,
    allow_authored_program: bool,
) -> Result<(), GameplayRuntimeHostError> {
    if definition.owner.owner_id.trim().is_empty()
        || definition.owner.provider_id.trim().is_empty()
        || definition
            .declared_events
            .iter()
            .collect::<BTreeSet<_>>()
            .len()
            != definition.declared_events.len()
        || definition
            .declared_proposals
            .iter()
            .collect::<BTreeSet<_>>()
            .len()
            != definition.declared_proposals.len()
    {
        return Err(GameplaySchedulerError::InvalidSnapshot(
            "scheduler owner and declared contracts must be nonempty and unique".to_owned(),
        )
        .into());
    }
    if definition
        .declared_events
        .iter()
        .any(|event| !registry.event_is_declared(event))
    {
        return Err(GameplaySchedulerError::UndeclaredEvent.into());
    }
    if definition.declared_proposals.iter().any(|proposal| {
        !(allow_authored_program
            && *proposal == crate::authored_behavior::authored_program_step_contract())
            && registry.proposal_owner(proposal).is_none()
    }) {
        return Err(GameplaySchedulerError::UndeclaredProposal.into());
    }
    Ok(())
}
