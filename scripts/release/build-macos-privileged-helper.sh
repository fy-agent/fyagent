#!/usr/bin/env bash
# Build the universal privileged helper executable and in-process client
# dylib, then copy them into the package dist directory with frozen names.
# This script does not sign, notarize, or embed into FyAgent.app.

set -euo pipefail

export LC_ALL=C
export MACOSX_DEPLOYMENT_TARGET="${MACOSX_DEPLOYMENT_TARGET:-12.0}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=macos-signing-policy.sh
source "$SCRIPT_DIR/macos-signing-policy.sh"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
PACKAGE_DIR="$REPO_ROOT/src-tauri/macos-privileged-helper"
DIST_DIR="$PACKAGE_DIR/dist"
HELPER_PRODUCT='com.fyagent.desktop.system-commit-helper'
CLIENT_PRODUCT='FyAgentPrivilegedClient'
HELPER_NAME="$EXPECTED_HELPER_IDENTIFIER"
CLIENT_NAME='libFyAgentPrivilegedClient.dylib'

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
    "$PACKAGE_DIR/.build/apple/Products/Release/$name" \
    "$PACKAGE_DIR/.build/release/$name"
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
  for path in "$PACKAGE_DIR/.build/"*"/release/$name"; do
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
    swift build \
      -c release \
      --product "$product" \
      --disable-automatic-resolution \
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

  if swift_build_product "$product" --arch arm64 --arch x86_64; then
    for name in "$output_name" FyAgentPrivilegedHelper; do
      built="$(find_named_artifact "$name" || true)"
      if [ -n "$built" ] && [ -f "$built" ]; then
        if archs="$(lipo -archs "$built")" &&
          [[ " $archs " == *" arm64 "* ]] &&
          [[ " $archs " == *" x86_64 "* ]]; then
          cp "$built" "$dest"
          return 0
        fi
      fi
      if [ "$output_name" != "$HELPER_NAME" ]; then
        break
      fi
    done
  fi

  swift_build_product "$product" --arch arm64
  swift_build_product "$product" --arch x86_64
  for name in "$output_name" FyAgentPrivilegedHelper; do
    arm64_bin="$(find_thin_artifact "$name" arm64 || true)"
    x86_bin="$(find_thin_artifact "$name" x86_64 || true)"
    if [ -n "$arm64_bin" ] && [ -n "$x86_bin" ]; then
      break
    fi
    if [ "$output_name" != "$HELPER_NAME" ]; then
      break
    fi
  done
  if [ -z "$arm64_bin" ] || [ -z "$x86_bin" ]; then
    echo "swift build did not produce $output_name for both slices" >&2
    exit 1
  fi
  lipo -create "$arm64_bin" "$x86_bin" -output "$dest"
}

if [ "$#" -ne 0 ]; then
  echo "Usage: build-macos-privileged-helper.sh" >&2
  exit 2
fi

require_command swift
require_command lipo

if [ ! -f "$PACKAGE_DIR/Package.swift" ] || [ -L "$PACKAGE_DIR/Package.swift" ]; then
  echo "Swift package manifest is missing: $PACKAGE_DIR/Package.swift" >&2
  exit 1
fi
if [ ! -f "$PACKAGE_DIR/Package.resolved" ] || [ -L "$PACKAGE_DIR/Package.resolved" ]; then
  echo "Swift package lockfile is missing: $PACKAGE_DIR/Package.resolved" >&2
  exit 1
fi

rm -rf "$DIST_DIR"
mkdir -p "$DIST_DIR"

copy_or_lipo_product "$HELPER_PRODUCT" "$HELPER_NAME"
copy_or_lipo_product "$CLIENT_PRODUCT" "$CLIENT_NAME"

chmod 0755 "$DIST_DIR/$HELPER_NAME" "$DIST_DIR/$CLIENT_NAME"
require_universal "$DIST_DIR/$HELPER_NAME" "privileged helper"
require_universal "$DIST_DIR/$CLIENT_NAME" "privileged client"

echo "built universal privileged helper and client into $DIST_DIR"
