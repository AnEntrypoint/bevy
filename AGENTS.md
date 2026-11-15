# AGENTS.md

Non-obvious facts for automated edits. Architecture and measured costs live in README.md.

## Rules

- Agent operating routine, git workflow, checks and secrets handling: `docs/AGENT-INTERFACE.md`.
- Source files carry no comments. Put a fact the code cannot express here, after checking it is still true.
- Bench and determinism runs use `TimeUpdateStrategy::ManualDuration` with the exact 64 Hz tick, so one `App::update` is one fixed tick on every machine.
- The benchmark disables `PhysicsTransformPlugin`, `PhysicsInterpolationPlugin` and `IslandSleepingPlugin`. Transform sync is not part of the solver, interpolation is display-only, and sleeping would make fast benchmarks incomparable with jayzee, which never sleeps.
- The network scene must keep `LightyearAvianPlugin` after `ClientPlugins` and `ServerPlugins`, and must disable `PhysicsTransformPlugin` and `PhysicsInterpolationPlugin`. Lightyear owns `Position` and `Rotation` replication, and the Avian transform sync would overwrite them.
- The network scene disables `IslandPlugin` and `IslandSleepingPlugin` for the same rollback reason as Lightyear's avian_3d example.
- The client must insert `PredictionManager::default()`. Lightyear gates every prediction set on its presence, and without it `PredictionHistory` stays empty while `ConfirmedHistory` still fills from replication, which looks like a working connection with no prediction.
- Lightyear 0.31.0's reference example is commit 8a926d6fece6a0cf04c2747de90c01258524fe0d, `examples/avian_3d`.
- The hash is over f32 bit patterns, not jayzee's f64, because Avian's default scalar is f32. `enhanced-determinism` makes the math portable; it does not widen the scalar.
- `panic = "abort"` is set in release. A panic under that profile gives no backtrace; use the dev profile to debug.
- `.cargo/config.toml` sets `target-cpu=native` for x86_64 Linux. A release binary built there does not run on older CPUs. Remove that flag for portable builds.
- `ActionState<PlayerAction>` is filled by leafwing on the client and by Lightyear's server input plugin on the server. Both sides run the same `apply_player_input` system in `FixedUpdate`, which is what makes prediction and authority agree.
- The client report compares `PredictionHistory<Position>::get(tick)` with `ConfirmedHistory<Position>::newest_present()` at the confirmed tick. Comparing current positions would include the client's lead over the server.
- The server is paced at one 64 Hz tick per loop, and the client at 60 Hz with Lightyear syncing `Time<Fixed>`. A dedicated server can run unpaced, but then verification timing becomes machine-dependent.
- Baseline on the 12-core container, measured back-to-back with jayzee: 0.40 ms vs 0.43 at 500 bodies, 1.04 vs 2.78 at 2000, 1.88 vs 8.96 at 5000, 3.56 vs 28.12 at 10000. A later run must be compared against these on the same host before it is called a regression.
