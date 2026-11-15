# Game-16 executable API matrix

Baseline: `/workspace/extra/game-16` (JS/YAML/JSON/MD corpus, places `hall` and `main`). This file records executable parity, which is a stronger claim than the documentation-row coverage in `FEATURE_MATRIX.md`. Hosted/account/network infrastructure and product names remain excluded.

## Coverage claim

| Layer | What it means | Witness |
|---|---|---|
| Doc row covered | A local feature named by `/workspace/extra/.local/docs/*.md` has a native implementation or a documented local subset. | `FEATURE_MATRIX.md`, `scripts/spec-test.js`, `testdata/spec-world/scripts/feature-audit.js` |
| Executable game-16 API covered | Every `ctx.*` token in game-16 JS/YAML/JSON/MD resolves to a real QuickJS context key, a Bevy-backed proxy field, a typed native feature resource, or an explicitly provenance-labeled deterministic local shim; both real places run 120 ticks with zero script errors. | `game16-report`, `scripts/check-game16-api.cjs` |

Run the executable witness with:

```sh
cargo build --bin bevy-jayzee
node scripts/check-game16-api.cjs /workspace/extra/game-16 ./target/debug/bevy-jayzee
```

The checker runs both of these commands and fails on non-zero exit, script errors, a non-empty `missingApi` list, or any extracted `ctx.*` top-level name not present in the runtime context shape:

```sh
./target/debug/bevy-jayzee world /workspace/extra/game-16 hall 120
./target/debug/bevy-jayzee world /workspace/extra/game-16 main 120
```

## `game16-report` fields

`src/world.rs` emits `{"kind":"game16-report","report":{...}}` after the world, `script-stats`, and `feature-report` lines. `src/script/mod.rs` builds it from live engine state.

| Field | Meaning |
|---|---|
| `root`, `place`, `ticks`, `errors`, `hash` | World identity, fixed-step count, script error count and deterministic ECS/body hash from the world report. |
| `ctxPaths`, `missingApi`, `missingApiCount` | All `ctx.*` tokens found in the source corpus, and those without a host/context/proxy/shim classification. |
| `modules`, `moduleCount` | QuickJS modules actually loaded in that place. |
| `hostVerbs`, `hostCalls` | Available rquickjs global bridges and feature host verbs, plus call counts for feature verbs exercised through `__host`. |
| `ctxKeys`, `provenance`, `ecs` | Context keys parsed from `__make_ctx`, each key/verb's provenance, and count of Bevy ECS-backed provenance entries. |

## Missing / fixed calls

The initial corpus-wide audit found 317 unique `ctx.*` tokens. Most were already reachable through the dynamic `obj:`/`place:`/`world:`/`state:` proxies and the prelude context. The executable report initially isolated these top-level gaps:

| Gap | Fix | Provenance |
|---|---|---|
| `ctx.getTerrainHeight` | New prelude method calling `terrainHeight`; Rust `terrainHeight` delegates to `read_field` with the `terrain` field over Bevy `TerrainData`. | Bevy ECS `TerrainData` |
| `ctx.sampleSpline`, `ctx.splineLength`, `ctx.projectToSpline` | Explicit context methods calling existing spline host verbs. | Bevy ECS world state |
| `ctx.spawnPlayer`, `ctx.setInput`, `ctx.setAnim` | Explicit context methods calling existing ECS spawn/input/animation host verbs. | Bevy ECS world state |
| `ctx.ensureScatter`, `ctx.getPixelProvenance` | Deterministic local render/authoring probes through `featureProbe`. | native probe / local shim |
| `ctx.day`, `ctx.field` | Deterministic local scalar/field helpers; `field` reads native terrain fields. | local shim / Bevy ECS terrain |
| `ctx.getFxState`, `ctx.stopFx`, `ctx.pulseFx`, `ctx.job`, `ctx.awaitJob`, `ctx.sql`, `ctx.trade`, `ctx.notifyDm*` | Already present; explicitly classified in `ctx_provenance()` rather than left undocumented. | native `FeatureState` or local shim |
| `ctx.place.*`, `ctx.world.*`, `ctx.self.*`, `ctx.state`, nested `session`/`view`/`anim`/`bones`/`material` fields | Dynamic `__make` proxies backed by `WorldData`, `ObjectState`, `InputState`, `AnimState`, `AudioState`, `WorldState`, `TerrainData`, and entity components. | Bevy ECS / resource state |

Shader, scatter, TSL and authoring callback locals such as `atlas`, `bone`, `canvas`, `crossing`, `scatter`, `tile`, `wang`, `x`, `y`, `z` are documented callback parameter contexts, not top-level behavior context keys. They are listed with explicit local-shim provenance in `game16_dynamic_paths()` and in the checker, and are not implemented as hosted services.

## Native reliance

- `src/world.rs` loads place specs and spawns Bevy entities, bodies, colliders, lights, models and scripted primitives.
- `src/runtime.rs` owns fixed-step systems and typed resources (`WorldState`, `TerrainData`, `InputState`, `AnimState`, `AudioState`, `GroundInfo`, liquids, overlaps, forces, joints).
- `src/features.rs` and `src/features_ecs.rs` own native deterministic feature records and typed Bevy ECS mirrors (`NativeClock`, `NativeRandom`, `NativeMathService`).
- `src/script/mod.rs` is the rquickjs host: QuickJS globals dispatch into Bevy ECS/world queries, native feature state, deterministic local files or explicitly labeled local shims.
- `src/script/prelude.js` is the API shim layer only: proxies and context methods adapt the jayzee-shaped API to the native host; it does not duplicate physics or world state.

No online infrastructure, accounts, hosted rooms, hosted SQL/LLM services or product-specific names are implemented.
