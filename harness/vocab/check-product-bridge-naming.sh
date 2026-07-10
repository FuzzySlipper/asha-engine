#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
BRIDGE_ROOT="$REPO_ROOT/engine-rs/crates/bridge"
API_SOURCE="$BRIDGE_ROOT/runtime-bridge-api/src"

if [[ -e "$API_SOURCE/reference" ]]; then
  echo "FAIL: Rust product RuntimeBridge authority must not live under src/reference" >&2
  exit 1
fi

if rg -n --glob '*.rs' '\bReferenceBridge\b' "$BRIDGE_ROOT"; then
  echo "FAIL: ReferenceBridge is reserved for non-product fixture vocabulary" >&2
  exit 1
fi

if ! rg -q 'pub use authority::EngineBridge;' "$API_SOURCE/lib.rs"; then
  echo "FAIL: runtime-bridge-api must export authority::EngineBridge" >&2
  exit 1
fi

echo "Rust product bridge authority naming: OK"
