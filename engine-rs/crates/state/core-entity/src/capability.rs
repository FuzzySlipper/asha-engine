//! Optional, typed capability records (design §2).
//!
//! Capabilities are **separate authority-owned tables keyed by [`EntityId`]**, not
//! fields on the core and not dynamic components. Each is a concrete typed record;
//! adding a capability to an entity means inserting into that table. Querying a
//! capability for an entity that lacks it returns `None` — never a default/phantom.
//! There is no `Box<dyn Any>` / generic component map.

use core_assets::AssetReference;
use core_ids::{ProcessId, SubjectId};

use crate::value::{Aabb, EntityTransform};

/// Runtime transform capability. Authority-owned; for scene-sourced entities it is
/// seeded from the scene initial transform at bootstrap and free to diverge.
/// Transform commands (#2388) mutate this; lifecycle (#2387) only seeds/clears it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransformCapability {
    pub transform: EntityTransform,
}

/// Spatial extent for entities that occupy space but need no transform-driven
/// render object (e.g. trigger volumes / navigation anchors).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoundsCapability {
    pub bounds: Aabb,
}

/// The fact that this entity *projects* to a render handle. The handle itself is
/// derived/ephemeral and lives in the render layer; it is **never** the durable
/// entity reference. This record only marks that a projection should be emitted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderProjectionCapability {
    /// Whether the projection is currently visible (distinct from lifecycle
    /// `Disabled`, which suppresses projection entirely).
    pub visible: bool,
}

/// Collision/query participation. Movement (#2390) reads this — never render state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CollisionCapability {
    /// Whether the collider is static (immovable) — movement of a static entity
    /// is rejected even though it is spatial.
    pub static_collider: bool,
}

/// Membership in a container/slot relation (design §5 relation 2). Explicitly
/// **not** a transform parent and does not require either end be spatial.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContainmentCapability {
    pub container: core_ids::EntityId,
}

/// Association to a controlling policy process or subject (design §5 relation 4).
/// Association, not ownership of the entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControllerCapability {
    Process(ProcessId),
    Subject(SubjectId),
}

/// Binding to a catalog/asset reference for mesh/sprite/material; absent for
/// purely logical entities.
#[derive(Debug, Clone, PartialEq)]
pub struct AssetBindingCapability {
    pub asset: AssetReference,
}
