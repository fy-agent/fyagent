#!/usr/bin/env bash
# Verify the nested privileged helper and in-process client inside FyAgent.app.
# Formal mode requires Developer ID, hardened runtime, timestamp, and universal
# slices. --structure-only checks frozen paths and architectures only.

set -euo pipefail

export LC_ALL=C

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=macos-signing-policy.sh
source "$SCRIPT_DIR/macos-signing-policy.sh"

structure_only=0
while [ "$#" -gt 0 ]; do
  case "$1" in
    --structure-only)
      structure_only=1
      shift
      ;;
    -*)
      echo "Usage: verify-macos-privileged-helper.sh [--structure-only] <FyAgent.app>" >&2
      exit 2
      ;;
    *)
      break
      ;;
  esac
done

if [ "$#" -ne 1 ]; then
  echo "Usage: verify-macos-privileged-helper.sh [--structure-only] <FyAgent.app>" >&2
  exit 2
fi

app_path="$1"
if [ ! -d "$app_path" ]; then
  echo "nested privileged helper verification target is not an app directory" >&2
  exit 1
fi
if [ "$(basename "$app_path")" != "$EXPECTED_BUNDLE_NAME" ]; then
  echo "nested privileged helper verification target is not $EXPECTED_BUNDLE_NAME" >&2
  exit 1
fi

helper_path="$app_path/$EXPECTED_PRIVILEGED_HELPER_RELPATH"
client_path="$app_path/$EXPECTED_PRIVILEGED_CLIENT_RELPATH"
launch_services="$(dirname "$helper_path")"
app_executable="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleExecutable' "$app_path/Contents/Info.plist")"
main_path="$app_path/Contents/MacOS/$app_executable"

require_regular_nested_file() {
  local path="$1"
  local label="$2"
  if [ ! -f "$path" ] || [ -L "$path" ]; then
    echo "$label is missing from the frozen $EXPECTED_BUNDLE_NAME layout" >&2
    exit 1
  fi
}

require_exact_launch_services() {
  local child
  local count=0
  for child in "$launch_services"/*; do
    if [ ! -e "$child" ]; then
      continue
    fi
    count=$((count + 1))
  done
  if [ "$count" -ne 1 ]; then
    echo "LaunchServices must contain exactly one privileged helper" >&2
    exit 1
  fi
  if [ "$(basename "$helper_path")" != "$EXPECTED_HELPER_IDENTIFIER" ]; then
    echo "LaunchServices contains an unexpected nested executable" >&2
    exit 1
  fi
}

require_universal_slices() {
  local path="$1"
  local label="$2"
  local archs
  if ! archs="$(lipo -archs "$path")"; then
    echo "unable to read $label architectures" >&2
    exit 1
  fi
  if [[ " $archs " != *" arm64 "* ]] || [[ " $archs " != *" x86_64 "* ]]; then
    echo "$label is not a universal arm64/x86_64 binary" >&2
    exit 1
  fi
}

require_mach_service_label() {
  if ! grep -a -F -q "$EXPECTED_HELPER_IDENTIFIER" "$helper_path"; then
    echo "nested privileged helper does not contain the expected Mach service label" >&2
    exit 1
  fi
}

require_client_linkage() {
  local linked load_commands
  require_regular_nested_file "$main_path" "FyAgent main executable"
  linked="$(otool -L "$main_path")"
  grep -Fq '@rpath/libFyAgentPrivilegedClient.dylib' <<<"$linked" || {
    echo "FyAgent main executable is not linked to the privileged client" >&2
    exit 1
  }
  load_commands="$(otool -l "$main_path")"
  grep -Fq 'path @executable_path/../Frameworks' <<<"$load_commands" || {
    echo "FyAgent main executable is missing the privileged client Frameworks rpath" >&2
    exit 1
  }
}

require_developer_id_signature() {
  local path="$1"
  local label="$2"
  local expected_identifier="${3:-}"
  local architecture signature_info timestamp_lines
  for architecture in arm64 x86_64; do
    if ! signature_info="$(codesign --display --verbose=4 --architecture "$architecture" "$path" 2>&1)"; then
      echo "unable to read the $label $architecture code signature" >&2
      exit 1
    fi
    if [ -n "$expected_identifier" ]; then
      grep -Fxq "Identifier=$expected_identifier" <<<"$signature_info" || {
        echo "$label $architecture signature identifier drifted" >&2
        exit 1
      }
    fi
    if grep -Fxq 'Signature=adhoc' <<<"$signature_info"; then
      echo "$label $architecture signature is still ad-hoc" >&2
      exit 1
    fi
    grep -Eq '^CodeDirectory .*flags=.*runtime' <<<"$signature_info" || {
      echo "$label $architecture CodeDirectory is not marked for the hardened runtime" >&2
      exit 1
    }
    if grep -Eq '^CodeDirectory .*flags=.*adhoc' <<<"$signature_info"; then
      echo "$label $architecture CodeDirectory is still marked ad-hoc" >&2
      exit 1
    fi
    grep -Fxq "Authority=$EXPECTED_AUTHORITY" <<<"$signature_info" || {
      echo "$label $architecture signature is not the expected Developer ID Application identity" >&2
      exit 1
    }
    grep -Fxq "TeamIdentifier=$EXPECTED_TEAM_ID" <<<"$signature_info" || {
      echo "$label $architecture signature team identity drifted" >&2
      exit 1
    }
    if grep -Eqi 'linker-signed' <<<"$signature_info"; then
      echo "$label $architecture signature contains unnormalized linker identity evidence" >&2
      exit 1
    fi
    timestamp_lines="$(grep -E '^Timestamp=' <<<"$signature_info" || true)"
    if [ -z "$timestamp_lines" ] || [ "$timestamp_lines" = 'Timestamp=none' ]; then
      echo "$label $architecture Developer ID signature is missing a secure timestamp" >&2
      exit 1
    fi
  done
  if ! codesign --verify --strict --verbose=4 "$path" >/dev/null 2>&1; then
    echo "$label code signature failed verification" >&2
    exit 1
  fi
}

require_regular_nested_file "$helper_path" "nested privileged helper"
require_regular_nested_file "$client_path" "nested privileged client"
require_exact_launch_services
require_universal_slices "$helper_path" "nested privileged helper"
require_universal_slices "$client_path" "nested privileged client"
require_mach_service_label
require_client_linkage

if [ "$structure_only" -eq 1 ]; then
  exit 0
fi

require_developer_id_signature \
  "$helper_path" \
  "nested privileged helper" \
  "$EXPECTED_HELPER_IDENTIFIER"
require_developer_id_signature \
  "$client_path" \
  "nested privileged client"
