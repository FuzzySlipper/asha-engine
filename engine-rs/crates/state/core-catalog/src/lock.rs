//! Asset locks and catalog-drift validation (scene-capability-03, subtask #2323).
//!
//! A world bundle records an [`AssetLock`]: the id, kind, version, content
//! fingerprint, and dependency ids of every asset it depends on. Loading
//! re-validates the lock against the *current* catalog and classifies any drift
//! (missing, wrong-kind, stale version/hash, dependency drift, or newly added
//! assets). Validation **never silently updates** the lock — it only reports, so a
//! developer decides whether to re-lock.

use std::collections::BTreeSet;

use core_assets::{AssetHash, AssetId, AssetKind};

use crate::entry::Catalog;

/// One locked asset's pinned identity + fingerprint.
#[derive(Debug, Clone, PartialEq)]
pub struct AssetLockEntry {
    pub id: AssetId,
    pub kind: AssetKind,
    pub version: u32,
    pub hash: Option<AssetHash>,
    /// Dependency ids, sorted (canonical).
    pub dependencies: Vec<AssetId>,
}

/// The asset lock embedded in a world bundle. Entries are sorted by id.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AssetLock {
    pub entries: Vec<AssetLockEntry>,
}

/// Generate a lock pinning every asset in `catalog` at its current
/// version/hash/dependencies. Deterministic: entries and dependency lists sorted.
pub fn generate_lock(catalog: &Catalog) -> AssetLock {
    let mut entries: Vec<AssetLockEntry> = catalog
        .entries
        .iter()
        .map(|e| {
            let mut deps: Vec<AssetId> = e.dependencies.iter().map(|d| d.id().clone()).collect();
            deps.sort_by(|a, b| a.as_str().cmp(b.as_str()));
            deps.dedup();
            AssetLockEntry {
                id: e.id.clone(),
                kind: e.kind(),
                version: e.version,
                hash: e.hash.clone(),
                dependencies: deps,
            }
        })
        .collect();
    entries.sort_by(|a, b| a.id.as_str().cmp(b.id.as_str()));
    AssetLock { entries }
}

/// A single classified lock-vs-catalog discrepancy.
#[derive(Debug, Clone, PartialEq)]
pub enum LockIssue {
    /// The locked asset is absent from the current catalog.
    Missing,
    /// The catalog asset has a different kind than the lock recorded.
    WrongKind {
        locked: AssetKind,
        current: AssetKind,
    },
    /// The catalog version differs from the locked version.
    StaleVersion { locked: u32, current: u32 },
    /// The catalog hash differs from the locked hash.
    StaleHash {
        locked: Option<AssetHash>,
        current: Option<AssetHash>,
    },
    /// The asset's dependency set changed since locking.
    DependencyDrift {
        added: Vec<AssetId>,
        removed: Vec<AssetId>,
    },
    /// The asset exists in the catalog but was not in the lock.
    NewInCatalog,
}

impl LockIssue {
    /// Short, stable label for diagnostics.
    pub fn label(&self) -> &'static str {
        match self {
            LockIssue::Missing => "missing",
            LockIssue::WrongKind { .. } => "wrong-kind",
            LockIssue::StaleVersion { .. } => "stale-version",
            LockIssue::StaleHash { .. } => "stale-hash",
            LockIssue::DependencyDrift { .. } => "dependency-drift",
            LockIssue::NewInCatalog => "new-in-catalog",
        }
    }
}

/// One asset's classified lock finding.
#[derive(Debug, Clone, PartialEq)]
pub struct LockFinding {
    pub id: AssetId,
    pub issue: LockIssue,
}

/// The outcome of validating a lock against a catalog.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LockValidationReport {
    pub findings: Vec<LockFinding>,
}

impl LockValidationReport {
    /// `true` when the lock exactly matches the catalog (no drift).
    pub fn is_clean(&self) -> bool {
        self.findings.is_empty()
    }
}

/// Validate `lock` against the current `catalog`, classifying every drift. Does
/// not mutate either side.
pub fn validate_lock(lock: &AssetLock, catalog: &Catalog) -> LockValidationReport {
    let mut findings = Vec::new();
    let locked_ids: BTreeSet<&str> = lock.entries.iter().map(|e| e.id.as_str()).collect();

    for locked in &lock.entries {
        match catalog.get(&locked.id) {
            None => findings.push(LockFinding {
                id: locked.id.clone(),
                issue: LockIssue::Missing,
            }),
            Some(current) => {
                if current.kind() != locked.kind {
                    findings.push(LockFinding {
                        id: locked.id.clone(),
                        issue: LockIssue::WrongKind {
                            locked: locked.kind,
                            current: current.kind(),
                        },
                    });
                    // Kind divergence makes version/hash comparison meaningless.
                    continue;
                }
                if current.version != locked.version {
                    findings.push(LockFinding {
                        id: locked.id.clone(),
                        issue: LockIssue::StaleVersion {
                            locked: locked.version,
                            current: current.version,
                        },
                    });
                }
                if current.hash != locked.hash {
                    findings.push(LockFinding {
                        id: locked.id.clone(),
                        issue: LockIssue::StaleHash {
                            locked: locked.hash.clone(),
                            current: current.hash.clone(),
                        },
                    });
                }
                let (added, removed) = dependency_drift(locked, current);
                if !added.is_empty() || !removed.is_empty() {
                    findings.push(LockFinding {
                        id: locked.id.clone(),
                        issue: LockIssue::DependencyDrift { added, removed },
                    });
                }
            }
        }
    }

    // Assets in the catalog the lock never pinned.
    let mut new_ids: Vec<&AssetId> = catalog
        .entries
        .iter()
        .filter(|e| !locked_ids.contains(e.id.as_str()))
        .map(|e| &e.id)
        .collect();
    new_ids.sort_by(|a, b| a.as_str().cmp(b.as_str()));
    for id in new_ids {
        findings.push(LockFinding {
            id: id.clone(),
            issue: LockIssue::NewInCatalog,
        });
    }

    LockValidationReport { findings }
}

/// `(added, removed)` dependency ids comparing the current entry to the lock.
fn dependency_drift(
    locked: &AssetLockEntry,
    current: &crate::entry::CatalogEntry,
) -> (Vec<AssetId>, Vec<AssetId>) {
    let locked_deps: BTreeSet<&str> = locked.dependencies.iter().map(|d| d.as_str()).collect();
    let current_deps: BTreeSet<String> = current
        .dependencies
        .iter()
        .map(|d| d.id().as_str().to_string())
        .collect();

    let mut added: Vec<AssetId> = current
        .dependencies
        .iter()
        .filter(|d| !locked_deps.contains(d.id().as_str()))
        .map(|d| d.id().clone())
        .collect();
    added.sort_by(|a, b| a.as_str().cmp(b.as_str()));
    added.dedup();

    let mut removed: Vec<AssetId> = locked
        .dependencies
        .iter()
        .filter(|d| !current_deps.contains(d.as_str()))
        .cloned()
        .collect();
    removed.sort_by(|a, b| a.as_str().cmp(b.as_str()));
    removed.dedup();

    (added, removed)
}
