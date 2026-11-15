#!/usr/bin/env bash
set -euo pipefail
root=$(cd "$(dirname "$0")/.." && pwd)
port=${PORT:-8092}
exec python3 "$root/scripts/serve-wasm.py"
