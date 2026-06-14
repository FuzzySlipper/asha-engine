//! Replay records for the policy proposal path (#2392).
//!
//! Every proposed command is recorded as a proposal paired with its authority
//! outcome (accepted event or classified rejection), in proposal order. The record
//! is the audit/replay unit: it shows exactly what a policy proposed and what
//! authority decided, deterministically. `render_tick_record` renders it as stable,
//! hand-checkable text for a golden fixture.

use core_entity::EntityStore;
use protocol_policy_view::{PolicyWorldCommand, PolicyWorldEvent, PolicyWorldOutcome};

use crate::validate::validate_and_apply;

/// One proposed command and the outcome authority returned for it.
#[derive(Debug, Clone, PartialEq)]
pub struct PolicyProposalRecord {
    pub command: PolicyWorldCommand,
    pub outcome: PolicyWorldOutcome,
}

/// The ordered record of one policy tick's proposals and their outcomes.
#[derive(Debug, Clone, PartialEq)]
pub struct PolicyTickRecord {
    pub tick: u64,
    pub proposals: Vec<PolicyProposalRecord>,
}

impl PolicyTickRecord {
    /// How many proposals were accepted.
    pub fn accepted_count(&self) -> usize {
        self.proposals
            .iter()
            .filter(|p| p.outcome.is_accepted())
            .count()
    }

    /// How many proposals were rejected.
    pub fn rejected_count(&self) -> usize {
        self.proposals.len() - self.accepted_count()
    }
}

/// Validate and apply each proposed command against `store` in order, recording the
/// outcome of each. Accepted commands mutate authority; rejected ones do not. The
/// returned record is the deterministic replay/audit unit for the tick.
pub fn run_proposals(
    store: &mut EntityStore,
    tick: u64,
    commands: &[PolicyWorldCommand],
) -> PolicyTickRecord {
    let proposals = commands
        .iter()
        .map(|command| PolicyProposalRecord {
            command: command.clone(),
            outcome: validate_and_apply(store, command),
        })
        .collect();
    PolicyTickRecord { tick, proposals }
}

fn render_command(command: &PolicyWorldCommand) -> String {
    match command {
        PolicyWorldCommand::RequestSetTransform { entity, transform } => format!(
            "requestSetTransform entity={} translation=[{}, {}, {}]",
            entity.raw(),
            transform.translation[0],
            transform.translation[1],
            transform.translation[2],
        ),
        PolicyWorldCommand::RequestAddLabel { entity, label } => {
            format!(
                "requestAddLabel entity={} label={}",
                entity.raw(),
                label.raw()
            )
        }
        PolicyWorldCommand::RequestDisable { entity } => {
            format!("requestDisable entity={}", entity.raw())
        }
        PolicyWorldCommand::NoopMarker { note } => format!("noopMarker note={note:?}"),
    }
}

fn render_outcome(outcome: &PolicyWorldOutcome) -> String {
    match outcome {
        PolicyWorldOutcome::Accepted { event } => format!("accepted {}", render_event(event)),
        PolicyWorldOutcome::Rejected { rejection } => format!("rejected {}", rejection.label()),
    }
}

fn render_event(event: &PolicyWorldEvent) -> String {
    match event {
        PolicyWorldEvent::TransformSet { entity, .. } => {
            format!("transformSet entity={}", entity.raw())
        }
        PolicyWorldEvent::LabelAdded { entity, label } => {
            format!("labelAdded entity={} label={}", entity.raw(), label.raw())
        }
        PolicyWorldEvent::Disabled { entity } => format!("disabled entity={}", entity.raw()),
        PolicyWorldEvent::NoopRecorded { note } => format!("noopRecorded note={note:?}"),
    }
}

/// Render a tick record as deterministic, hand-checkable text for a golden fixture.
pub fn render_tick_record(record: &PolicyTickRecord) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "policy-tick {} accepted={} rejected={}\n",
        record.tick,
        record.accepted_count(),
        record.rejected_count()
    ));
    for (i, proposal) in record.proposals.iter().enumerate() {
        out.push_str(&format!(
            "  [{i}] {} -> {}\n",
            render_command(&proposal.command),
            render_outcome(&proposal.outcome)
        ));
    }
    out
}

/// Deterministic fixtures for the proposal-path golden, shared by the
/// `dump_policy_replay` example and the golden test so the committed bytes have a
/// single source of truth.
pub mod fixtures {
    use super::*;
    use core_entity::{EntityLifecycleCommand, EntitySource, EntityStore, EntityTransform};
    use core_ids::{EntityId, TagId};
    use protocol_policy_view::PolicyTransform;

