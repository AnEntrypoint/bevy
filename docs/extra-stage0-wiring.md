# Extra gaps stage 0: wire the test-only ports into the binary (design, Phase A, no code)

Source: specs/bevy-extra-gaps.md (decision 2026-11-23, stage order) and /workspace/extra/docs/bevy-gaps/stage0-wiring.tsv (13 modules, three waves). The decision text says 16 test-only ports; the table lists 13. The three not in the table are not yet identified, so the first step of stage 0 is to reconcile the two lists against src/main.rs before any code moves.

## Starting state (checked 2026-11-23 at c394fb9)

- src/main.rs declares 42 modules; the modules in the wiring table each sit under #[cfg(test)], so the binary never compiles them. Nothing outside their own tests calls them, so removing the attribute adds no behaviour change by itself.
- Their own tests (#[test] lines): wave 1 hud 1, terrain_grid_sample 1, world_name 1 (3 in total); wave 2 baked_field_load 1, camera_rig 1, fire_terrain 1, game_rules 7, hitscan 2, hud_overlay 3 (15); wave 3 character 7, chart_reanchor_service 2, fire_stage_map 1, terrain_mesh_surface 1 (11). Total 29.
- Warnings only: Cargo.toml has no [lints] table, src/main.rs has no deny attribute, and no script passes -D warnings, so un-gating cannot fail the build. Each un-used pub item warns as dead code until a caller exists.

## Per-module rule

Each module moves in one commit, in this order: remove #[cfg(test)] from its mod line in src/main.rs; keep #[cfg(test)] on its inner test module and on any fixture or testkit item used only by tests; remove the attribute from any item-gated import it needs (for example js_math in terrain_grid_sample); then build. Where the table's wiring_needed names a caller (hud_overlay needs hud; character needs a real avian shape-cast), the caller is wired in the same wave or the module stays gated and the row is recorded as blocked, not silently skipped.

## Witness (one per wave, run as a gate window)

1. Build: cargo build --bin bevy-jayzee with no new warning categories beyond dead_code for the un-called pub items.
2. Module tests run in the binary: cargo test --bin bevy-jayzee -- <module>:: reports at least the module's test count from the list above, and the total test count rises by the wave total (3, then 15, then 11).
3. Wiring check: a node script (to be written in the first wave commit, scripts/check-stage0-wired.mjs) reads src/main.rs and fails if any module named in stage0-wiring.tsv is still under #[cfg(test)] at its mod line, or if a module outside the table gained the attribute removal.

## Red checks (mutant rows, one per wave, in scripts/mutants/extra-stage0.tsv)

- stage0-reattach-cfg-test: re-add #[cfg(test)] to one wave-1 mod line; the wiring check must fail and the module's tests must disappear from the total.
- stage0-drop-inner-test: delete one module's #[test] function; the per-module count witness must fail.
- stage0-skip-wave: leave one wave-2 module gated; the wiring check must name it.

## Dependencies and conflicts

- src/main.rs is touched by stage 5 (planet_scene seam, default_launch). Stage 0 commits must not land in the same window as a stage 5 main.rs commit; they are sequenced after stage 5 Phase B, as the queue orders them.
- hud and hud_overlay wire into the viewer overlay (src/viewer.rs ViewerOverlay, src/viewer.rs:263 and :348). That is a visible change to the running viewer and is decided in wave 2 with the owner's rule against changing placement rules.

## Done for stage 0

- All 13 table modules are compiled into the binary or recorded as blocked with the reason named (character's shape-cast, the three unidentified ports).
- The wiring check passes on the final tree and its red checks are detected.

