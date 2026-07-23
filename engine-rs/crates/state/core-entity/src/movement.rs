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
use crate::transform::{TransformCommand, TransformError, TransformEvent};
use crate::value::{Aabb, EntityTransform, Quat};

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

/// Bounded first-person actor/camera input, in the same semantic vocabulary as
/// `protocol-view::FirstPersonCameraInput` but owned by the authority state lane.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FirstPersonMotionInput {
    pub move_forward: f32,
    pub move_right: f32,
    pub move_up: f32,
    pub yaw_delta_degrees: f32,
    pub pitch_delta_degrees: f32,
    pub dt_seconds: f32,
    pub move_speed_units_per_second: f32,
}

/// A proposed first-person motion/look update for a transform-capable entity.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FirstPersonMotionCommand {
    pub id: EntityId,
    pub input: FirstPersonMotionInput,
    pub tick: u64,
}

/// Authority pose readout after first-person motion integration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FirstPersonPose {
    pub position: Vec3,
    pub yaw_degrees: f32,
    pub pitch_degrees: f32,
}

/// Camera-style basis derived from an authority pose. Projection consumers may
/// use this readout; renderers do not own or mutate it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FirstPersonBasis {
    pub forward: Vec3,
    pub right: Vec3,
    pub up: Vec3,
}

/// Deterministic projection/readout emitted with an accepted first-person update.
#[derive(Debug, Clone, PartialEq)]
pub struct FirstPersonMotionReadout {
    pub id: EntityId,
    pub tick: u64,
    pub pose: FirstPersonPose,
    pub basis: FirstPersonBasis,
    pub pose_hash: u64,
}

/// The authoritative record of an accepted first-person actor/camera update.
#[derive(Debug, Clone, PartialEq)]
pub struct FirstPersonMotionEvent {
    pub id: EntityId,
    pub tick: u64,
    pub input: FirstPersonMotionInput,
    pub from: FirstPersonPose,
    pub to: FirstPersonPose,
    pub transform: TransformEvent,
    pub readout: FirstPersonMotionReadout,
}

/// Collision/debug readout for a first-person motion command resolved through
/// authority movement collision.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FirstPersonCollisionReadout {
    pub outcome: MovementOutcome,
    pub hit: Option<EntityId>,
    pub projection_changed: bool,
}

/// Accepted first-person motion resolved through the entity collision substrate.
#[derive(Debug, Clone, PartialEq)]
pub struct FirstPersonCollisionMotionEvent {
    pub id: EntityId,
    pub tick: u64,
    pub input: FirstPersonMotionInput,
    pub from: FirstPersonPose,
    pub attempted: FirstPersonPose,
    pub to: FirstPersonPose,
    pub movement: MovementEvent,
    pub transform: TransformEvent,
    pub readout: FirstPersonMotionReadout,
    pub collision: FirstPersonCollisionReadout,
}

/// Why a collision-aware first-person motion command was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FirstPersonCollisionMotionError {
    Input(FirstPersonMotionError),
    Movement(MovementError),
    Transform(TransformError),
}

impl FirstPersonCollisionMotionError {
    pub fn label(&self) -> &'static str {
        match self {
            FirstPersonCollisionMotionError::Input(e) => e.label(),
            FirstPersonCollisionMotionError::Movement(e) => e.label(),
            FirstPersonCollisionMotionError::Transform(e) => e.label(),
        }
    }

    pub fn category(&self) -> ErrorCategory {
        match self {
            FirstPersonCollisionMotionError::Input(e) => e.category(),
            FirstPersonCollisionMotionError::Movement(e) => e.category(),
            FirstPersonCollisionMotionError::Transform(e) => e.category(),
        }
    }
}

/// Why a first-person motion command was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FirstPersonMotionError {
    Transform(TransformError),
    NonFinite { id: EntityId },
    NegativeTimeOrSpeed { id: EntityId },
}

