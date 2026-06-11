#!/usr/bin/env bash
# Build the wasm-api replay module and verify the sim-replay divergence authority
# runs under WASM from TS (ADR 0006 / determinism.md, #2251).
#
# OPT-IN: not part of check-all.sh — needs the wasm32 target + wasm-bindgen CLI.
# When the module is absent, @asha/wasm-replay-bridge's WASM-authority tests skip,
# so offline check-all stays green.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
export PATH="$HOME/.cargo/bin:$PATH"

if ! command -v wasm-bindgen >/dev/null 2>&1; then
  echo "FAIL: wasm-bindgen CLI not found. Install: cargo install wasm-bindgen-cli --version 0.2.123" >&2
  exit 1
fi

WASM_OUT="$REPO_ROOT/ts/packages/wasm-replay-bridge/dist/wasm"
TARGET_WASM="$REPO_ROOT/engine-rs/target/wasm32-unknown-unknown/release/wasm_api.wasm"

echo "==> Building wasm-api for wasm32-unknown-unknown (release)"
cargo build --manifest-path "$REPO_ROOT/engine-rs/Cargo.toml" \
  -p wasm-api --target wasm32-unknown-unknown --release

echo "==> wasm-bindgen --target nodejs -> $WASM_OUT"
mkdir -p "$WASM_OUT"
wasm-bindgen --target nodejs --out-dir "$WASM_OUT" "$TARGET_WASM"
# The package is type:module; the nodejs glue is CommonJS — give it a .cjs extension.
mv -f "$WASM_OUT/wasm_api.js" "$WASM_OUT/wasm_api.cjs"

echo "==> wasm-replay-bridge tests (WASM-authority tests now run, not skipped)"
( cd "$REPO_ROOT/ts" && pnpm --filter @asha/wasm-replay-bridge test )
