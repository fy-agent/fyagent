#!/usr/bin/env bash

set -euo pipefail

export LC_ALL=C

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=macos-signing-policy.sh
source "$SCRIPT_DIR/macos-signing-policy.sh"

REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
ENTITLEMENTS="$REPO_ROOT/src-tauri/entitlements.macos.plist"
APPLE_ROOT_CA="$SCRIPT_DIR/apple-root-ca.cer"
G2_CA="$SCRIPT_DIR/apple-developer-id-g2-ca.cer"
STATE_DIR="${RUNNER_TEMP:-${TMPDIR:-/tmp}}/fyagent-macos-signing"
STATE_FILE="$STATE_DIR/state.env"
CERT_PATH="$STATE_DIR/developer-id.p12"
ORIGINAL_KEYCHAINS_FILE="$STATE_DIR/original-keychains"
ORIGINAL_DEFAULT_FILE="$STATE_DIR/original-default"

usage() {
  echo "Usage: macos-developer-id.sh prepare|sign-app|notarize-app|sign-dmg|notarize-dmg|teardown [path]" >&2
  exit 2
}

require_regular_file() {
  local path="$1"
  local label="$2"
  if [ ! -f "$path" ] || [ -L "$path" ]; then
    echo "$label is not a regular file: $path" >&2
    exit 1
  fi
}

require_app_bundle() {
  local path="$1"
  if [ ! -d "$path" ] || [ "$(basename "$path")" != "$EXPECTED_BUNDLE_NAME" ]; then
    echo "Developer ID target is not $EXPECTED_BUNDLE_NAME: $path" >&2
    exit 1
  fi
}

write_state() {
  umask 077
  mkdir -p "$STATE_DIR"
  cat >"$STATE_FILE" <<EOF
KEYCHAIN_PATH=$(printf '%q' "$KEYCHAIN_PATH")
KEYCHAIN_PASSWORD=$(printf '%q' "$KEYCHAIN_PASSWORD")
EOF
}

restore_original_keychains() {
  if [ -s "$ORIGINAL_DEFAULT_FILE" ]; then
    security default-keychain -d user -s "$(cat "$ORIGINAL_DEFAULT_FILE")" >/dev/null 2>&1 || true
  fi
  if [ -s "$ORIGINAL_KEYCHAINS_FILE" ]; then
    local originals=()
    while IFS= read -r keychain_path; do
      [ -n "$keychain_path" ] && originals+=("$keychain_path")
    done <"$ORIGINAL_KEYCHAINS_FILE"
    if [ "${#originals[@]}" -gt 0 ]; then
      security list-keychains -d user -s "${originals[@]}" >/dev/null 2>&1 || true
    fi
  fi
}

load_state() {
  if [ ! -f "$STATE_FILE" ]; then
    echo "macOS Developer ID keychain was not prepared" >&2
    exit 1
  fi
  # shellcheck disable=SC1090
  source "$STATE_FILE"
  if [ -z "${KEYCHAIN_PATH:-}" ] || [ ! -f "$KEYCHAIN_PATH" ]; then
    echo "macOS Developer ID keychain is missing" >&2
    exit 1
  fi
  security unlock-keychain -p "$KEYCHAIN_PASSWORD" "$KEYCHAIN_PATH" >/dev/null
}

require_secret() {
  local name="$1"
  local value="${2:-}"
  if [ -z "$value" ]; then
    echo "$name must be configured as a GitHub Actions secret" >&2
    exit 1
  fi
}

submit_for_notarization() {
  local artifact="$1"
  require_regular_file "$artifact" "Notarization input"
  local submission_json="$STATE_DIR/notary-$(basename "$artifact").json"
  xcrun notarytool submit "$artifact" \
    --keychain-profile "$NOTARY_PROFILE" \
    --keychain "$KEYCHAIN_PATH" \
    --output-format json \
    --wait \
    --timeout 1800 \
    >"$submission_json"
  python3 - "$submission_json" "$artifact" <<'PY'
import json
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
artifact = sys.argv[2]
payload = json.loads(path.read_text())
status = payload.get("status")
if status != "Accepted":
    raise SystemExit(
        f"Apple notarization did not accept {artifact}: status={status!r}"
    )
PY
}

