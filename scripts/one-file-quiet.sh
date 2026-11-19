#!/usr/bin/env bash
set -u
repo=$(cd "$(dirname "$0")/.." && pwd)
cd "$repo"
prefix=$1
attempts=${ATTEMPTS:-6}
want=${WANT:-3}
record=${RECORD:-$repo/target/witness/build-measure.jsonl}
quiet=0
for n in $(seq 1 "$attempts"); do
    touch src/viewer.rs
    "$repo/scripts/measure-build.sh" "$prefix-$n" build --profile iterate --timings -v > /dev/null 2>&1
    if grep "\"label\":\"$prefix-$n\"" "$record" | tail -n 1 | grep -q '"swapNoisy":false'; then
        quiet=$((quiet + 1))
    fi
    [ "$quiet" -ge "$want" ] && break
done
node "$repo/scripts/lib/quiet-figure.cjs" "$record" "$prefix-"
