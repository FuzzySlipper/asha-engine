//! Bridge from compact voxel material ids to catalog material assets
//! (material-wiring super, epic #2353; subtask #2375).
//!
//! # The problem
//!
//! Voxel storage keeps a tiny `Copy` per-cell value — a [`VoxelMaterialId`] is a
//! `u16`, never an asset string — so large volumes stay cheap (voxel-capability-02).
//! The broader asset system identifies materials by string [`AssetId`]. This table
//! is the **one** place those two worlds meet: a `VoxelMaterialId → AssetId` map,
//! held *outside* the voxel grid, so storage stays compact and deterministic while
//! a voxel still resolves to a real catalog material.
//!
//! # The projection split survives the bridge
//!
//! A resolved voxel material is still projected **disjointly**: the renderer gets a
//! [`RenderMaterial`] (colour/texture, no collision) and collision/authority gets a
//! [`CollisionMaterial`] (structural flags, no visual). The two never mix
//! (boundary 18) — the bridge resolves an id to an asset, then each consumer asks
//! for only its half.
//!
//! # Missing-material policy is context-sensitive
//!
//! A voxel material id that does not resolve to a catalog material is classified by
//! *who is asking*:
//! - **Render (visual-only)**: deterministic fallback (debug grey) so a chunk still
//!   draws something — a missing cosmetic material is not fatal.
//! - **Collision/authority (authority-critical)**: a hard error — authority must not
//!   silently treat an unknown material as non-colliding.

use std::collections::BTreeMap;

use core_assets::AssetId;
use core_voxel::VoxelMaterialId;

use crate::entry::Catalog;
use crate::material::{CollisionMaterial, MaterialDef, RenderMaterial, Rgba};

/// Maps each compact [`VoxelMaterialId`] to a catalog material [`AssetId`]. Held
/// alongside a world/scene/bundle, never inside the voxel grid, so per-cell storage
/// stays a bare `u16`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct VoxelMaterialTable {
    by_id: BTreeMap<u16, AssetId>,
}

/// Why a voxel material id failed to resolve to a catalog material.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VoxelMaterialError {
    /// The id has no entry in the table.
    Unmapped(VoxelMaterialId),
    /// The id maps to an asset that is not a material (or is absent from the
    /// catalog / carries no [`MaterialDef`]).
    NotAMaterial { id: VoxelMaterialId, asset: String },
}

/// A render resolution: the visual material plus whether a fallback was used (so a
/// caller can surface a fallback-used diagnostic — #2376).
#[derive(Debug, Clone, PartialEq)]
pub struct VoxelRenderResolution {
    pub material: RenderMaterial,
    pub used_fallback: bool,
}

/// The outcome of validating the voxel material ids a world/bundle actually uses
/// against the table + catalog. Render is always satisfiable (fallback); collision
/// is satisfiable only when `unresolved` is empty.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct VoxelMaterialTableReport {
    /// Used ids that resolve to no catalog material. Render falls back for these;
    /// collision treats them as fatal.
    pub unresolved: Vec<VoxelMaterialError>,
}

impl VoxelMaterialTableReport {
    /// `true` when every used id resolves — i.e. collision is safe (no authority
    /// material is unknown).
    pub fn is_collision_safe(&self) -> bool {
        self.unresolved.is_empty()
    }
}

impl VoxelMaterialTable {
    /// Build a table from `(voxel id, catalog material asset)` pairs. A later pair
    /// for the same id wins (deterministic last-write).
    pub fn from_pairs(pairs: impl IntoIterator<Item = (VoxelMaterialId, AssetId)>) -> Self {
        let mut by_id = BTreeMap::new();
        for (id, asset) in pairs {
            by_id.insert(id.raw(), asset);
        }
        Self { by_id }
    }

    /// The catalog material asset a voxel material id maps to, if mapped.
    pub fn material_asset(&self, id: VoxelMaterialId) -> Option<&AssetId> {
        self.by_id.get(&id.raw())
    }

