#!/usr/bin/env bash
set -u
repo=$(cd "$(dirname "$0")/.." && pwd)
cargo_bin=${CARGO_BIN:-}
[ -n "$cargo_bin" ] || cargo_bin=/config/.local/bin/cargo
jobs=""
root=""
keep=0
usage() {
  echo "usage: measure-clean-build.sh [--jobs N] [--target-root DIR] [--keep]" >&2
}
while [ $# -gt 0 ]; do
  case "$1" in
    --jobs)
      [ $# -ge 2 ] || { usage; exit 2; }
      jobs=$2
      shift 2
      ;;
    --target-root)
      [ $# -ge 2 ] || { usage; exit 2; }
      root=$2
      shift 2
      ;;
    --keep)
      keep=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "measure-clean-build: unknown argument: $1" >&2
      usage
      exit 2
      ;;
  esac
done
if [ -z "$jobs" ]; then
  jobs=$(sed -n 's/^jobs = \([0-9][0-9]*\)$/\1/p' "$repo/.cargo/config.toml" | head -n 1)
fi
case "$jobs" in
  ''|*[!0-9]*)
    echo "measure-clean-build: jobs must be a positive integer, got '$jobs'" >&2
    exit 2
    ;;
esac
[ "$jobs" -ge 1 ] || { echo "measure-clean-build: jobs must be at least 1" >&2; exit 2; }
if [ -n "$root" ]; then
  case "$(basename "$root")" in
    debug|release)
      echo "measure-clean-build: --target-root must be a target root, not a profile directory: $root" >&2
      exit 2
      ;;
  esac
  mkdir -p "$root" || exit 2
  if [ -n "$(ls -A "$root")" ]; then
    echo "measure-clean-build: --target-root is not empty: $root" >&2
    exit 2
  fi
else
  root=$(mktemp -d /tmp/bevy-clean.XXXXXX)
fi
si=0
so=0
if [ -z "${MEASURE_NO_GATE:-}" ]; then
  read -r si so < <(vmstat 1 2 | tail -n 1 | awk '{print $7, $8}')
  load_before=$(cut -d' ' -f1 /proc/loadavg)
  if ! { [ "$si" -lt 1000 ] && [ "$so" -lt 1000 ] && awk -v l="$load_before" 'BEGIN { exit !(l <= 4) }'; }; then
    echo "measure-clean-build: gate closed (si=$si so=$so load1=$load_before); no build" >&2
    rmdir "$root" 2>/dev/null
    exit 3
  fi
else
  load_before=$(cut -d' ' -f1 /proc/loadavg)
fi
log=$(mktemp /tmp/measure-clean-build.XXXXXX.log)
samples=$(mktemp /tmp/measure-clean-build.XXXXXX.load)
(while :; do cut -d' ' -f1 /proc/loadavg >> "$samples"; sleep 1; done) > /dev/null 2>&1 &
sampler=$!
start=$(date +%s.%N)
(cd "$repo" && CARGO_TARGET_DIR="$root" "$cargo_bin" build --jobs "$jobs") > "$log" 2>&1
rc=$?
end=$(date +%s.%N)
kill "$sampler" 2>/dev/null
wait "$sampler" 2>/dev/null
wall=$(awk -v s="$start" -v e="$end" 'BEGIN { printf "%.1f", e - s }')
finished=$(grep -E '^ *Finished ' "$log" | tail -n 1 | sed -n 's/.* in \([0-9.]*\)s$/\1/p')
[ -n "$finished" ] || finished=0
peak=$(awk 'BEGIN { m = 0 } $1 + 0 > m { m = $1 + 0 } END { printf "%.2f", m }' "$samples")
rm -f "$samples"
printf 'CLEAN_BUILD jobs=%s rc=%s wall_s=%s cargo_finished_s=%s peak_load1=%s gate_si=%s gate_so=%s load1_before=%s target_root=%s\n' "$jobs" "$rc" "$wall" "$finished" "$peak" "$si" "$so" "$load_before" "$root"
echo "measure-clean-build: log $log" >&2
if [ "$keep" -eq 0 ]; then
  rm -rf "$root"
fi
exit "$rc"
