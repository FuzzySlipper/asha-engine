//! Golden-fixture drift guard for the cross-vocabulary entity matrix.
//!
//! Pins `harness/fixtures/entities/families.golden` against the in-crate builders
//! so a model/serialization change fails loudly. Regenerate with:
//!   cargo run -p core-entity --example dump_entity_fixtures > \
//!     harness/fixtures/entities/families.golden

use std::path::PathBuf;

fn golden_path() -> PathBuf {
    // .../engine-rs/crates/state/core-entity -> repo root is four ancestors up.
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .expect("core-entity is nested four levels under the repo root")
        .join("harness/fixtures/entities/families.golden")
}

#[test]
fn fixture_dump_matches_golden() {
    let actual = core_entity::fixtures::dump_all_families();
    let expected = std::fs::read_to_string(golden_path()).expect("read families.golden");
    assert_eq!(
        actual, expected,
        "entity fixture dump drifted from the golden; regenerate with the example if intended"
    );
}
