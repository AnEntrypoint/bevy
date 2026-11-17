#!/usr/bin/env bash
set -u
repo=$(cd "$(dirname "$0")/.." && pwd)
cd "$repo" || exit 1
place=${PLACE:-main}
world=${WORLD:-/workspace/neon-syndicates/world}
display=${DISPLAY:-:1}
witness=target/witness
status=web/status-$place.json
binary=${BINARY:-target/debug/bevy-jayzee}
export VIEWER_EYE="${VIEWER_EYE:-0,300,1}"
mkdir -p "$witness"
rm -f "$status"

DISPLAY=$display "$binary" viewer "$world" "$place" > "$witness/hud-viewer.out" 2> "$witness/hud-viewer.err" < /dev/null &
vpid=$!
echo "viewer_pid=$vpid"

for _ in $(seq 1 120); do
    xwininfo -root -tree 2>/dev/null | grep -q 'bevy-jayzee viewer' && break
    sleep 0.5
done

ready=0
for _ in $(seq 1 240); do
    if [ -f "$status" ] && node -e 'const s = require(process.argv[1]); process.exit(s.uiLabels > 0 ? 0 : 1)' "$repo/$status" 2>/dev/null; then
        ready=1
        break
    fi
    sleep 0.5
done
sleep 2

wid=$(xwininfo -root -tree | awk '/bevy-jayzee viewer/ {print $1; exit}')
xwd -id "$wid" -silent -out "$witness/neon-hud.xwd"
python3 -I scripts/xwd2png.py "$witness/neon-hud.xwd" "$witness/neon-hud.png"
png_rc=$?
cp "$status" "$witness/hud-status-$place.json"

kill "$vpid" 2>/dev/null
wait "$vpid" 2>/dev/null

node -e '
const fs = require("node:fs");
const s = JSON.parse(fs.readFileSync(process.argv[1], "utf8"));
const out = {
  place: process.argv[2],
  ready: process.argv[3] === "1",
  uiLabels: s.uiLabels,
  uiLabelOverlaps: s.uiLabelOverlaps,
  uiUncoveredGlyphs: s.uiUncoveredGlyphs,
  uiWindowWidth: s.uiWindowWidth,
  uiFullWidthLabels: s.uiFullWidthLabels,
  textureTarget: s.textureTarget,
  modelsLoaded: s.modelsLoaded,
  modelsMissing: (s.modelsMissing || []).length,
  uiAnchorsInScene: s.uiAnchorsInScene,
  uiAnchorsPlaced: s.uiAnchorsPlaced,
  uiAnchorsUnresolved: s.uiAnchorsUnresolved,
  uiAnchorsBehind: s.uiAnchorsBehind,
  uiAnchorsOffscreen: s.uiAnchorsOffscreen,
  uiAnchorViewport: s.uiAnchorViewport,
  uiAnchorPositions: s.uiAnchorPositions,
  uiAnchorWorld: s.uiAnchorWorld,
  viewerEye: process.env.VIEWER_EYE,
  topCentreOffset: (() => {
    const tops = (s.uiTranslatedBoxes || []).filter((r) => r[1] < 80);
    return tops.length ? Math.min(...tops.map((r) => Math.abs((r[0] + r[2]) / 2 - s.uiWindowWidth / 2))) : null;
  })(),
  anchorsInsideViewport: (s.uiAnchorPositions || []).filter((p) => p.x >= 0 && p.y >= 0 && p.x <= s.uiAnchorViewport[0] && p.y <= s.uiAnchorViewport[1]).length,
  screenshot: process.argv[4],
  ok: process.argv[3] === "1" && (s.uiTranslatedBoxes || []).some((r) => r[1] < 80 && Math.abs((r[0] + r[2]) / 2 - s.uiWindowWidth / 2) <= 2) && s.uiLabels > 0 && s.uiLabelOverlaps === 0 && s.uiUncoveredGlyphs === 0 && s.uiFullWidthLabels === 0 && (s.modelsMissing || []).length === 0 && s.uiAnchorsInScene > 0 && s.uiAnchorsPlaced === s.uiAnchorsInScene && (s.uiAnchorPositions || []).every((p) => p.x >= 0 && p.y >= 0 && p.x <= s.uiAnchorViewport[0] && p.y <= s.uiAnchorViewport[1]),
};
fs.writeFileSync(process.argv[5], JSON.stringify(out, null, 2));
console.log(JSON.stringify(out, null, 2));
process.exit(out.ok ? 0 : 1);
' "$repo/$witness/hud-status-$place.json" "$place" "$ready" "$witness/neon-hud.png" "$witness/hud-overlap-$place.json"
rc=$?
if [ "$png_rc" -ne 0 ]; then
    echo "screenshot conversion failed (rc=$png_rc)" >&2
    exit 1
fi
exit "$rc"
