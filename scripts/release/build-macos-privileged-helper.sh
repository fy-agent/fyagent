#!/usr/bin/env bash
# Build the universal privileged helper executable and in-process client
# dylib, then copy them into the package dist directory with frozen names.
# Production remains the default. The development variant changes only the
# app/helper identity namespaces and build output; protocol and transaction
# implementation stay shared. This script does not sign, notarize, or embed.

set -euo pipefail

export LC_ALL=C
export MACOSX_DEPLOYMENT_TARGET="${MACOSX_DEPLOYMENT_TARGET:-12.0}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=macos-signing-policy.sh
source "$SCRIPT_DIR/macos-signing-policy.sh"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
PACKAGE_DIR="$REPO_ROOT/src-tauri/macos-privileged-helper"
HELPER_PRODUCT='com.fyagent.desktop.system-commit-helper'
CLIENT_PRODUCT='FyAgentPrivilegedClient'
CLIENT_NAME='libFyAgentPrivilegedClient.dylib'

VARIANT='production'
if [ "$#" -eq 2 ] && [ "$1" = '--variant' ]; then
  VARIANT="$2"
elif [ "$#" -ne 0 ]; then
  echo "Usage: build-macos-privileged-helper.sh [--variant production|development]" >&2
  exit 2
fi
case "$VARIANT" in
  production)
    APP_IDENTIFIER="$EXPECTED_IDENTIFIER"
    HELPER_NAME="$EXPECTED_HELPER_IDENTIFIER"
    HELPER_VERSION="$(awk '
      /^\[workspace.package\]$/ { in_workspace = 1; next }
      /^\[/ { in_workspace = 0 }
      in_workspace && /^version = "/ {
        value = $0
        sub(/^version = "/, "", value)
        sub(/".*$/, "", value)
        print value
        exit
      }
    ' "$REPO_ROOT/src-tauri/Cargo.toml")"
    DIST_DIR="$PACKAGE_DIR/dist"
    SCRATCH_PATH="$PACKAGE_DIR/.build"
    ;;
  development)
    APP_IDENTIFIER='com.fyagent.desktop.dev'
    HELPER_NAME='com.fyagent.desktop.dev.system-commit-helper'
    # Assigned after tool validation by a persistent monotonic generator.
    HELPER_VERSION=''
    DIST_DIR="$PACKAGE_DIR/dist/development"
    SCRATCH_PATH="$PACKAGE_DIR/.build-development"
    ;;
  *)
    echo "Unsupported privileged helper variant: $VARIANT" >&2
    exit 2
    ;;
esac
if [ "$VARIANT" = 'production' ] && [ -z "$HELPER_VERSION" ]; then
  echo "Unable to resolve privileged helper version" >&2
  exit 1
fi

require_command() {
  local name="$1"
  if ! command -v "$name" >/dev/null 2>&1; then
    echo "build-macos-privileged-helper.sh requires $name" >&2
    exit 1
  fi
}

is_regular_file() {
  [ -n "${1:-}" ] && [ -f "$1" ] && [ ! -L "$1" ]
}

require_universal() {
  local path="$1"
  local label="$2"
  local archs
  if ! archs="$(lipo -archs "$path")"; then
    echo "unable to read $label architectures" >&2
    exit 1
  fi
  if [[ " $archs " != *" arm64 "* ]] || [[ " $archs " != *" x86_64 "* ]]; then
    echo "$label is not a universal arm64/x86_64 binary: $archs" >&2
    exit 1
  fi
}

find_named_artifact() {
  local name="$1"
  local path
  for path in \
    "$SCRATCH_PATH/apple/Products/Release/$name" \
    "$SCRATCH_PATH/release/$name"
  do
    if is_regular_file "$path" || { [ -f "$path" ] && [ -L "$path" ]; }; then
      printf '%s\n' "$path"
      return 0
    fi
  done
  return 1
}

find_thin_artifact() {
  local name="$1"
  local arch="$2"
  local path archs
  shopt -s nullglob
  for path in "$SCRATCH_PATH/"*"/release/$name"; do
    if [ ! -f "$path" ]; then
      continue
    fi
    archs="$(lipo -archs "$path")"
    if [ "$archs" = "$arch" ]; then
      printf '%s\n' "$path"
      shopt -u nullglob
      return 0
    fi
  done
  shopt -u nullglob
  return 1
}

swift_build_product() {
  local product="$1"
  shift
  (
    cd "$PACKAGE_DIR"
    FYAGENT_PRIVILEGED_VARIANT="$VARIANT" \
    FYAGENT_PRIVILEGED_HELPER_INFO_PLIST="$HELPER_INFO_PLIST" \
    FYAGENT_PRIVILEGED_HELPER_LAUNCHD_PLIST="$HELPER_LAUNCHD_PLIST" \
    swift build \
      -c release \
      --scratch-path "$SCRATCH_PATH" \
      --product "$product" \
      --disable-automatic-resolution \
      -Xswiftc -D \
      -Xswiftc "$BUILD_FINGERPRINT" \
      "$@"
  )
}

