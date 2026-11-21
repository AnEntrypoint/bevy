#!/usr/bin/env bash
set -u
name=${1:?usage: census-run.sh <name>}
cd /workspace/bevy || exit 2
mkdir -p target/witness
png=target/witness/$name.png
log=target/witness/$name.log
rm -f "$png"
BEVY_JAYZEE_CENSUS=1 BEVY_JAYZEE_CENSUS_OUT="$PWD/$png" timeout 150 target/debug/bevy-jayzee > "$log" 2>&1 &
pid=$!
sleep 2
child=$(pgrep -P "$pid" | head -1)
echo "PID census $name timeout=$pid bevy=$child $(date -u +%T)Z"
ready=no
for i in $(seq 1 120); do
  if grep -q 'census: captured' "$log" 2>/dev/null && [ -s "$png" ]; then sleep 5; ready=yes; break; fi
  sleep 1
done
echo "png_ready=$ready waited=${i}s size=$(stat -c %s "$png" 2>/dev/null)"
[ -n "$child" ] && kill "$child" 2>/dev/null
kill "$pid" 2>/dev/null
wait "$pid"
echo "census game stopped rc=$?"
echo "frozen_lines=$(grep -c 'census: simulation frozen' "$log") captured_lines=$(grep -c 'census: captured' "$log")"
