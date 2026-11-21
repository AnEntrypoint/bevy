#!/usr/bin/env bash
log=target/witness/window-planet-stage1.log
. /workspace/bevy/scripts/lib/window.sh
: > "$log"
run build-debug cargo build --bin bevy-jayzee
run warnings-bin bash -c 'touch src/main.rs; out=$(cargo build --bin bevy-jayzee 2>&1); echo "$out" | tail -n 3; ! echo "$out" | grep -q "^warning"'
run warnings-test bash -c 'touch src/main.rs; out=$(cargo test --bin bevy-jayzee --no-run 2>&1); echo "$out" | tail -n 3; ! echo "$out" | grep -q "^warning"'
run mutant-tables node scripts/check-mutant-tables.mjs
run bin-witness cargo test --bin bevy-jayzee
for gen in quadtree near-field collider patch; do
  run gen-planet-$gen bash -c "node scripts/gen-planet-$gen.mjs /workspace/spoint && git diff --exit-code -- testdata/parity/spoint-planet-$gen.json"
done
for table in scripts/mutants/*.tsv; do mutants "$table"; done
finish
