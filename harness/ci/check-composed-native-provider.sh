#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
FIXTURE="$ROOT/harness/fixtures/composed-native-provider"
CONSUMER="$ROOT/harness/fixtures/canonical-project-consumer"
TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target/composed-native-provider}"
OUTPUT_DIR="$ROOT/harness/smoke-out/composed-native-provider"
DESTINATION="$OUTPUT_DIR/asha-composed-native-provider.node"

echo "==> Checking committed canonical project content"
CARGO_TARGET_DIR="$TARGET_DIR" cargo run --locked \
  --manifest-path "$CONSUMER/Cargo.toml" -- --check

echo "==> Building downstream-shaped composed native provider"
CARGO_TARGET_DIR="$TARGET_DIR" cargo build --locked --release \
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

echo "==> Walking canonical project load, authoring save, and packaged reload"
(
  cd "$ROOT/ts"
  pnpm --filter '@asha/browser-host...' build
)
node "$FIXTURE/smoke.mjs" "$DESTINATION" "$ROOT"

echo "Composed native provider gate passed."
