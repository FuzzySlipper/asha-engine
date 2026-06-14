//! Validate and apply proposed policy world commands (#2392).
//!
//! A policy only *proposes*. Authority validates each proposal against the live
//! entity store and either applies it (returning an accepted event) or refuses it
//! (returning a classified rejection). Validation reuses `core-entity`'s atomic,
//! fail-closed authority operations — a rejected command mutates nothing — so a
//! policy can never bypass lifecycle/transform/asset rules or corrupt state.

use core_entity::{
    EntityLifecycleCommand, EntityLifecycleError, EntityStore, EntityTransform, Quat,
    TransformCommand, TransformError,
};
use core_math::Vec3;
use protocol_policy_view::{
    PolicyTransform, PolicyWorldCommand, PolicyWorldEvent, PolicyWorldOutcome, PolicyWorldRejection,
};

fn to_entity_transform(t: &PolicyTransform) -> EntityTransform {
    EntityTransform {
        translation: Vec3::new(t.translation[0], t.translation[1], t.translation[2]),
        rotation: Quat {
            x: t.rotation[0],
            y: t.rotation[1],
            z: t.rotation[2],
            w: t.rotation[3],
        },
        scale: Vec3::new(t.scale[0], t.scale[1], t.scale[2]),
    }
}

/// `true` when no scale axis is zero — a zero-scale transform is degenerate and
/// rejected as invalid before it ever reaches the store.
fn scale_is_nonzero(t: &PolicyTransform) -> bool {
    t.scale.iter().all(|s| *s != 0.0)
}

fn map_transform_error(err: TransformError) -> PolicyWorldRejection {
    match err {
        TransformError::UnknownEntity { .. } | TransformError::Tombstoned { .. } => {
            PolicyWorldRejection::UnknownEntity
        }
        TransformError::Disabled { .. } => PolicyWorldRejection::EntityDisabled,
        TransformError::NotTransformEligible { .. } => PolicyWorldRejection::NotSpatial,
        TransformError::Immovable { .. } => PolicyWorldRejection::Immovable,
        TransformError::NonFinite { .. } => PolicyWorldRejection::InvalidTransform,
    }
}

fn map_lifecycle_error(err: EntityLifecycleError) -> PolicyWorldRejection {
    match err {
        EntityLifecycleError::UnknownEntity { .. }
        | EntityLifecycleError::Tombstoned { .. }
        | EntityLifecycleError::IdRetired { .. } => PolicyWorldRejection::UnknownEntity,
        EntityLifecycleError::LabelAlreadyPresent { .. } => {
            PolicyWorldRejection::LabelAlreadyPresent
        }
        // The only lifecycle ops a policy issues are Disable/AddLabel; an illegal
        // transition here means Disable on an already-disabled entity.
        EntityLifecycleError::InvalidTransition { .. }
        | EntityLifecycleError::AlreadyExists { .. } => PolicyWorldRejection::AlreadyDisabled,
        EntityLifecycleError::LabelAbsent { .. } => PolicyWorldRejection::UnknownEntity,
    }
}

fn accepted(event: PolicyWorldEvent) -> PolicyWorldOutcome {
    PolicyWorldOutcome::Accepted { event }
}

fn rejected(rejection: PolicyWorldRejection) -> PolicyWorldOutcome {
    PolicyWorldOutcome::Rejected { rejection }
}

