#!/usr/bin/env bash
set -u
repo=$(cd "$(dirname "$0")/.." && pwd)
fake=$(mktemp -d)
target=$(mktemp -d)
cat > "$fake/cargo" <<'FAKE'
#!/bin/sh
sleep 0.4
echo "    Finished iterate profile [optimized] target(s) in 0.10s"
sleep 0.2
FAKE
chmod +x "$fake/cargo"
json=$(CARGO_BIN="$fake/cargo" CARGO_TARGET_DIR="$target" "$repo/scripts/build-timed.sh" build --profile iterate)
rm -rf "$fake" "$target"
echo "$json" | node -e 'const x = JSON.parse(require("fs").readFileSync(0, "utf8")); const ok = x.rc === 0 && x.cargoFinishedS === 0.1 && x.outsideCargoS >= 0.4 && x.outsideCargoS <= 0.9 && x.wallS >= 0.5; console.log(JSON.stringify({ ok, rc: x.rc, wallS: x.wallS, cargoFinishedS: x.cargoFinishedS, outsideCargoS: x.outsideCargoS })); process.exit(ok ? 0 : 1)'
