//! Deterministic physics integration primitives.
//!
//! # Lane
//!
//! `rust-service` — owns deterministic physics math only. This crate has no
//! renderer, protocol, TypeScript, wall-clock, or ambient-randomness dependency.
//! It exposes explicit authority inputs and returns typed diagnostics for modes
//! this first slice does not yet implement.
//!
//! # Current scope
//!
//! The initial implementation is a bounded kinematic integrator:
//!
//! - time is expressed as [`core_time::TickDelta`] plus an explicit
//!   seconds-per-tick scale supplied by the caller;
//! - velocity is advanced by body acceleration plus world gravity;
//! - position is advanced by the new velocity (semi-implicit Euler);
//! - collision-aware movement fails closed through [`PhysicsError::CollisionQueryRequired`]
//!   until a later task wires this crate to a sanctioned collision query boundary.
//!
//! This is deliberately not a rigid-body solver. There is no mass, impulse,
//! angular velocity, broadphase, wall-clock delta, or hidden global simulation
//! state.

#![forbid(unsafe_code)]

use core_error::ErrorCategory;
use core_math::Vec3;
use core_time::TickDelta;

/// A fixed simulation step derived from authoritative tick time.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PhysicsStep {
    ticks: TickDelta,
    seconds_per_tick: f32,
}

impl PhysicsStep {
    /// Construct a step from ASHA tick time and an explicit fixed-rate scale.
    ///
    /// `seconds_per_tick` must be finite and positive. A zero [`TickDelta`] is
    /// valid and produces a no-motion integration result.
    pub fn new(ticks: TickDelta, seconds_per_tick: f32) -> Result<Self, PhysicsError> {
        if !seconds_per_tick.is_finite() || seconds_per_tick <= 0.0 {
            return Err(PhysicsError::InvalidStep { seconds_per_tick });
        }

        Ok(Self {
            ticks,
            seconds_per_tick,
        })
    }

    pub fn ticks(self) -> TickDelta {
        self.ticks
    }

    pub fn seconds_per_tick(self) -> f32 {
        self.seconds_per_tick
    }

    /// The elapsed simulation seconds for this fixed step.
    pub fn elapsed_seconds(self) -> f32 {
        self.ticks.raw() as f32 * self.seconds_per_tick
    }
}

/// Gravity and other world-level integration settings.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PhysicsWorld {
    pub gravity: Vec3,
}

impl PhysicsWorld {
    pub const ZERO_GRAVITY: Self = Self {
        gravity: Vec3::ZERO,
    };

    /// Common right-handed Y-up gravity, in units per second squared.
    pub const Y_DOWN_GRAVITY: Self = Self {
        gravity: Vec3 {
            x: 0.0,
            y: -9.8,
            z: 0.0,
        },
    };
}

impl Default for PhysicsWorld {
    fn default() -> Self {
        Self::ZERO_GRAVITY
    }
}

/// Whether this body can be advanced by the local kinematic integrator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollisionMode {
    /// No collision query is needed; integrate freely.
    None,
    /// A collision projection/query is required before authority can accept the
    /// movement. This first physics slice fails closed for this mode.
    QueryRequired,
}

/// A body advanced by the deterministic kinematic integrator.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KinematicBody {
    pub position: Vec3,
    pub velocity: Vec3,
    pub acceleration: Vec3,
    pub gravity_scale: f32,
    pub collision_mode: CollisionMode,
}

impl KinematicBody {
    pub fn stationary(position: Vec3) -> Self {
        Self {
            position,
            velocity: Vec3::ZERO,
            acceleration: Vec3::ZERO,
            gravity_scale: 1.0,
            collision_mode: CollisionMode::None,
        }
    }

    pub fn with_velocity(mut self, velocity: Vec3) -> Self {
        self.velocity = velocity;
        self
    }

    pub fn with_acceleration(mut self, acceleration: Vec3) -> Self {
        self.acceleration = acceleration;
        self
    }

    pub fn with_gravity_scale(mut self, gravity_scale: f32) -> Self {
        self.gravity_scale = gravity_scale;
        self
    }

    pub fn requiring_collision_query(mut self) -> Self {
        self.collision_mode = CollisionMode::QueryRequired;
        self
    }
}

/// Result of one deterministic integration step.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IntegrationResult {
    pub previous_position: Vec3,
    pub next_position: Vec3,
    pub previous_velocity: Vec3,
    pub next_velocity: Vec3,
    pub elapsed_seconds: f32,
}

/// Typed diagnostics for unsupported or invalid physics inputs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PhysicsError {
    /// The fixed tick scale was not finite and positive.
    InvalidStep { seconds_per_tick: f32 },
    /// A body requested collision-aware movement, but no sanctioned query
    /// boundary is wired into this crate yet.
    CollisionQueryRequired,
    /// Input contains NaN or infinite vector/scalar data.
    NonFiniteInput,
}

impl PhysicsError {
    pub fn category(self) -> ErrorCategory {
        match self {
            PhysicsError::InvalidStep { .. } | PhysicsError::NonFiniteInput => {
                ErrorCategory::Invalid
            }
            PhysicsError::CollisionQueryRequired => ErrorCategory::Unsupported,
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            PhysicsError::InvalidStep { .. } => "invalid_step",
            PhysicsError::CollisionQueryRequired => "collision_query_required",
            PhysicsError::NonFiniteInput => "non_finite_input",
        }
    }
}

