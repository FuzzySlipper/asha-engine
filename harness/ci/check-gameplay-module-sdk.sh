#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$REPO_ROOT/target/gameplay-module-sdk}"
FIXTURE="$REPO_ROOT/harness/fixtures/gameplay-module-sdk/downstream-module"
SCRATCH="$(mktemp -d)"
trap 'rm -rf "$SCRATCH"' EXIT

echo "==> Checking public gameplay-module facade"
if rg -n --fixed-strings "engine-rs/crates" "$FIXTURE/Cargo.toml"; then
  echo "Downstream gameplay-module fixture must depend only on the public facade." >&2
  exit 1
fi
python3 "$REPO_ROOT/harness/identity/execution.py" \
  --execution rust.downstream-gameplay-module \
  --attribution gate.gameplay-module-sdk

echo "==> Checking gameplay-module scaffold"
"$REPO_ROOT/harness/tools/new-gameplay-module.sh" \
  "$SCRATCH/scaffolded-module" \
  "scaffolded-gameplay-module" \
  "fixture.scaffolded.module"

echo "Gameplay-module SDK public fixture and scaffold passed."
