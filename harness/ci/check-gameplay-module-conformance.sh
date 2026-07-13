#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$REPO_ROOT/target/gameplay-module-conformance}"
PUBLIC_CRATE="$REPO_ROOT/public-rust/gameplay-module-conformance/Cargo.toml"
FIXTURE="$REPO_ROOT/harness/fixtures/gameplay-module-sdk/downstream-module/Cargo.toml"
REPORT="$(mktemp)"
trap 'rm -f "$REPORT"' EXIT

echo "==> Checking public gameplay-module conformance crate"
cargo test --locked --offline --manifest-path "$PUBLIC_CRATE"

echo "==> Running downstream module conformance and negative fixtures"
python3 "$REPO_ROOT/harness/identity/execution.py" \
  --execution rust.downstream-gameplay-module \
  --attribution gate.gameplay-module-conformance
cargo run --locked --offline --manifest-path "$FIXTURE" --bin conformance -- --json "$REPORT"
jq -e '.valid == true and (.gaps | length) == 0 and (.checks | all(.passed))' "$REPORT" >/dev/null

echo "==> Checking stable registry, read, state, and binding negatives"
cargo test --locked --offline --manifest-path "$REPO_ROOT/engine-rs/Cargo.toml" -p svc-gameplay-fabric --test registry
cargo test --locked --offline --manifest-path "$REPO_ROOT/engine-rs/Cargo.toml" -p rule-gameplay-fabric
cargo test --locked --offline --manifest-path "$REPO_ROOT/engine-rs/Cargo.toml" -p rule-project-bundle --test gameplay_bindings

echo "Gameplay-module downstream conformance passed."
