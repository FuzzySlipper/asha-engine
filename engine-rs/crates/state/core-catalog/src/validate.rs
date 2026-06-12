//! Catalog validation with a classified report (scene-capability-03, subtask
//! #2322).
//!
//! Validation is **Rust-owned**: a TS layer may author catalog data, but only
//! this pass decides whether a catalog may enter authority/runtime. Every failure
//! is a typed [`CatalogValidationError`] (with a cycle path where relevant) so a
//! future protocol diagnostic routes on the variant rather than parsing prose.

use std::collections::BTreeSet;

use core_assets::{AssetId, AssetKind};

use crate::dag::DependencyGraph;
use crate::entry::Catalog;

/// One classified catalog validation failure.
#[derive(Debug, Clone, PartialEq)]
pub enum CatalogValidationError {
    /// Two entries share a stable asset id.
    DuplicateAssetId { id: AssetId },
    /// A material-kind entry is missing its [`MaterialDef`](crate::MaterialDef).
    MaterialPayloadMissing { id: AssetId },
    /// A non-material entry carries a material payload.
    MaterialPayloadOnNonMaterial { id: AssetId, kind: AssetKind },
    /// A typed slot holds a reference of the wrong kind (e.g. a material's texture
    /// slot pointing at a non-texture asset).
    WrongKindReference {
        from: AssetId,
        slot: &'static str,
        expected: AssetKind,
        actual: AssetKind,
        reference: AssetId,
    },
    /// An entry depends on an asset id not present in the catalog.
    UnknownDependency { from: AssetId, dependency: AssetId },
    /// The dependency edges form a cycle; `path` is the cycle in order, closed
    /// (`a -> b -> a`).
    DependencyCycle { path: Vec<AssetId> },
    /// An entry's source path is present but empty.
    EmptySourcePath { id: AssetId },
}

impl CatalogValidationError {
    /// Short, stable label for diagnostics/serialization.
    pub fn label(&self) -> &'static str {
        match self {
            CatalogValidationError::DuplicateAssetId { .. } => "duplicate-asset-id",
            CatalogValidationError::MaterialPayloadMissing { .. } => "material-payload-missing",
            CatalogValidationError::MaterialPayloadOnNonMaterial { .. } => {
                "material-payload-on-non-material"
            }
            CatalogValidationError::WrongKindReference { .. } => "wrong-kind-reference",
            CatalogValidationError::UnknownDependency { .. } => "unknown-dependency",
            CatalogValidationError::DependencyCycle { .. } => "dependency-cycle",
            CatalogValidationError::EmptySourcePath { .. } => "empty-source-path",
        }
    }
}

/// The outcome of validating a catalog: every error found, not just the first.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CatalogValidationReport {
    pub errors: Vec<CatalogValidationError>,
}

impl CatalogValidationReport {
    /// `true` if no errors were found.
    pub fn is_ok(&self) -> bool {
        self.errors.is_empty()
    }
}

/// Validate a catalog, returning every classified error.
pub fn validate(catalog: &Catalog) -> CatalogValidationReport {
    let mut errors = Vec::new();

    // 1. Duplicate ids (report each colliding id once).
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    let mut reported: BTreeSet<&str> = BTreeSet::new();
    for e in &catalog.entries {
        let key = e.id.as_str();
        if !seen.insert(key) && reported.insert(key) {
            errors.push(CatalogValidationError::DuplicateAssetId { id: e.id.clone() });
        }
    }

    // 2. Per-entry payload, typed-slot, source-path, and dependency-existence checks.
    for e in &catalog.entries {
        let is_material = e.kind() == AssetKind::Material;
        match (&e.material, is_material) {
            (None, true) => {
                errors.push(CatalogValidationError::MaterialPayloadMissing { id: e.id.clone() })
            }
            (Some(_), false) => errors.push(CatalogValidationError::MaterialPayloadOnNonMaterial {
                id: e.id.clone(),
                kind: e.kind(),
            }),
            _ => {}
        }

        // The material texture slot must reference a texture asset.
        if let Some(material) = &e.material {
            if let Some(tex) = &material.style.texture {
                if tex.kind() != AssetKind::Texture {
                    errors.push(CatalogValidationError::WrongKindReference {
                        from: e.id.clone(),
                        slot: "material.style.texture",
                        expected: AssetKind::Texture,
                        actual: tex.kind(),
                        reference: tex.id().clone(),
                    });
                }
            }
        }

        if let Some(path) = &e.source_path {
            if path.is_empty() {
                errors.push(CatalogValidationError::EmptySourcePath { id: e.id.clone() });
            }
        }

        for dep in &e.dependencies {
            if !catalog.contains(dep.id()) {
                errors.push(CatalogValidationError::UnknownDependency {
                    from: e.id.clone(),
                    dependency: dep.id().clone(),
                });
            }
        }
    }

    // 3. Dependency cycles (only present edges participate).
    if let Some(path) = DependencyGraph::build(catalog).detect_cycle() {
        errors.push(CatalogValidationError::DependencyCycle { path });
    }

    CatalogValidationReport { errors }
}
