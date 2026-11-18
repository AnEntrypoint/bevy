#!/usr/bin/env bash
set -u
repo=$(cd "$(dirname "$0")/.." && pwd)
cd "$repo" || exit 1
place=${PLACE:-main}
world=${WORLD:-/workspace/neon-syndicates/world}
display=${DISPLAY:-:1}
binary=${BINARY:-target/release/bevy-jayzee}
settle=${SETTLE_S:-10}
witness=target/witness
status=web/status-$place.json
export VIEWER_EYE="${VIEWER_EYE:-0,300,1}"
mkdir -p "$witness"
modes=${MODES:-bc7 rgba8}
failed=0

for mode in $modes; do
    case $mode in
        bc7) extra=() ;;
        rgba8) extra=(VIEWER_DISABLE_BC=1 VIEWER_DISABLE_ASTC=1) ;;
        *) echo "unknown mode $mode" >&2; exit 2 ;;
    esac
    rm -f "$status"
    env ${extra[@]+"${extra[@]}"} DISPLAY=$display "$binary" viewer "$world" "$place" > "$witness/resources-$mode.out" 2> "$witness/resources-$mode.err" < /dev/null &
    vpid=$!
    echo "mode=$mode viewer_pid=$vpid"
    ready=0
    for _ in $(seq 1 240); do
        if [ -f "$status" ] && node -e 'const s = require(process.argv[1]); process.exit(s.uiLabels > 0 ? 0 : 1)' "$repo/$status" 2>/dev/null; then
            ready=1
            break
        fi
        sleep 0.5
    done
    sleep "$settle"
    cp "$status" "$witness/resources-$mode.json" 2>/dev/null
    kill "$vpid" 2>/dev/null
    wait "$vpid" 2>/dev/null
    if [ "$ready" -ne 1 ]; then
        echo "mode $mode: status never reported labels" >&2
        failed=1
    fi
done

node -e '
const fs = require("node:fs");
const [witness, place, modesText] = process.argv.slice(1);
const expected = { bc7: "bc7", rgba8: "rgba8" };
const rows = modesText.split(" ").map((mode) => {
  const path = `${witness}/resources-${mode}.json`;
  if (!fs.existsSync(path)) return { mode, ok: false, reason: "no status file" };
  const s = JSON.parse(fs.readFileSync(path, "utf8"));
  const loadErrors = (fs.readFileSync(`${witness}/resources-${mode}.err`, "utf8").match(/Failed to load asset/g) || []).length;
  const row = {
    mode,
    textureTarget: s.textureTarget,
    bcSupported: s.bcSupported,
    modelsLoaded: s.modelsLoaded,
    modelsMissing: (s.modelsMissing || []).length,
    loadErrors,
    objects: s.objects,
    processRssMiB: s.processRssKib == null ? null : +(s.processRssKib / 1024).toFixed(1),
    imageCount: s.imageCount,
    imageMiB: s.imageBytes == null ? null : +(s.imageBytes / 1048576).toFixed(2),
    frameMsAverage: s.frameMsAverage == null ? null : +s.frameMsAverage.toFixed(3),
    fpsAverage: s.fpsAverage == null ? null : +s.fpsAverage.toFixed(2),
  };
  row.ok = s.textureTarget === expected[mode] && row.modelsLoaded > 0 && row.modelsMissing === 0 && row.loadErrors === 0 && row.processRssMiB !== null && row.frameMsAverage !== null;
  if (!row.ok) row.reason = `textureTarget ${s.textureTarget} expected ${expected[mode]}, loaded ${row.modelsLoaded}, missing ${row.modelsMissing}, load errors ${row.loadErrors}`;
  return row;
});
const out = { kind: "viewer-resources", place, rows, ok: rows.every((row) => row.ok) };
fs.writeFileSync(`${witness}/viewer-resources-${place}.json`, JSON.stringify(out, null, 2));
console.log(JSON.stringify(out, null, 2));
process.exit(out.ok ? 0 : 1);
' "$repo/$witness" "$place" "$modes"
rc=$?
if [ "$failed" -ne 0 ]; then
    exit 1
fi
exit "$rc"
