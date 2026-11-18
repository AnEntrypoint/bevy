#!/usr/bin/env bash
set -u
repo=$(cd "$(dirname "$0")/.." && pwd)
target=${CARGO_TARGET_DIR:-$repo/target}
case "$(basename "$target")" in
  debug|release)
    echo "build-timed: CARGO_TARGET_DIR must be a target root, not a profile directory: $target" >&2
    exit 2
    ;;
esac
log="$repo/target/build-timed-last.log"
mkdir -p "$repo/target"
start=$(date +%s.%N)
(cd "$repo" && CARGO_TARGET_DIR="$target" /config/.local/bin/cargo "$@") > "$log" 2>&1
rc=$?
end=$(date +%s.%N)
crates=$(grep -c '^ *Compiling ' "$log")
warnings=$(grep -cE '^(warning|error)' "$log")
wall=$(awk -v s="$start" -v e="$end" 'BEGIN { printf "%.1f", e - s }')
printf '{"kind":"build-timed","args":"%s","rc":%d,"wallS":%s,"cratesCompiled":%d,"warnings":%d,"target":"%s"}\n' "$*" "$rc" "$wall" "$crates" "$warnings" "$target"
exit $rc
