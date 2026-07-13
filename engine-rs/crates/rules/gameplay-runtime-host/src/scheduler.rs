//! Public composition of replayable gameplay-action scheduling into the host.

use std::collections::BTreeSet;

use core_entity::EntityStore;
use protocol_game_extension::{GameplayContractRef, GameplayOwnerRef};
use rule_gameplay_fabric::{GameplayFabricCoordinator, GameplayRoutingEvidence};
use serde::{Deserialize, Serialize};

pub use rule_scheduler::{
    EventConditionedActionDraft, GameplayActionScheduler, GameplayEventCondition,
    GameplayScheduledDispatch, GameplaySchedulerCommand, GameplaySchedulerError,
    GameplaySchedulerFact, GameplaySchedulerReceipt, ScheduledActionId,
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
    pub fact_count: u32,
    pub pending_actions: Vec<ScheduledGameplayAction>,
    pub outstanding_dispatches: Vec<GameplayScheduledDispatch>,
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
        Ok(GameplayRuntimeSchedulerRoutingReceipt {
            routing: routing_evidence,
            fact,
            readout: self.scheduler_readout(),
        })
    }

    pub fn scheduler_readout(&self) -> GameplayRuntimeSchedulerReadout {
        let pending_action_count = self.scheduler.pending_len();
        let outstanding_dispatch_count = self.scheduler.outstanding_dispatches().len();
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
        GameplayRuntimeSchedulerReadout {
            owner_id: self.scheduler.owner().owner_id.clone(),
            state_hash: self.scheduler.state_hash(),
            pending_action_count: u32::try_from(pending_action_count).unwrap_or(u32::MAX),
            outstanding_dispatch_count: u32::try_from(outstanding_dispatch_count)
                .unwrap_or(u32::MAX),
            fact_count: u32::try_from(self.scheduler.facts().len()).unwrap_or(u32::MAX),
            pending_actions,
            outstanding_dispatches,
            truncated: pending_action_count > MAX_SCHEDULER_READOUT_ITEMS
                || outstanding_dispatch_count > MAX_SCHEDULER_READOUT_ITEMS,
        }
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
