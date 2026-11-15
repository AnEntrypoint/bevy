#!/usr/bin/env bash
set -euo pipefail
root=$(cd "$(dirname "$0")/.." && pwd)
"$root/scripts/generate_viewer_report.sh" "${@}" >/dev/null
port=${PORT:-8080}
cd "$root"
exec python3 -m http.server "$port" --bind 127.0.0.1
