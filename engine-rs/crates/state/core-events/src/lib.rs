//! Domain event types and event batch for the ASHA authority core.
//!
//! # Lane
//!
//! `rust-state` — may depend on `core-ids`, `core-error`, `core-state`.
//! Must not reference protocol, render, UI, or any TypeScript package.
//!
//! # Design
//!
//! [`DomainEvent`] variants represent *accepted, committed* state changes —
//! the authoritative record of what happened, past tense. They are produced
//! by the event applier after a command passes validation and must not carry
//! render, telemetry, or replay transport concerns.
//!
//! [`EventBatch`] is an ordered collection of events produced in one tick.
//! It is the unit handed from the validator/applier to the snapshot layer.
//! Ordering within a batch is significant; consumers must not reorder.

#![forbid(unsafe_code)]

pub mod voxel;
pub use voxel::VoxelEditEvent;

use core_ids::{EntityId, ModeId, ProcessId, SignalId, SubjectId, TagId};

// ── DomainEvent ───────────────────────────────────────────────────────────────

/// Authoritative record of an accepted state change.
///
/// Each variant maps one-to-one to a command outcome. No variant carries
/// render or telemetry data; those concerns live in separate protocol crates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainEvent {
    // Entity lifecycle
    EntityCreated { id: EntityId },
    EntityTagAdded { id: EntityId, tag: TagId },
    EntityTagRemoved { id: EntityId, tag: TagId },
    EntityDeleted { id: EntityId },

    // Subject lifecycle
    SubjectCreated { id: SubjectId },
    SubjectDeleted { id: SubjectId },

    // Process lifecycle
    ProcessStarted { id: ProcessId },
    ProcessModeSet { id: ProcessId, mode: ModeId },
    ProcessStopped { id: ProcessId },

    // Mode definitions
    ModeDefined { id: ModeId },
    ModeUndefined { id: ModeId },

    // Signal definitions
    SignalDefined { id: SignalId },
    SignalUndefined { id: SignalId },

    // Tag definitions
    TagDefined { id: TagId },
    TagUndefined { id: TagId },
}

// ── EventBatch ────────────────────────────────────────────────────────────────

/// An ordered batch of [`DomainEvent`]s produced within a single tick.
///
/// The batch is append-only during construction and read-only once handed off
/// to appliers or snapshot layers. Drain via [`EventBatch::drain`] to consume
/// and clear.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct EventBatch {
    events: Vec<DomainEvent>,
}

impl EventBatch {
    pub fn new() -> Self {
        Self::default()
    }

    /// Append one event to the batch.
    pub fn push(&mut self, event: DomainEvent) {
        self.events.push(event);
    }

    /// View the events in order.
    pub fn events(&self) -> &[DomainEvent] {
        &self.events
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Drain and return all events, leaving the batch empty.
    pub fn drain(&mut self) -> impl Iterator<Item = DomainEvent> + '_ {
        self.events.drain(..)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use core_ids::{EntityId, ModeId, ProcessId, TagId};

    // ── DomainEvent shape ─────────────────────────────────────────────────

    #[test]
    fn event_shape_entity_created() {
        let ev = DomainEvent::EntityCreated {
            id: EntityId::new(1),
        };
        assert!(matches!(ev, DomainEvent::EntityCreated { .. }));
    }

    #[test]
    fn event_shape_entity_tag_added() {
        let ev = DomainEvent::EntityTagAdded {
            id: EntityId::new(2),
            tag: TagId::new(9),
        };
        if let DomainEvent::EntityTagAdded { id, tag } = ev {
            assert_eq!(id, EntityId::new(2));
            assert_eq!(tag, TagId::new(9));
        } else {
            panic!("wrong variant");
        }
    }

    #[test]
    fn event_shape_process_mode_set() {
        let ev = DomainEvent::ProcessModeSet {
            id: ProcessId::new(3),
            mode: ModeId::new(7),
        };
        assert!(matches!(ev, DomainEvent::ProcessModeSet { .. }));
    }

    // ── EventBatch ordering and isolation ────────────────────────────────

    #[test]
    fn event_batch_ordering() {
        let mut batch = EventBatch::new();
        batch.push(DomainEvent::EntityCreated {
            id: EntityId::new(1),
        });
        batch.push(DomainEvent::EntityTagAdded {
            id: EntityId::new(1),
            tag: TagId::new(5),
        });
        batch.push(DomainEvent::EntityDeleted {
            id: EntityId::new(1),
        });

        let events = batch.events();
        assert_eq!(events.len(), 3);
        assert!(matches!(events[0], DomainEvent::EntityCreated { .. }));
        assert!(matches!(events[1], DomainEvent::EntityTagAdded { .. }));
        assert!(matches!(events[2], DomainEvent::EntityDeleted { .. }));
    }

    #[test]
    fn event_batch_len_and_is_empty() {
        let mut batch = EventBatch::new();
        assert!(batch.is_empty());
        batch.push(DomainEvent::TagDefined {
            id: core_ids::TagId::new(1),
        });
        assert_eq!(batch.len(), 1);
        assert!(!batch.is_empty());
    }

    #[test]
    fn event_batch_drain_clears() {
        let mut batch = EventBatch::new();
        batch.push(DomainEvent::SubjectCreated {
            id: core_ids::SubjectId::new(10),
        });
        batch.push(DomainEvent::SubjectDeleted {
            id: core_ids::SubjectId::new(10),
        });

        let drained: Vec<_> = batch.drain().collect();
        assert_eq!(drained.len(), 2);
        assert!(batch.is_empty(), "batch must be empty after drain");
    }

    #[test]
    fn event_batch_type_separation() {
        // Prove events and commands live in distinct type hierarchies.
        // A DomainEvent cannot be used where a Command is expected.
        let _ev: DomainEvent = DomainEvent::EntityCreated {
            id: EntityId::new(0),
        };
        // The lack of any core-commands import in this crate is the real proof.
    }
}
