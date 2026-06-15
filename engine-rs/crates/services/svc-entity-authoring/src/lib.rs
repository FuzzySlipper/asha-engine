//! Validate and apply proposed generic **entity authoring** commands
//! (post-launchable-03, Den task #2485).
//!
//! # Lane
//!
//! `rust-service`. A UI/devtools authoring surface only *proposes*: it builds a
//! [`protocol_entity_authoring::EntityAuthoringCommand`] and hands it here.
//! Authority validates each proposal against the live [`EntityStore`] and either
//! applies it (returning an accepted event) or refuses it (returning a classified
//! rejection). Validation reuses `core-entity`'s atomic, fail-closed authority
//! operations — a rejected command mutates nothing — so an authoring surface can
//! never bypass lifecycle/transform/relation/movement rules or corrupt state, and
//! never holds a second copy of authority.
//!
//! This mirrors `svc-policy-view`'s validate/apply role, but for the fuller
//! operator/agent authoring surface (create/destroy/attach/relate/move) rather
//! than the narrow sandboxed policy set.

#![forbid(unsafe_code)]

use core_assets::{AssetId, AssetReference, AssetVersionReq};
use core_entity::{
    Aabb, EntityLifecycleCommand, EntityLifecycleError, EntitySource, EntityStore, EntityTransform,
    MovementCommand, MovementError, Quat, RelationCommand, RelationError, TransformCommand,
    TransformError,
};
use core_math::Vec3;
use protocol_entity_authoring::{
    AuthoringCapability, AuthoringEventKind, AuthoringRejectionReason, AuthoringSource,
    AuthoringTransform, EntityAuthoringCommand, EntityAuthoringEvent, EntityAuthoringOutcome,
    EntityAuthoringRejection,
};

// ── Border ⇄ core value mapping ───────────────────────────────────────────────

