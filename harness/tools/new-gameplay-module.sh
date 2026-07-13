#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 3 ]]; then
  echo "usage: $0 <destination> <crate-name> <module-id>" >&2
  exit 2
fi

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$REPO_ROOT/target/gameplay-module-scaffolds}"
TEMPLATE="$REPO_ROOT/harness/templates/gameplay-module"
DESTINATION="$1"
CRATE_NAME="$2"
MODULE_ID="$3"

if [[ -e "$DESTINATION" ]]; then
  echo "destination already exists: $DESTINATION" >&2
  exit 1
fi
if [[ ! "$CRATE_NAME" =~ ^[a-z][a-z0-9-]*$ ]]; then
  echo "crate name must be lowercase kebab-case" >&2
  exit 1
fi
if [[ ! "$MODULE_ID" =~ ^[a-z][a-z0-9.-]*$ ]]; then
  echo "module id must be lowercase dot/kebab scoped" >&2
  exit 1
fi

TYPE_NAME="$(printf '%s' "$CRATE_NAME" | awk -F- '{ for (i=1; i<=NF; i++) printf "%s%s", toupper(substr($i,1,1)), substr($i,2) }')"
SDK_PATH="$REPO_ROOT/public-rust/gameplay-module-sdk"

mkdir -p "$DESTINATION/src" "$DESTINATION/generated"
cp "$TEMPLATE/Cargo.toml.in" "$DESTINATION/Cargo.toml"
cp "$TEMPLATE/lib.rs.in" "$DESTINATION/src/lib.rs"
cp "$TEMPLATE/projection.ts.in" "$DESTINATION/generated/moduleProjection.ts"
sed -i \
  -e "s|__CRATE_NAME__|$CRATE_NAME|g" \
  -e "s|__MODULE_ID__|$MODULE_ID|g" \
  -e "s|__TYPE_NAME__|$TYPE_NAME|g" \
  -e "s|__SDK_PATH__|$SDK_PATH|g" \
  "$DESTINATION/Cargo.toml" "$DESTINATION/src/lib.rs" \
  "$DESTINATION/generated/moduleProjection.ts"

if rg -n 'engine-rs/crates' "$DESTINATION/Cargo.toml"; then
  echo "generated module contains a forbidden private engine dependency" >&2
  exit 1
fi

cargo fmt --manifest-path "$DESTINATION/Cargo.toml"
cargo fmt --manifest-path "$DESTINATION/Cargo.toml" -- --check
cargo test --offline --manifest-path "$DESTINATION/Cargo.toml"
if rg -n 'function|class|callback|handler|mutate|command' \
  "$DESTINATION/generated/moduleProjection.ts"; then
  echo "generated TypeScript must remain configuration/projection-only" >&2
  exit 1
fi
echo "created and checked $CRATE_NAME at $DESTINATION"
