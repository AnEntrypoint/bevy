#!/usr/bin/env bash
# usage: DISPLAY=:0 capture-viewer-frames.sh <out-dir> <frames> <seconds-apart>
# Captures the bevy-jayzee viewer window with xwd and runs check-viewer-streaks on each frame.
out=$1
frames=$2
gap=$3
root=$(cd "$(dirname "$0")/.." && pwd)
mkdir -p "$out"
win=$(xdotool search --name 'bevy-jayzee viewer' | head -1)
[ -n "$win" ] || { echo 'no bevy-jayzee viewer window on this display' >&2; exit 3; }
for i in $(seq 1 "$frames"); do
  f="$out/frame-$(printf %03d "$i").xwd"
  xwd -silent -id "$win" > "$f" || exit 4
  echo "$i $(date +%T) $(node "$root/scripts/check-viewer-streaks.cjs" "$f")" >> "$out/series.log"
  sleep "$gap"
done
