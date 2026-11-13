# bevy-jayzee

The minimal Bevy + Avian replacement for jayzee's deterministic multiplayer physics: fixed-step simulation, cross-platform deterministic math, client prediction and server authority, with no hand-written physics or netcode kernels.

## Stack

| Layer | Crate | Version |
|---|---|---|
| Engine and schedule | bevy | 0.20 |
| Physics | avian3d | 0.8 |
| Netcode | lightyear | 0.31 |
| Input | leafwing-input-manager | 0.22 |

Avian ships the solver, broadphase, contact cache and island management. Lightyear ships input buffering, prediction, rollback, reconciliation and replication. The project owns only the scene, the protocol registration and the movement rule.

## Commands

```sh
cargo run --release -- bench 2000 300
cargo run --release -- det 2000 300
cargo run --release -- server
cargo run --release -- client
scripts/verify.sh
```

`bench` reproduces jayzee's benchmark scene: 500, 2000 or 5000 spheres in a 60 m square, dropped from y = 5 to 45 onto the same 20 x 1 x 20 box and ground plane, 64 Hz fixed step, 30 warm-up ticks, average and tail tick cost in milliseconds.

`det` runs the same scene twice in one process and compares an FNV-1a hash over the f32 bits of every position and velocity.

`server` and `client` run the networking witness: the server owns a floor and one dynamic sphere per client, sends fixed ticks of replicated state, and the client predicts its own sphere from a scripted input. The client prints a report comparing the predicted state with the confirmed server state at the same tick, then exits.

## Measured

One 12-core Linux container, native CPU target, Avian defaults (6 substeps), always-awake bodies, 300 measured ticks after 30 warm-up ticks.

| Bodies | mean ms | p50 ms | p95 ms | p99 ms | ticks/s | jayzee mean |
|---|---|---|---|---|---|---|
| 500 | 0.40 | 0.39 | 0.48 | 0.64 | 2499 | 0.42 |
| 2000 | 1.11 | 1.10 | 1.30 | 1.56 | 899 | 3.1 |
| 5000 | 1.96 | 1.95 | 2.29 | 2.45 | 510 | 10 |

`det` gives the same hash for two runs at every size. The networking witness reports one predicted player whose predicted position at confirmed tick 353 is [14.4995, 1.1, 0.0], the confirmed position is the same to six decimals, and the error is 0.000000 m.

## Performance configuration

`Cargo.toml` sets `opt-level = 3`, fat LTO, one codegen unit, `panic = "abort"`, no overflow checks, no debug assertions and stripped symbols. `.cargo/config.toml` adds `-C target-cpu=native` for x86_64 Linux and `+simd128` for wasm32.

Avian runs with `enhanced-determinism` (libm-backed math for cross-platform bit-compatibility) and `parallel`. The benchmark disables Avian's transform sync, interpolation and sleeping, so the measured tick is the solver for the same always-awake scene jayzee uses.

The network scene follows Lightyear's Avian integration contract: `LightyearAvianPlugin` in `Position` mode, Avian's transform and interpolation plugins disabled, and island and sleeping plugins disabled so rollback restores a complete state. The client inserts `PredictionManager`; without it Lightyear records no prediction history and only replication runs.

## Mapping from jayzee

| jayzee | here |
|---|---|
| `src/kernel` (jz numeric kernel) | `avian3d` solver, no own kernel |
| `src/world/World.js` fixed clock, body table | Bevy `Time<Fixed>` + ECS |
| `World#hash` | per-tick FNV-1a over Avian `Position` and `LinearVelocity` |
| `src/protocol` codecs | Lightyear replication and messages |
| `src/netcode` (Authority, Predictor, InputBuffer, StateSender) | Lightyear client/server, prediction and rollback |
| `src/kernel/load.js` | none; the solver is linked Rust |

Removed from the maintenance surface: the kernel build, wasm module loading, hand-written wire layouts, the prediction loop, the rollback ring and the input queue.
