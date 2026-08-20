#!/usr/bin/env bash

set -euo pipefail

export LC_ALL=C

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=macos-signing-policy.sh
source "$SCRIPT_DIR/macos-signing-policy.sh"

if [ "$#" -ne 1 ]; then
  echo "Usage: verify-macos-signed-dmg.sh <FyAgent-macOS.dmg>" >&2
  exit 2
fi

dmg_path="$1"
if [ ! -f "$dmg_path" ] || [ -L "$dmg_path" ]; then
  echo "macOS Developer ID DMG verification target is not a regular file: $dmg_path" >&2
  exit 1
fi

if ! signature_info="$(codesign --display --verbose=4 "$dmg_path" 2>&1)"; then
  echo "Unable to read the DMG code signature from $dmg_path" >&2
  printf '%s\n' "$signature_info" >&2
  exit 1
fi

printf '%s\n' "$signature_info"
if grep -Fxq 'Signature=adhoc' <<<"$signature_info" ||
  grep -Eqi 'code object is not signed at all' <<<"$signature_info"; then
  echo "DMG is not Developer ID signed: $dmg_path" >&2
  exit 1
fi
grep -Fxq "Authority=$EXPECTED_AUTHORITY" <<<"$signature_info" || {
  echo "DMG is not signed with the expected Developer ID Application identity" >&2
  exit 1
}
grep -Fxq "TeamIdentifier=$EXPECTED_TEAM_ID" <<<"$signature_info" || {
  echo "DMG signature team identity drifted" >&2
  exit 1
}
timestamp_lines="$(grep -E '^Timestamp=' <<<"$signature_info" || true)"
if [ -z "$timestamp_lines" ] || [ "$timestamp_lines" = 'Timestamp=none' ]; then
  echo "DMG Developer ID signature is missing a secure timestamp" >&2
  exit 1
fi

codesign --verify --verbose=4 "$dmg_path"
if ! xcrun stapler validate "$dmg_path" >/dev/null; then
  echo "Developer ID DMG is missing a notarization ticket: $dmg_path" >&2
  exit 1
fi
