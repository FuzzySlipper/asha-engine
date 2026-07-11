#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
INSTALLER="$REPO_ROOT/harness/ci/install-native-addon.sh"
TEMP_ROOT="$(mktemp -d)"
cleanup() {
  rm -rf "$TEMP_ROOT"
}
trap cleanup EXIT

artifact_one="$TEMP_ROOT/native-v1.node"
artifact_two="$TEMP_ROOT/native-v2.node"
destination="$TEMP_ROOT/dist/native-bridge.node"
printf 'native-addon-v1\n' > "$artifact_one"
printf 'native-addon-v2\n' > "$artifact_two"

"$INSTALLER" "$artifact_one" "$destination" >/dev/null
inode_before="$(stat -c '%i' "$destination")"
exec 9<"$destination"

"$INSTALLER" "$artifact_two" "$destination" >/dev/null
inode_after="$(stat -c '%i' "$destination")"
IFS= read -r mapped_content <&9
exec 9<&-

if [ "$inode_before" = "$inode_after" ]; then
  echo "FAIL: native addon install replaced content in place" >&2
  exit 1
fi
if [ "$mapped_content" != 'native-addon-v1' ]; then
  echo "FAIL: open native addon mapping changed during replacement" >&2
  exit 1
fi
if [ "$(cat "$destination")" != 'native-addon-v2' ]; then
  echo "FAIL: new native addon path does not expose replacement content" >&2
  exit 1
fi

echo "Native addon atomic install fixture: OK"
