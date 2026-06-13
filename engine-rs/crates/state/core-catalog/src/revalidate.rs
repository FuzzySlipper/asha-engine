//! Single-asset revalidation + development change-impact diagnostics
//! (scene-capability-03, subtask #2325).
//!
//! Development iteration needs to validate **one** changed asset and understand
//! its blast radius without a full catalog reboot. This is a *planning/diagnostic*
//! API: given the kind of change to an asset, it reports the dependent assets
//! affected (reverse-DAG) and whether the change is a safe visual-only reproject
//! or an unsafe authority/structural change that requires fuller revalidation or a
//! full reload. It never mutates the catalog or any renderer resource — it only
//! advises, so nothing bypasses Rust validation.

use core_assets::AssetId;

use crate::dag::DependencyGraph;
use crate::entry::Catalog;
use crate::material::MaterialDef;

/// The nature of a source change to an asset, as classified by the dev tooling
/// that detected the changed hash.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeKind {
    /// Only visual style changed (colour/texture/roughness/emissive/UV).
    VisualOnly,
    /// A material's authority flags changed (solid/collidable/occlusion/structural).
    AuthorityImpacting,
    /// Identity-shaping change: kind, dependency set, or structural geometry.
    Structural,
}

/// The recommended reload action for a change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReloadSuggestion {
    /// Re-derive render descriptors for the asset and its dependents; live-safe.
    Reproject,
    /// Re-run authority validation for the asset and dependents before reprojecting.
    RevalidateDependents,
    /// The change cannot be applied live; a full reload is required.
    RequiresFullReload,
}

/// The impact of a single-asset change.
#[derive(Debug, Clone, PartialEq)]
pub struct ChangeImpactReport {
    pub asset: AssetId,
    pub change: ChangeKind,
    /// Assets that transitively depend on `asset`, sorted by id.
    pub affected_dependents: Vec<AssetId>,
    /// `true` only for a live-safe visual-only change.
    pub safe: bool,
    /// `true` when the change cannot be applied without a full reload.
    pub requires_full_reload: bool,
    pub suggestion: ReloadSuggestion,
}

/// Revalidate a single changed asset and report its impact, or `None` if the
/// asset is not in the catalog.
pub fn revalidate_asset(
    catalog: &Catalog,
    asset: &AssetId,
    change: ChangeKind,
) -> Option<ChangeImpactReport> {
    if !catalog.contains(asset) {
        return None;
    }
    let affected_dependents = DependencyGraph::build(catalog).dependents_of(asset);

    let (safe, requires_full_reload, suggestion) = match change {
        ChangeKind::VisualOnly => (true, false, ReloadSuggestion::Reproject),
        ChangeKind::AuthorityImpacting => (false, false, ReloadSuggestion::RevalidateDependents),
        ChangeKind::Structural => (false, true, ReloadSuggestion::RequiresFullReload),
    };

    Some(ChangeImpactReport {
        asset: asset.clone(),
        change,
        affected_dependents,
        safe,
        requires_full_reload,
        suggestion,
    })
}

/// Classify a material change by diffing its before/after [`MaterialDef`]
/// (material-wiring super, #2376). The **authority** half (solid/collidable/
/// occlusion/structural class) decides safety: a pure style edit is a live-safe
/// visual reproject; any authority-flag change is authority-impacting and must
/// revalidate dependents before it can apply. An identical def is treated as a
/// no-op visual change (the caller decides whether to skip it).
pub fn classify_material_change(old: &MaterialDef, new: &MaterialDef) -> ChangeKind {
    if old.authority != new.authority {
        ChangeKind::AuthorityImpacting
    } else {
        // Only style differs (or nothing) — visual-only either way.
        ChangeKind::VisualOnly
    }
}

/// Revalidate a changed **material** asset from its before/after definition,
/// deriving the [`ChangeKind`] rather than trusting a hand-supplied one (#2376).
/// Returns `None` if the asset is absent from the catalog.
pub fn material_change_impact(
    catalog: &Catalog,
    asset: &AssetId,
    old: &MaterialDef,
    new: &MaterialDef,
) -> Option<ChangeImpactReport> {
    revalidate_asset(catalog, asset, classify_material_change(old, new))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entry::CatalogEntry;
    use crate::material::{MaterialAuthority, MaterialStyle, Rgba, StructuralClass};

    fn def(color: Rgba, solid: bool) -> MaterialDef {
        MaterialDef {
            authority: MaterialAuthority {
                solid,
                collidable: solid,
                occludes: solid,
                structural_class: if solid {
                    StructuralClass::Solid
                } else {
                    StructuralClass::Decorative
                },
            },
            style: MaterialStyle::flat(color),
        }
    }

    fn catalog_with(asset: &str, m: MaterialDef) -> (Catalog, AssetId) {
        let id = AssetId::parse(asset).unwrap();
        (
            Catalog {
                entries: vec![CatalogEntry::new(id.clone(), 1).with_material(m)],
            },
            id,
        )
    }

    #[test]
    fn a_pure_style_edit_is_a_live_safe_visual_reproject() {
        let old = def(Rgba::WHITE, true);
        let new = def(Rgba::DEBUG_GREY, true); // only colour changed
        assert_eq!(classify_material_change(&old, &new), ChangeKind::VisualOnly);

        let (catalog, id) = catalog_with("material/wall", old.clone());
        let report = material_change_impact(&catalog, &id, &old, &new).unwrap();
        assert!(report.safe);
        assert!(!report.requires_full_reload);
        assert_eq!(report.suggestion, ReloadSuggestion::Reproject);
    }

    #[test]
    fn an_authority_flag_change_is_not_live_safe() {
        let old = def(Rgba::WHITE, true); // collidable
        let new = def(Rgba::WHITE, false); // now non-collidable — authority change
        assert_eq!(
            classify_material_change(&old, &new),
            ChangeKind::AuthorityImpacting
        );

        let (catalog, id) = catalog_with("material/wall", old.clone());
        let report = material_change_impact(&catalog, &id, &old, &new).unwrap();
        assert!(!report.safe, "authority change is not a live visual update");
        assert_eq!(report.suggestion, ReloadSuggestion::RevalidateDependents);
    }

    #[test]
    fn a_structural_change_requires_full_reload() {
        let (catalog, id) = catalog_with("material/wall", def(Rgba::WHITE, true));
        let report = revalidate_asset(&catalog, &id, ChangeKind::Structural).unwrap();
        assert!(report.requires_full_reload);
        assert_eq!(report.suggestion, ReloadSuggestion::RequiresFullReload);
    }
}
