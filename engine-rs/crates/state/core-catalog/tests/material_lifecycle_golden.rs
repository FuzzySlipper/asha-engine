//! Material/texture update-lifecycle diagnostic golden (material-wiring super,
//! epic #2353; subtask #2376).
//!
//! Renders the change-impact classifications a material edit produces — live-safe
//! visual reproject vs authority-impacting vs requires-full-reload — plus a
//! fallback-used line, to a deterministic text golden. The golden doubles as an
//! agent-training example of the "do not silently no-op an unsafe update" rule:
//! an unsafe edit reports `requires-full-reload`, never a partial live mutation.
//!
//! Regenerate: `BLESS=1 cargo test -p core-catalog --test material_lifecycle_golden`.

use std::path::PathBuf;

use core_assets::{AssetId, AssetReference, AssetVersionReq};
use core_catalog::material::{
    MaterialAuthority, MaterialDef, MaterialStyle, Rgba, StructuralClass,
};
use core_catalog::{
    material_change_impact, revalidate_asset, Catalog, CatalogEntry, ChangeImpactReport,
    ChangeKind, ReloadSuggestion,
};

fn golden_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../../harness/fixtures/materials/material-change-impact.txt")
}

fn check_golden(actual: &str) {
    let path = golden_path();
    if std::env::var_os("BLESS").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, actual).unwrap();
        return;
    }
    let expected = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read {}: {e} (BLESS=1 to create)", path.display()));
    assert_eq!(actual, expected, "material change-impact golden drifted");
}

fn material(color: Rgba, solid: bool) -> MaterialDef {
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

/// `mesh/wall` depends on `material/brick`, so a brick change has a dependent.
fn catalog() -> Catalog {
    Catalog {
        entries: vec![
            CatalogEntry::new(AssetId::parse("material/brick").unwrap(), 1)
                .with_material(material(Rgba::WHITE, true)),
            CatalogEntry::new(AssetId::parse("mesh/wall").unwrap(), 1).with_dependencies(vec![
                AssetReference::new(
                    AssetId::parse("material/brick").unwrap(),
                    AssetVersionReq::Any,
                    None,
                ),
            ]),
        ],
    }
}

fn suggestion_label(s: ReloadSuggestion) -> &'static str {
    match s {
        ReloadSuggestion::Reproject => "reproject",
        ReloadSuggestion::RevalidateDependents => "revalidate-dependents",
        ReloadSuggestion::RequiresFullReload => "requires-full-reload",
    }
}

fn change_label(c: ChangeKind) -> &'static str {
    match c {
        ChangeKind::VisualOnly => "visual-only",
        ChangeKind::AuthorityImpacting => "authority-impacting",
        ChangeKind::Structural => "structural",
    }
}

fn render_report(scenario: &str, r: &ChangeImpactReport) -> String {
    let deps: Vec<&str> = r.affected_dependents.iter().map(|a| a.as_str()).collect();
    format!(
        "{scenario}\n  asset: {}\n  change: {}\n  safe: {}\n  requires_full_reload: {}\n  suggestion: {}\n  affected_dependents: [{}]\n",
        r.asset.as_str(),
        change_label(r.change),
        r.safe,
        r.requires_full_reload,
        suggestion_label(r.suggestion),
        deps.join(", "),
    )
}

#[test]
fn material_change_impact_renders_to_the_committed_golden() {
    let catalog = catalog();
    let brick = AssetId::parse("material/brick").unwrap();
    let base = material(Rgba::WHITE, true);

    // 1) Visual-only edit (recolour) — live-safe reproject.
    let recolour = material(Rgba::DEBUG_GREY, true);
    let visual = material_change_impact(&catalog, &brick, &base, &recolour).unwrap();

    // 2) Authority edit (now non-collidable) — not live-safe.
    let soften = material(Rgba::WHITE, false);
    let authority = material_change_impact(&catalog, &brick, &base, &soften).unwrap();

    // 3) Structural edit — cannot apply live, requires full reload.
    let structural = revalidate_asset(&catalog, &brick, ChangeKind::Structural).unwrap();

    let mut out = String::new();
    out.push_str(&render_report("visual-only-recolour", &visual));
    out.push('\n');
    out.push_str(&render_report("authority-soften", &authority));
    out.push('\n');
    out.push_str(&render_report("structural-change", &structural));
    out.push('\n');
    // A fallback-used line: a referenced cosmetic material with no definition draws
    // the deterministic grey placeholder (see renderer fallbackMaterials()).
    out.push_str("fallback-used\n  material: material/missing-decal\n  visual: debug-grey\n");

    check_golden(&out);
}
