#!/usr/bin/env bash
# Runtime bridge boundary checks (ADR 0006, tasks #2249+).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

echo "==> Validating curated bridge manifest"
python3 "$REPO_ROOT/harness/bridge/validate-manifest.py"

echo "==> Scanning bridge guardrails (no opaque escape hatches in stable surfaces)"
bash "$REPO_ROOT/harness/bridge/check-bridge-guardrails.sh"
