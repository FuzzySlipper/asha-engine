#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 2 ]; then
  echo "usage: $0 <built-addon> <installed-addon>" >&2
  exit 2
fi

artifact="$1"
destination="$2"

if [ ! -s "$artifact" ]; then
  echo "FAIL: native addon artifact is missing or empty: $artifact" >&2
  exit 1
fi

destination_dir="$(dirname "$destination")"
destination_name="$(basename "$destination")"
mkdir -p "$destination_dir"

temporary="$(mktemp "$destination_dir/.${destination_name}.tmp.XXXXXX")"
cleanup() {
  rm -f "$temporary"
}
trap cleanup EXIT

cp "$artifact" "$temporary"
chmod 0755 "$temporary"
mv -f "$temporary" "$destination"
trap - EXIT

echo "Installed native addon atomically -> $destination"
