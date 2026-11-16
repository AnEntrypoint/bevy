#!/usr/bin/env bash
set -euo pipefail
root=$(cd "$(dirname "$0")/.." && pwd)
profile=${WASM_PROFILE:-release}
out="$root/web/wasm"
cli="$root/target/tools-wasm/bin/wasm-bindgen"
if [ ! -x "$cli" ]; then
  cargo install wasm-bindgen-cli --version 0.2.129 --offline --locked --root "$root/target/tools-wasm"
fi
export CARGO_TARGET_DIR="$root/target/wasm-viewer"
if [ "$profile" = release ]; then
  cargo build --manifest-path "$root/wasm-viewer/Cargo.toml" --release --target wasm32-unknown-unknown --locked
  wasm="$CARGO_TARGET_DIR/wasm32-unknown-unknown/release/bevy_jayzee_wasm.wasm"
else
  cargo build --manifest-path "$root/wasm-viewer/Cargo.toml" --target wasm32-unknown-unknown --locked
  wasm="$CARGO_TARGET_DIR/wasm32-unknown-unknown/debug/bevy_jayzee_wasm.wasm"
fi
test -f "$wasm"
mkdir -p "$out"
"$cli" --target web --out-dir "$out" --out-name bevy-jayzee-wasm "$wasm"
cp "$root/web/wasm-index.html" "$out/index.html"
cp "$root/web/wasm-boot.js" "$out/wasm-boot.js"
cp "$root/web/favicon.svg" "$out/favicon.svg"
cp "$root/web/inspector.js" "$out/inspector.js"
cp "$root/web/status.json" "$out/status.json" 2>/dev/null || true
printf '{"kind":"wasm-build","profile":"%s","cli":"%s","output":"%s"}\n' "$profile" "$("$cli" --version)" "$out"
find "$out" -maxdepth 1 -type f -printf '%f %s bytes\n' | sort
sha256sum "$out"/*.wasm "$out"/*.js
