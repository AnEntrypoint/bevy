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
cargo_bin=${CARGO_BIN:-/config/.local/bin/cargo}
mkdir -p "$repo/target"
start=$(date +%s.%N)
(cd "$repo" && CARGO_TARGET_DIR="$target" "$cargo_bin" "$@") > "$log" 2>&1
rc=$?
end=$(date +%s.%N)
crates=$(grep -c '^ *Compiling ' "$log")
warnings=$(grep -cE '^(warning|error)' "$log")
wall=$(awk -v s="$start" -v e="$end" 'BEGIN { printf "%.1f", e - s }')
finished=$(grep -E '^ *Finished ' "$log" | tail -n 1 | sed -n 's/.* in \([0-9.]*\)s$/\1/p')
[ -n "$finished" ] || finished=0
outside=$(awk -v w="$wall" -v f="$finished" 'BEGIN { printf "%.1f", w - f }')
printf '{"kind":"build-timed","args":"%s","rc":%d,"wallS":%s,"cargoFinishedS":%s,"outsideCargoS":%s,"cratesCompiled":%d,"warnings":%d,"target":"%s"}\n' "$*" "$rc" "$wall" "$finished" "$outside" "$crates" "$warnings" "$target"
exit $rc
