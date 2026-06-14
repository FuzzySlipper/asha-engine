//! Print the canonical policy proposal-path tick record. The committed golden in
//! `harness/fixtures/policy/world-proposals.golden` is this output; the
//! `proposal_replay_golden` test pins it.

fn main() {
    print!("{}", svc_policy_view::replay::fixtures::dump());
}