impl FirstPersonMotionError {
    pub fn label(&self) -> &'static str {
        match self {
            FirstPersonMotionError::Transform(e) => e.label(),
            FirstPersonMotionError::NonFinite { .. } => "nonFinite",
            FirstPersonMotionError::NegativeTimeOrSpeed { .. } => "negativeTimeOrSpeed",
        }
    }

    pub fn category(&self) -> ErrorCategory {
        match self {
            FirstPersonMotionError::Transform(e) => e.category(),
            FirstPersonMotionError::NonFinite { .. }
            | FirstPersonMotionError::NegativeTimeOrSpeed { .. } => ErrorCategory::Invalid,
        }
    }
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

    /// Validate and apply first-person actor/camera motion without collision. The
    /// accepted update mutates only the entity's transform capability and returns
    /// a deterministic pose/basis readout for camera projection consumers.
    pub fn apply_first_person_motion(
        &mut self,
        command: FirstPersonMotionCommand,
    ) -> Result<FirstPersonMotionEvent, FirstPersonMotionError> {
        validate_first_person_input(command.id, command.input)?;
        self.transform_eligible(command.id)
            .map_err(FirstPersonMotionError::Transform)?;
        let current = self
            .transform(command.id)
            .expect("transform eligibility guarantees a transform capability")
            .transform;
        let from = pose_from_transform(current);
        let basis = basis_from_pose(from);
        let distance = command.input.dt_seconds * command.input.move_speed_units_per_second;
        let delta = Vec3::new(
            (basis.forward.x * command.input.move_forward
                + basis.right.x * command.input.move_right
                + basis.up.x * command.input.move_up)
                * distance,
            (basis.forward.y * command.input.move_forward
                + basis.right.y * command.input.move_right
                + basis.up.y * command.input.move_up)
                * distance,
            (basis.forward.z * command.input.move_forward
                + basis.right.z * command.input.move_right
                + basis.up.z * command.input.move_up)
                * distance,
        );
        let to = FirstPersonPose {
            position: Vec3::new(
                from.position.x + delta.x,
                from.position.y + delta.y,
                from.position.z + delta.z,
            ),
            yaw_degrees: from.yaw_degrees + command.input.yaw_delta_degrees,
            pitch_degrees: (from.pitch_degrees + command.input.pitch_delta_degrees)
                .clamp(-89.0, 89.0),
        };
        let next = EntityTransform {
            translation: to.position,
            rotation: quat_from_yaw_pitch(to.yaw_degrees, to.pitch_degrees),
            ..current
        };
        let transform = self
            .apply_transform(TransformCommand::Set {
                id: command.id,
                transform: next,
            })
            .map_err(FirstPersonMotionError::Transform)?;
        let readout = FirstPersonMotionReadout {
            id: command.id,
            tick: command.tick,
            pose: to,
            basis: basis_from_pose(to),
            pose_hash: pose_hash(to),
        };
        Ok(FirstPersonMotionEvent {
            id: command.id,
            tick: command.tick,
            input: command.input,
            from,
            to,
            transform,
            readout,
        })
    }

    /// Validate and apply first-person actor/camera motion through the existing
    /// authority collision movement substrate. Translation is resolved by
    /// [`EntityStore::apply_movement`]; yaw/pitch still update through the
    /// transform capability so a blocked body can look around.
    pub fn apply_first_person_motion_with_collision(
        &mut self,
        command: FirstPersonMotionCommand,
    ) -> Result<FirstPersonCollisionMotionEvent, FirstPersonCollisionMotionError> {
        validate_first_person_input(command.id, command.input)
            .map_err(FirstPersonCollisionMotionError::Input)?;
        self.movement_eligible(command.id)
            .map_err(FirstPersonCollisionMotionError::Movement)?;
        let current = self
            .transform(command.id)
            .expect("movement eligibility guarantees a transform capability")
            .transform;
        let from = pose_from_transform(current);
        let attempted = integrate_first_person_pose(from, command.input);
        let movement = self
            .apply_movement(MovementCommand {
                id: command.id,
                delta: Vec3::new(
                    attempted.position.x - from.position.x,
                    attempted.position.y - from.position.y,
                    attempted.position.z - from.position.z,
                ),
            })
            .map_err(FirstPersonCollisionMotionError::Movement)?;
        let resolved_position = movement_outcome_position(movement.outcome);
        let to = FirstPersonPose {
            position: resolved_position,
            yaw_degrees: attempted.yaw_degrees,
            pitch_degrees: attempted.pitch_degrees,
        };
        let next = EntityTransform {
            translation: to.position,
            rotation: quat_from_yaw_pitch(to.yaw_degrees, to.pitch_degrees),
            ..current
        };
        let transform = self
            .apply_transform(TransformCommand::Set {
                id: command.id,
                transform: next,
            })
            .map_err(FirstPersonCollisionMotionError::Transform)?;
        let readout = FirstPersonMotionReadout {
            id: command.id,
            tick: command.tick,
            pose: to,
            basis: basis_from_pose(to),
            pose_hash: pose_hash(to),
        };
        let collision = FirstPersonCollisionReadout {
            outcome: movement.outcome,
            hit: movement.hit,
            projection_changed: movement.projection_changed || transform.projection_changed,
        };
        Ok(FirstPersonCollisionMotionEvent {
            id: command.id,
            tick: command.tick,
            input: command.input,
            from,
            attempted,
            to,
            movement,
            transform,
            readout,
            collision,
        })
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
        match self.active_collision(id) {
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
            let Some(_collision) = self.active_collision(id) else {
                continue; // rendered-but-non-colliding entities are not obstacles
            };
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

fn validate_first_person_input(
    id: EntityId,
    input: FirstPersonMotionInput,
) -> Result<(), FirstPersonMotionError> {
    let values = [
        input.move_forward,
        input.move_right,
        input.move_up,
        input.yaw_delta_degrees,
        input.pitch_delta_degrees,
        input.dt_seconds,
        input.move_speed_units_per_second,
    ];
    if values.iter().any(|v| !v.is_finite()) {
        return Err(FirstPersonMotionError::NonFinite { id });
    }
    if input.dt_seconds < 0.0 || input.move_speed_units_per_second < 0.0 {
        return Err(FirstPersonMotionError::NegativeTimeOrSpeed { id });
    }
    Ok(())
}

fn pose_from_transform(transform: EntityTransform) -> FirstPersonPose {
    let (yaw_degrees, pitch_degrees) = yaw_pitch_from_quat(transform.rotation);
    FirstPersonPose {
        position: transform.translation,
        yaw_degrees,
        pitch_degrees,
    }
}

fn basis_from_pose(pose: FirstPersonPose) -> FirstPersonBasis {
    let yaw = pose.yaw_degrees.to_radians();
    let pitch = pose.pitch_degrees.to_radians();
    let cp = pitch.cos();
    let sp = pitch.sin();
    let sy = yaw.sin();
    let cy = yaw.cos();
    FirstPersonBasis {
        forward: Vec3::new(sy * cp, sp, -cy * cp),
        right: Vec3::new(cy, 0.0, sy),
        up: Vec3::new(-sy * sp, cp, cy * sp),
    }
}

fn integrate_first_person_pose(
    from: FirstPersonPose,
    input: FirstPersonMotionInput,
) -> FirstPersonPose {
    let basis = basis_from_pose(from);
    let distance = input.dt_seconds * input.move_speed_units_per_second;
    let delta = Vec3::new(
        (basis.forward.x * input.move_forward
            + basis.right.x * input.move_right
            + basis.up.x * input.move_up)
            * distance,
        (basis.forward.y * input.move_forward
            + basis.right.y * input.move_right
            + basis.up.y * input.move_up)
            * distance,
        (basis.forward.z * input.move_forward
            + basis.right.z * input.move_right
            + basis.up.z * input.move_up)
            * distance,
    );
    FirstPersonPose {
        position: Vec3::new(
            from.position.x + delta.x,
            from.position.y + delta.y,
            from.position.z + delta.z,
        ),
        yaw_degrees: from.yaw_degrees + input.yaw_delta_degrees,
        pitch_degrees: (from.pitch_degrees + input.pitch_delta_degrees).clamp(-89.0, 89.0),
    }
}

fn movement_outcome_position(outcome: MovementOutcome) -> Vec3 {
    match outcome {
        MovementOutcome::Moved { to } | MovementOutcome::Slid { to, .. } => to,
        MovementOutcome::Blocked { at } => at,
    }
}

fn quat_from_yaw_pitch(yaw_degrees: f32, pitch_degrees: f32) -> Quat {
    let yaw = yaw_degrees.to_radians() * 0.5;
    let pitch = pitch_degrees.to_radians() * 0.5;
    let (sy, cy) = yaw.sin_cos();
    let (sp, cp) = pitch.sin_cos();
    Quat {
        x: cy * sp,
        y: sy * cp,
        z: -sy * sp,
        w: cy * cp,
    }
}

fn yaw_pitch_from_quat(q: Quat) -> (f32, f32) {
    let sin_pitch = 2.0 * (q.w * q.x - q.y * q.z);
    let pitch = sin_pitch.clamp(-1.0, 1.0).asin();
    let yaw = (2.0 * (q.w * q.y - q.z * q.x)).atan2(1.0 - 2.0 * (q.x * q.x + q.y * q.y));
    (yaw.to_degrees(), pitch.to_degrees())
}

fn pose_hash(pose: FirstPersonPose) -> u64 {
    let mut h = Fnv1aLocal::new();
    h.write_f32(pose.position.x);
    h.write_f32(pose.position.y);
    h.write_f32(pose.position.z);
    h.write_f32(pose.yaw_degrees);
    h.write_f32(pose.pitch_degrees);
    h.finish()
}

struct Fnv1aLocal(u64);

impl Fnv1aLocal {
    fn new() -> Self {
        Self(0xcbf2_9ce4_8422_2325)
    }

    fn write_f32(&mut self, value: f32) {
        for b in value.to_bits().to_le_bytes() {
            self.0 ^= b as u64;
            self.0 = self.0.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }

    fn finish(self) -> u64 {
        self.0
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
