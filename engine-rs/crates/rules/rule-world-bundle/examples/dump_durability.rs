//! Regenerator for the voxel durability checkpoint golden (task #2440).
//!
//! `cargo run -p rule-world-bundle --example dump_durability` and redirect into
//! `harness/fixtures/world-bundle/voxel-durability.txt`.

#[path = "../tests/support/render.rs"]
mod render;

fn main() {
    print!(
        "{}",
        render::render_durability(&render::sample_durability_evidence())
    );
}
