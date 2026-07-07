//! Golden test for the imported-artifact bytes (#2384, #2385).
//!
//! Pins `harness/fixtures/asset-import/imported.golden` to the deterministic output
//! of `dump_import`. Regenerate with:
//!   cargo run -p asset-import --example dump_import > \
//!     harness/fixtures/asset-import/imported.golden

use std::path::PathBuf;

use asset_import::{artifacts, fixtures, import_text, manifest, IMPORTER_VERSION};

fn golden_path() -> PathBuf {
    let rel = "harness/fixtures/asset-import/imported.golden";
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .ancestors()
        .map(|ancestor| ancestor.join(rel))
        .find(|candidate| candidate.exists())
        .unwrap_or_else(|| manifest_dir.join(rel))
}

fn dump_one(label: &str, source_path: &str, source: &str, out: &mut String) {
    let outcome = import_text(source, source_path);
    let assets = outcome.assets.expect("fixture imports cleanly");
    let name = assets
        .static_mesh
        .asset
        .strip_prefix("mesh/")
        .unwrap()
        .to_string();
    let arts = artifacts::render_artifacts(&name, &assets);
    out.push_str(&format!("=== {label} ===\n"));
    for art in &arts {
        out.push_str(&format!("--- {} ---\n", art.rel_path));
        out.push_str(&art.contents);
    }
    let m = manifest::build_manifest(
        source_path,
        source,
        IMPORTER_VERSION,
        1,
        &assets.static_mesh.asset,
        &arts,
    );
    out.push_str(&format!("--- {name}.import.json ---\n"));
    out.push_str(&m.render());
    out.push('\n');
}

fn rendered() -> String {
    let mut out = String::new();
    dump_one(
        "triangle (textured, aabb collision)",
        "fixtures/import-fixture-a.mesh.json",
        fixtures::VALID_TRIANGLE,
        &mut out,
    );
    dump_one(
        "quad (two material slots)",
        "fixtures/import-fixture-b.mesh.json",
        fixtures::VALID_QUAD,
        &mut out,
    );
    out
}

#[test]
fn imported_artifacts_match_committed_golden() {
    let expected = std::fs::read_to_string(golden_path()).expect("golden fixture present");
    assert_eq!(
        rendered(),
        expected,
        "imported artifacts drifted from the committed golden; regenerate with the example if intended"
    );
}
