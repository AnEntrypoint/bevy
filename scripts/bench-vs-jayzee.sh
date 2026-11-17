#!/usr/bin/env bash
set -u
repo=$(cd "$(dirname "$0")/.." && pwd)
jayzee=${JAYZEE:-/workspace/jayzee}
binary=${BINARY:-$repo/target/release/bevy-jayzee}
ticks=${TICKS:-300}
out=$repo/target/witness/bench-vs-jayzee.json
mkdir -p "$repo/target/witness"
lines=()
for bodies in 500 2000 5000; do
    j=$(cd "$jayzee" && node scripts/bench.mjs "$bodies" "$ticks") || exit 1
    b=$("$binary" bench "$bodies" "$ticks" | grep '"kind":"bench"' | tail -1) || exit 1
    lines+=("{\"bodies\":$bodies,\"jayzee\":$j,\"bevy\":$b}")
done
printf '%s\n' "${lines[@]}" | node -e '
const rows = require("node:fs").readFileSync(0, "utf8").trim().split("\n").map((line) => JSON.parse(line));
const table = rows.map((row) => ({ bodies: row.bodies, jayzeeMeanMs: row.jayzee.meanMs, bevyMeanMs: row.bevy.meanMs, speedup: Number((row.jayzee.meanMs / row.bevy.meanMs).toFixed(2)), jayzeeP95Ms: row.jayzee.p95Ms, bevyP95Ms: row.bevy.p95Ms, bevyHash: row.bevy.hash, node: row.jayzee.node }));
const result = { ticks: rows[0]?.jayzee.ticks, table, rows };
require("node:fs").writeFileSync(process.argv[1], JSON.stringify(result, null, 2));
console.log(JSON.stringify({ ticks: result.ticks, table }, null, 2));
' "$out"
