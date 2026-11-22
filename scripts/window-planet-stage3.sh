#!/usr/bin/env bash
part=${1:?usage: window-planet-stage3.sh core|trees-a|trees-b|rockgrass-a|rockgrass-b|gpu|rgrender|stream}
log=target/witness/window-planet-stage3-$part.log
. /workspace/bevy/scripts/lib/window.sh
: > "$log"
tables() { for name in "$@"; do mutants scripts/mutants/planet-$name.tsv; done; }
rows() { sed -n "$2,$3p" scripts/mutants/planet-$1.tsv > "$scratch/$1-$2.tsv"; mutants "$scratch/$1-$2.tsv"; }
case "$part" in
  core)
    run build-debug cargo build --bin bevy-jayzee
    run warnings-bin bash -c 'touch src/main.rs; out=$(cargo build --bin bevy-jayzee 2>&1); echo "$out" | tail -n 3; ! echo "$out" | grep -q "^warning"'
    run warnings-test bash -c 'touch src/main.rs; out=$(cargo test --bin bevy-jayzee --no-run 2>&1); echo "$out" | tail -n 3; ! echo "$out" | grep -q "^warning"'
    run mutant-tables node scripts/check-mutant-tables.mjs
    run bin-witness cargo test --bin bevy-jayzee
    for gen in quadtree near-field collider patch climate splat atmosphere waves layers veg-placement rock-placement grass-placement species grass-clump grass-wind; do
      run gen-planet-$gen bash -c "node scripts/gen-planet-$gen.mjs /workspace/spoint && git diff --exit-code -- testdata/parity/spoint-planet-$gen.json"
    done
    ;;
  trees-a) rows veg-trees 1 80 ;;
  trees-b) rows veg-trees 81 157 ;;
  rockgrass-a) rows veg-rockgrass 1 60 ;;
  rockgrass-b) rows veg-rockgrass 61 120 ;;
  gpu) tables gpucore trees-render species ;;
  rgrender) tables veg-rockgrass-render ;;
  stream) tables veg-stream ;;
  *) echo "unknown part $part"; exit 2 ;;
esac
finish
