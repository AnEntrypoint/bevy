#!/usr/bin/env bash
set -euo pipefail
root=$(cd "$(dirname "$0")/.." && pwd)
bin="$root/target/debug/bevy-jayzee"
if [ ! -x "$bin" ]; then
  bin="$root/target/release/bevy-jayzee"
fi
directory=${1:-testdata/spec-world}
place=${2:-test}
"$bin" web "$directory" "$place" >/dev/null
name=$(basename "$directory")-${place}
cp "$root/web/status-${place}.json" "$root/web/status.json"
printf '{"kind":"web-report","path":"web/status.json","source":"%s","place":"%s"}\n' "$directory" "$place"