impl core::fmt::Display for PhysicsError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            PhysicsError::InvalidStep { seconds_per_tick } => {
                write!(
                    f,
                    "invalid fixed physics step: seconds_per_tick must be finite and positive, got {seconds_per_tick}"
                )
            }
            PhysicsError::CollisionQueryRequired => {
                write!(
                    f,
                    "collision-aware physics integration requires a collision query boundary"
                )
            }
            PhysicsError::NonFiniteInput => write!(f, "physics input contains NaN or infinity"),
        }
    }
}

impl std::error::Error for PhysicsError {}

/// Integrate one kinematic body for one deterministic fixed step.
pub fn integrate_kinematic(
    body: KinematicBody,
    world: PhysicsWorld,
    step: PhysicsStep,
) -> Result<IntegrationResult, PhysicsError> {
    if body.collision_mode == CollisionMode::QueryRequired {
        return Err(PhysicsError::CollisionQueryRequired);
    }

    validate_body(body, world)?;

    let elapsed_seconds = step.elapsed_seconds();
    let total_acceleration = body.acceleration + world.gravity * body.gravity_scale;
    let next_velocity = body.velocity + total_acceleration * elapsed_seconds;
    let next_position = body.position + next_velocity * elapsed_seconds;

    Ok(IntegrationResult {
        previous_position: body.position,
        next_position,
        previous_velocity: body.velocity,
        next_velocity,
        elapsed_seconds,
    })
}

fn validate_body(body: KinematicBody, world: PhysicsWorld) -> Result<(), PhysicsError> {
    if finite_vec3(body.position)
        && finite_vec3(body.velocity)
        && finite_vec3(body.acceleration)
        && finite_vec3(world.gravity)
        && body.gravity_scale.is_finite()
    {
        Ok(())
    } else {
        Err(PhysicsError::NonFiniteInput)
    }
}

fn finite_vec3(v: Vec3) -> bool {
    v.x.is_finite() && v.y.is_finite() && v.z.is_finite()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step(ticks: u64, seconds_per_tick: f32) -> PhysicsStep {
        PhysicsStep::new(TickDelta::new(ticks), seconds_per_tick).unwrap()
    }

    #[test]
    fn integrates_velocity_and_acceleration_deterministically() {
        let body = KinematicBody::stationary(Vec3::new(1.0, 2.0, 3.0))
            .with_velocity(Vec3::new(2.0, 0.0, -1.0))
            .with_acceleration(Vec3::new(0.0, 4.0, 0.0))
            .with_gravity_scale(0.0);

        let result = integrate_kinematic(body, PhysicsWorld::ZERO_GRAVITY, step(2, 0.25)).unwrap();

        assert_eq!(result.elapsed_seconds, 0.5);
        assert_eq!(result.previous_position, Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(result.previous_velocity, Vec3::new(2.0, 0.0, -1.0));
        assert_eq!(result.next_velocity, Vec3::new(2.0, 2.0, -1.0));
        assert_eq!(result.next_position, Vec3::new(2.0, 3.0, 2.5));

        let repeated =
            integrate_kinematic(body, PhysicsWorld::ZERO_GRAVITY, step(2, 0.25)).unwrap();
        assert_eq!(repeated, result);
    }

    #[test]
    fn applies_world_gravity_through_body_scale() {
        let body = KinematicBody::stationary(Vec3::ZERO).with_gravity_scale(0.5);

        let result = integrate_kinematic(body, PhysicsWorld::Y_DOWN_GRAVITY, step(1, 1.0)).unwrap();

        assert_eq!(result.next_velocity, Vec3::new(0.0, -4.9, 0.0));
        assert_eq!(result.next_position, Vec3::new(0.0, -4.9, 0.0));
    }

    #[test]
    fn zero_tick_step_is_a_valid_no_motion_result() {
        let body = KinematicBody::stationary(Vec3::new(3.0, 4.0, 5.0))
            .with_velocity(Vec3::new(10.0, 0.0, 0.0))
            .with_acceleration(Vec3::new(0.0, 10.0, 0.0));

        let result =
            integrate_kinematic(body, PhysicsWorld::Y_DOWN_GRAVITY, step(0, 0.25)).unwrap();

        assert_eq!(result.elapsed_seconds, 0.0);
        assert_eq!(result.next_velocity, body.velocity);
        assert_eq!(result.next_position, body.position);
    }

    #[test]
    fn invalid_step_is_rejected() {
        let err = PhysicsStep::new(TickDelta::new(1), 0.0).unwrap_err();
        assert_eq!(
            err,
            PhysicsError::InvalidStep {
                seconds_per_tick: 0.0
            }
        );
        assert_eq!(err.category(), ErrorCategory::Invalid);
        assert_eq!(err.code(), "invalid_step");

        assert!(matches!(
            PhysicsStep::new(TickDelta::new(1), f32::NAN),
            Err(PhysicsError::InvalidStep { .. })
        ));
    }

    #[test]
    fn collision_required_movement_fails_closed() {
        let body = KinematicBody::stationary(Vec3::ZERO).requiring_collision_query();

        let err =
            integrate_kinematic(body, PhysicsWorld::ZERO_GRAVITY, step(1, 0.016)).unwrap_err();

        assert_eq!(err, PhysicsError::CollisionQueryRequired);
        assert_eq!(err.category(), ErrorCategory::Unsupported);
        assert_eq!(err.code(), "collision_query_required");
    }

    #[test]
    fn non_finite_inputs_are_rejected() {
        let body = KinematicBody::stationary(Vec3::new(f32::INFINITY, 0.0, 0.0));

        let err = integrate_kinematic(body, PhysicsWorld::ZERO_GRAVITY, step(1, 1.0)).unwrap_err();

        assert_eq!(err, PhysicsError::NonFiniteInput);
        assert_eq!(err.category(), ErrorCategory::Invalid);
    }
}
