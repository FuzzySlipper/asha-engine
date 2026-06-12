//! Regenerator for the asset-lock drift golden fixture.
//!
//! `cargo run -p core-catalog --example dump_lock_drift` → redirect into
//! `harness/fixtures/asset-catalog/lock-drift.txt`.

#[path = "../tests/support/fixtures.rs"]
mod fixtures;

fn main() {
    print!("{}", fixtures::render_lock(&fixtures::lock_drift_report()));
}
