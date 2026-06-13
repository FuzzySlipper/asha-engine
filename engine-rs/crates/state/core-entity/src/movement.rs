//! Deterministic kinematic movement over collision queries (#2390).
//!
//! Movement eligibility is **capability-based**, not implied by existence,
//! renderability, transform presence alone, or adjacency. An entity may move only
//! if it is alive+active, has a [`TransformCapability`] *and* a
//! [`CollisionCapability`], and is not static/immovable. Non-spatial, contained-
//! only, rendered-but-non-colliding, destroyed, disabled, and static entities are
//! rejected with classified [`MovementError`]s.
//!
//! The collision query reads only authority [`CollisionCapability`] +
//! [`BoundsCapability`] AABBs — **never render state** — so visual data cannot leak
//! into authority movement. Movement is axis-separated (move X, then Y, then Z),
//! yielding a deterministic `Moved` / `Slid` / `Blocked` outcome that the caller
//! applies as a transform update.
//!
//! This is the authority *substrate*; a production path may route the query through
//! `svc-collision`. Rapier dynamics are explicitly out of scope here.
//!
//! [`TransformCapability`]: crate::capability::TransformCapability
//! [`CollisionCapability`]: crate::capability::CollisionCapability
//! [`BoundsCapability`]: crate::capability::BoundsCapability

use core_error::ErrorCategory;
use core_ids::EntityId;
use core_math::Vec3;

use crate::core::EntityLifecycle;
use crate::store::EntityStore;
use crate::value::{Aabb, EntityTransform};

/// A proposed kinematic move by a world-space delta.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MovementCommand {
    pub id: EntityId,
    pub delta: Vec3,
}

/// The deterministic result of a movement query.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MovementOutcome {
    /// The full delta was applied.
    Moved { to: Vec3 },
    /// Part of the delta was applied; one or more axes were blocked (slide).
    Slid { to: Vec3, blocked: [bool; 3] },
    /// No movement was possible; the entity stays put.
    Blocked { at: Vec3 },
}

/// The authoritative record of an accepted movement.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MovementEvent {
    pub id: EntityId,
    pub from: Vec3,
    pub outcome: MovementOutcome,
    /// The first collider hit, for diagnostics (if any axis was blocked).
    pub hit: Option<EntityId>,
    /// Whether a (visible) render projection should be updated.
    pub projection_changed: bool,
}

/// Why a movement command was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MovementError {
    UnknownEntity {
        id: EntityId,
    },
    Tombstoned {
        id: EntityId,
    },
    Disabled {
        id: EntityId,
    },
    /// No transform capability — the entity is non-spatial/logical/contained-only.
    NotSpatial {
        id: EntityId,
    },
    /// No collision capability — movement requires a collision query participant
    /// (e.g. a rendered-but-non-colliding entity).
    NoCollider {
        id: EntityId,
    },
    /// The entity is spatial+colliding but static/immovable.
    Immovable {
        id: EntityId,
    },
    /// The delta contained a non-finite value.
    NonFinite {
        id: EntityId,
    },
}

impl MovementError {
    pub fn category(&self) -> ErrorCategory {
        match self {
            MovementError::UnknownEntity { .. } => ErrorCategory::NotFound,
            MovementError::Tombstoned { .. }
            | MovementError::Disabled { .. }
            | MovementError::Immovable { .. } => ErrorCategory::Conflict,
            MovementError::NotSpatial { .. }
            | MovementError::NoCollider { .. }
            | MovementError::NonFinite { .. } => ErrorCategory::Invalid,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            MovementError::UnknownEntity { .. } => "unknownEntity",
            MovementError::Tombstoned { .. } => "tombstoned",
            MovementError::Disabled { .. } => "disabled",
            MovementError::NotSpatial { .. } => "notSpatial",
            MovementError::NoCollider { .. } => "noCollider",
            MovementError::Immovable { .. } => "immovable",
            MovementError::NonFinite { .. } => "nonFinite",
        }
    }
}

impl EntityStore {
    /// Query + validate + apply a kinematic move. On success the mover's transform
    /// is updated to the resolved position and a [`MovementEvent`] returned; on
    /// failure nothing is mutated and a classified [`MovementError`] is returned.
    pub fn apply_movement(
        &mut self,
        command: MovementCommand,
    ) -> Result<MovementEvent, MovementError> {
        let id = command.id;
        self.check_movement_eligible(id)?;
        if !command.delta.x.is_finite()
            || !command.delta.y.is_finite()
            || !command.delta.z.is_finite()
        {
            return Err(MovementError::NonFinite { id });
        }

        let from = self
            .transform(id)
            .expect("eligibility guarantees a transform")
            .transform
            .translation;
        let mover_local = self
            .bounds(id)
            .map(|b| b.bounds)
            .unwrap_or(Aabb::new(Vec3::ZERO, Vec3::ZERO));

        let (resolved, blocked, hit) = self.sweep(id, mover_local, from, command.delta);

        let outcome = if blocked == [false, false, false] {
            MovementOutcome::Moved { to: resolved }
        } else if resolved == from {
            MovementOutcome::Blocked { at: from }
        } else {
            MovementOutcome::Slid {
                to: resolved,
                blocked,
            }
        };

        // Apply the resolved transform (movement is deterministic authority).
        if resolved != from {
            let current = self.transform(id).expect("eligible").transform;
            let updated = EntityTransform {
                translation: resolved,
                ..current
            };
            let applied = self.attach_transform(id, updated);
            debug_assert!(applied);
        }

        Ok(MovementEvent {
            id,
            from,
            outcome,
            hit,
            projection_changed: resolved != from && self.is_visible_projection_pub(id),
        })
    }

