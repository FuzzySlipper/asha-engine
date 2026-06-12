//! Shared catalog builders + deterministic renderers for the golden tests and
//! regenerator examples. Abstract fixture nouns only (no product-domain content).
#![allow(dead_code)]

use core_assets::{AssetHash, AssetId, AssetReference, AssetVersionReq};
use core_catalog::{
    AssetLock, Catalog, CatalogEntry, CatalogValidationReport, LockValidationReport,
    MaterialAuthority, MaterialDef, MaterialStyle, Rgba, StructuralClass,
};

fn id(s: &str) -> AssetId {
    AssetId::parse(s).unwrap()
}

fn dep(s: &str) -> AssetReference {
    AssetReference::new(id(s), AssetVersionReq::Any, None)
}

/// A valid abstract catalog: texture ← material ← static mesh.
pub fn sample_catalog() -> Catalog {
    let texture = CatalogEntry::new(id("texture/surface-atlas-a"), 1)
        .with_hash(AssetHash::parse("aa01").unwrap())
        .with_source("textures/surface-atlas-a.png")
        .with_label("Surface Atlas A");

    let material = MaterialDef {
        authority: MaterialAuthority {
            solid: true,
            collidable: true,
            occludes: true,
            structural_class: StructuralClass::Structural,
        },
        style: MaterialStyle {
            texture: Some(dep("texture/surface-atlas-a")),
            ..MaterialStyle::flat(Rgba::DEBUG_GREY)
        },
    };
    let material_entry = CatalogEntry::new(id("material/surface-a"), 2)
        .with_hash(AssetHash::parse("bb02").unwrap())
        .with_material(material)
        .with_dependencies(vec![dep("texture/surface-atlas-a")]);

    let mesh = CatalogEntry::new(id("mesh/fixture-a"), 1)
        .with_hash(AssetHash::parse("cc03").unwrap())
        .with_dependencies(vec![dep("material/surface-a")]);

    Catalog::from_entries(vec![texture, material_entry, mesh])
}

/// The sample catalog drifted: mesh version bumped and texture hash changed, plus
/// a new sprite asset — used for the lock-drift golden.
pub fn drifted_catalog() -> Catalog {
    let mut c = sample_catalog();
    c.entries[2].version = 7;
    c.entries[0].hash = Some(AssetHash::parse("ffff").unwrap());
    c.entries.push(CatalogEntry::new(id("sprite/new-a"), 1));
    c
}

/// Render a catalog validation report deterministically, one error per line.
pub fn render_validation(report: &CatalogValidationReport) -> String {
    use core::fmt::Write;
    let mut s = String::new();
    let _ = writeln!(s, "errors {}", report.errors.len());
    for e in &report.errors {
        use core_catalog::CatalogValidationError as E;
        let detail = match e {
            E::DuplicateAssetId { id } => id.as_str().to_string(),
            E::MaterialPayloadMissing { id } => id.as_str().to_string(),
            E::MaterialPayloadOnNonMaterial { id, kind } => format!("{} ({kind})", id.as_str()),
            E::WrongKindReference {
                from,
                slot,
                expected,
                actual,
                reference,
            } => format!(
                "{} {slot} expected={expected} actual={actual} ref={}",
                from.as_str(),
                reference.as_str()
            ),
            E::UnknownDependency { from, dependency } => {
                format!("{} -> {}", from.as_str(), dependency.as_str())
            }
            E::DependencyCycle { path } => path
                .iter()
                .map(|i| i.as_str())
                .collect::<Vec<_>>()
                .join(" -> "),
            E::EmptySourcePath { id } => id.as_str().to_string(),
        };
        let _ = writeln!(s, "  {} {detail}", e.label());
    }
    s
}

/// Render a lock validation report deterministically, one finding per line.
pub fn render_lock(report: &LockValidationReport) -> String {
    use core::fmt::Write;
    use core_catalog::LockIssue as I;
    let mut s = String::new();
    let _ = writeln!(s, "findings {}", report.findings.len());
    for f in &report.findings {
        let detail = match &f.issue {
            I::Missing | I::NewInCatalog => String::new(),
            I::WrongKind { locked, current } => format!("locked={locked} current={current}"),
            I::StaleVersion { locked, current } => format!("locked={locked} current={current}"),
            I::StaleHash { locked, current } => format!(
                "locked={} current={}",
                locked.as_ref().map(|h| h.as_str()).unwrap_or("none"),
                current.as_ref().map(|h| h.as_str()).unwrap_or("none"),
            ),
            I::DependencyDrift { added, removed } => format!(
                "added=[{}] removed=[{}]",
                added
                    .iter()
                    .map(|i| i.as_str())
                    .collect::<Vec<_>>()
                    .join(","),
                removed
                    .iter()
                    .map(|i| i.as_str())
                    .collect::<Vec<_>>()
                    .join(","),
            ),
        };
        let _ = writeln!(s, "  {} {} {detail}", f.id.as_str(), f.issue.label());
    }
    s
}

/// Build, generate a lock from the sample catalog, and validate it against the
/// drifted catalog — the lock-drift golden subject.
pub fn lock_drift_report() -> LockValidationReport {
    let lock: AssetLock = core_catalog::generate_lock(&sample_catalog());
    core_catalog::validate_lock(&lock, &drifted_catalog())
}
