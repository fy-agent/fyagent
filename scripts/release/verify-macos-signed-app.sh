#!/usr/bin/env bash

set -euo pipefail

export LC_ALL=C

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=macos-signing-policy.sh
source "$SCRIPT_DIR/macos-signing-policy.sh"

if [ "$#" -ne 1 ]; then
  echo "Usage: verify-macos-signed-app.sh <FyAgent.app>" >&2
  exit 2
fi

app_path="$1"
if [ ! -d "$app_path" ]; then
  echo "macOS Developer ID verification target is not an app directory: $app_path" >&2
  exit 1
fi
if [ "$(basename "$app_path")" != "$EXPECTED_BUNDLE_NAME" ]; then
  echo "macOS Developer ID verification target is not $EXPECTED_BUNDLE_NAME: $app_path" >&2
  exit 1
fi

for architecture in arm64 x86_64; do
  if ! signature_info="$(codesign --display --verbose=4 --architecture "$architecture" "$app_path" 2>&1)"; then
    echo "Unable to read the $architecture code signature from $app_path" >&2
    printf '%s\n' "$signature_info" >&2
    exit 1
  fi

  # This output is public build evidence. A Developer ID seal must name the
  # exact publisher identity, team, hardened runtime, and secure timestamp.
  printf '%s\n' "$signature_info"
  grep -Fxq "Identifier=$EXPECTED_IDENTIFIER" <<<"$signature_info" || {
    echo "$architecture signature identifier drifted" >&2
    exit 1
  }
  if grep -Fxq 'Signature=adhoc' <<<"$signature_info"; then
    echo "$architecture signature is still ad-hoc" >&2
    exit 1
  fi
  grep -Eq '^CodeDirectory .*flags=.*runtime' <<<"$signature_info" || {
    echo "$architecture CodeDirectory is not marked for the hardened runtime" >&2
    exit 1
  }
  if grep -Eq '^CodeDirectory .*flags=.*adhoc' <<<"$signature_info"; then
    echo "$architecture CodeDirectory is still marked ad-hoc" >&2
    exit 1
  fi
  grep -Fxq "Authority=$EXPECTED_AUTHORITY" <<<"$signature_info" || {
    echo "$architecture signature is not the expected Developer ID Application identity" >&2
    exit 1
  }
  grep -Fxq "TeamIdentifier=$EXPECTED_TEAM_ID" <<<"$signature_info" || {
    echo "$architecture signature team identity drifted" >&2
    exit 1
  }
  grep -Eq '^Sealed Resources version=' <<<"$signature_info" || {
    echo "$architecture signature does not seal the application resources" >&2
    exit 1
  }
  if grep -Eqi 'linker-signed' <<<"$signature_info"; then
    echo "$architecture signature contains unnormalized linker identity evidence" >&2
    exit 1
  fi
  timestamp_lines="$(grep -E '^Timestamp=' <<<"$signature_info" || true)"
  if [ -z "$timestamp_lines" ] || [ "$timestamp_lines" = 'Timestamp=none' ]; then
    echo "$architecture Developer ID signature is missing a secure timestamp" >&2
    exit 1
  fi
done

codesign --verify --deep --strict --verbose=4 "$app_path"
if ! xcrun stapler validate "$app_path" >/dev/null; then
  echo "Developer ID application is missing a notarization ticket: $app_path" >&2
  exit 1
fi
