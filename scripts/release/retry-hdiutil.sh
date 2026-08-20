#!/usr/bin/env bash

set -euo pipefail

export LC_ALL=C

if [ "$#" -lt 3 ] || [ "$2" != "--" ]; then
  echo "Usage: retry-hdiutil.sh <output-path> -- <hdiutil-arguments...>" >&2
  exit 2
fi

output_path="$1"
shift 2

if [ -z "$output_path" ] || [ "$#" -eq 0 ]; then
  echo "retry-hdiutil.sh requires a non-empty output path and hdiutil arguments" >&2
  exit 2
fi

operation="$1"
case "$operation" in
  create)
    remove_output_on_failure=true
    ;;
  verify)
    remove_output_on_failure=false
    if [ ! -f "$output_path" ]; then
      echo "retry-hdiutil.sh verify input does not exist: $output_path" >&2
      exit 2
    fi
    ;;
  *)
    echo "retry-hdiutil.sh supports only hdiutil create or verify" >&2
    exit 2
    ;;
esac

log_file="$(mktemp "${TMPDIR:-/tmp}/fyagent-hdiutil.XXXXXX")"
cleanup_log() {
  rm -f -- "$log_file"
}
trap cleanup_log EXIT

if [ "$remove_output_on_failure" = true ]; then
  if ! rm -f -- "$output_path"; then
    echo "Unable to remove the existing hdiutil output: $output_path" >&2
    exit 1
  fi
fi

attempt=1
max_attempts=5

# Capture directly to a file so diagnostic inspection never puts hdiutil in a
# pipeline with a reader that can exit while the disk-image helper is active.
while true; do
  if hdiutil "$@" >"$log_file" 2>&1; then
    cat "$log_file"
    exit 0
  else
    status=$?
  fi

  cat "$log_file" >&2
  if [ "$remove_output_on_failure" = true ]; then
    if ! rm -f -- "$output_path"; then
      echo "Unable to remove the partial hdiutil output: $output_path" >&2
      exit "$status"
    fi
  fi

  if grep -Fq -- 'Resource busy' "$log_file"; then
    transient_diagnostic='Resource busy'
  elif grep -Fq -- 'Resource temporarily unavailable' "$log_file"; then
    transient_diagnostic='Resource temporarily unavailable'
  else
    exit "$status"
  fi
  if [ "$attempt" -ge "$max_attempts" ]; then
    exit "$status"
  fi

  delay=$((1 << attempt))
  echo "hdiutil reported $transient_diagnostic; retrying attempt $((attempt + 1)) of $max_attempts after ${delay}s" >&2
  sleep "$delay"
  attempt=$((attempt + 1))
done