    fn identity() -> PolicyTransform {
        PolicyTransform {
            translation: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            scale: [1.0, 1.0, 1.0],
        }
    }

    /// A small abstract world: a spatial entity (1), a logical entity (2), and a
    /// disabled spatial entity (3).
    pub fn scenario_world() -> EntityStore {
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
            .apply(EntityLifecycleCommand::Create {
                id: EntityId::new(3),
                source: EntitySource::RuntimeCreated { by: None },
                labels: Vec::new(),
            })
            .unwrap();
        store.attach_transform(EntityId::new(3), EntityTransform::IDENTITY);
        store
            .apply(EntityLifecycleCommand::Disable {
                id: EntityId::new(3),
            })
            .unwrap();
        store
    }

    /// A mix of accepted and rejected proposals exercising every command and a
    /// representative rejection of each kind.
    pub fn scenario_proposals() -> Vec<PolicyWorldCommand> {
        vec![
            PolicyWorldCommand::RequestSetTransform {
                entity: EntityId::new(1),
                transform: PolicyTransform {
                    translation: [4.0, 0.0, 0.0],
                    ..identity()
                },
            },
            PolicyWorldCommand::RequestSetTransform {
                entity: EntityId::new(2),
                transform: identity(),
            },
            PolicyWorldCommand::RequestSetTransform {
                entity: EntityId::new(3),
                transform: identity(),
            },
            PolicyWorldCommand::RequestAddLabel {
                entity: EntityId::new(1),
                label: TagId::new(5),
            },
            PolicyWorldCommand::RequestDisable {
                entity: EntityId::new(99),
            },
            PolicyWorldCommand::NoopMarker {
                note: "tick-complete".to_string(),
            },
        ]
    }

    /// Render the canonical proposal-path tick record as deterministic text.
    pub fn dump() -> String {
        let mut store = scenario_world();
        let record = run_proposals(&mut store, 1, &scenario_proposals());
        render_tick_record(&record)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_entity::{EntityLifecycleCommand, EntitySource, EntityTransform};
    use core_ids::{EntityId, TagId};

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

    fn mixed_proposals() -> Vec<PolicyWorldCommand> {
        vec![
            // Accepted: move the spatial entity 1.
            PolicyWorldCommand::RequestSetTransform {
                entity: EntityId::new(1),
                transform: PolicyTransformFixture::moved(),
            },
            // Rejected: entity 2 is not spatial.
            PolicyWorldCommand::RequestSetTransform {
                entity: EntityId::new(2),
                transform: PolicyTransformFixture::identity(),
            },
            // Accepted: label entity 2.
            PolicyWorldCommand::RequestAddLabel {
                entity: EntityId::new(2),
                label: TagId::new(5),
            },
            // Rejected: unknown entity.
            PolicyWorldCommand::RequestDisable {
                entity: EntityId::new(99),
            },
            // Accepted: audit marker.
            PolicyWorldCommand::NoopMarker {
                note: "tick-done".to_string(),
            },
        ]
    }

    struct PolicyTransformFixture;
    impl PolicyTransformFixture {
        fn identity() -> protocol_policy_view::PolicyTransform {
            protocol_policy_view::PolicyTransform {
                translation: [0.0, 0.0, 0.0],
                rotation: [0.0, 0.0, 0.0, 1.0],
                scale: [1.0, 1.0, 1.0],
            }
        }
        fn moved() -> protocol_policy_view::PolicyTransform {
            protocol_policy_view::PolicyTransform {
                translation: [2.0, 0.0, 0.0],
                ..Self::identity()
            }
        }
    }

    #[test]
    fn records_proposed_accepted_and_rejected_in_order() {
        let mut store = world();
        let record = run_proposals(&mut store, 1, &mixed_proposals());
        assert_eq!(record.proposals.len(), 5);
        assert_eq!(record.accepted_count(), 3);
        assert_eq!(record.rejected_count(), 2);
        // Proposal order is preserved.
        assert!(record.proposals[0].outcome.is_accepted());
        assert!(!record.proposals[1].outcome.is_accepted());
    }

    #[test]
    fn render_is_deterministic_and_stable() {
        let mut store_a = world();
        let mut store_b = world();
        let a = render_tick_record(&run_proposals(&mut store_a, 1, &mixed_proposals()));
        let b = render_tick_record(&run_proposals(&mut store_b, 1, &mixed_proposals()));
        assert_eq!(a, b);
        assert!(a.starts_with("policy-tick 1 accepted=3 rejected=2\n"));
        assert!(a.contains("rejected notSpatial"));
        assert!(a.contains("rejected unknownEntity"));
    }
}
