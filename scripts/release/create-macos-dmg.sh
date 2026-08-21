#!/usr/bin/env bash

set -euo pipefail

export LC_ALL=C

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RETRY_HDIUTIL="$SCRIPT_DIR/retry-hdiutil.sh"
LAYOUT_WRITER="$SCRIPT_DIR/write-dmg-layout.py"

usage() {
  echo "Usage: create-macos-dmg.sh --app <FyAgent.app> --output <dmg> --background <png>" >&2
  exit 2
}

app_path=""
output_path=""
background_path=""

while [ "$#" -gt 0 ]; do
  case "$1" in
    --app)
      [ "$#" -ge 2 ] || usage
      app_path="$2"
      shift 2
      ;;
    --output)
      [ "$#" -ge 2 ] || usage
      output_path="$2"
      shift 2
      ;;
    --background)
      [ "$#" -ge 2 ] || usage
      background_path="$2"
      shift 2
      ;;
    *)
      usage
      ;;
  esac
done

if [ -z "$app_path" ] || [ -z "$output_path" ] || [ -z "$background_path" ]; then
  usage
fi
if [ ! -d "$app_path" ] || [ "$(basename "$app_path")" != 'FyAgent.app' ]; then
  echo "create-macos-dmg.sh requires a FyAgent.app bundle: $app_path" >&2
  exit 1
fi
if [ ! -f "$background_path" ] || [ -L "$background_path" ]; then
  echo "create-macos-dmg.sh requires a regular background PNG: $background_path" >&2
  exit 1
fi

scratch_root="${RUNNER_TEMP:-${TMPDIR:-/tmp}}"
scratch="$(mktemp -d "$scratch_root/fyagent-dmg.XXXXXX")"
stage="$scratch/stage"
udrw_path="$scratch/FyAgent-rw.dmg"
mount_point="$scratch/mount"
mounted=false

detach_mount() {
  attempt=1
  max_attempts=5
  while true; do
    if hdiutil detach "$mount_point"; then
      mounted=false
      return 0
    fi
    status=$?
    if [ "$attempt" -ge "$max_attempts" ]; then
      return "$status"
    fi
    delay=$((1 << attempt))
    echo "hdiutil detach reported a busy volume; retrying attempt $((attempt + 1)) of $max_attempts after ${delay}s" >&2
    sleep "$delay"
    attempt=$((attempt + 1))
  done
}

attach_udrw() {
  attempt=1
  max_attempts=5
  while true; do
    if hdiutil attach "$udrw_path" -readwrite -noverify -nobrowse -noautoopen -mountpoint "$mount_point"; then
      mounted=true
      return 0
    fi
    status=$?
    if [ "$attempt" -ge "$max_attempts" ]; then
      return "$status"
    fi
    delay=$((1 << attempt))
    echo "hdiutil attach reported a busy volume; retrying attempt $((attempt + 1)) of $max_attempts after ${delay}s" >&2
    sleep "$delay"
    attempt=$((attempt + 1))
  done
}

cleanup() {
  status=$?
  trap - EXIT
  set +e
  if [ "$mounted" = true ]; then
    detach_mount || true
  fi
  rm -rf "$scratch"
  exit "$status"
}
trap cleanup EXIT

mkdir -p "$stage/.background" "$mount_point" "$(dirname "$output_path")"
ditto "$app_path" "$stage/FyAgent.app"
ln -s /Applications "$stage/Applications"
cp "$background_path" "$stage/.background/background.png"

"$RETRY_HDIUTIL" \
  "$udrw_path" \
  -- \
  create -volname 'FyAgent' -srcfolder "$stage" -ov -fs HFS+ -format UDRW "$udrw_path"

attach_udrw

uv run --locked --group dmg-layout python "$LAYOUT_WRITER" \
  --mount "$mount_point" \
  --app FyAgent.app \
  --applications Applications \
  --background .background/background.png \
  --window 660x400 \
  --icon-size 128 \
  --app-xy 180,188 \
  --apps-xy 480,188

[ -f "$mount_point/.DS_Store" ]
rm -rf \
  "$mount_point/.fseventsd" \
  "$mount_point/.Trashes" \
  "$mount_point/.Spotlight-V100"
if [ -e "$mount_point/.background" ]; then
  chflags hidden "$mount_point/.background"
fi
if [ -e "$mount_point/.fseventsd" ]; then
  chflags hidden "$mount_point/.fseventsd"
fi
sync
detach_mount

"$RETRY_HDIUTIL" \
  "$output_path" \
  -- \
  convert "$udrw_path" -format UDZO -imagekey zlib-level=9 -ov -o "$output_path"
"$RETRY_HDIUTIL" \
  "$output_path" \
  -- \
  verify "$output_path"
