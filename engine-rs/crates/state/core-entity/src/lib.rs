//! Generic runtime entity substrate (entity-model-design, #2387).
//!
//! # Lane
//!
//! `rust-state`. Owns the **generic** entity core (identity + lifecycle + source),
//! optional typed capability tables, lifecycle commands/events, deterministic
//! replay hashing, and save/restore. Depends only on foundation value/id crates.
//! `core-scene` will be refactored to *compose* this crate (design §7) — the
//! dependency never runs the other way, and this crate knows nothing of render,
//! scene documents, wasm, or any TypeScript package.
//!
//! # Design posture (from the gate)
//!
//! A runtime entity is an **authority record with an identity, a lifecycle, a
//! source provenance, and a set of optional authority-owned capability records**.
//! It is *not* a game actor, ECS component bag, renderer object, voxel occupant,
//! or policy object. Transform, render projection, collision, voxel/chunk
//! membership, controller association, and asset binding are all **optional
//! capabilities** — the core never stores a position.

#![forbid(unsafe_code)]

pub mod capability;
pub mod command;
pub mod core;
pub mod fixtures;
pub mod movement;
pub mod persist;
pub mod relation;
pub mod store;
pub mod transform;
pub mod value;

pub use capability::{
    AssetBindingCapability, BoundsCapability, CollisionCapability, ContainmentCapability,
    ControllerCapability, RenderProjectionCapability, TransformCapability,
};
pub use command::{EntityLifecycleCommand, EntityLifecycleError, EntityLifecycleEvent};
pub use core::{EntityCore, EntityLifecycle, EntitySource};
pub use movement::{
    FirstPersonBasis, FirstPersonCollisionMotionError, FirstPersonCollisionMotionEvent,
    FirstPersonCollisionReadout, FirstPersonMotionCommand, FirstPersonMotionError,
    FirstPersonMotionEvent, FirstPersonMotionInput, FirstPersonMotionReadout, FirstPersonPose,
    MovementCommand, MovementError, MovementEvent, MovementOutcome,
};
pub use persist::{decode_snapshot, encode_snapshot, SnapshotDecodeError, SNAPSHOT_SCHEMA_VERSION};
pub use relation::{RelationCommand, RelationError, RelationKind};
pub use store::{EntityHash, EntityRecord, EntitySnapshot, EntityStore};
pub use transform::{TransformCommand, TransformError, TransformEvent};
pub use value::{Aabb, EntityTransform, Quat};