copy_or_lipo_product() {
  local product="$1"
  local output_name="$2"
  local dest="$DIST_DIR/$output_name"
  local built=""
  local arm64_bin=""
  local x86_bin=""
  local archs
  local names=("$output_name")
  if [ "$product" = "$HELPER_PRODUCT" ]; then
    names+=("$HELPER_PRODUCT" "FyAgentPrivilegedHelper")
  fi

  if swift_build_product "$product" --arch arm64 --arch x86_64; then
    for name in "${names[@]}"; do
      built="$(find_named_artifact "$name" || true)"
      if [ -n "$built" ] && [ -f "$built" ]; then
        if archs="$(lipo -archs "$built")" &&
          [[ " $archs " == *" arm64 "* ]] &&
          [[ " $archs " == *" x86_64 "* ]]; then
          cp "$built" "$dest"
          return 0
        fi
      fi
    done
  fi

  swift_build_product "$product" --arch arm64
  swift_build_product "$product" --arch x86_64
  for name in "${names[@]}"; do
    arm64_bin="$(find_thin_artifact "$name" arm64 || true)"
    x86_bin="$(find_thin_artifact "$name" x86_64 || true)"
    if [ -n "$arm64_bin" ] && [ -n "$x86_bin" ]; then
      break
    fi
  done
  if [ -z "$arm64_bin" ] || [ -z "$x86_bin" ]; then
    echo "swift build did not produce $output_name for both slices" >&2
    exit 1
  fi
  lipo -create "$arm64_bin" "$x86_bin" -output "$dest"
}

require_command swift
require_command lipo
require_command otool
require_command python3

if [ "$VARIANT" = 'development' ]; then
  # Encode a monotonic minute counter in Apple's numeric CFBundleVersion
  # field limits (major <= 4 digits, minor/patch <= 2 digits). The persistent
  # counter also makes repeated builds within one minute strictly increasing,
  # which SMJobBless requires when replacing an installed development helper.
  DEVELOPMENT_VERSION_STATE="$HOME/Library/Caches/FyAgent/DevelopmentHelper/helper-version-counter"
  HELPER_VERSION="$(python3 - "$DEVELOPMENT_VERSION_STATE" <<'PY'
