#!/usr/bin/env bash
cd /workspace/bevy || exit 2
scratch=/workspace/tmp/bevy/window-table
log=target/witness/window-planet-table.log
mkdir -p target/witness "$scratch"
overall=0
note() { echo "$@" | tee -a "$log"; }
: > "$log"

node scripts/check-planet-parity-table.mjs > "$scratch/real.out" 2>&1
rc=$?
note "WITNESS planet-table rc=$rc $(head -n 1 "$scratch/real.out")"
[ "$rc" -ne 0 ] && overall=1

red() {
  name=$1; row=$2; cell=$3; replacement=$4
  python3 -I - "$row" "$cell" "$replacement" "$scratch/$name.md" <<'PY'
import pathlib, sys
row, cell, replacement, out = sys.argv[1], int(sys.argv[2]), sys.argv[3], sys.argv[4]
lines = pathlib.Path('docs/planet-parity.md').read_text(encoding='utf-8').split('\n')
hits = [i for i, l in enumerate(lines) if l.startswith('| ' + row + ' | ')]
assert len(hits) == 1, hits
cells = lines[hits[0]][2:-2].split(' | ')
cells[cell] = replacement
lines[hits[0]] = '| ' + ' | '.join(cells) + ' |'
pathlib.Path(out).write_text('\n'.join(lines), encoding='utf-8')
PY
  [ $? -ne 0 ] && { note "MUTANT-APPLY-FAILED $name"; overall=1; return; }
  node scripts/check-planet-parity-table.mjs "$scratch/$name.md" > "$scratch/$name.out" 2>&1
  mrc=$?
  if [ "$mrc" -ne 0 ]; then note "DETECTED $name rc=$mrc $(sed -n 2p "$scratch/$name.out" | cut -c1-90)"; else note "NOT-DETECTED $name"; overall=1; fi
}

red source-line-gone geo:rad.R 3 'packages/mapspinner/src/planet-radius.js:99999'
red value-not-on-line geo:rad.R 2 'qqqqqq'
red source-file-gone geo:frm.defaultSize 3 'packages/mapspinner/src/quadtree-gone.js:1'

rm -rf "$scratch"
note "WITNESS-SET overall=$overall $(date -u +%T)Z"
exit $overall
