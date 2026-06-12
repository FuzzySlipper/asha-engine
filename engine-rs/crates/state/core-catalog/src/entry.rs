//! Catalog entries and the catalog container (scene-capability-03, subtask #2322).
//!
//! A [`Catalog`] is the Rust-validated registry of asset definitions a TS layer
//! may author as data. Each [`CatalogEntry`] carries stable identity (an
//! [`AssetId`] from `core-assets`), a version, an optional content hash and source
//! path (path ≠ identity), a display label, its outgoing asset dependencies, and —
//! for material assets — the [`MaterialDef`] authority/style payload.

use core_assets::{AssetHash, AssetId, AssetKind, AssetReference};

use crate::material::MaterialDef;

/// One asset definition in a catalog.
#[derive(Debug, Clone, PartialEq)]
pub struct CatalogEntry {
    /// Stable, kind-prefixed identity. The entry's kind is `id.kind()`.
    pub id: AssetId,
    /// Catalog version of this asset (monotonic; bumped on content change).
    pub version: u32,
    /// Optional content fingerprint. Source path may change; this and the id are
    /// what survive project moves.
    pub hash: Option<AssetHash>,
    /// Optional on-disk source path — metadata only, never identity.
    pub source_path: Option<String>,
    /// Optional human-readable label — metadata only, never identity.
    pub label: Option<String>,
    /// Outgoing asset dependencies (e.g. mesh → material, material → texture).
    pub dependencies: Vec<AssetReference>,
    /// Material payload, required iff `id.kind() == Material` (enforced by
    /// validation), forbidden otherwise.
    pub material: Option<MaterialDef>,
}

impl CatalogEntry {
    /// A minimal entry with no dependencies, hash, path, label, or material.
    pub fn new(id: AssetId, version: u32) -> CatalogEntry {
        CatalogEntry {
            id,
            version,
            hash: None,
            source_path: None,
            label: None,
            dependencies: Vec::new(),
            material: None,
        }
    }

    /// The entry's asset kind (from its id).
    pub fn kind(&self) -> AssetKind {
        self.id.kind()
    }

    /// Builder: set the content hash.
    pub fn with_hash(mut self, hash: AssetHash) -> Self {
        self.hash = Some(hash);
        self
    }

    /// Builder: set the source path.
    pub fn with_source(mut self, path: impl Into<String>) -> Self {
        self.source_path = Some(path.into());
        self
    }

    /// Builder: set the display label.
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Builder: set the dependencies.
    pub fn with_dependencies(mut self, deps: Vec<AssetReference>) -> Self {
        self.dependencies = deps;
        self
    }

    /// Builder: set the material payload.
    pub fn with_material(mut self, material: MaterialDef) -> Self {
        self.material = Some(material);
        self
    }
}

/// A registry of asset definitions. Construction does not validate; call
/// [`crate::validate`].
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Catalog {
    pub entries: Vec<CatalogEntry>,
}

impl Catalog {
    /// An empty catalog.
    pub fn new() -> Catalog {
        Catalog {
            entries: Vec::new(),
        }
    }

    /// Build from a list of entries.
    pub fn from_entries(entries: Vec<CatalogEntry>) -> Catalog {
        Catalog { entries }
    }

    /// Find an entry by id.
    pub fn get(&self, id: &AssetId) -> Option<&CatalogEntry> {
        self.entries.iter().find(|e| &e.id == id)
    }

    /// Whether an asset id is present.
    pub fn contains(&self, id: &AssetId) -> bool {
        self.entries.iter().any(|e| &e.id == id)
    }

    /// A copy with entries sorted by id (deterministic on-disk/lock order).
    pub fn canonical(&self) -> Catalog {
        let mut c = self.clone();
        c.entries.sort_by(|a, b| a.id.as_str().cmp(b.id.as_str()));
        c
    }
}
