//! Golden-fixture drift + readback for catalog validation, the dependency-cycle
//! diagnostic, and asset-lock drift.
//!
//! Regenerate with:
//!   cargo run -p core-catalog --example dump_catalog > \
//!     harness/fixtures/asset-catalog/sample-catalog.json
//!   cargo run -p core-catalog --example dump_cycle_diagnostic > \
//!     harness/fixtures/asset-catalog/cycle-diagnostic.txt
//!   cargo run -p core-catalog --example dump_lock_drift > \
//!     harness/fixtures/asset-catalog/lock-drift.txt

use std::path::PathBuf;

use core_catalog::{decode, encode, validate, CatalogValidationError};

#[path = "support/fixtures.rs"]
mod fixtures;

fn dir() -> PathBuf {
    // .../engine-rs/crates/state/core-catalog -> repo root is four ancestors up.
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .expect("core-catalog is nested four levels under the repo root")
        .join("harness/fixtures/asset-catalog")
}

#[test]
fn catalog_encoding_matches_committed_golden() {
    let committed =
        std::fs::read_to_string(dir().join("sample-catalog.json")).expect("read sample-catalog");
    let encoded = encode(&fixtures::sample_catalog());
    assert_eq!(
        encoded, committed,
        "catalog encoding drifted from harness/fixtures/asset-catalog/sample-catalog.json; \
         regenerate with `cargo run -p core-catalog --example dump_catalog`"
    );
}

#[test]
fn committed_catalog_decodes_validates_and_round_trips() {
    let committed =
        std::fs::read_to_string(dir().join("sample-catalog.json")).expect("read sample-catalog");
    let catalog = decode(&committed).expect("golden decodes");
    assert!(validate(&catalog).is_ok(), "golden must validate clean");
    assert_eq!(
        encode(&catalog),
        committed,
        "decode∘encode is a fixed point"
    );
    assert_eq!(catalog.canonical(), fixtures::sample_catalog().canonical());
}

#[test]
fn invalid_cycle_fixture_is_classified_with_path() {
    let raw =
        std::fs::read_to_string(dir().join("invalid-cycle.json")).expect("read invalid-cycle");
    // TS-authored data decodes structurally; the cycle is a *semantic* failure.
    let catalog = decode(&raw).expect("decodes");
    let report = validate(&catalog);
    let path = report
        .errors
        .iter()
        .find_map(|e| match e {
            CatalogValidationError::DependencyCycle { path } => Some(path.clone()),
            _ => None,
        })
        .expect("classified cycle");
    assert_eq!(
        path.first().unwrap().as_str(),
        path.last().unwrap().as_str()
    );

    // The rendered diagnostic matches the committed golden.
    let committed =
        std::fs::read_to_string(dir().join("cycle-diagnostic.txt")).expect("read cycle-diagnostic");
    assert_eq!(
        fixtures::render_validation(&report),
        committed,
        "cycle diagnostic drifted; regenerate with `cargo run -p core-catalog --example dump_cycle_diagnostic`"
    );
}

#[test]
fn lock_drift_matches_committed_golden() {
    let committed = std::fs::read_to_string(dir().join("lock-drift.txt")).expect("read lock-drift");
    let rendered = fixtures::render_lock(&fixtures::lock_drift_report());
    assert_eq!(
        rendered, committed,
        "lock drift report drifted; regenerate with `cargo run -p core-catalog --example dump_lock_drift`"
    );
}
