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
# Apple's published typical bound is under an hour, but this team's first
# Developer ID submissions stayed In Progress past 30 and 60 minutes and later
# reached Accepted. Poll `notarytool info` instead of `notarytool wait`:
# wait --timeout writes JSON to stderr and exits 124, which `set -e` treats as
# failure even while Apple is still processing. GitHub-hosted jobs allow 6
# hours; leave about one hour for build and packaging.
NOTARY_WAIT_SECONDS="${FYAGENT_NOTARY_WAIT_SECONDS:-18000}"
NOTARY_POLL_SECONDS="${FYAGENT_NOTARY_POLL_SECONDS:-20}"
NOTARY_HEARTBEAT_SECONDS="${FYAGENT_NOTARY_HEARTBEAT_SECONDS:-120}"

usage() {
  echo "Usage: macos-developer-id.sh prepare|sign-app|sign-dmg|notarize-dmg|staple-app|teardown [path]" >&2
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

read_notary_status() {
  python3 - "$1" <<'PY'
import json
import pathlib
import sys

raw = pathlib.Path(sys.argv[1]).read_text() if pathlib.Path(sys.argv[1]).is_file() else ""
start = raw.find("{")
end = raw.rfind("}")
if start < 0 or end < start:
    print("UNKNOWN")
    raise SystemExit(0)
try:
    payload = json.loads(raw[start : end + 1])
except json.JSONDecodeError:
    print("UNKNOWN")
    raise SystemExit(0)
status = payload.get("status")
print(status.strip() if isinstance(status, str) and status.strip() else "UNKNOWN")
PY
}

require_notary_accepted() {
  python3 - "$1" "$2" <<'PY'
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

wait_for_notarization() {
  local submission_id="$1"
  local artifact="$2"
  local info_json="$STATE_DIR/notary-info-$(basename "$artifact").json"
  local info_err="$STATE_DIR/notary-info-$(basename "$artifact").err"
  local log_json="$STATE_DIR/notary-log-$(basename "$artifact").json"
  local started_at elapsed last_heartbeat last_status status info_rc
  started_at="$(date +%s)"
  last_heartbeat=-"$NOTARY_HEARTBEAT_SECONDS"
  last_status=""

  while :; do
    elapsed=$(($(date +%s) - started_at))
    set +e
    xcrun notarytool info "$submission_id" \
      --keychain-profile "$NOTARY_PROFILE" \
      --keychain "$KEYCHAIN_PATH" \
      --output-format json \
      >"$info_json" 2>"$info_err"
    info_rc=$?
    set -e
    if [ ! -s "$info_json" ] && [ -s "$info_err" ]; then
      cp "$info_err" "$info_json"
    fi
    status="$(read_notary_status "$info_json")"
    if [ "$status" != "$last_status" ] ||
      [ $((elapsed - last_heartbeat)) -ge "$NOTARY_HEARTBEAT_SECONDS" ]; then
      echo "Apple notarization $submission_id status=$status elapsed=${elapsed}s"
      last_heartbeat="$elapsed"
      last_status="$status"
    fi

    case "$status" in
      Accepted)
        require_notary_accepted "$info_json" "$artifact"
        return 0
        ;;
      Invalid|Rejected)
        set +e
        xcrun notarytool log "$submission_id" \
          --keychain-profile "$NOTARY_PROFILE" \
          --keychain "$KEYCHAIN_PATH" \
          "$log_json"
        set -e
        if [ -s "$log_json" ]; then
          cat "$log_json" >&2
        elif [ -s "$info_err" ]; then
          cat "$info_err" >&2
        fi
        echo "Apple notarization did not accept $artifact: status=$status" >&2
        exit 1
        ;;
    esac

    if [ "$elapsed" -ge "$NOTARY_WAIT_SECONDS" ]; then
      echo "Apple notarization $submission_id still ${status} after ${NOTARY_WAIT_SECONDS}s for $artifact (info_rc=$info_rc)" >&2
      if [ -s "$info_err" ]; then
        cat "$info_err" >&2
      fi
      exit 1
    fi
    sleep "$NOTARY_POLL_SECONDS"
  done
}

