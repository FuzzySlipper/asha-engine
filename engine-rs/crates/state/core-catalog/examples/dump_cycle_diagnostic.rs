//! Regenerator for the dependency-cycle diagnostic golden fixture.
//!
//! Reads the committed `invalid-cycle.json` (TS-authored data), validates it in
//! Rust, and renders the classified report.
//! `cargo run -p core-catalog --example dump_cycle_diagnostic` → redirect into
//! `harness/fixtures/asset-catalog/cycle-diagnostic.txt`.

use std::path::PathBuf;

use core_catalog::{decode, validate};

#[path = "../tests/support/fixtures.rs"]
mod fixtures;

fn main() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .unwrap()
        .join("harness/fixtures/asset-catalog/invalid-cycle.json");
    let catalog = decode(&std::fs::read_to_string(path).unwrap()).unwrap();
    print!("{}", fixtures::render_validation(&validate(&catalog)));
}