/// Validate a single proposed command against `store` and, if accepted, apply it.
/// On rejection the store is left untouched (the underlying authority operations
/// are atomic and fail-closed). Returns the classified outcome either way.
pub fn validate_and_apply(
    store: &mut EntityStore,
    command: &PolicyWorldCommand,
) -> PolicyWorldOutcome {
    match command {
        PolicyWorldCommand::RequestSetTransform { entity, transform } => {
            // Reject a degenerate transform before touching authority so the reason
            // is the policy-facing `InvalidTransform`, not a deep store assert.
            if !scale_is_nonzero(transform) {
                return rejected(PolicyWorldRejection::InvalidTransform);
            }
            let cmd = TransformCommand::Set {
                id: *entity,
                transform: to_entity_transform(transform),
            };
            match store.apply_transform(cmd) {
                Ok(_) => accepted(PolicyWorldEvent::TransformSet {
                    entity: *entity,
                    transform: *transform,
                }),
                Err(err) => rejected(map_transform_error(err)),
            }
        }
        PolicyWorldCommand::RequestAddLabel { entity, label } => {
            let cmd = EntityLifecycleCommand::AddLabel {
                id: *entity,
                tag: *label,
            };
            match store.apply(cmd) {
                Ok(_) => accepted(PolicyWorldEvent::LabelAdded {
                    entity: *entity,
                    label: *label,
                }),
                Err(err) => rejected(map_lifecycle_error(err)),
            }
        }
        PolicyWorldCommand::RequestDisable { entity } => {
            let cmd = EntityLifecycleCommand::Disable { id: *entity };
            match store.apply(cmd) {
                Ok(_) => accepted(PolicyWorldEvent::Disabled { entity: *entity }),
                Err(err) => rejected(map_lifecycle_error(err)),
            }
        }
        PolicyWorldCommand::NoopMarker { note } => {
            // A no-op marker changes no authority state; it is always accepted and
            // recorded for audit only.
            accepted(PolicyWorldEvent::NoopRecorded { note: note.clone() })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_entity::{EntityLifecycle, EntitySource};
    use core_ids::{EntityId, TagId};

    fn identity_transform() -> PolicyTransform {
        PolicyTransform {
            translation: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            scale: [1.0, 1.0, 1.0],
        }
    }

    fn spatial_entity(store: &mut EntityStore, id: u64) {
        store
            .apply(EntityLifecycleCommand::Create {
                id: EntityId::new(id),
                source: EntitySource::RuntimeCreated { by: None },
                labels: Vec::new(),
            })
            .unwrap();
        store.attach_transform(EntityId::new(id), EntityTransform::IDENTITY);
    }

    fn logical_entity(store: &mut EntityStore, id: u64) {
        store
            .apply(EntityLifecycleCommand::Create {
                id: EntityId::new(id),
                source: EntitySource::RuntimeCreated { by: None },
                labels: Vec::new(),
            })
            .unwrap();
    }

    #[test]
    fn accepts_and_applies_a_valid_transform() {
        let mut store = EntityStore::new();
        spatial_entity(&mut store, 1);
        let t = PolicyTransform {
            translation: [5.0, 0.0, 0.0],
            ..identity_transform()
        };
        let outcome = validate_and_apply(
            &mut store,
            &PolicyWorldCommand::RequestSetTransform {
                entity: EntityId::new(1),
                transform: t,
            },
        );
        assert!(outcome.is_accepted());
        // The store really changed.
        assert_eq!(
            store
                .transform(EntityId::new(1))
                .unwrap()
                .transform
                .translation,
            Vec3::new(5.0, 0.0, 0.0)
        );
    }

    #[test]
    fn rejects_transform_on_unknown_entity_without_mutation() {
        let mut store = EntityStore::new();
        let outcome = validate_and_apply(
            &mut store,
            &PolicyWorldCommand::RequestSetTransform {
                entity: EntityId::new(99),
                transform: identity_transform(),
            },
        );
        assert_eq!(
            outcome,
            PolicyWorldOutcome::Rejected {
                rejection: PolicyWorldRejection::UnknownEntity
            }
        );
    }

    #[test]
    fn rejects_transform_on_non_spatial_entity() {
        let mut store = EntityStore::new();
        logical_entity(&mut store, 1);
        let outcome = validate_and_apply(
            &mut store,
            &PolicyWorldCommand::RequestSetTransform {
                entity: EntityId::new(1),
                transform: identity_transform(),
            },
        );
        assert_eq!(
            outcome,
            PolicyWorldOutcome::Rejected {
                rejection: PolicyWorldRejection::NotSpatial
            }
        );
    }

    #[test]
    fn rejects_zero_scale_transform_as_invalid() {
        let mut store = EntityStore::new();
        spatial_entity(&mut store, 1);
        let bad = PolicyTransform {
            scale: [0.0, 1.0, 1.0],
            ..identity_transform()
        };
        let outcome = validate_and_apply(
            &mut store,
            &PolicyWorldCommand::RequestSetTransform {
                entity: EntityId::new(1),
                transform: bad,
            },
        );
        assert_eq!(
            outcome,
            PolicyWorldOutcome::Rejected {
                rejection: PolicyWorldRejection::InvalidTransform
            }
        );
    }

    #[test]
    fn add_label_accepts_then_rejects_duplicate() {
        let mut store = EntityStore::new();
        logical_entity(&mut store, 1);
        let cmd = PolicyWorldCommand::RequestAddLabel {
            entity: EntityId::new(1),
            label: TagId::new(7),
        };
        assert!(validate_and_apply(&mut store, &cmd).is_accepted());
        assert_eq!(
            validate_and_apply(&mut store, &cmd),
            PolicyWorldOutcome::Rejected {
                rejection: PolicyWorldRejection::LabelAlreadyPresent
            }
        );
    }

    #[test]
    fn disable_accepts_then_rejects_already_disabled() {
        let mut store = EntityStore::new();
        logical_entity(&mut store, 1);
        let cmd = PolicyWorldCommand::RequestDisable {
            entity: EntityId::new(1),
        };
        assert!(validate_and_apply(&mut store, &cmd).is_accepted());
        assert_eq!(
            store.lifecycle(EntityId::new(1)),
            Some(EntityLifecycle::Disabled)
        );
        assert_eq!(
            validate_and_apply(&mut store, &cmd),
            PolicyWorldOutcome::Rejected {
                rejection: PolicyWorldRejection::AlreadyDisabled
            }
        );
    }

    #[test]
    fn transform_on_disabled_entity_is_rejected() {
        let mut store = EntityStore::new();
        spatial_entity(&mut store, 1);
        store
            .apply(EntityLifecycleCommand::Disable {
                id: EntityId::new(1),
            })
            .unwrap();
        let outcome = validate_and_apply(
            &mut store,
            &PolicyWorldCommand::RequestSetTransform {
                entity: EntityId::new(1),
                transform: identity_transform(),
            },
        );
        assert_eq!(
            outcome,
            PolicyWorldOutcome::Rejected {
                rejection: PolicyWorldRejection::EntityDisabled
            }
        );
    }

    #[test]
    fn noop_marker_is_always_accepted_and_changes_nothing() {
        let mut store = EntityStore::new();
        let before = store.hash();
        let outcome = validate_and_apply(
            &mut store,
            &PolicyWorldCommand::NoopMarker {
                note: "audit".to_string(),
            },
        );
        assert!(outcome.is_accepted());
        assert_eq!(store.hash(), before);
    }
}
