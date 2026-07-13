//! Concrete RuntimeSession owner ports shared by decisions and scheduler routes.

use core_entity::EntityStore;
use protocol_entity_authoring::{
    ActivatableCapabilityKind, CapabilityActivationAction, CapabilityActivationOutcome,
    CapabilityActivationRequest,
};
use protocol_game_extension::GameplayOwnerRef;
use rule_gameplay_fabric::{
    gameplay_module_payload_hash, CapabilityActivationGameplayProposal, GameplayDecisionOwner,
    GameplayOwnerRoutingCall, GameplayOwnerRoutingOutput, GameplayProposalRouter,
    StandardGameplayProposalKind, CAPABILITY_ACTIVATION_PROPOSAL_OWNER_ID,
};
use svc_entity_authoring::{apply_rule_owned_capability_activation, EcrpRuleOwner};

use crate::{EntityId, GameplayRuntimeDecisionOwner};

pub(crate) struct RuntimeSessionDecisionOwner<'a> {
    pub(crate) owner: &'a mut dyn GameplayRuntimeDecisionOwner,
}

impl GameplayDecisionOwner for RuntimeSessionDecisionOwner<'_> {
    fn revision_hash(&self, owner: &GameplayOwnerRef) -> String {
        self.owner.revision_hash(owner)
    }

    fn route_precommit(&mut self, call: &GameplayOwnerRoutingCall) -> GameplayOwnerRoutingOutput {
        let output = self.owner.route_precommit(&call.owner, &call.proposal);
        GameplayOwnerRoutingOutput {
            accepted: output.accepted,
            fact_hashes: output.fact_hashes,
            events: output.events,
            diagnostic_codes: output.diagnostic_codes,
        }
    }
}

pub(crate) struct RuntimeSessionOwnerRouter<'a> {
    pub(crate) entities: &'a mut EntityStore,
}

impl GameplayProposalRouter for RuntimeSessionOwnerRouter<'_> {
    fn route(&mut self, call: &GameplayOwnerRoutingCall) -> GameplayOwnerRoutingOutput {
        if call.proposal.proposal
            != StandardGameplayProposalKind::SetCapabilityActivation.contract()
            || call.owner.owner_id != CAPABILITY_ACTIVATION_PROPOSAL_OWNER_ID
        {
            return rejected_owner_output("unsupportedOwnerProposal");
        }
        let payload: CapabilityActivationGameplayProposal =
            match serde_json::from_slice(&call.proposal.canonical_payload) {
                Ok(payload) => payload,
                Err(_) => return rejected_owner_output("proposalDecodeFailed"),
            };
        if payload.entity == 0
            || payload.entity
                != call
                    .proposal
                    .targets
                    .first()
                    .map_or(0, |target| target.entity.raw())
        {
            return rejected_owner_output("proposalTargetMismatch");
        }
        let (capability, owner) = match payload.capability.as_str() {
            "collision" => (
                ActivatableCapabilityKind::Collision,
                EcrpRuleOwner::CollisionRule,
            ),
            "controller" => (
                ActivatableCapabilityKind::Controller,
                EcrpRuleOwner::ControllerRule,
            ),
            _ => return rejected_owner_output("unsupportedCapability"),
        };
        let action = match payload.action.as_str() {
            "activate" => CapabilityActivationAction::Activate,
            "deactivate" => CapabilityActivationAction::Deactivate,
            _ => return rejected_owner_output("unsupportedActivationAction"),
        };
        match apply_rule_owned_capability_activation(
            self.entities,
            owner,
            CapabilityActivationRequest {
                entity: EntityId::new(payload.entity),
                capability,
                action,
            },
        ) {
            CapabilityActivationOutcome::Accepted { .. } => GameplayOwnerRoutingOutput {
                accepted: true,
                fact_hashes: vec![gameplay_module_payload_hash(
                    &call.proposal.canonical_payload,
                )],
                ..GameplayOwnerRoutingOutput::default()
            },
            CapabilityActivationOutcome::Rejected { diagnostic }
            | CapabilityActivationOutcome::Forbidden { diagnostic } => GameplayOwnerRoutingOutput {
                accepted: false,
                diagnostic_codes: vec![format!("{:?}", diagnostic.code)],
                ..GameplayOwnerRoutingOutput::default()
            },
        }
    }
}

fn rejected_owner_output(code: &str) -> GameplayOwnerRoutingOutput {
    GameplayOwnerRoutingOutput {
        accepted: false,
        diagnostic_codes: vec![code.to_owned()],
        ..GameplayOwnerRoutingOutput::default()
    }
}
