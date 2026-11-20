#!/usr/bin/env bash
set -u
name=${1:?usage: capture-default-game.sh <name>}
cd "$(dirname "$0")/.." || exit 2
mkdir -p target/witness
timeout 150 target/debug/bevy-jayzee > "target/witness/$name.log" 2>&1 &
pid=$!
sleep 2
child=$(pgrep -P "$pid" | head -1)
echo "PID game timeout=$pid bevy=$child"
sleep 45
xwd -root -silent -display "${DISPLAY:-:1}" -out "target/witness/$name.xwd"
echo "xwd rc=$?"
[ -n "$child" ] && kill "$child" 2>/dev/null
kill "$pid" 2>/dev/null
wait "$pid"
echo "game stopped rc=$?"
