//! Regenerator for the valid-catalog golden fixture.
//!
//! `cargo run -p core-catalog --example dump_catalog` → redirect into
//! `harness/fixtures/asset-catalog/sample-catalog.json`.

use core_catalog::encode;

#[path = "../tests/support/fixtures.rs"]
mod fixtures;

fn main() {
    print!("{}", encode(&fixtures::sample_catalog()));
}
