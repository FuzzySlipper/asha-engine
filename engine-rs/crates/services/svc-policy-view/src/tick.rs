//! The deterministic policy tick stage and its replay/audit record (#2394).
//!
//! The world-layer policy stage is an explicit, ordered pipeline — **not** a generic
//! event bus:
//!
//! ```text
//! project view → (policy proposes, in TS) → validate → apply accepted → record
//! ```
//!
//! This module owns the authority half: it re-projects the audited view, validates
//! and applies each proposed command (atomically, fail-closed), and records the
//! deterministic input envelope plus the proposed/accepted/rejected path. A policy
//! crash, malformed command, or rejection is isolated here — a rejected proposal
//! mutates nothing, so policy failure can never corrupt authority state.
//!
//! Command, event, rejection, and replay types stay separate (see
//! `protocol-policy-view`): the report references each in its own role.

use core_entity::EntityStore;
use protocol_policy_view::{PolicyWorldCommand, PolicyWorldView};

use crate::project::{project_world_view, AssetStatusMap};
use crate::replay::{run_proposals, PolicyTickRecord};

/// The deterministic input envelope recorded for a policy tick. Replaying with the
/// same envelope and proposals reproduces the same outcomes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PolicyTickEnvelope {
    pub tick: u64,
    /// The seed the TS host used to build the policy's deterministic RNG stream.
    pub seed: u64,
}

/// The audit record of one policy tick: the envelope, a snapshot of the projected
/// view the stage ran against, and the proposal/outcome record.
#[derive(Debug, Clone, PartialEq)]
pub struct PolicyTickReport {
    pub envelope: PolicyTickEnvelope,
    /// The view that was projected at the start of the stage (audit snapshot).
    pub projected_view: PolicyWorldView,
    pub record: PolicyTickRecord,
}

impl PolicyTickReport {
    pub fn accepted_count(&self) -> usize {
        self.record.accepted_count()
    }

    pub fn rejected_count(&self) -> usize {
        self.record.rejected_count()
    }
}

/// Run the authority half of a policy tick: project the audited view, then validate
/// and apply each proposed command in order, recording every outcome. Accepted
/// commands mutate `store`; rejected ones leave it untouched. Deterministic: the
/// same store, envelope, and proposals always yield the same report.
pub fn run_policy_tick(
    store: &mut EntityStore,
    envelope: PolicyTickEnvelope,
    asset_statuses: &AssetStatusMap,
    proposals: &[PolicyWorldCommand],
) -> PolicyTickReport {
    let projected_view = project_world_view(envelope.tick, store, asset_statuses);
    let record = run_proposals(store, envelope.tick, proposals);
    PolicyTickReport {
        envelope,
        projected_view,
        record,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_entity::{EntityLifecycleCommand, EntitySource, EntityTransform};
    use core_ids::{EntityId, TagId};
    use protocol_policy_view::{PolicyTransform, PolicyWorldRejection};

    fn world() -> EntityStore {
        let mut store = EntityStore::new();
        store
            .apply(EntityLifecycleCommand::Create {
                id: EntityId::new(1),
                source: EntitySource::RuntimeCreated { by: None },
                labels: Vec::new(),
            })
            .unwrap();
        store.attach_transform(EntityId::new(1), EntityTransform::IDENTITY);
        store
            .apply(EntityLifecycleCommand::Create {
                id: EntityId::new(2),
                source: EntitySource::RuntimeCreated { by: None },
                labels: Vec::new(),
            })
            .unwrap();
        store
    }

    fn envelope() -> PolicyTickEnvelope {
        PolicyTickEnvelope {
            tick: 1,
            seed: 1234,
        }
    }

    fn mixed() -> Vec<PolicyWorldCommand> {
        vec![
            PolicyWorldCommand::RequestAddLabel {
                entity: EntityId::new(1),
                label: TagId::new(9),
            },
            PolicyWorldCommand::RequestSetTransform {
                entity: EntityId::new(2),
                transform: PolicyTransform {
                    translation: [0.0, 0.0, 0.0],
                    rotation: [0.0, 0.0, 0.0, 1.0],
                    scale: [1.0, 1.0, 1.0],
                },
            },
        ]
    }

    #[test]
    fn tick_produces_deterministic_outcomes() {
        let report_a = run_policy_tick(&mut world(), envelope(), &AssetStatusMap::new(), &mixed());
        let report_b = run_policy_tick(&mut world(), envelope(), &AssetStatusMap::new(), &mixed());
        assert_eq!(report_a, report_b);
        assert_eq!(report_a.envelope, envelope());
        assert_eq!(report_a.accepted_count(), 1); // label accepted
        assert_eq!(report_a.rejected_count(), 1); // entity 2 not spatial
                                                  // The audited view snapshot saw both entities before the stage applied edits.
        assert_eq!(report_a.projected_view.entities.len(), 2);
    }

    #[test]
    fn accepted_command_applies_rejected_does_not() {
        let mut store = world();
        run_policy_tick(&mut store, envelope(), &AssetStatusMap::new(), &mixed());
        // The accepted label landed...
        assert!(store
            .core(EntityId::new(1))
            .unwrap()
            .labels
            .contains(&TagId::new(9)));
        // ...and the rejected transform left entity 2 with no transform capability.
        assert!(store.transform(EntityId::new(2)).is_none());
    }

    #[test]
    fn an_all_rejected_tick_leaves_authority_state_unchanged() {
        let mut store = world();
        let before = store.hash();
        // Every proposal targets a missing entity or is otherwise refused.
        let doomed = vec![
            PolicyWorldCommand::RequestDisable {
                entity: EntityId::new(99),
            },
            PolicyWorldCommand::RequestAddLabel {
                entity: EntityId::new(98),
                label: TagId::new(1),
            },
        ];
        let report = run_policy_tick(&mut store, envelope(), &AssetStatusMap::new(), &doomed);
        assert_eq!(report.accepted_count(), 0);
        assert_eq!(report.rejected_count(), 2);
        // Authority state is untouched — policy failure cannot corrupt it.
        assert_eq!(store.hash(), before);
    }

    #[test]
    fn rejections_are_classified_not_silent() {
        let mut store = world();
        let report = run_policy_tick(
            &mut store,
            envelope(),
            &AssetStatusMap::new(),
            &[PolicyWorldCommand::RequestSetTransform {
                entity: EntityId::new(2),
                transform: PolicyTransform {
                    translation: [0.0, 0.0, 0.0],
                    rotation: [0.0, 0.0, 0.0, 1.0],
                    scale: [1.0, 1.0, 1.0],
                },
            }],
        );
        match &report.record.proposals[0].outcome {
            protocol_policy_view::PolicyWorldOutcome::Rejected { rejection } => {
                assert_eq!(*rejection, PolicyWorldRejection::NotSpatial);
            }
            other => panic!("expected a classified rejection, got {other:?}"),
        }
    }
}
