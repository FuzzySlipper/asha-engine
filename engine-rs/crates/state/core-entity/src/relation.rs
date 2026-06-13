//! Scoped relation taxonomy and deterministic transform propagation (#2389).
//!
//! The design gate (§5) insists `attachment` is **not one overloaded relation**.
//! There are five distinct relation kinds, each with its own rules:
//!
//! 1. **Spatial transform attachment** (`transform_parent`) — child world transform
//!    derives from parent; cycle-checked; both ends need a transform. *Only this
//!    relation propagates transforms.*
//! 2. **Logical containment** (`contained_in`) — membership; no transform
//!    propagation; cycle-checked; neither end need be spatial.
//! 3. **Source ancestry** (`derived_from`) — read-only provenance trace; not a graph
//!    to walk; not destroyed by detach.
//! 4. **Controller/policy association** (`controlled_by`) — many→one; not a cycle
//!    domain (see [`crate::capability::ControllerCapability`]).
//! 5. **Render/projection grouping** — projection-only, never authority truth; this
//!    crate refuses to store it (it would be a render handle masquerading as durable
//!    truth) and returns a deferred diagnostic instead.
//!
//! Cycle checks apply only to (1) and (2). Transform propagation applies only to (1).

use core_ids::EntityId;
use core_math::Vec3;

use crate::core::EntityLifecycle;
use crate::store::EntityStore;
use crate::value::EntityTransform;

/// A proposed relation change. Only the accepted authority relations have verbs;
/// render grouping is deliberately routed to a deferred diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationCommand {
    /// Relation 1: make `child`'s transform derive from `parent`'s.
    AttachTransformParent { child: EntityId, parent: EntityId },
    /// Relation 1: detach `child`, re-rooting it to world space.
    DetachTransformParent { child: EntityId },
    /// Relation 2: place `member` inside `container`.
    SetContainment {
        member: EntityId,
        container: EntityId,
    },
    /// Relation 2: remove `member` from its container.
    ClearContainment { member: EntityId },
    /// Relation 3: record that `derived` originated from `origin` (read-only trace).
    SetDerivedFrom { derived: EntityId, origin: EntityId },
    /// Relation 5: projection grouping — refused (projection-only, not authority).
    SetRenderGroup { member: EntityId },
}

/// The accepted relation kinds (for diagnostics / mismatch reporting).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationKind {
    TransformParent,
    Containment,
    SourceAncestry,
    Controller,
    RenderGrouping,
}

impl RelationKind {
    pub fn label(self) -> &'static str {
        match self {
            RelationKind::TransformParent => "transformParent",
            RelationKind::Containment => "containment",
            RelationKind::SourceAncestry => "sourceAncestry",
            RelationKind::Controller => "controller",
            RelationKind::RenderGrouping => "renderGrouping",
        }
    }
}

/// Why a relation command was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationError {
    /// An endpoint does not exist.
    UnknownEntity { id: EntityId },
    /// An endpoint is tombstoned.
    Tombstoned { id: EntityId },
    /// The relation would create a cycle (transform attachment or containment only).
    Cycle { kind: RelationKind, at: EntityId },
    /// A transform attachment endpoint lacks a transform capability.
    NotTransformEligible { id: EntityId },
    /// An entity cannot be its own parent/container.
    SelfRelation { kind: RelationKind, id: EntityId },
    /// Detach/clear of a relation that does not exist.
    NoSuchRelation { kind: RelationKind, id: EntityId },
    /// The relation kind is not stored as authority truth (render grouping) and is
    /// deferred to the projection layer.
    ProjectionOnly { kind: RelationKind },
}

impl EntityStore {
    /// Validate and apply a relation command. Fail-closed and atomic.
    pub fn apply_relation(&mut self, command: RelationCommand) -> Result<(), RelationError> {
        match command {
            RelationCommand::AttachTransformParent { child, parent } => {
                self.attach_transform_parent(child, parent)
            }
            RelationCommand::DetachTransformParent { child } => self.detach_transform_parent(child),
            RelationCommand::SetContainment { member, container } => {
                self.set_containment_checked(member, container)
            }
            RelationCommand::ClearContainment { member } => self.clear_containment(member),
            RelationCommand::SetDerivedFrom { derived, origin } => {
                self.set_derived_from(derived, origin)
            }
            RelationCommand::SetRenderGroup { .. } => Err(RelationError::ProjectionOnly {
                kind: RelationKind::RenderGrouping,
            }),
        }
    }

