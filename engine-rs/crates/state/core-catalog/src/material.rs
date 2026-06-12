//! Material asset definition with a strict **authority / style** split
//! (scene-capability-03, subtask #2324; voxel-capability-14).
//!
//! A material asset is the single source, but consumers receive *separate
//! projections*: authority/collision/pathfinding sees structural flags, the
//! renderer sees visual descriptors. The projection types are deliberately
//! disjoint — [`CollisionMaterial`] carries no texture/colour and
//! [`RenderMaterial`] carries no collision class — so a boundary leak is a type
//! error, not a code-review nit.

use core_assets::AssetReference;

/// How a material occupies space for authority/collision/pathfinding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StructuralClass {
    /// No structural role (decorative).
    Decorative,
    /// Ordinary solid matter.
    Solid,
    /// Load-bearing / structural matter.
    Structural,
}

/// Authority-relevant material flags. Consumed by collision/pathfinding/authority;
/// never carries any visual field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MaterialAuthority {
    /// Whether the material fills its cell (vs passable air).
    pub solid: bool,
    /// Whether physics/collision treats it as an obstacle.
    pub collidable: bool,
    /// Whether it blocks line-of-sight/occlusion queries.
    pub occludes: bool,
    /// Structural role for authority systems.
    pub structural_class: StructuralClass,
}

impl MaterialAuthority {
    /// A passable, non-colliding decorative default.
    pub const DECORATIVE: MaterialAuthority = MaterialAuthority {
        solid: false,
        collidable: false,
        occludes: false,
        structural_class: StructuralClass::Decorative,
    };
}

/// How a material's surface samples colour across geometry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UvStrategy {
    /// A single flat colour, no texture sampling.
    Flat,
    /// Sample the bound texture with planar UVs.
    Planar,
    /// Sample an atlas sub-rectangle (sprite sheets).
    Atlas,
}

/// A linear RGBA colour (0..=1 per channel). A tiny in-crate type so the catalog
/// stays std-only with no math-crate dependency.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rgba {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Rgba {
    pub const WHITE: Rgba = Rgba {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        a: 1.0,
    };

    /// The neutral grey debug material colour (missing-cosmetic fallback).
    pub const DEBUG_GREY: Rgba = Rgba {
        r: 0.5,
        g: 0.5,
        b: 0.5,
        a: 1.0,
    };

    /// The magenta debug colour (missing-overlay-sprite fallback).
    pub const DEBUG_MAGENTA: Rgba = Rgba {
        r: 1.0,
        g: 0.0,
        b: 1.0,
        a: 1.0,
    };
}

/// Visual style for the renderer. Carries no collision/authority field.
#[derive(Debug, Clone, PartialEq)]
pub struct MaterialStyle {
    pub color: Rgba,
    /// Optional bound texture asset (a `texture/...` reference).
    pub texture: Option<AssetReference>,
    pub roughness: f32,
    pub emissive: f32,
    pub uv_strategy: UvStrategy,
}

impl MaterialStyle {
    /// A flat-coloured untextured style.
    pub fn flat(color: Rgba) -> MaterialStyle {
        MaterialStyle {
            color,
            texture: None,
            roughness: 1.0,
            emissive: 0.0,
            uv_strategy: UvStrategy::Flat,
        }
    }
}

/// A material asset: authority source + visual source, projected separately.
#[derive(Debug, Clone, PartialEq)]
pub struct MaterialDef {
    pub authority: MaterialAuthority,
    pub style: MaterialStyle,
}

/// The renderer-facing projection of a material. **No collision class.**
#[derive(Debug, Clone, PartialEq)]
pub struct RenderMaterial {
    pub color: Rgba,
    pub texture: Option<AssetReference>,
    pub roughness: f32,
    pub emissive: f32,
    pub uv_strategy: UvStrategy,
}

/// The collision/authority-facing projection of a material. **No texture/colour.**
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CollisionMaterial {
    pub solid: bool,
    pub collidable: bool,
    pub occludes: bool,
    pub structural_class: StructuralClass,
}

impl MaterialDef {
    /// Project the renderer descriptor (visual fields only).
    pub fn render_projection(&self) -> RenderMaterial {
        RenderMaterial {
            color: self.style.color,
            texture: self.style.texture.clone(),
            roughness: self.style.roughness,
            emissive: self.style.emissive,
            uv_strategy: self.style.uv_strategy,
        }
    }

    /// Project the collision/authority descriptor (structural fields only).
    pub fn collision_projection(&self) -> CollisionMaterial {
        CollisionMaterial {
            solid: self.authority.solid,
            collidable: self.authority.collidable,
            occludes: self.authority.occludes,
            structural_class: self.authority.structural_class,
        }
    }
}
