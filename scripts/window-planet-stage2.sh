#!/usr/bin/env bash
part=${1:?usage: window-planet-stage2.sh core|pure|terrain|water|sky|light}
log=target/witness/window-planet-stage2-$part.log
. /workspace/bevy/scripts/lib/window.sh
: > "$log"
tables() { for name in "$@"; do mutants scripts/mutants/planet-$name.tsv; done; }
case "$part" in
  core)
    run build-debug cargo build --bin bevy-jayzee
    run warnings-bin bash -c 'touch src/main.rs; out=$(cargo build --bin bevy-jayzee 2>&1); echo "$out" | tail -n 3; ! echo "$out" | grep -q "^warning"'
    run warnings-test bash -c 'touch src/main.rs; out=$(cargo test --bin bevy-jayzee --no-run 2>&1); echo "$out" | tail -n 3; ! echo "$out" | grep -q "^warning"'
    run mutant-tables node scripts/check-mutant-tables.mjs
    run bin-witness cargo test --bin bevy-jayzee
    for gen in quadtree near-field collider patch climate splat atmosphere waves layers; do
      run gen-planet-$gen bash -c "node scripts/gen-planet-$gen.mjs /workspace/spoint && git diff --exit-code -- testdata/parity/spoint-planet-$gen.json"
    done
    ;;
  pure) tables climate splat atmosphere layers patch stream patch-climate near-floor cleanup ;;
  terrain) tables terrain ;;
  water) tables water ;;
  sky) tables sky ;;
  light) tables light ;;
  *) echo "unknown part $part"; exit 2 ;;
esac
finish
