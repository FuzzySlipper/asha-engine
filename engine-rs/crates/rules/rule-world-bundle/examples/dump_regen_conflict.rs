//! Regenerator for the regenerate-and-replay conflict-diagnostic golden fixture.
//!
//! `cargo run -p rule-world-bundle --example dump_regen_conflict` and redirect into
//! `harness/fixtures/world-bundle/regen-conflict.txt`.

#[path = "../tests/support/render.rs"]
mod render;

fn main() {
    print!("{}", render::render_report(&render::conflict_report()));
}
