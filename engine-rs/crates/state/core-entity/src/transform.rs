//! Authoritative runtime transform commands, events, and validation (#2388).
//!
//! Transform is an **optional capability**, not a core field. These commands only
//! succeed for *transform-eligible* entities — alive, active, holding a
//! [`TransformCapability`], and not immovable/static. Every other case (non-spatial
//! logical, contained-only, tombstoned, disabled, static/immovable, non-finite
//! input) is rejected with a classified [`TransformError`] and **no mutation**.
//!
//! An accepted transform emits a [`TransformEvent`] whose `projection_changed`
//! flag is true only when the entity has a *visible* render projection — so a
//! renderer/source-trace projection update is emitted only when applicable.

use core_error::ErrorCategory;
use core_ids::EntityId;
use core_math::Vec3;

use crate::core::EntityLifecycle;
use crate::store::EntityStore;
use crate::value::EntityTransform;

/// A proposed runtime transform change for a transform-eligible entity.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TransformCommand {
    /// Overwrite the entity's runtime transform.
    Set {
        id: EntityId,
        transform: EntityTransform,
    },
    /// Translate the entity's runtime transform by a world-space delta.
    Translate { id: EntityId, delta: Vec3 },
}

impl TransformCommand {
    pub fn entity(&self) -> EntityId {
        match self {
            TransformCommand::Set { id, .. } | TransformCommand::Translate { id, .. } => *id,
        }
    }
}

/// The authoritative record of an accepted transform change.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransformEvent {
    pub id: EntityId,
    /// The resulting runtime transform.
    pub transform: EntityTransform,
    /// Whether this change affects a (visible) render projection — i.e. a renderer
    /// projection update should be emitted. False for non-rendered spatial entities.
    pub projection_changed: bool,
}

/// Why a transform command was rejected. Classified so an agent can route without
/// parsing prose.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransformError {
    /// The entity does not exist.
    UnknownEntity { id: EntityId },
    /// The entity is tombstoned (logically destroyed).
    Tombstoned { id: EntityId },
    /// The entity is disabled (excluded from runtime transform/movement).
    Disabled { id: EntityId },
    /// The entity has no [`TransformCapability`] — it is non-spatial (logical,
    /// contained-only, projection-only). Transform is not applicable.
    NotTransformEligible { id: EntityId },
    /// The entity is spatial but immovable (has a static collider); its transform
    /// is fixed and may not be moved.
    Immovable { id: EntityId },
    /// The resulting transform contained a non-finite (NaN/∞) value.
    NonFinite { id: EntityId },
}

impl TransformError {
    pub fn entity(&self) -> EntityId {
        match self {
            TransformError::UnknownEntity { id }
            | TransformError::Tombstoned { id }
            | TransformError::Disabled { id }
            | TransformError::NotTransformEligible { id }
            | TransformError::Immovable { id }
            | TransformError::NonFinite { id } => *id,
        }
    }

    pub fn category(&self) -> ErrorCategory {
        match self {
            TransformError::UnknownEntity { .. } => ErrorCategory::NotFound,
            TransformError::Tombstoned { .. }
            | TransformError::Disabled { .. }
            | TransformError::Immovable { .. } => ErrorCategory::Conflict,
            TransformError::NotTransformEligible { .. } | TransformError::NonFinite { .. } => {
                ErrorCategory::Invalid
            }
        }
    }

    /// Stable label for diagnostics.
    pub fn label(&self) -> &'static str {
        match self {
            TransformError::UnknownEntity { .. } => "unknownEntity",
            TransformError::Tombstoned { .. } => "tombstoned",
            TransformError::Disabled { .. } => "disabled",
            TransformError::NotTransformEligible { .. } => "notTransformEligible",
            TransformError::Immovable { .. } => "immovable",
            TransformError::NonFinite { .. } => "nonFinite",
        }
    }
}

impl EntityStore {
    /// Validate and apply a transform command. On success the entity's
    /// [`TransformCapability`] is updated and a [`TransformEvent`] returned; on
    /// failure nothing is mutated and a classified [`TransformError`] is returned.
    pub fn apply_transform(
        &mut self,
        command: TransformCommand,
    ) -> Result<TransformEvent, TransformError> {
        let id = command.entity();
        self.check_transform_eligible(id)?;

        let current = self
            .transform(id)
            .expect("eligibility check guarantees a transform capability")
            .transform;

        let next = match command {
            TransformCommand::Set { transform, .. } => transform,
            TransformCommand::Translate { delta, .. } => EntityTransform {
                translation: Vec3::new(
                    current.translation.x + delta.x,
                    current.translation.y + delta.y,
                    current.translation.z + delta.z,
                ),
                ..current
            },
        };

        if !transform_is_finite(&next) {
            return Err(TransformError::NonFinite { id });
        }

        let applied = self.attach_transform(id, next);
        debug_assert!(applied, "eligibility was already checked");

        Ok(TransformEvent {
            id,
            transform: next,
            projection_changed: self.is_visible_projection(id),
        })
    }

    /// Whether a transform command would be accepted for `id` (without applying it).
    pub fn transform_eligible(&self, id: EntityId) -> Result<(), TransformError> {
        self.check_transform_eligible(id)
    }

    fn check_transform_eligible(&self, id: EntityId) -> Result<(), TransformError> {
        let core = match self.core(id) {
            None => return Err(TransformError::UnknownEntity { id }),
            Some(core) => core,
        };
        match core.lifecycle {
            EntityLifecycle::Tombstoned => return Err(TransformError::Tombstoned { id }),
            EntityLifecycle::Disabled => return Err(TransformError::Disabled { id }),
            EntityLifecycle::Active => {}
        }
        if self.transform(id).is_none() {
            return Err(TransformError::NotTransformEligible { id });
        }
        if self
            .collision(id)
            .map(|c| c.static_collider)
            .unwrap_or(false)
        {
            return Err(TransformError::Immovable { id });
        }
        Ok(())
    }

    /// Whether `id` has a render projection capability that is currently visible.
    fn is_visible_projection(&self, id: EntityId) -> bool {
        self.render_projection(id)
            .map(|r| r.visible)
            .unwrap_or(false)
    }
}

fn transform_is_finite(t: &EntityTransform) -> bool {
    let vals = [
        t.translation.x,
        t.translation.y,
        t.translation.z,
        t.rotation.x,
        t.rotation.y,
        t.rotation.z,
        t.rotation.w,
        t.scale.x,
        t.scale.y,
        t.scale.z,
    ];
    vals.iter().all(|v| v.is_finite())
}
