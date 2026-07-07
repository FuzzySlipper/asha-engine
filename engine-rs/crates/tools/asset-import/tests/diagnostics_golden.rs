//! Golden test for classified import diagnostics (#2385).
//!
//! Pins `harness/fixtures/asset-import/diagnostics.golden`. Regenerate with:
//!   cargo run -p asset-import --example dump_import_diagnostics > \
//!     harness/fixtures/asset-import/diagnostics.golden

use std::path::PathBuf;

use asset_import::{
    fingerprint, fixtures, import_text, import_with_context, manifest, parse_source, ImportContext,
    ImportDiagnostic,
};

fn golden_path() -> PathBuf {
    let rel = "harness/fixtures/asset-import/diagnostics.golden";
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .ancestors()
        .map(|ancestor| ancestor.join(rel))
        .find(|candidate| candidate.exists())
        .unwrap_or_else(|| manifest_dir.join(rel))
}

fn render(diags: &[ImportDiagnostic], out: &mut String) {
    if diags.is_empty() {
        out.push_str("(none)\n");
    }
    for d in diags {
        out.push_str(&d.render());
        out.push('\n');
    }
}

fn rendered() -> String {
    let mut out = String::new();
    out.push_str("=== unsupported feature ===\n");
    render(
        &import_text(
            fixtures::UNSUPPORTED_FEATURE,
            "fixtures/unsupported.mesh.json",
        )
        .diagnostics,
        &mut out,
    );
    out.push_str("=== non-triangle topology ===\n");
    render(
        &import_text(fixtures::BAD_TOPOLOGY, "fixtures/bad-topology.mesh.json").diagnostics,
        &mut out,
    );
    out.push_str("=== missing external texture ===\n");
    let parsed = parse_source(
        fixtures::VALID_TRIANGLE,
        "fixtures/import-fixture-a.mesh.json",
    )
    .mesh
    .unwrap();
    let outcome = import_with_context(&parsed, &ImportContext::with_textures(Vec::<String>::new()));
    render(&outcome.diagnostics, &mut out);
    out.push_str("=== source fingerprint drift vs asset lock ===\n");
    let locked = fingerprint::fingerprint_hex(fixtures::VALID_TRIANGLE.as_bytes());
    let changed_source = fixtures::VALID_TRIANGLE.replace("0.8", "0.9");
    let current = fingerprint::fingerprint_hex(changed_source.as_bytes());
    match manifest::detect_source_drift(&locked, &current, "mesh/import-fixture-a") {
        Some(d) => out.push_str(&format!("{}\n", d.render())),
        None => out.push_str("(no drift)\n"),
    }
    out
}

#[test]
fn import_diagnostics_match_committed_golden() {
    let expected = std::fs::read_to_string(golden_path()).expect("golden present");
    assert_eq!(
        rendered(),
        expected,
        "import diagnostics drifted from the committed golden"
    );
}
