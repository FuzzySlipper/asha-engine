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