    /// Whether a movement command would be accepted for `id` (without applying it).
    pub fn movement_eligible(&self, id: EntityId) -> Result<(), MovementError> {
        self.check_movement_eligible(id)
    }

    fn check_movement_eligible(&self, id: EntityId) -> Result<(), MovementError> {
        let core = match self.core(id) {
            None => return Err(MovementError::UnknownEntity { id }),
            Some(core) => core,
        };
        match core.lifecycle {
            EntityLifecycle::Tombstoned => return Err(MovementError::Tombstoned { id }),
            EntityLifecycle::Disabled => return Err(MovementError::Disabled { id }),
            EntityLifecycle::Active => {}
        }
        if self.transform(id).is_none() {
            return Err(MovementError::NotSpatial { id });
        }
        match self.collision(id) {
            None => return Err(MovementError::NoCollider { id }),
            Some(c) if c.static_collider => return Err(MovementError::Immovable { id }),
            Some(_) => {}
        }
        Ok(())
    }

    /// Axis-separated AABB sweep. Returns the resolved translation, which axes were
    /// blocked, and the first collider hit.
    fn sweep(
        &self,
        mover: EntityId,
        mover_local: Aabb,
        from: Vec3,
        delta: Vec3,
    ) -> (Vec3, [bool; 3], Option<EntityId>) {
        let obstacles = self.solid_obstacles(mover);
        let mut pos = from;
        let mut blocked = [false; 3];
        let mut hit = None;

        for (axis, blocked_axis) in blocked.iter_mut().enumerate() {
            let step = axis_component(delta, axis);
            if step == 0.0 {
                continue;
            }
            let mut candidate = pos;
            set_axis(&mut candidate, axis, axis_component(pos, axis) + step);
            let mover_world = offset_aabb(mover_local, candidate);
            let blocker = obstacles
                .iter()
                .find(|(_, obb)| aabb_overlap(mover_world, *obb));
            match blocker {
                Some((id, _)) => {
                    *blocked_axis = true;
                    if hit.is_none() {
                        hit = Some(*id);
                    }
                }
                None => pos = candidate,
            }
        }
        (pos, blocked, hit)
    }

    /// World-space AABBs of every *other* entity that is a collision participant.
    /// Reads only collision + bounds capabilities — never render state.
    fn solid_obstacles(&self, mover: EntityId) -> Vec<(EntityId, Aabb)> {
        let mut out = Vec::new();
        for core in self.entities() {
            let id = core.id;
            if id == mover || !core.lifecycle.is_alive() {
                continue;
            }
            if self.collision(id).is_none() {
                continue; // rendered-but-non-colliding entities are not obstacles
            }
            let Some(bounds) = self.bounds(id) else {
                continue; // a collider with no bounds occupies no space here
            };
            let origin = self
                .transform(id)
                .map(|t| t.transform.translation)
                .unwrap_or(Vec3::ZERO);
            out.push((id, offset_aabb(bounds.bounds, origin)));
        }
        out
    }

    fn is_visible_projection_pub(&self, id: EntityId) -> bool {
        self.render_projection(id)
            .map(|r| r.visible)
            .unwrap_or(false)
    }
}

fn axis_component(v: Vec3, axis: usize) -> f32 {
    match axis {
        0 => v.x,
        1 => v.y,
        _ => v.z,
    }
}

fn set_axis(v: &mut Vec3, axis: usize, value: f32) {
    match axis {
        0 => v.x = value,
        1 => v.y = value,
        _ => v.z = value,
    }
}

fn offset_aabb(local: Aabb, origin: Vec3) -> Aabb {
    Aabb::new(
        Vec3::new(
            local.min.x + origin.x,
            local.min.y + origin.y,
            local.min.z + origin.z,
        ),
        Vec3::new(
            local.max.x + origin.x,
            local.max.y + origin.y,
            local.max.z + origin.z,
        ),
    )
}

/// Strict AABB overlap (touching faces do not count as a collision).
fn aabb_overlap(a: Aabb, b: Aabb) -> bool {
    a.min.x < b.max.x
        && a.max.x > b.min.x
        && a.min.y < b.max.y
        && a.max.y > b.min.y
        && a.min.z < b.max.z
        && a.max.z > b.min.z
}
