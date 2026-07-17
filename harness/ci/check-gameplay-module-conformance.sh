#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
PUBLIC_CRATE="$REPO_ROOT/public-rust/gameplay-module-conformance/Cargo.toml"
FIXTURE="$REPO_ROOT/harness/fixtures/gameplay-module-sdk/downstream-module/Cargo.toml"
REPORT="$(mktemp)"
trap 'rm -f "$REPORT"' EXIT

echo "==> Checking public gameplay-module conformance crate"
cargo fetch --locked --manifest-path "$PUBLIC_CRATE"
cargo test --locked --offline --manifest-path "$PUBLIC_CRATE"

echo "==> Running downstream module conformance and negative fixtures"
cargo fetch --locked --manifest-path "$FIXTURE"
python3 "$REPO_ROOT/harness/identity/execution.py" \
  --execution rust.downstream-gameplay-module \
  --attribution gate.gameplay-module-conformance
cargo run --locked --offline --manifest-path "$FIXTURE" --bin conformance -- --json "$REPORT"
jq -e '.valid == true and (.gaps | length) == 0 and (.checks | all(.passed))' "$REPORT" >/dev/null

echo "Gameplay-module downstream conformance passed."