fn to_entity_transform(t: &AuthoringTransform) -> EntityTransform {
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

fn to_entity_source(source: &AuthoringSource) -> Result<EntitySource, AuthoringRejectionReason> {
    Ok(match source {
        AuthoringSource::SceneBootstrap { node } => EntitySource::SceneBootstrap { node: *node },
        AuthoringSource::RuntimeCreated { by } => EntitySource::RuntimeCreated { by: *by },
        AuthoringSource::Imported { asset } => {
            let id = AssetId::parse(asset).map_err(|_| AuthoringRejectionReason::InvalidAsset)?;
            EntitySource::Imported {
                asset: AssetReference::new(id, AssetVersionReq::Any, None),
            }
        }
        AuthoringSource::DiagnosticTooling => EntitySource::DiagnosticTooling,
        AuthoringSource::PolicyProposed { by } => EntitySource::PolicyProposed { by: *by },
    })
}

// ── Error mapping ─────────────────────────────────────────────────────────────

fn map_lifecycle(err: EntityLifecycleError) -> AuthoringRejectionReason {
    match err {
        EntityLifecycleError::AlreadyExists { .. } => AuthoringRejectionReason::AlreadyExists,
        EntityLifecycleError::IdRetired { .. } => AuthoringRejectionReason::IdRetired,
        EntityLifecycleError::UnknownEntity { .. } => AuthoringRejectionReason::UnknownEntity,
        EntityLifecycleError::Tombstoned { .. } => AuthoringRejectionReason::Tombstoned,
        EntityLifecycleError::InvalidTransition { .. } => {
            AuthoringRejectionReason::InvalidTransition
        }
        EntityLifecycleError::LabelAlreadyPresent { .. } => {
            AuthoringRejectionReason::LabelAlreadyPresent
        }
        EntityLifecycleError::LabelAbsent { .. } => AuthoringRejectionReason::LabelAbsent,
    }
}

fn map_transform(err: TransformError) -> AuthoringRejectionReason {
    match err {
        TransformError::UnknownEntity { .. } => AuthoringRejectionReason::UnknownEntity,
        TransformError::Tombstoned { .. } => AuthoringRejectionReason::Tombstoned,
        TransformError::Disabled { .. } => AuthoringRejectionReason::InvalidTransition,
        TransformError::NotTransformEligible { .. } => {
            AuthoringRejectionReason::NotTransformEligible
        }
        TransformError::Immovable { .. } => AuthoringRejectionReason::Immovable,
        TransformError::NonFinite { .. } => AuthoringRejectionReason::NonFinite,
    }
}

fn map_movement(err: MovementError) -> AuthoringRejectionReason {
    match err {
        MovementError::UnknownEntity { .. } => AuthoringRejectionReason::UnknownEntity,
        MovementError::Tombstoned { .. } => AuthoringRejectionReason::Tombstoned,
        MovementError::Disabled { .. } => AuthoringRejectionReason::InvalidTransition,
        MovementError::NotSpatial { .. } => AuthoringRejectionReason::NotSpatial,
        MovementError::NoCollider { .. } => AuthoringRejectionReason::NoCollider,
        MovementError::Immovable { .. } => AuthoringRejectionReason::Immovable,
        MovementError::NonFinite { .. } => AuthoringRejectionReason::NonFinite,
    }
}

fn map_relation(err: RelationError) -> AuthoringRejectionReason {
    match err {
        RelationError::UnknownEntity { .. } => AuthoringRejectionReason::UnknownEntity,
        RelationError::Tombstoned { .. } => AuthoringRejectionReason::Tombstoned,
        RelationError::Cycle { .. } => AuthoringRejectionReason::RelationCycle,
        RelationError::NotTransformEligible { .. } => {
            AuthoringRejectionReason::EndpointNotTransformEligible
        }
        RelationError::SelfRelation { .. } => AuthoringRejectionReason::SelfRelation,
        RelationError::NoSuchRelation { .. } => AuthoringRejectionReason::NoSuchRelation,
        RelationError::ProjectionOnly { .. } => AuthoringRejectionReason::ProjectionOnly,
    }
}

// ── Outcome helpers ───────────────────────────────────────────────────────────

fn accepted(kind: AuthoringEventKind, entity: core_ids::EntityId) -> EntityAuthoringOutcome {
    EntityAuthoringOutcome::Accepted {
        event: EntityAuthoringEvent { kind, entity },
    }
}

fn rejected(
    reason: AuthoringRejectionReason,
    entity: core_ids::EntityId,
) -> EntityAuthoringOutcome {
    EntityAuthoringOutcome::Rejected {
        rejection: EntityAuthoringRejection { reason, entity },
    }
}

// ── Validate + apply ──────────────────────────────────────────────────────────

/// Validate a single proposed authoring command against `store` and, if accepted,
/// apply it. On rejection the store is left untouched (the underlying authority
/// operations are atomic and fail-closed). Returns the classified outcome either
/// way.
pub fn validate_and_apply(
    store: &mut EntityStore,
    command: &EntityAuthoringCommand,
) -> EntityAuthoringOutcome {
    use AuthoringEventKind as E;
    match command {
        EntityAuthoringCommand::Create { id, source, labels } => {
            let source = match to_entity_source(source) {
                Ok(s) => s,
                Err(reason) => return rejected(reason, *id),
            };
            match store.apply(EntityLifecycleCommand::Create {
                id: *id,
                source,
                labels: labels.clone(),
            }) {
                Ok(_) => accepted(E::Created, *id),
                Err(e) => rejected(map_lifecycle(e), *id),
            }
        }
        EntityAuthoringCommand::Destroy { id } => lifecycle(
            store,
            EntityLifecycleCommand::Destroy { id: *id },
            E::Destroyed,
            *id,
        ),
        EntityAuthoringCommand::Disable { id } => lifecycle(
            store,
            EntityLifecycleCommand::Disable { id: *id },
            E::Disabled,
            *id,
        ),
        EntityAuthoringCommand::Enable { id } => lifecycle(
            store,
            EntityLifecycleCommand::Enable { id: *id },
            E::Enabled,
            *id,
        ),
        EntityAuthoringCommand::AddLabel { id, tag } => lifecycle(
            store,
            EntityLifecycleCommand::AddLabel { id: *id, tag: *tag },
            E::LabelAdded,
            *id,
        ),
        EntityAuthoringCommand::RemoveLabel { id, tag } => lifecycle(
            store,
            EntityLifecycleCommand::RemoveLabel { id: *id, tag: *tag },
            E::LabelRemoved,
            *id,
        ),
        EntityAuthoringCommand::AttachCapability { id, capability } => {
            attach_capability(store, *id, capability)
        }
        EntityAuthoringCommand::SetTransform { id, transform } => {
            let cmd = TransformCommand::Set {
                id: *id,
                transform: to_entity_transform(transform),
            };
            match store.apply_transform(cmd) {
                Ok(_) => accepted(E::TransformSet, *id),
                Err(e) => rejected(map_transform(e), *id),
            }
        }
        EntityAuthoringCommand::Move { id, delta } => {
            let cmd = MovementCommand {
                id: *id,
                delta: Vec3::new(delta[0], delta[1], delta[2]),
            };
            match store.apply_movement(cmd) {
                Ok(_) => accepted(E::Moved, *id),
                Err(e) => rejected(map_movement(e), *id),
            }
        }
        EntityAuthoringCommand::AttachTransformParent { child, parent } => relation(
            store,
            RelationCommand::AttachTransformParent {
                child: *child,
                parent: *parent,
            },
            E::RelationSet,
            *child,
        ),
        EntityAuthoringCommand::DetachTransformParent { child } => relation(
            store,
            RelationCommand::DetachTransformParent { child: *child },
            E::RelationCleared,
            *child,
        ),
        EntityAuthoringCommand::SetContainment { member, container } => relation(
            store,
            RelationCommand::SetContainment {
                member: *member,
                container: *container,
            },
            E::RelationSet,
            *member,
        ),
        EntityAuthoringCommand::ClearContainment { member } => relation(
            store,
            RelationCommand::ClearContainment { member: *member },
            E::RelationCleared,
            *member,
        ),
        EntityAuthoringCommand::SetDerivedFrom { derived, origin } => relation(
            store,
            RelationCommand::SetDerivedFrom {
                derived: *derived,
                origin: *origin,
            },
            E::RelationSet,
            *derived,
        ),
    }
}

fn lifecycle(
    store: &mut EntityStore,
    cmd: EntityLifecycleCommand,
    on_ok: AuthoringEventKind,
    id: core_ids::EntityId,
) -> EntityAuthoringOutcome {
    match store.apply(cmd) {
        Ok(_) => accepted(on_ok, id),
        Err(e) => rejected(map_lifecycle(e), id),
    }
}

fn relation(
    store: &mut EntityStore,
    cmd: RelationCommand,
    on_ok: AuthoringEventKind,
    id: core_ids::EntityId,
) -> EntityAuthoringOutcome {
    match store.apply_relation(cmd) {
        Ok(()) => accepted(on_ok, id),
        Err(e) => rejected(map_relation(e), id),
    }
}

/// Capability attach is a no-op on a dead/unknown entity; classify those rather
/// than silently dropping the proposal.
fn attach_capability(
    store: &mut EntityStore,
    id: core_ids::EntityId,
    capability: &AuthoringCapability,
) -> EntityAuthoringOutcome {
    match store.lifecycle(id) {
        None => return rejected(AuthoringRejectionReason::UnknownEntity, id),
        Some(core_entity::EntityLifecycle::Tombstoned) => {
            return rejected(AuthoringRejectionReason::Tombstoned, id)
        }
        Some(_) => {}
    }
    let attached = match capability {
        AuthoringCapability::Transform { transform } => {
            store.attach_transform(id, to_entity_transform(transform))
        }
        AuthoringCapability::Render { visible } => store.attach_render_projection(id, *visible),
        AuthoringCapability::Collision { static_collider } => {
            store.attach_collision(id, *static_collider)
        }
        AuthoringCapability::Bounds { min, max } => store.attach_bounds(
            id,
            Aabb::new(
                Vec3::new(min[0], min[1], min[2]),
                Vec3::new(max[0], max[1], max[2]),
            ),
        ),
    };
    if attached {
        accepted(AuthoringEventKind::CapabilityAttached, id)
    } else {
        // Lifecycle check above already excluded unknown/tombstoned; a false here
        // means disabled (attach is alive-only).
        rejected(AuthoringRejectionReason::EntityNotAlive, id)
    }
}

// ── Eligibility preview (capability discipline, no mutation) ───────────────────

/// Whether a transform/movement-style command would be accepted for `id`, without
/// applying anything — for a UI to disable an ineligible control and explain why.
pub fn transform_eligible(
    store: &EntityStore,
    id: core_ids::EntityId,
) -> Result<(), AuthoringRejectionReason> {
    store.transform_eligible(id).map_err(map_transform)
}

/// Whether a kinematic move would be accepted for `id`, without applying it.
pub fn movement_eligible(
    store: &EntityStore,
    id: core_ids::EntityId,
) -> Result<(), AuthoringRejectionReason> {
    store.movement_eligible(id).map_err(map_movement)
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_ids::{EntityId, TagId};
    use protocol_entity_authoring::EntityAuthoringOutcome as O;

    fn ident() -> AuthoringTransform {
        AuthoringTransform {
            translation: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            scale: [1.0, 1.0, 1.0],
        }
    }

    fn create(store: &mut EntityStore, id: u64) -> EntityAuthoringOutcome {
        validate_and_apply(
            store,
            &EntityAuthoringCommand::Create {
                id: EntityId::new(id),
                source: AuthoringSource::RuntimeCreated { by: None },
                labels: vec![],
            },
        )
    }

    #[test]
    fn create_then_attach_then_transform_is_accepted() {
        let mut store = EntityStore::new();
        assert!(matches!(create(&mut store, 1), O::Accepted { .. }));
        assert!(matches!(
            validate_and_apply(
                &mut store,
                &EntityAuthoringCommand::AttachCapability {
                    id: EntityId::new(1),
                    capability: AuthoringCapability::Transform { transform: ident() },
                }
            ),
            O::Accepted { .. }
        ));
        let out = validate_and_apply(
            &mut store,
            &EntityAuthoringCommand::SetTransform {
                id: EntityId::new(1),
                transform: AuthoringTransform {
                    translation: [3.0, 0.0, 0.0],
                    ..ident()
                },
            },
        );
        assert!(matches!(out, O::Accepted { .. }));
    }

    #[test]
    fn transform_on_non_spatial_entity_is_classified_not_eligible() {
        let mut store = EntityStore::new();
        create(&mut store, 1); // no transform capability attached
        let out = validate_and_apply(
            &mut store,
            &EntityAuthoringCommand::SetTransform {
                id: EntityId::new(1),
                transform: ident(),
            },
        );
        assert_eq!(
            out,
            rejected(
                AuthoringRejectionReason::NotTransformEligible,
                EntityId::new(1)
            )
        );
    }

    #[test]
    fn rejected_command_mutates_nothing() {
        let mut store = EntityStore::new();
        create(&mut store, 1);
        let hash_before = store.hash();
        // SetTransform on a non-spatial entity is rejected → no mutation.
        let out = validate_and_apply(
            &mut store,
            &EntityAuthoringCommand::SetTransform {
                id: EntityId::new(1),
                transform: ident(),
            },
        );
        assert!(matches!(out, O::Rejected { .. }));
        assert_eq!(
            store.hash(),
            hash_before,
            "a rejected command must not mutate authority"
        );
    }

    #[test]
    fn duplicate_create_is_classified_already_exists() {
        let mut store = EntityStore::new();
        create(&mut store, 1);
        let out = create(&mut store, 1);
        assert_eq!(
            out,
            rejected(AuthoringRejectionReason::AlreadyExists, EntityId::new(1))
        );
    }

    #[test]
    fn containment_and_source_relations_are_accepted_and_distinct() {
        let mut store = EntityStore::new();
        create(&mut store, 1);
        create(&mut store, 2);
        let contain = validate_and_apply(
            &mut store,
            &EntityAuthoringCommand::SetContainment {
                member: EntityId::new(1),
                container: EntityId::new(2),
            },
        );
        assert!(matches!(contain, O::Accepted { .. }));
        assert_eq!(
            store.containment(EntityId::new(1)).map(|c| c.container),
            Some(EntityId::new(2))
        );
        let derive = validate_and_apply(
            &mut store,
            &EntityAuthoringCommand::SetDerivedFrom {
                derived: EntityId::new(1),
                origin: EntityId::new(2),
            },
        );
        assert!(matches!(derive, O::Accepted { .. }));
        // Distinct relation taxonomy: containment is not source ancestry.
        assert_eq!(store.derived_from(EntityId::new(1)), Some(EntityId::new(2)));
    }

    #[test]
    fn self_containment_is_classified_self_relation() {
        let mut store = EntityStore::new();
        create(&mut store, 1);
        let out = validate_and_apply(
            &mut store,
            &EntityAuthoringCommand::SetContainment {
                member: EntityId::new(1),
                container: EntityId::new(1),
            },
        );
        assert_eq!(
            out,
            rejected(AuthoringRejectionReason::SelfRelation, EntityId::new(1))
        );
    }

    #[test]
    fn add_label_round_trips_through_authority() {
        let mut store = EntityStore::new();
        create(&mut store, 1);
        let out = validate_and_apply(
            &mut store,
            &EntityAuthoringCommand::AddLabel {
                id: EntityId::new(1),
                tag: TagId::new(7),
            },
        );
        assert!(matches!(out, O::Accepted { .. }));
        assert!(store
            .core(EntityId::new(1))
            .unwrap()
            .has_label(TagId::new(7)));
    }

    #[test]
    fn eligibility_preview_does_not_mutate() {
        let mut store = EntityStore::new();
        create(&mut store, 1);
        let before = store.hash();
        assert_eq!(
            transform_eligible(&store, EntityId::new(1)),
            Err(AuthoringRejectionReason::NotTransformEligible)
        );
        assert_eq!(store.hash(), before);
    }
}
