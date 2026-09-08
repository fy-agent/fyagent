#!/usr/bin/env bash
# Copy universal privileged helper/client into the frozen FyAgent.app layout
# when source artifacts exist. Missing sources are a no-op and never delete
# the application bundle. This script does not sign or notarize.

set -euo pipefail

export LC_ALL=C

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=macos-signing-policy.sh
source "$SCRIPT_DIR/macos-signing-policy.sh"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

usage() {
  echo "Usage: embed-macos-privileged-helper.sh <FyAgent.app>" >&2
  exit 2
}

is_regular_file() {
  [ -n "${1:-}" ] && [ -f "$1" ] && [ ! -L "$1" ]
}

if [ "$#" -ne 1 ]; then
  usage
fi

app_path="$1"
if [ ! -d "$app_path" ] || [ "$(basename "$app_path")" != "$EXPECTED_BUNDLE_NAME" ]; then
  echo "embed target is not $EXPECTED_BUNDLE_NAME" >&2
  exit 1
fi

helper_dest="$app_path/$EXPECTED_PRIVILEGED_HELPER_RELPATH"
client_dest="$app_path/$EXPECTED_PRIVILEGED_CLIENT_RELPATH"

helper_src=""
client_src=""
artifact_root="${FYAGENT_PRIVILEGED_ARTIFACT_ROOT:-$REPO_ROOT/src-tauri/macos-privileged-helper}"
helper_candidates=(
  "${FYAGENT_PRIVILEGED_HELPER_BIN:-}"
  "$artifact_root/dist/$EXPECTED_HELPER_IDENTIFIER"
  "$artifact_root/.build/apple/Products/Release/$EXPECTED_HELPER_IDENTIFIER"
  "$artifact_root/.build/release/$EXPECTED_HELPER_IDENTIFIER"
  "$artifact_root/.build/apple/Products/Release/FyAgentPrivilegedHelper"
  "$artifact_root/.build/release/FyAgentPrivilegedHelper"
)
client_candidates=(
  "${FYAGENT_PRIVILEGED_CLIENT_DYLIB:-}"
  "$artifact_root/dist/libFyAgentPrivilegedClient.dylib"
  "$artifact_root/.build/apple/Products/Release/libFyAgentPrivilegedClient.dylib"
  "$artifact_root/.build/release/libFyAgentPrivilegedClient.dylib"
)

for candidate in "${helper_candidates[@]}"; do
  if is_regular_file "$candidate"; then
    helper_src="$candidate"
    break
  fi
done
for candidate in "${client_candidates[@]}"; do
  if is_regular_file "$candidate"; then
    client_src="$candidate"
    break
  fi
done

if [ -z "$helper_src" ] && [ -z "$client_src" ]; then
  if [ "${FYAGENT_REQUIRE_PRIVILEGED_HELPER:-0}" = "1" ]; then
    echo "formal privileged helper artifacts are required before embedding" >&2
    exit 1
  fi
  echo "privileged helper artifacts are absent; leaving $EXPECTED_BUNDLE_NAME unchanged" >&2
  exit 0
fi

if [ -z "$helper_src" ] || [ -z "$client_src" ]; then
  echo "privileged helper and client artifacts must both exist before embedding" >&2
  exit 1
fi

mkdir -p "$(dirname "$helper_dest")" "$(dirname "$client_dest")"
cp "$helper_src" "$helper_dest"
cp "$client_src" "$client_dest"
chmod 0755 "$helper_dest" "$client_dest"
echo "embedded nested privileged helper and client into $EXPECTED_BUNDLE_NAME"
