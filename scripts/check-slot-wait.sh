#!/usr/bin/env bash
set -u
repo=$(cd "$(dirname "$0")/.." && pwd)
record=$(mktemp)
mkdir -p /tmp/cargo-slots
flock /tmp/cargo-slots/1.lock sleep 3 &
holder1=$!
flock /tmp/cargo-slots/2.lock sleep 3 &
holder2=$!
sleep 0.3
RECORD="$record" "$repo/scripts/measure-build.sh" slot-wait-check --version > /dev/null 2>&1
rc=$?
kill "$holder1" "$holder2" 2>/dev/null
wait "$holder1" "$holder2" 2>/dev/null
result=$(node -e 'const x = JSON.parse(require("fs").readFileSync(process.argv[1], "utf8").trim()); const wait = Number(x.slotWaitS); const wall = Number(x.build.wallS); const ok = x.rc === 0 && wait >= 1.5 && wait <= 3.5 && wall < 1.0; console.log(JSON.stringify({ ok, rc: x.rc, slotWaitS: wait, buildWallS: wall }))' "$record")
rm -f "$record"
echo "$result"
node -e 'process.exit(JSON.parse(process.argv[1]).ok ? 0 : 1)' "$result"
