//! Small value types used by optional spatial capabilities.
//!
//! These are deliberately defined here (over `core-math`) rather than imported
//! from `core-scene`: `core-scene` will be refactored to *compose* this crate's
//! capability tables (design §7), so a dependency the other way would be circular.
//! The transform/bounds shapes mirror `core-scene::SceneTransform` so a later
//! unification (#2388) is a straight mapping, not a reinterpretation.

use core_math::Vec3;

/// A rotation quaternion in `(x, y, z, w)` order — matching the render border's
/// `rotation` tuple so a later projection is a straight copy.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Quat {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Quat {
    /// The identity rotation `(0, 0, 0, 1)`.
    pub const IDENTITY: Quat = Quat {
        x: 0.0,
        y: 0.0,
        z: 0.0,
        w: 1.0,
    };
}

/// A runtime transform: the value an entity's optional `TransformCapability` holds.
/// Transform is **not** a core entity field — only entities with the capability
/// have one (design §1/§2).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EntityTransform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl EntityTransform {
    /// The identity transform (origin, no rotation, unit scale).
    pub const IDENTITY: EntityTransform = EntityTransform {
        translation: Vec3::ZERO,
        rotation: Quat::IDENTITY,
        scale: Vec3::ONE,
    };

    /// An identity transform translated to `translation`.
    pub fn at(translation: Vec3) -> Self {
        EntityTransform {
            translation,
            ..EntityTransform::IDENTITY
        }
    }
}

/// An axis-aligned bounding box: the value a `BoundsCapability` holds for an
/// entity that occupies space without a visible render object.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

impl Aabb {
    pub fn new(min: Vec3, max: Vec3) -> Self {
        Aabb { min, max }
    }
}
