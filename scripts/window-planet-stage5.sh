#!/usr/bin/env bash
part=${1:?usage: window-planet-stage5.sh unit|capture|red}
log=target/witness/window-planet-stage5-$part.log
. /workspace/bevy/scripts/lib/window.sh
: > "$log"
case "$part" in
  unit) run launch-unit cargo test --bin bevy-jayzee -- default_launch_tests planet_poses bench_planet ;;
  capture) run pose-capture cargo test --bin bevy-jayzee -- the_default_launch_passes_the_pose_captures --test-threads=1 ;;
  red) mutants scripts/mutants/planet-s5-launch.tsv; mutants scripts/mutants/planet-s5-poses.tsv ;;
  *) echo "unknown part $part"; exit 2 ;;
esac
finish
