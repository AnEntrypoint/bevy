# Pending validation

Each row names a committed change, the file it touches, and the witness that proves it. A row is green when its witness was run on the tree it was committed in and the output is quoted in the status.

| Commit change | File | Witness | Result |
|---|---|---|---|
| Seeded-random and feature-report unit tests | src/features.rs | `cargo test --bin bevy-jayzee features::tests` | pass: `test result: ok` for the features tests, within `tests::` 31 passed |
| Place program `sim.js`, neon-syndicates ctx keys, `ctx.spawn`, `__wrap` root sync, builtin `tsl` names | src/script/mod.rs, src/script/prelude.js, AGENTS.md | `target/debug/bevy-jayzee world /workspace/neon-syndicates/world main 120` exit 0 with `errors:0`, `missingApiCount:0`, `objects:58`, `behaviors:1` | pass, see STATUS.md section "Neon-syndicates ctx reconciliation" |
| Same change, script side | src/script/mod.rs, src/script/prelude.js | `node scripts/check-game16-api.cjs` exit 0, `"ok": true` | pass |
| Player template skip in world spawn | src/world.rs | `world ... main 120` shows the `player` template spawned (`templates/player.js` in modules) and no bare `local-player` | pass, see run log |
| BasisLZ KTX2 transcode (already on origin/main at 63700b4) | src/basisu.rs, src/glb.rs | `target/debug/bevy-jayzee gltf-check /workspace/neon-syndicates/assets/cdn`: `bevy_failed:0`, `bevy_loaded:64`, `pass:64` | pass (PNG failures outside scope) |
| Viewer HUD check on the new binary | viewer and HUD path | `/tmp/ns-viewer/run.sh` PNGs and `viewer_status` line | open: text still not drawn (see STATUS.md) |
| Globe unit test `globe::tests::elevation_is_deterministic_and_bounded` | src/globe.rs (unchanged in this stretch) | `cargo test --bin bevy-jayzee elevation_is_deterministic` | FAILS: assertion at src/globe.rs:550, not caused by this stretch but not proven pre-existing |