submit_for_notarization() {
  local artifact="$1"
  require_regular_file "$artifact" "Notarization input"
  local submission_json="$STATE_DIR/notary-$(basename "$artifact").json"
  # Submit without --wait so Apple starts one job immediately. Poll info on
  # that same id until Accepted, Invalid, or the wait budget expires.
  xcrun notarytool submit "$artifact" \
    --keychain-profile "$NOTARY_PROFILE" \
    --keychain "$KEYCHAIN_PATH" \
    --output-format json \
    >"$submission_json"
  local submission_id
  submission_id="$(
    python3 - "$submission_json" <<'PY'
import json
import pathlib
import sys

payload = json.loads(pathlib.Path(sys.argv[1]).read_text())
submission_id = payload.get("id")
if not isinstance(submission_id, str) or not submission_id:
    raise SystemExit("Apple notarization did not return a submission id")
print(submission_id)
PY
  )"
  echo "Apple notarization submission $submission_id for $artifact"
  wait_for_notarization "$submission_id" "$artifact"
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

sign_nested_privileged_code() {
  local app_path="$1"
  local client_path="$app_path/$EXPECTED_PRIVILEGED_CLIENT_RELPATH"
  local helper_path="$app_path/$EXPECTED_PRIVILEGED_HELPER_RELPATH"
  local client_present=0
  local helper_present=0

  if [ -f "$client_path" ] && [ ! -L "$client_path" ]; then
    client_present=1
  fi
  if [ -f "$helper_path" ] && [ ! -L "$helper_path" ]; then
    helper_present=1
  fi

  if [ "$client_present" -eq 0 ] && [ "$helper_present" -eq 0 ]; then
    if [ "${FYAGENT_ALLOW_APP_ONLY_SIGN:-0}" = "1" ]; then
      echo "nested privileged helper is absent; signing $EXPECTED_BUNDLE_NAME only" >&2
      return 0
    fi
    echo "formal Developer ID signing requires the nested privileged helper and client" >&2
    exit 1
  fi
  if [ "$client_present" -eq 0 ] || [ "$helper_present" -eq 0 ]; then
    echo "nested privileged helper and client must both be present before signing" >&2
    exit 1
  fi

  # Inside-out: sign the in-process client, then the helper, then the main app.
  # Do not use --deep to sign nested code, and do not apply app entitlements here.
  codesign --force \
    --sign "$EXPECTED_AUTHORITY" \
    --options runtime \
    --timestamp \
    "$client_path"
  codesign --force \
    --sign "$EXPECTED_AUTHORITY" \
    --identifier "$EXPECTED_HELPER_IDENTIFIER" \
    --options runtime \
    --timestamp \
    "$helper_path"
}

sign_app() {
  local app_path="$1"
  require_app_bundle "$app_path"
  load_state
  sign_nested_privileged_code "$app_path"
  codesign --force \
    --sign "$EXPECTED_AUTHORITY" \
    --options runtime \
    --timestamp \
    --entitlements "$ENTITLEMENTS" \
    "$app_path"
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

staple_app() {
  local app_path="$1"
  require_app_bundle "$app_path"
  load_state
  xcrun stapler staple "$app_path"
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
  sign-dmg)
    [ "$#" -eq 1 ] || usage
    sign_dmg "$1"
    ;;
  notarize-dmg)
    [ "$#" -eq 1 ] || usage
    notarize_dmg "$1"
    ;;
  staple-app)
    [ "$#" -eq 1 ] || usage
    staple_app "$1"
    ;;
  teardown)
    [ "$#" -eq 0 ] || usage
    teardown
    ;;
  *)
    usage
    ;;
esac