import datetime
import fcntl
import os
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
path.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
epoch = datetime.datetime(2020, 1, 1, tzinfo=datetime.timezone.utc)
now = datetime.datetime.now(datetime.timezone.utc)
clock_counter = int((now - epoch).total_seconds() // 60)

with path.open("a+", encoding="ascii") as state:
    os.chmod(path, 0o600)
    fcntl.flock(state.fileno(), fcntl.LOCK_EX)
    state.seek(0)
    raw = state.read().strip()
    previous = int(raw) if raw.isdigit() else 0
    counter = max(clock_counter, previous + 1)
    if counter >= 100_000_000:
        raise SystemExit("development helper version counter exhausted")
    state.seek(0)
    state.truncate()
    state.write(f"{counter}\n")
    state.flush()
    os.fsync(state.fileno())

major = counter // 10_000
minor = (counter // 100) % 100
patch = counter % 100
if major <= 0 or major > 9_999:
    raise SystemExit("development helper version is outside CFBundleVersion limits")
print(f"{major}.{minor}.{patch}")
PY
)"
fi

if [ -z "$HELPER_VERSION" ]; then
  echo "Unable to resolve privileged helper version" >&2
  exit 1
fi

# SwiftPM does not model the content of linker `-sectcreate` inputs as a build
# dependency. Use a versioned path and a version-specific compiler define so a
# helper rebuild cannot silently reuse a binary with stale embedded plists.
GENERATED_ROOT="$PACKAGE_DIR/.generated/$VARIANT"
GENERATED_DIR="$GENERATED_ROOT/$HELPER_VERSION"
HELPER_INFO_PLIST="$GENERATED_DIR/helper-info.plist"
HELPER_LAUNCHD_PLIST="$GENERATED_DIR/helper-launchd.plist"
BUILD_FINGERPRINT="FYAGENT_PRIVILEGED_BUILD_$(printf '%s' "$VARIANT-$HELPER_VERSION" | tr -c 'A-Za-z0-9_' '_')"

if [ ! -f "$PACKAGE_DIR/Package.swift" ] || [ -L "$PACKAGE_DIR/Package.swift" ]; then
  echo "Swift package manifest is missing: $PACKAGE_DIR/Package.swift" >&2
  exit 1
fi
if [ ! -f "$PACKAGE_DIR/Package.resolved" ] || [ -L "$PACKAGE_DIR/Package.resolved" ]; then
  echo "Swift package lockfile is missing: $PACKAGE_DIR/Package.resolved" >&2
  exit 1
fi

rm -rf "$DIST_DIR" "$GENERATED_ROOT"
mkdir -p "$DIST_DIR"

if [ "$VARIANT" = 'development' ]; then
  mkdir -p "$GENERATED_DIR"
  cat >"$HELPER_INFO_PLIST" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleIdentifier</key>
  <string>$HELPER_NAME</string>
  <key>CFBundleName</key>
  <string>FyAgent Development System Commit Helper</string>
  <key>CFBundleVersion</key>
  <string>$HELPER_VERSION</string>
  <key>CFBundleInfoDictionaryVersion</key>
  <string>6.0</string>
  <key>SMAuthorizedClients</key>
  <array>
    <string>anchor apple generic and identifier "$APP_IDENTIFIER" and info[CFBundleVersion] &gt;= "0.4.2" and certificate leaf[subject.OU] = "$EXPECTED_TEAM_ID"</string>
  </array>
</dict>
</plist>
EOF
  cat >"$HELPER_LAUNCHD_PLIST" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>$HELPER_NAME</string>
  <key>MachServices</key>
  <dict>
    <key>$HELPER_NAME</key>
    <true/>
  </dict>
</dict>
</plist>
EOF
else
  HELPER_INFO_PLIST="$PACKAGE_DIR/Resources/helper-info.plist"
  HELPER_LAUNCHD_PLIST="$PACKAGE_DIR/Resources/helper-launchd.plist"
fi

copy_or_lipo_product "$HELPER_PRODUCT" "$HELPER_NAME"
copy_or_lipo_product "$CLIENT_PRODUCT" "$CLIENT_NAME"

chmod 0755 "$DIST_DIR/$HELPER_NAME" "$DIST_DIR/$CLIENT_NAME"
require_universal "$DIST_DIR/$HELPER_NAME" "privileged helper"
require_universal "$DIST_DIR/$CLIENT_NAME" "privileged client"

python3 - \
  "$DIST_DIR/$HELPER_NAME" \
  "$DIST_DIR/$CLIENT_NAME" \
  "$APP_IDENTIFIER" \
  "$HELPER_NAME" \
  "$HELPER_VERSION" \
  "$EXPECTED_TEAM_ID" <<'PY'
import plistlib
import subprocess
import sys

helper, client, app_id, helper_id, helper_version, team_id = sys.argv[1:]
authorized_client = (
    f'anchor apple generic and identifier "{app_id}" '
    f'and info[CFBundleVersion] >= "0.4.2" '
    f'and certificate leaf[subject.OU] = "{team_id}"'
)

for architecture in ("arm64", "x86_64"):
    plist_output = subprocess.run(
        ["otool", "-arch", architecture, "-P", helper],
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    ).stdout
    start = plist_output.find(b"<?xml")
    if start < 0:
        raise SystemExit(
            f"{architecture} privileged helper has no readable embedded info plist"
        )
    info = plistlib.loads(plist_output[start:])
    if info.get("CFBundleIdentifier") != helper_id:
        raise SystemExit(
            f"{architecture} privileged helper identifier does not match the build manifest"
        )
    if info.get("CFBundleVersion") != helper_version:
        raise SystemExit(
            f"{architecture} privileged helper version is stale: "
            f"{info.get('CFBundleVersion')!r} != {helper_version!r}"
        )
    if info.get("SMAuthorizedClients") != [authorized_client]:
        raise SystemExit(
            f"{architecture} privileged helper authorized-client requirement drifted"
        )

    install_name = subprocess.run(
        ["otool", "-arch", architecture, "-D", client],
        check=True,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    ).stdout.splitlines()
    if "@rpath/libFyAgentPrivilegedClient.dylib" not in install_name:
        raise SystemExit(
            f"{architecture} privileged client install name is not the frozen @rpath value"
        )
PY

python3 - "$DIST_DIR/manifest.json" "$VARIANT" "$APP_IDENTIFIER" "$HELPER_NAME" "$HELPER_VERSION" "$CLIENT_NAME" <<'PY'
import json
import pathlib
import sys

path, variant, app_id, helper_id, helper_version, client_name = sys.argv[1:]
payload = {
    "schema": "fyagent-macos-privileged-artifacts/v1",
    "variant": variant,
    "appIdentifier": app_id,
    "helperIdentifier": helper_id,
    "machService": helper_id,
    "helperVersion": helper_version,
    "teamIdentifier": "HY446996QX",
    "helperFile": helper_id,
    "clientFile": client_name,
}
pathlib.Path(path).write_text(json.dumps(payload, indent=2) + "\n")
PY

echo "built universal $VARIANT privileged helper and client into $DIST_DIR"
