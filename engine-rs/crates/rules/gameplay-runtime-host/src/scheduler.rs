//! Public composition of replayable gameplay-action scheduling into the host.

use std::collections::BTreeSet;

use core_entity::EntityStore;
use protocol_game_extension::{GameplayContractRef, GameplayOwnerRef};
use rule_gameplay_fabric::{
    GameplayFabricCoordinator, GameplayReactionSourceFact, GameplayRoutingEvidence,
};
use serde::{Deserialize, Serialize};

pub use rule_scheduler::{
    EventConditionedActionDraft, GameplayActionScheduler, GameplayEventCondition,
    GameplayScheduledDispatch, GameplayScheduledEventDelivery, GameplaySchedulerCommand,
    GameplaySchedulerError, GameplaySchedulerFact, GameplaySchedulerReceipt, ScheduledActionId,
    ScheduledActionRejectionReason, ScheduledActionValidity, ScheduledGameplayAction,
    TickScheduledActionDraft,
};

use crate::{
    limits_from_registry, GameplayRuntimeHost, GameplayRuntimeHostError, RuntimeSessionOwnerRouter,
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
    /// Apply one owner-gated scheduler command. Triggering commands retain the
    /// complete proposal in recoverable scheduler state until the caller routes
    /// it through [`Self::route_scheduled_action`].
    pub fn apply_scheduler_command(
        &mut self,
        command: GameplaySchedulerCommand,
    ) -> Result<GameplayRuntimeSchedulerCommandReceipt, GameplayRuntimeHostError> {
        validate_scheduler_command_codecs(self.session.registry(), &command)?;
        let owner = self.scheduler.owner().clone();
        let scheduler = self.scheduler.apply(&owner, command)?;
        Ok(GameplayRuntimeSchedulerCommandReceipt {
            scheduler,
            readout: self.scheduler_readout(),
        })
    }

    /// Route one recoverable scheduled dispatch through the same closed fabric
    /// registry and concrete RuntimeSession owner router used by module output.
    pub fn route_scheduled_action(
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
                        } if routed_action_id == action_id
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
        let owner = self.scheduler.owner().clone();
        let routing_evidence = routing.evidence().clone();
        let recorded = self.scheduler.apply(
            &owner,
            GameplaySchedulerCommand::RecordRouting {
                action_id: action_id.clone(),
                receipt: routing,
            },
        );
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
        let scheduler_owner = self.scheduler.owner().clone();
        let completed = self.scheduler.apply(
            &scheduler_owner,
            GameplaySchedulerCommand::CompleteEventDelivery {
                action_id: delivery.action_id.clone(),
                routing_hash: delivery.routing.routing_hash.clone(),
            },
        )?;
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
) -> Result<(), GameplayRuntimeHostError> {
    for fact in scheduler.facts() {
        match fact {
            GameplaySchedulerFact::Scheduled { action } => {
                registry
                    .admit_proposal(scheduled_action_proposal(action))
                    .map_err(|error| {
                        GameplayRuntimeHostError::Snapshot(format!(
                            "replayed scheduler proposal failed codec admission: {error}"
                        ))
                    })?;
            }
            GameplaySchedulerFact::Triggered { dispatch, .. } => {
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
    if definition
        .declared_proposals
        .iter()
        .any(|proposal| registry.proposal_owner(proposal).is_none())
    {
        return Err(GameplaySchedulerError::UndeclaredProposal.into());
    }
    Ok(())
}
