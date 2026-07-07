//! Golden test for the CLI dry-run report (#2386).
//!
//! Pins `harness/fixtures/asset-import/cli-report.golden`. Regenerate with:
//!   cargo run -p asset-import --example dump_cli_report > \
//!     harness/fixtures/asset-import/cli-report.golden

use std::path::PathBuf;

use asset_import::cli::{plan, Mode};
use asset_import::fixtures;

fn golden_path() -> PathBuf {
    let rel = "harness/fixtures/asset-import/cli-report.golden";
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .ancestors()
        .map(|ancestor| ancestor.join(rel))
        .find(|candidate| candidate.exists())
        .unwrap_or_else(|| manifest_dir.join(rel))
}

#[test]
fn cli_report_matches_committed_golden() {
    let expected = std::fs::read_to_string(golden_path()).expect("golden present");
    let report = plan(
        "import-fixture-a.mesh.json",
        fixtures::VALID_TRIANGLE,
        &Mode::DryRun,
        None,
    )
    .report;
    assert_eq!(
        report, expected,
        "CLI report drifted from the committed golden"
    );
}
