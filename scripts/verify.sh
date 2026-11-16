#!/usr/bin/env bash
set -euo pipefail
root=$(cd "$(dirname "$0")/.." && pwd)
bin="$root/target/release/bevy-jayzee"
for bodies in 500 2000 5000; do
  "$bin" bench "$bodies" 300
done
"$bin" det 2000 300
"$bin" server >"$root/target/server.log" 2>&1 &
server_pid=$!
sleep 0.5
set +e
timeout 25 "$bin" client >"$root/target/client.log" 2>&1
client_status=$?
set -e
kill "$server_pid" 2>/dev/null || true
wait "$server_pid" 2>/dev/null || true
cat "$root/target/client.log"
tail -n 3 "$root/target/server.log"
if [ "$client_status" -ne 0 ]; then
  exit "$client_status"
fi
node "$root/scripts/check-network.cjs" "$root/target/client.log" "$root/target/server.log"
