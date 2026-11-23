cd /workspace/bevy || exit 2
cargo run --bin bevy-jayzee -- bench-planet
rc=$?
mkdir -p target/witness/timing-history
cp target/witness/bench-planet.json target/witness/timing-history/$(date +%s).json
node scripts/timing-summary.mjs
exit $rc
