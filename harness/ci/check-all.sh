#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

run() { echo "==> $*"; "$@"; }

run "$REPO_ROOT/harness/ci/check-rust.sh"
run "$REPO_ROOT/harness/ci/check-ts.sh"
run "$REPO_ROOT/harness/ci/check-contracts.sh"
run "$REPO_ROOT/harness/ci/check-depgraph.sh"
run "$REPO_ROOT/harness/ci/check-bridge.sh"
run "$REPO_ROOT/harness/ci/check-replays.sh"
run "$REPO_ROOT/harness/ci/check-render-goldens.sh"

echo ""
echo "All checks passed."
