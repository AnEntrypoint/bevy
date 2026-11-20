#!/usr/bin/env bash
cd "$(dirname "$0")/.." || exit 2
port=${PORT:-18093}
PORT=$port python3 scripts/serve-wasm.py > /tmp/bt/static-headers-server.log 2>&1 &
pid=$!
trap 'kill $pid 2>/dev/null' EXIT
echo "SERVER pid=$pid port=$port cwd=$(pwd)"
for _ in $(seq 1 50); do curl -s -o /dev/null "http://127.0.0.1:$port/" && break; sleep 0.2; done
status=0
for path in /AGENTS.md /no-such-file; do
  headers=$(curl -s -D - -o /dev/null "http://127.0.0.1:$port$path" | tr -d '\r')
  coop=$(printf '%s\n' "$headers" | awk -F': ' 'tolower($1)=="cross-origin-opener-policy"{print $2}')
  coep=$(printf '%s\n' "$headers" | awk -F': ' 'tolower($1)=="cross-origin-embedder-policy"{print $2}')
  echo "$path coop=$coop coep=$coep"
  [ "$coop" = same-origin ] && [ "$coep" = require-corp ] || status=1
done
kill $pid 2>/dev/null; wait $pid 2>/dev/null; trap - EXIT
echo "SERVER stopped pid=$pid"
if [ $status = 0 ]; then echo STATIC-HEADERS PASS; else echo STATIC-HEADERS FAIL; fi
exit $status
