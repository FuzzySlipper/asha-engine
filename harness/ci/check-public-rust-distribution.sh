#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

echo "==> Proving exact-revision public Rust Git consumption"
python3 "$REPO_ROOT/harness/public-surface/check-public-rust-distribution.py"

