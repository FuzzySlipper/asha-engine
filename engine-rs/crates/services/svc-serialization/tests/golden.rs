//! Golden-fixture drift + readback for the project-bundle manifest and load plan.
//!
//! Pins the committed fixtures against the in-crate builders so a serialization or
//! load-ordering change fails loudly, and proves the committed bytes decode,
//! validate, and re-encode/replan to themselves.
//!
//! Regenerate with:
//!   cargo run -p svc-serialization --example dump_manifest > \
//!     harness/fixtures/project-bundle/sample-manifest.json
//!   cargo run -p svc-serialization --example dump_load_plan > \
//!     harness/fixtures/project-bundle/load-plan.txt

use std::path::PathBuf;

use svc_serialization::{decode, encode, LoadPlan};

#[path = "support/fixtures.rs"]
mod fixtures;

fn dir() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .ancestors()
        .find(|ancestor| ancestor.join("engine-rs").is_dir() && ancestor.join("harness").is_dir())
        .expect("repo root")
        .join("harness/fixtures/project-bundle")
}

#[test]
fn manifest_encoding_matches_committed_golden() {
    let committed =
        std::fs::read_to_string(dir().join("sample-manifest.json")).expect("read sample-manifest");
    let encoded = encode(&fixtures::sample_manifest());
    assert_eq!(
        encoded, committed,
        "manifest encoding drifted from harness/fixtures/project-bundle/sample-manifest.json; \
         regenerate with `cargo run -p svc-serialization --example dump_manifest`"
    );
}

#[test]
fn committed_manifest_decodes_validates_and_round_trips() {
    let committed =
        std::fs::read_to_string(dir().join("sample-manifest.json")).expect("read sample-manifest");
    let manifest = decode(&committed).expect("golden decodes");
    assert!(manifest.validate().is_ok(), "golden must validate clean");
    assert_eq!(
        encode(&manifest),
        committed,
        "decode∘encode is a fixed point"
    );
    assert_eq!(
        manifest.canonical(),
        fixtures::sample_manifest().canonical()
    );
    // Cache removal must not change the durable load set.
    assert_eq!(
        manifest
            .without_cache()
            .load_required_artifacts()
            .iter()
            .map(|a| a.path.clone())
            .collect::<Vec<_>>(),
        manifest
            .load_required_artifacts()
            .iter()
            .map(|a| a.path.clone())
            .collect::<Vec<_>>(),
    );
}

#[test]
fn load_plan_matches_committed_golden_and_is_stable() {
    let committed =
        std::fs::read_to_string(dir().join("load-plan.txt")).expect("read load-plan.txt");
    let manifest = fixtures::sample_manifest();
    let rendered = LoadPlan::build(&manifest).expect("plan").render();
    assert_eq!(
        rendered, committed,
        "load plan drifted from harness/fixtures/project-bundle/load-plan.txt; \
         regenerate with `cargo run -p svc-serialization --example dump_load_plan`"
    );
    // Build order is stable across runs.
    assert_eq!(rendered, LoadPlan::build(&manifest).unwrap().render());
}