prepare() {
  require_secret "FYAGENT_APPLE_CERTIFICATE_P12_BASE64" "${FYAGENT_APPLE_CERTIFICATE_P12_BASE64:-}"
  require_secret "FYAGENT_APPLE_CERTIFICATE_PASSWORD" "${FYAGENT_APPLE_CERTIFICATE_PASSWORD:-}"
  require_secret "FYAGENT_APPLE_ID" "${FYAGENT_APPLE_ID:-}"
  require_secret "FYAGENT_APPLE_APP_SPECIFIC_PASSWORD" "${FYAGENT_APPLE_APP_SPECIFIC_PASSWORD:-}"
  require_regular_file "$ENTITLEMENTS" "macOS entitlements"
  require_regular_file "$APPLE_ROOT_CA" "Apple Root CA"
  require_regular_file "$G2_CA" "Developer ID G2 CA"

  umask 077
  rm -rf "$STATE_DIR"
  mkdir -p "$STATE_DIR"
  security list-keychains -d user | sed -E 's/^[[:space:]]*"|"$//g' >"$ORIGINAL_KEYCHAINS_FILE"
  security default-keychain -d user | sed -E 's/^[[:space:]]*"|"$//g' >"$ORIGINAL_DEFAULT_FILE"

  KEYCHAIN_PATH="$STATE_DIR/signing.keychain-db"
  KEYCHAIN_PASSWORD="$(python3 -c 'import secrets; print(secrets.token_urlsafe(32))')"

  printf '%s' "$FYAGENT_APPLE_CERTIFICATE_P12_BASE64" | tr -d '\r\n' | base64 -D >"$CERT_PATH"
  if [ ! -s "$CERT_PATH" ]; then
    echo "FYAGENT_APPLE_CERTIFICATE_P12_BASE64 did not decode to a PKCS#12 file" >&2
    exit 1
  fi

  security create-keychain -p "$KEYCHAIN_PASSWORD" "$KEYCHAIN_PATH"
  security set-keychain-settings -lut 21600 "$KEYCHAIN_PATH"
  security unlock-keychain -p "$KEYCHAIN_PASSWORD" "$KEYCHAIN_PATH"
  security import "$APPLE_ROOT_CA" -k "$KEYCHAIN_PATH" -T /usr/bin/codesign >/dev/null
  security import "$G2_CA" -k "$KEYCHAIN_PATH" -T /usr/bin/codesign >/dev/null
  security import "$CERT_PATH" \
    -k "$KEYCHAIN_PATH" \
    -P "$FYAGENT_APPLE_CERTIFICATE_PASSWORD" \
    -f pkcs12 \
    -T /usr/bin/codesign \
    -T /usr/bin/security >/dev/null
  security set-key-partition-list \
    -S apple-tool:,apple:,codesign: \
    -s -k "$KEYCHAIN_PASSWORD" \
    "$KEYCHAIN_PATH" >/dev/null
  local originals=()
  while IFS= read -r keychain_path; do
    [ -n "$keychain_path" ] && originals+=("$keychain_path")
  done <"$ORIGINAL_KEYCHAINS_FILE"
  security list-keychains -d user -s "$KEYCHAIN_PATH" "${originals[@]}"
  security default-keychain -d user -s "$KEYCHAIN_PATH"
  rm -f "$CERT_PATH"

  xcrun notarytool store-credentials "$NOTARY_PROFILE" \
    --apple-id "$FYAGENT_APPLE_ID" \
    --password "$FYAGENT_APPLE_APP_SPECIFIC_PASSWORD" \
    --team-id "$EXPECTED_TEAM_ID" \
    --keychain "$KEYCHAIN_PATH" >/dev/null

  if ! security find-identity -v -p codesigning "$KEYCHAIN_PATH" | grep -Fq "$EXPECTED_AUTHORITY"; then
    echo "Imported keychain does not contain a valid $EXPECTED_AUTHORITY identity" >&2
    security find-identity -v -p codesigning "$KEYCHAIN_PATH" >&2
    exit 1
  fi

  write_state
}

sign_app() {
  local app_path="$1"
  require_app_bundle "$app_path"
  load_state
  codesign --force \
    --sign "$EXPECTED_AUTHORITY" \
    --options runtime \
    --timestamp \
    --entitlements "$ENTITLEMENTS" \
    "$app_path"
}

notarize_app() {
  local app_path="$1"
  require_app_bundle "$app_path"
  load_state
  local notarize_zip="$STATE_DIR/$EXPECTED_BUNDLE_NAME.zip"
  rm -f "$notarize_zip"
  ditto -c -k --sequesterRsrc --keepParent "$app_path" "$notarize_zip"
  submit_for_notarization "$notarize_zip"
  xcrun stapler staple "$app_path"
  rm -f "$notarize_zip"
}

sign_dmg() {
  local dmg_path="$1"
  require_regular_file "$dmg_path" "macOS DMG"
  load_state
  codesign --force \
    --sign "$EXPECTED_AUTHORITY" \
    --timestamp \
    "$dmg_path"
}

notarize_dmg() {
  local dmg_path="$1"
  require_regular_file "$dmg_path" "macOS DMG"
  load_state
  submit_for_notarization "$dmg_path"
  xcrun stapler staple "$dmg_path"
}

teardown() {
  restore_original_keychains
  if [ -f "$STATE_FILE" ]; then
    # shellcheck disable=SC1090
    source "$STATE_FILE"
  fi
  if [ -n "${KEYCHAIN_PATH:-}" ] && [ -f "$KEYCHAIN_PATH" ]; then
    security delete-keychain "$KEYCHAIN_PATH" >/dev/null 2>&1 || true
  elif [ -f "$STATE_DIR/signing.keychain-db" ]; then
    security delete-keychain "$STATE_DIR/signing.keychain-db" >/dev/null 2>&1 || true
  fi
  rm -rf "$STATE_DIR"
}

if [ "$#" -lt 1 ]; then
  usage
fi

command="$1"
shift

case "$command" in
  prepare)
    [ "$#" -eq 0 ] || usage
    prepare
    ;;
  sign-app)
    [ "$#" -eq 1 ] || usage
    sign_app "$1"
    ;;
  notarize-app)
    [ "$#" -eq 1 ] || usage
    notarize_app "$1"
    ;;
  sign-dmg)
    [ "$#" -eq 1 ] || usage
    sign_dmg "$1"
    ;;
  notarize-dmg)
    [ "$#" -eq 1 ] || usage
    notarize_dmg "$1"
    ;;
  teardown)
    [ "$#" -eq 0 ] || usage
    teardown
    ;;
  *)
    usage
    ;;
esac