    /// Number of mapped voxel materials.
    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }

    /// Resolve a voxel material id to its catalog [`MaterialDef`] via the table.
    fn material_def<'c>(
        &self,
        catalog: &'c Catalog,
        id: VoxelMaterialId,
    ) -> Result<&'c MaterialDef, VoxelMaterialError> {
        let asset = self
            .by_id
            .get(&id.raw())
            .ok_or(VoxelMaterialError::Unmapped(id))?;
        catalog
            .entries
            .iter()
            .find(|e| &e.id == asset)
            .and_then(|e| e.material.as_ref())
            .ok_or_else(|| VoxelMaterialError::NotAMaterial {
                id,
                asset: asset.as_str().to_string(),
            })
    }

    /// Resolve the **visual** render material for a voxel id. Visual is never fatal:
    /// an unresolved id yields a deterministic debug-grey fallback so a chunk still
    /// renders, flagged via [`VoxelRenderResolution::used_fallback`].
    pub fn render_material(&self, catalog: &Catalog, id: VoxelMaterialId) -> VoxelRenderResolution {
        match self.material_def(catalog, id) {
            Ok(def) => VoxelRenderResolution {
                material: def.render_projection(),
                used_fallback: false,
            },
            Err(_) => VoxelRenderResolution {
                material: fallback_render_material(),
                used_fallback: true,
            },
        }
    }

    /// Resolve the **collision/authority** material for a voxel id. Authority is
    /// fatal on a miss: an unknown material must not be silently treated as
    /// non-colliding, so this returns a classified [`VoxelMaterialError`].
    pub fn collision_material(
        &self,
        catalog: &Catalog,
        id: VoxelMaterialId,
    ) -> Result<CollisionMaterial, VoxelMaterialError> {
        self.material_def(catalog, id)
            .map(MaterialDef::collision_projection)
    }

    /// Validate the voxel material ids a world/bundle uses against this table +
    /// catalog. Render can always proceed (fallback); collision is safe iff the
    /// report is empty.
    pub fn validate_used(
        &self,
        catalog: &Catalog,
        used: impl IntoIterator<Item = VoxelMaterialId>,
    ) -> VoxelMaterialTableReport {
        let mut seen = std::collections::BTreeSet::new();
        let mut unresolved = Vec::new();
        for id in used {
            if !seen.insert(id.raw()) {
                continue;
            }
            if let Err(e) = self.material_def(catalog, id) {
                unresolved.push(e);
            }
        }
        VoxelMaterialTableReport { unresolved }
    }
}

