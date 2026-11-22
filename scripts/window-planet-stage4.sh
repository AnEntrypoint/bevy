#!/usr/bin/env bash
part=${1:?usage: window-planet-stage4.sh core|red-a1|red-a2|red-water|red-veg|red-bench|legacy-census|legacy-mutants|timing}
log=target/witness/window-planet-stage4-$part.log
. /workspace/bevy/scripts/lib/window.sh
: > "$log"
tables() { for name in "$@"; do mutants scripts/mutants/planet-$name.tsv; done; }
changed="src/globe.rs src/main.rs src/offscreen.rs src/render_stats.rs src/bench.rs src/bench_planet.rs src/planet_lod/stream.rs src/planet_lod/collider.rs src/planet_lod/guard.rs src/planet_veg/stream.rs src/planet_water/surface.rs src/planet_water/leaves.rs src/planet_water/mesh.rs"
case "$part" in
  core)
    run build-debug cargo build --bin bevy-jayzee
    run warnings-bin bash -c 'touch src/main.rs; out=$(cargo build --bin bevy-jayzee 2>&1); echo "$out" | tail -n 3; ! echo "$out" | grep -q "^warning"'
    run warnings-test bash -c 'touch src/main.rs; out=$(cargo test --bin bevy-jayzee --no-run 2>&1); echo "$out" | tail -n 3; ! echo "$out" | grep -q "^warning"'
    run mutant-tables node scripts/check-mutant-tables.mjs
    run no-mutant-applied node scripts/check-no-mutant-applied.mjs
    run bin-witness cargo test --bin bevy-jayzee
    for gen in quadtree near-field collider patch climate splat atmosphere waves layers veg-placement rock-placement grass-placement species grass-clump grass-wind; do
      run gen-planet-$gen bash -c "node scripts/gen-planet-$gen.mjs /workspace/spoint && git diff --exit-code -- testdata/parity/spoint-planet-$gen.json"
    done
    ;;
  red-a1) tables stream near-floor patch-climate ;;
  red-a2) tables collider origin s4-lod ;;
  red-water) tables water s4-water ;;
  red-veg) tables veg-stream veg-budget ;;
  red-bench) tables s4-bench ;;
  legacy-census) ONLY="build-debug census-d1 census-d2 census-d1-check census-d2-check census-png-sha" bash scripts/phase-b.sh || overall=1 ;;
  legacy-mutants)
    names=$(bash -c "mutant() { case \" $changed \" in *\" \$2 \"*) echo \$1;; esac; }; eval \"\$(grep '^mutant ' scripts/phase-b.sh)\"" | tr '\n' ' ')
    note "LEGACY-ONLY $names"
    ONLY="$names" bash scripts/phase-b.sh || overall=1
    ;;
  timing)
    run bench-planet cargo run --bin bevy-jayzee -- bench-planet
    ;;
  *) echo "unknown part $part"; exit 2 ;;
esac
finish
