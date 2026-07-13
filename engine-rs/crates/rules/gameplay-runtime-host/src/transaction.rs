use super::*;

/// Opaque evidence checkpoint used by the enclosing RuntimeSession to make a
/// decision, owner commit, and resulting owner-event cascade one transaction.
/// It contains no module behavior, registry, EntityStore, or mutable authority
/// handle and is deliberately not a downstream persistence surface.
#[derive(Clone)]
pub struct GameplayRuntimeTransactionCheckpoint {
    reaction_frames: Vec<GameplayReactionFrame>,
    decision_continuations: GameplayDecisionContinuations,
    decision_receipts: Vec<GameplayDecisionReceipt>,
}

impl GameplayRuntimeHost {
    #[doc(hidden)]
    pub fn checkpoint_transaction_evidence(&self) -> GameplayRuntimeTransactionCheckpoint {
        GameplayRuntimeTransactionCheckpoint {
            reaction_frames: self.reaction_frames.clone(),
            decision_continuations: self.decision_continuations.clone(),
            decision_receipts: self.decision_receipts.clone(),
        }
    }

    #[doc(hidden)]
    pub fn restore_transaction_evidence(
        &mut self,
        checkpoint: GameplayRuntimeTransactionCheckpoint,
    ) {
        self.reaction_frames = checkpoint.reaction_frames;
        self.decision_continuations = checkpoint.decision_continuations;
        self.decision_receipts = checkpoint.decision_receipts;
    }
}

pub(super) fn activation_hash(activation: &GameplayModuleBindingActivationReceipt) -> String {
    let bytes = serde_json::to_vec(activation).expect("activation receipt serializes");
    rule_gameplay_fabric::gameplay_module_payload_hash(&bytes)
}
