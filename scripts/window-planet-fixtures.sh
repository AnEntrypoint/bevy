#!/usr/bin/env bash
cd /workspace/bevy || exit 2
scratch=/workspace/tmp/bevy/window-fixtures
log=target/witness/window-planet-fixtures.log
mkdir -p target/witness "$scratch"
: > "$log"
overall=0
note() { echo "$@" | tee -a "$log"; }
for gen in climate splat atmosphere veg-placement rock-placement grass-placement; do
  fixture=testdata/parity/spoint-planet-$gen.json
  node scripts/gen-planet-$gen.mjs /workspace/spoint > "$scratch/$gen.out" 2>&1
  git diff --exit-code --quiet -- "$fixture" && git ls-files --error-unmatch "$fixture" > /dev/null 2>&1
  rc=$?
  note "WITNESS gen-planet-$gen rc=$rc"
  [ "$rc" -ne 0 ] && overall=1
  cp "$fixture" "$scratch/$gen.good"
  python3 -I - "$fixture" <<'PY'
import pathlib, sys
p = pathlib.Path(sys.argv[1])
t = p.read_text(encoding="utf-8")
i = next(k for k, c in enumerate(t) if c.isdigit() and k > 200)
p.write_text(t[:i] + ("1" if t[i] != "1" else "2") + t[i + 1:], encoding="utf-8")
PY
  cmp -s "$fixture" "$scratch/$gen.good"; note "MUTANT gen-$gen-fixture-drift file-differs-rc=$?"
  node scripts/gen-planet-$gen.mjs /workspace/spoint > "$scratch/$gen.redo.out" 2>&1
  cmp -s "$fixture" "$scratch/$gen.good" && note "DETECTED gen-$gen-fixture-drift regenerated fixture restored the bytes" || { note "NOT-DETECTED gen-$gen-fixture-drift"; overall=1; }
  cp "$scratch/$gen.good" "$fixture"
done
git -C /workspace/spoint status --porcelain | wc -l | xargs -I{} bash -c '[ {} -eq 0 ] && echo "WITNESS spoint-clean rc=0" || echo "WITNESS spoint-clean rc=1"' | tee -a "$log"
rm -rf "$scratch"
note "WITNESS-SET overall=$overall $(date -u +%T)Z"
exit $overall
