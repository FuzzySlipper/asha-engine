#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
FIXTURE="$ROOT/harness/fixtures/composed-native-provider"
TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target/composed-native-provider}"
OUTPUT_DIR="$ROOT/harness/smoke-out/composed-native-provider"
DESTINATION="$OUTPUT_DIR/asha-composed-native-provider.node"

echo "==> Building downstream-shaped composed native provider"
CARGO_TARGET_DIR="$TARGET_DIR" cargo build --locked --offline --release \
  --manifest-path "$FIXTURE/Cargo.toml"

ARTIFACT="$(find "$TARGET_DIR/release" -maxdepth 1 \
  \( -name 'libasha_composed_native_provider_fixture.so' \
     -o -name 'libasha_composed_native_provider_fixture.dylib' \
     -o -name 'asha_composed_native_provider_fixture.dll' \) \
  | head -1)"
if [[ -z "$ARTIFACT" ]]; then
  echo "FAIL: composed native provider artifact was not produced" >&2
  exit 1
fi

mkdir -p "$OUTPUT_DIR"
"$ROOT/harness/ci/install-native-addon.sh" "$ARTIFACT" "$DESTINATION"

echo "==> Verifying generated exports and composed primary-fire authority"
node "$FIXTURE/smoke.mjs" "$DESTINATION" "$ROOT"

echo "Composed native provider gate passed."
