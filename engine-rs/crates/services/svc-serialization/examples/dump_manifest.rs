//! Regenerator for the project-bundle manifest golden fixture.
//!
//! `cargo run -p svc-serialization --example dump_manifest` and redirect into
//! `harness/fixtures/project-bundle/sample-manifest.json`.

use svc_serialization::encode;

#[path = "../tests/support/fixtures.rs"]
mod fixtures;

fn main() {
    print!("{}", encode(&fixtures::sample_manifest()));
}
