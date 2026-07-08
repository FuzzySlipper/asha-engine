//! Regenerator for the bootstrap golden fixture.
//!
//! Bootstraps the committed `sample-flat.json` into world id 7 and prints a
//! deterministic JSON summary of the single `BootstrapRecord`. Run with
//! `cargo run -p core-scene --example dump_bootstrap_summary` and redirect into
//! `harness/fixtures/scenes/bootstrap-summary.json`. The `bootstrap_golden`
//! integration test pins the committed bytes against this same computation.

use std::path::PathBuf;

use core_ids::RuntimeSessionId;
use core_scene::{bootstrap_scene, decode};

fn main() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .unwrap()
        .join("harness/fixtures/scenes/sample-flat.json");
    let doc = decode(&std::fs::read_to_string(path).unwrap()).unwrap();
    let (_world, record) = bootstrap_scene(&doc, RuntimeSessionId::new(7)).unwrap();
    print!("{}", core_scene_bootstrap_summary::render(&record));
}

/// Inline module so the golden test and this regenerator share one renderer.
#[path = "../tests/support/bootstrap_summary_fmt.rs"]
mod core_scene_bootstrap_summary;
