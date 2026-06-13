//! Cross-boundary schema for the asset catalog (scene-capability-super, epic
//! #2351, subtask #2367).
//!
//! # Lane
//!
//! `contract-steward` — owns the border shape TypeScript/devtools/renderer use to
//! **display catalog validation, asset-lock drift, fallback decisions, and the
//! authority/style material projections**. Validation is Rust-owned
//! (`core-catalog`): a TS layer may author catalog data, but only Rust decides
//! whether a catalog enters authority. This crate carries no validation logic.
//!
//! # The authority / style split is a border invariant
//!
//! A material has one source but two **disjoint** projections: the renderer sees
//! `RenderMaterial` (colour/texture/uv — *no collision class*) and authority sees
//! `CollisionMaterial` (solid/collidable/occludes/structural — *no texture or
//! colour*). The generated TypeScript keeps them separate types so a renderer
//! that imports `RenderMaterial` cannot even name a collision field, and vice
//! versa. A read-only devtools `MaterialProjection` may bundle both for
//! inspection; the pure render path never does.
//!
//! # Single home for stable vocabularies
//!
//! Asset-kind tags are sourced from `core_assets::AssetKind` (drift-checked by a
//! test); catalog validation codes, lock-issue codes, structural classes, uv
//! strategies, and fallback context/visual/outcome tags each have one `const`
//! table here that `protocol-codegen` mirrors.

#![forbid(unsafe_code)]

/// Stable kind-prefix tags for every asset kind, sourced from
/// `core_assets::AssetKind::prefix` (see the drift test below).
pub const ASSET_KINDS: &[&str] = &[
    "material",
    "mesh",
    "sprite",
    "sprite-sheet",
    "texture",
    "voxel-volume",
    "voxel-object",
    "script",
    "scene",
];

/// Stable classified catalog-validation codes. Mirrors
/// `core_catalog::CatalogValidationError::label`.
pub const CATALOG_VALIDATION_CODES: &[&str] = &[
    "duplicate-asset-id",
    "material-payload-missing",
    "material-payload-on-non-material",
    "wrong-kind-reference",
    "unknown-dependency",
    "dependency-cycle",
    "empty-source-path",
];

/// Stable classified asset-lock issue codes. Mirrors `core_catalog::LockIssue::label`.
pub const LOCK_ISSUE_CODES: &[&str] = &[
    "missing",
    "wrong-kind",
    "stale-version",
    "stale-hash",
    "dependency-drift",
    "new-in-catalog",
];

/// Stable structural-class tags (authority/collision side of a material).
pub const STRUCTURAL_CLASSES: &[&str] = &["decorative", "solid", "structural"];

/// Stable uv-strategy tags (visual side of a material).
pub const UV_STRATEGIES: &[&str] = &["flat", "planar", "atlas"];

/// Stable fallback-context tags. Mirrors `core_catalog::AssetContext`.
pub const FALLBACK_CONTEXTS: &[&str] = &[
    "debugOverlay",
    "cosmeticSurface",
    "collisionCritical",
    "backgroundDecoration",
];

/// Stable fallback-visual tags. Mirrors `core_catalog::FallbackVisual`.
pub const FALLBACK_VISUALS: &[&str] = &["magentaSquare", "greyMaterial"];

/// Stable fallback-outcome discriminants. Mirrors `core_catalog::FallbackOutcome`.
pub const FALLBACK_OUTCOMES: &[&str] = &["useFallback", "failClosed", "skip"];

#[cfg(test)]
mod tests {
    use super::*;
    use core_assets::AssetKind;

    #[test]
    fn asset_kind_table_matches_core_assets() {
        let from_source: Vec<&str> = AssetKind::ALL.iter().map(|k| k.prefix()).collect();
        assert_eq!(
            from_source, ASSET_KINDS,
            "ASSET_KINDS drifted from core_assets::AssetKind::ALL prefixes"
        );
    }

    #[test]
    fn vocabulary_tables_are_nonempty_and_unique() {
        for table in [
            ASSET_KINDS,
            CATALOG_VALIDATION_CODES,
            LOCK_ISSUE_CODES,
            STRUCTURAL_CLASSES,
            UV_STRATEGIES,
            FALLBACK_CONTEXTS,
            FALLBACK_VISUALS,
            FALLBACK_OUTCOMES,
        ] {
            assert!(!table.is_empty());
            let mut sorted = table.to_vec();
            sorted.sort_unstable();
            sorted.dedup();
            assert_eq!(sorted.len(), table.len(), "duplicate in {table:?}");
        }
    }
}
