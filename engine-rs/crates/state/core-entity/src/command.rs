//! Entity lifecycle commands, events, and classified errors (#2387).
//!
//! Commands are *proposed* lifecycle changes; the [`crate::store::EntityStore`]
//! validates a command and, on success, produces the corresponding past-tense
//! [`EntityLifecycleEvent`]. A rejected command produces a classified
//! [`EntityLifecycleError`] and **no state mutation** (fail closed, atomic).

use core_error::ErrorCategory;
use core_ids::{EntityId, TagId};

use crate::core::{EntityLifecycle, EntitySource};

/// A proposed lifecycle change. Capability seeding (transform/bounds/etc.) is a
/// separate store concern (bootstrap/import attaches capabilities); these verbs
/// govern *existence* and *classification* only.
#[derive(Debug, Clone, PartialEq)]
pub enum EntityLifecycleCommand {
    /// Create a new entity with a source provenance and an initial label set.
    Create {
        id: EntityId,
        source: EntitySource,
        labels: Vec<TagId>,
    },
    /// Tombstone an entity (terminal logical destruction).
    Destroy { id: EntityId },
    /// Move an `Active` entity to `Disabled`.
    Disable { id: EntityId },
    /// Move a `Disabled` entity back to `Active`.
    Enable { id: EntityId },
    /// Add a classification label to a live entity.
    AddLabel { id: EntityId, tag: TagId },
    /// Remove a classification label from a live entity.
    RemoveLabel { id: EntityId, tag: TagId },
}

/// The authoritative, past-tense record of an accepted lifecycle change.
#[derive(Debug, Clone, PartialEq)]
pub enum EntityLifecycleEvent {
    Created {
        id: EntityId,
        source: EntitySource,
        labels: Vec<TagId>,
    },
    Destroyed {
        id: EntityId,
    },
    Disabled {
        id: EntityId,
    },
    Enabled {
        id: EntityId,
    },
    LabelAdded {
        id: EntityId,
        tag: TagId,
    },
    LabelRemoved {
        id: EntityId,
        tag: TagId,
    },
}

/// Why a lifecycle command was rejected. Classified so an orchestrator/agent can
/// route deterministically without parsing prose.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntityLifecycleError {
    /// `Create` for an id that already exists (active or disabled).
    AlreadyExists { id: EntityId },
    /// `Create` for an id that was tombstoned — retired ids are never reused.
    IdRetired { id: EntityId },
    /// An operation referenced an id that never existed.
    UnknownEntity { id: EntityId },
    /// A mutation targeted a tombstoned (logically destroyed) entity.
    Tombstoned { id: EntityId },
    /// A lifecycle transition is not legal from the current state
    /// (e.g. `Enable` an already-active entity).
    InvalidTransition {
        id: EntityId,
        from: EntityLifecycle,
        op: &'static str,
    },
    /// `AddLabel` for a label already present.
    LabelAlreadyPresent { id: EntityId, tag: TagId },
    /// `RemoveLabel` for a label that is not present.
    LabelAbsent { id: EntityId, tag: TagId },
}

impl EntityLifecycleError {
    /// Map to the shared foundation category for uniform tooling.
    pub fn category(&self) -> ErrorCategory {
        match self {
            EntityLifecycleError::AlreadyExists { .. }
            | EntityLifecycleError::IdRetired { .. }
            | EntityLifecycleError::Tombstoned { .. } => ErrorCategory::Conflict,
            EntityLifecycleError::UnknownEntity { .. }
            | EntityLifecycleError::LabelAbsent { .. } => ErrorCategory::NotFound,
            EntityLifecycleError::InvalidTransition { .. }
            | EntityLifecycleError::LabelAlreadyPresent { .. } => ErrorCategory::Invalid,
        }
    }
}

impl core::fmt::Display for EntityLifecycleError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            EntityLifecycleError::AlreadyExists { id } => {
                write!(f, "entity {} already exists", id.raw())
            }
            EntityLifecycleError::IdRetired { id } => {
                write!(f, "entity {} id is retired (tombstoned)", id.raw())
            }
            EntityLifecycleError::UnknownEntity { id } => {
                write!(f, "unknown entity {}", id.raw())
            }
            EntityLifecycleError::Tombstoned { id } => {
                write!(f, "entity {} is tombstoned", id.raw())
            }
            EntityLifecycleError::InvalidTransition { id, from, op } => {
                write!(f, "entity {} cannot {} from {}", id.raw(), op, from.label())
            }
            EntityLifecycleError::LabelAlreadyPresent { id, tag } => {
                write!(f, "entity {} already has label {}", id.raw(), tag.raw())
            }
            EntityLifecycleError::LabelAbsent { id, tag } => {
                write!(f, "entity {} has no label {}", id.raw(), tag.raw())
            }
        }
    }
}

impl std::error::Error for EntityLifecycleError {}