    fn attach_transform_parent(
        &mut self,
        child: EntityId,
        parent: EntityId,
    ) -> Result<(), RelationError> {
        if child == parent {
            return Err(RelationError::SelfRelation {
                kind: RelationKind::TransformParent,
                id: child,
            });
        }
        self.require_alive(child)?;
        self.require_alive(parent)?;
        // Both ends must be transform-eligible (relation 1 is spatial only).
        if self.transform(child).is_none() {
            return Err(RelationError::NotTransformEligible { id: child });
        }
        if self.transform(parent).is_none() {
            return Err(RelationError::NotTransformEligible { id: parent });
        }
        // Cycle check: walking parent's ancestry must not reach `child`.
        if self.transform_ancestor_reaches(parent, child) {
            return Err(RelationError::Cycle {
                kind: RelationKind::TransformParent,
                at: child,
            });
        }
        self.set_transform_parent(child, parent);
        Ok(())
    }

    fn detach_transform_parent(&mut self, child: EntityId) -> Result<(), RelationError> {
        if self.transform_parent_of(child).is_none() {
            return Err(RelationError::NoSuchRelation {
                kind: RelationKind::TransformParent,
                id: child,
            });
        }
        self.remove_transform_parent(child);
        Ok(())
    }

    fn set_containment_checked(
        &mut self,
        member: EntityId,
        container: EntityId,
    ) -> Result<(), RelationError> {
        if member == container {
            return Err(RelationError::SelfRelation {
                kind: RelationKind::Containment,
                id: member,
            });
        }
        self.require_alive(member)?;
        self.require_alive(container)?;
        // Cycle check: container must not be contained (transitively) by member.
        if self.containment_ancestor_reaches(container, member) {
            return Err(RelationError::Cycle {
                kind: RelationKind::Containment,
                at: member,
            });
        }
        // Containment does NOT require either end be spatial (design §5).
        let attached = self.attach_containment(member, container);
        debug_assert!(attached);
        Ok(())
    }

    fn clear_containment(&mut self, member: EntityId) -> Result<(), RelationError> {
        if self.containment(member).is_none() {
            return Err(RelationError::NoSuchRelation {
                kind: RelationKind::Containment,
                id: member,
            });
        }
        self.remove_containment(member);
        Ok(())
    }

    fn set_derived_from(
        &mut self,
        derived: EntityId,
        origin: EntityId,
    ) -> Result<(), RelationError> {
        // Source ancestry is a read-only trace: the origin may even be tombstoned
        // (a dangling provenance pointer is allowed). The derived end must exist.
        self.require_known(derived)?;
        self.require_known(origin)?;
        self.set_derived_from_raw(derived, origin);
        Ok(())
    }

    // ── World transform propagation (relation 1 only) ─────────────────────────

    /// The world transform of `id`, composing the `transform_parent` chain. For an
    /// unattached entity this is just its local transform. Translation composes
    /// additively and scale multiplicatively; rotation composition is deferred
    /// (identity-rotation fixtures today), matching the scene transform posture.
    pub fn world_transform(&self, id: EntityId) -> Option<EntityTransform> {
        let local = self.transform(id)?.transform;
        match self.transform_parent_of(id) {
            None => Some(local),
            Some(parent) => {
                let parent_world = self.world_transform(parent)?;
                Some(compose(parent_world, local))
            }
        }
    }

    fn transform_ancestor_reaches(&self, start: EntityId, target: EntityId) -> bool {
        let mut cursor = Some(start);
        while let Some(node) = cursor {
            if node == target {
                return true;
            }
            cursor = self.transform_parent_of(node);
        }
        false
    }

    fn containment_ancestor_reaches(&self, start: EntityId, target: EntityId) -> bool {
        let mut cursor = Some(start);
        while let Some(node) = cursor {
            if node == target {
                return true;
            }
            cursor = self.containment(node).map(|c| c.container);
        }
        false
    }

    fn require_alive(&self, id: EntityId) -> Result<(), RelationError> {
        match self.core(id).map(|c| c.lifecycle) {
            None => Err(RelationError::UnknownEntity { id }),
            Some(EntityLifecycle::Tombstoned) => Err(RelationError::Tombstoned { id }),
            Some(_) => Ok(()),
        }
    }

    fn require_known(&self, id: EntityId) -> Result<(), RelationError> {
        if self.contains(id) {
            Ok(())
        } else {
            Err(RelationError::UnknownEntity { id })
        }
    }
}

/// Compose a parent's world transform with a child's local transform: translation
/// is offset by the parent's (scaled), scale multiplies. Rotation composition is
/// deferred (today's fixtures use identity rotation), matching the scene posture.
fn compose(parent: EntityTransform, local: EntityTransform) -> EntityTransform {
    EntityTransform {
        translation: Vec3::new(
            parent.translation.x + local.translation.x * parent.scale.x,
            parent.translation.y + local.translation.y * parent.scale.y,
            parent.translation.z + local.translation.z * parent.scale.z,
        ),
        rotation: local.rotation,
        scale: Vec3::new(
            parent.scale.x * local.scale.x,
            parent.scale.y * local.scale.y,
            parent.scale.z * local.scale.z,
        ),
    }
}
