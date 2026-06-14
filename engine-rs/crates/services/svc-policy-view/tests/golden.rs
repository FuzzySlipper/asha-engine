//! Golden test for the policy proposal-path replay record (#2392).
//!
//! Pins the committed `harness/fixtures/policy/world-proposals.golden` to the
//! deterministic output of the shared fixture. Regenerate with:
//!   cargo run -p svc-policy-view --example dump_policy_replay > \
//!     harness/fixtures/policy/world-proposals.golden

use std::path::PathBuf;

fn golden_path() -> PathBuf {
    // <repo>/engine-rs/crates/services/svc-policy-view -> up five to <repo>.
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .unwrap()
        .join("harness/fixtures/policy/world-proposals.golden")
}

#[test]
fn proposal_replay_matches_committed_golden() {
    let expected = std::fs::read_to_string(golden_path()).expect("golden fixture is present");
    let actual = svc_policy_view::replay::fixtures::dump();
    assert_eq!(
        actual, expected,
        "policy proposal replay drifted from the committed golden; \
         regenerate with the example if the change is intended"
    );
}