/// The deterministic missing-cosmetic fallback for voxels: neutral debug grey,
/// matching the static-mesh fallback (#2373) so both share one placeholder look.
fn fallback_render_material() -> RenderMaterial {
    RenderMaterial {
        color: Rgba::DEBUG_GREY,
        texture: None,
        roughness: 1.0,
        texture_tint: Rgba::WHITE,
        emission_color: Rgba::DEBUG_GREY,
        emissive: 0.0,
        uv_strategy: crate::material::UvStrategy::Flat,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entry::CatalogEntry;
    use crate::material::{MaterialAuthority, MaterialStyle, StructuralClass};

    fn material_entry(id: &str, color: Rgba, structural: StructuralClass) -> CatalogEntry {
        CatalogEntry::new(AssetId::parse(id).unwrap(), 1).with_material(MaterialDef {
            authority: MaterialAuthority {
                solid: true,
                collidable: true,
                occludes: true,
                structural_class: structural,
            },
            style: MaterialStyle::flat(color),
        })
    }

    fn stone_dirt_catalog() -> Catalog {
        Catalog {
            entries: vec![
                material_entry(
                    "material/stone",
                    Rgba {
                        r: 0.5,
                        g: 0.5,
                        b: 0.55,
                        a: 1.0,
                    },
                    StructuralClass::Structural,
                ),
                material_entry(
                    "material/dirt",
                    Rgba {
                        r: 0.4,
                        g: 0.25,
                        b: 0.1,
                        a: 1.0,
                    },
                    StructuralClass::Solid,
                ),
            ],
        }
    }

    fn table() -> VoxelMaterialTable {
        VoxelMaterialTable::from_pairs([
            (
                VoxelMaterialId::new(1),
                AssetId::parse("material/stone").unwrap(),
            ),
            (
                VoxelMaterialId::new(2),
                AssetId::parse("material/dirt").unwrap(),
            ),
        ])
    }

    #[test]
    fn resolves_two_voxel_materials_to_distinct_render_styles() {
        let (catalog, table) = (stone_dirt_catalog(), table());
        let stone = table.render_material(&catalog, VoxelMaterialId::new(1));
        let dirt = table.render_material(&catalog, VoxelMaterialId::new(2));
        assert!(!stone.used_fallback && !dirt.used_fallback);
        assert_eq!(
            stone.material.color,
            Rgba {
                r: 0.5,
                g: 0.5,
                b: 0.55,
                a: 1.0
            }
        );
        assert_ne!(
            stone.material.color, dirt.material.color,
            "distinct catalog styles"
        );
    }

    #[test]
    fn collision_resolves_structural_flags_with_no_visual_leakage() {
        let (catalog, table) = (stone_dirt_catalog(), table());
        let stone = table
            .collision_material(&catalog, VoxelMaterialId::new(1))
            .unwrap();
        assert!(stone.solid && stone.collidable);
        assert_eq!(stone.structural_class, StructuralClass::Structural);
        // CollisionMaterial is a disjoint type: it has no colour/texture field at
        // all, so a visual leak is impossible by construction. Assert the render
        // and collision projections of the same id carry independent data.
        let render = table.render_material(&catalog, VoxelMaterialId::new(1));
        let _ = render.material.color; // exists on render
                                       // (stone.color does not compile — enforced by the type, not this test.)
        let _ = stone.structural_class; // exists on collision
    }

    #[test]
    fn unmapped_id_is_visual_fallback_but_collision_fatal() {
        let (catalog, table) = (stone_dirt_catalog(), table());
        let ghost = VoxelMaterialId::new(99);

        // Render: deterministic grey fallback, flagged.
        let r = table.render_material(&catalog, ghost);
        assert!(r.used_fallback);
        assert_eq!(r.material.color, Rgba::DEBUG_GREY);

        // Collision: fatal, classified.
        assert_eq!(
            table.collision_material(&catalog, ghost),
            Err(VoxelMaterialError::Unmapped(ghost))
        );
    }

    #[test]
    fn mapped_to_non_material_asset_is_classified() {
        let catalog = Catalog {
            entries: vec![CatalogEntry::new(AssetId::parse("mesh/crate").unwrap(), 1)],
        };
        let table = VoxelMaterialTable::from_pairs([(
            VoxelMaterialId::new(1),
            AssetId::parse("mesh/crate").unwrap(),
        )]);
        assert_eq!(
            table.collision_material(&catalog, VoxelMaterialId::new(1)),
            Err(VoxelMaterialError::NotAMaterial {
                id: VoxelMaterialId::new(1),
                asset: "mesh/crate".into()
            })
        );
    }

    #[test]
    fn validate_used_flags_unresolved_for_collision_but_render_proceeds() {
        let (catalog, table) = (stone_dirt_catalog(), table());
        let used = [
            VoxelMaterialId::new(1),
            VoxelMaterialId::new(2),
            VoxelMaterialId::new(1), // dup ignored
            VoxelMaterialId::new(7), // unmapped
        ];
        let report = table.validate_used(&catalog, used);
        assert!(!report.is_collision_safe());
        assert_eq!(report.unresolved.len(), 1);
        assert_eq!(
            report.unresolved[0],
            VoxelMaterialError::Unmapped(VoxelMaterialId::new(7))
        );

        // A fully-mapped use set is collision-safe.
        let ok = table.validate_used(&catalog, [VoxelMaterialId::new(1), VoxelMaterialId::new(2)]);
        assert!(ok.is_collision_safe());
    }
}
