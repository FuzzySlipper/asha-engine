#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

echo "==> Running term-gravity self-test"
bash "$REPO_ROOT/harness/vocab/check-term-gravity.sh" --self-test

echo "==> Checking ECRP vocabulary term gravity"
bash "$REPO_ROOT/harness/vocab/check-term-gravity.sh"

echo "==> Checking Rust product bridge authority naming"
bash "$REPO_ROOT/harness/vocab/check-product-bridge-naming.sh"
