#!/usr/bin/env bash
set -u
repo=$(cd "$(dirname "$0")/.." && pwd)
jayzee=${JAYZEE:-/workspace/jayzee}
binary=${BINARY:-$repo/target/release/bevy-jayzee}
ticks=${TICKS:-300}
runs=${RUNS:-5}
out=$repo/target/witness/bench-vs-jayzee.json
mkdir -p "$repo/target/witness"
lines=()
for bodies in 500 2000 5000; do
    for run in $(seq 1 "$runs"); do
        before=$(cat /proc/loadavg)
        j=$(cd "$jayzee" && node scripts/bench.mjs "$bodies" "$ticks") || exit 1
        b=$("$binary" bench "$bodies" "$ticks" | grep '"kind":"bench"' | tail -1) || exit 1
        after=$(cat /proc/loadavg)
        lines+=("{\"bodies\":$bodies,\"run\":$run,\"loadBefore\":\"$before\",\"loadAfter\":\"$after\",\"jayzee\":$j,\"bevy\":$b}")
    done
done
printf '%s\n' "${lines[@]}" | CORES="$(nproc)" node "$repo/scripts/bench-median.cjs" "$out"
