//! Regenerator for the project-bundle load-plan golden fixture.
//!
//! `cargo run -p svc-serialization --example dump_load_plan` and redirect into
//! `harness/fixtures/project-bundle/load-plan.txt`.

use svc_serialization::LoadPlan;

#[path = "../tests/support/fixtures.rs"]
mod fixtures;

fn main() {
    let plan = LoadPlan::build(&fixtures::sample_manifest()).expect("plan");
    print!("{}", plan.render());
}
