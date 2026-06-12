//! Golden-fixture drift for the compacted save bundle section (#2320) and the
//! regenerate-and-replay conflict diagnostic (#2321).
//!
//! Regenerate with:
//!   cargo run -p rule-world-bundle --example dump_compacted_save > \
//!     harness/fixtures/world-bundle/compacted-save.txt
//!   cargo run -p rule-world-bundle --example dump_regen_conflict > \
//!     harness/fixtures/world-bundle/regen-conflict.txt

use std::path::PathBuf;

#[path = "support/render.rs"]
mod render;

fn dir() -> PathBuf {
    // .../engine-rs/crates/rules/rule-world-bundle -> repo root is four up.
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .expect("crate is nested four levels under the repo root")
        .join("harness/fixtures/world-bundle")
}

#[test]
fn compacted_save_matches_committed_golden() {
    let committed =
        std::fs::read_to_string(dir().join("compacted-save.txt")).expect("read compacted-save");
    let rendered = render::render_compacted_save(&render::sample_compacted_save());
    assert_eq!(
        rendered, committed,
        "compacted save section drifted from harness/fixtures/world-bundle/compacted-save.txt; \
         regenerate with `cargo run -p rule-world-bundle --example dump_compacted_save`"
    );
}

#[test]
fn regen_conflict_matches_committed_golden() {
    let committed =
        std::fs::read_to_string(dir().join("regen-conflict.txt")).expect("read regen-conflict");
    let report = render::conflict_report();
    // The fixture must be a genuine conflict, not an accidentally-clean replay.
    assert!(
        !report.is_clean(),
        "regen-conflict fixture must report at least one conflict"
    );
    let rendered = render::render_report(&report);
    assert_eq!(
        rendered, committed,
        "regen conflict diagnostic drifted from harness/fixtures/world-bundle/regen-conflict.txt; \
         regenerate with `cargo run -p rule-world-bundle --example dump_regen_conflict`"
    );
}
