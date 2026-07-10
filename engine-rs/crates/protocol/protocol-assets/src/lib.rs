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

// ── Public asset/catalog DTO shapes ───────────────────────────────────────────

/// A public asset reference used by catalog/dependency surfaces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetReference {
    pub id: String,
    pub kind: String,
}

/// A linear RGBA colour (0..=1 per channel).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rgba {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

/// The renderer-facing projection of a material. No collision class.
#[derive(Debug, Clone, PartialEq)]
pub struct RenderMaterial {
    pub color: Rgba,
    pub texture: Option<AssetReference>,
    pub roughness: f32,
    pub emissive: f32,
    pub uv_strategy: String,
}

/// The collision/authority-facing projection of a material. No texture or colour.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollisionMaterial {
    pub solid: bool,
    pub collidable: bool,
    pub occludes: bool,
    pub structural_class: String,
}

/// A read-only bundle of both disjoint material projections.
#[derive(Debug, Clone, PartialEq)]
pub struct MaterialProjection {
    pub render: RenderMaterial,
    pub collision: CollisionMaterial,
}

/// One catalog entry. `material` is present only for material-kind assets.
#[derive(Debug, Clone, PartialEq)]
pub struct CatalogEntry {
    pub id: String,
    pub kind: String,
    pub version: u64,
    pub hash: Option<String>,
    pub source_path: Option<String>,
    pub label: Option<String>,
    pub dependencies: Vec<AssetReference>,
    pub material: Option<MaterialProjection>,
}

/// The asset registry above the asset-reference vocabulary.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Catalog {
    pub entries: Vec<CatalogEntry>,
}

/// One classified catalog-validation failure on the public border.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogValidationError {
    pub code: String,
    pub id: Option<String>,
    pub kind: Option<String>,
    pub from: Option<String>,
    pub slot: Option<String>,
    pub expected: Option<String>,
    pub actual: Option<String>,
    pub reference: Option<String>,
    pub dependency: Option<String>,
    pub cycle_path: Vec<String>,
}

/// Complete catalog validation readout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogValidationReport {
    pub errors: Vec<CatalogValidationError>,
}

/// One pinned asset-lock entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetLockEntry {
    pub id: String,
    pub kind: String,
    pub version: u64,
    pub hash: Option<String>,
    pub dependencies: Vec<String>,
}

/// Durable project-bundle asset lock.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetLock {
    pub entries: Vec<AssetLockEntry>,
}

/// One classified asset-lock drift finding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockFinding {
    pub id: String,
    pub code: String,
    pub locked_kind: Option<String>,
    pub current_kind: Option<String>,
    pub locked_version: Option<u64>,
    pub current_version: Option<u64>,
    pub locked_hash: Option<String>,
    pub current_hash: Option<String>,
    pub added_dependencies: Vec<String>,
    pub removed_dependencies: Vec<String>,
}

/// Complete asset-lock validation readout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockValidationReport {
    pub findings: Vec<LockFinding>,
}

/// Public fallback decision DTO.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FallbackDecision {
    UseFallback { reason: String, visual: String },
    FailClosed { reason: String },
    Skip { reason: String },
}

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
