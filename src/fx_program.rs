use std::collections::HashSet;
use serde_json::{Map, Value};
const SINKS: [&str; 2] = ["sprite", "mesh"];
const BLENDS: [&str; 5] = ["alpha", "add", "multiply", "subtract", "screen"];
const MAX_POPULATIONS: usize = 16;
const MAX_RATE: f64 = 10_000.0;
const MAX_BURST: f64 = 100_000.0;
const MAX_LIFETIME_S: f64 = 600.0;
const MAX_SPEED_MPS: f64 = 1_000.0;
const MAX_DRAG: f64 = 100.0;
const MAX_CONE_DEG: f64 = 180.0;
const MAX_PARTICLES_LIMIT: f64 = 100_000.0;
const MAX_SEED: f64 = 4_294_967_295.0;
const CHANNELS: u64 = 4;
const CHANNEL_LIFETIME: u64 = 0;
const CHANNEL_AZIMUTH: u64 = 1;
const CHANNEL_CONE: u64 = 2;
const CHANNEL_SPEED: u64 = 3;

#[derive(Debug, Clone, PartialEq)]
pub struct FxProgramError {
    pub reason_code: &'static str,
    pub detail: Option<String>,
}

fn refuse(reason_code: &'static str, detail: Option<String>) -> FxProgramError {
    FxProgramError { reason_code, detail }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Velocity {
    pub speed: [f64; 2],
    pub cone_deg: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Accelerate {
    pub gravity: [f64; 3],
    pub drag: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Render {
    pub kind: String,
    pub blend: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Population {
    pub name: String,
    pub rate: f64,
    pub burst: u64,
    pub lifetime: [f64; 2],
    pub velocity: Velocity,
    pub accelerate: Accelerate,
    pub render: Render,
    pub max_particles: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FxProgram {
    pub seed: u32,
    pub populations: Vec<Population>,
}

fn optional<'a>(object: &'a Map<String, Value>, key: &str) -> Option<&'a Value> {
    object.get(key).filter(|value| !value.is_null())
}

fn number_in(value: &Value, min: f64, max: f64) -> Option<f64> {
    value.as_f64().filter(|n| n.is_finite() && *n >= min && *n <= max)
}

fn integer_in(value: &Value, min: f64, max: f64) -> Option<f64> {
    number_in(value, min, max).filter(|n| n.fract() == 0.0)
}

fn pair_in(value: &Value, min: f64, max: f64) -> Option<[f64; 2]> {
    let [lo, hi] = value.as_array()?.as_slice() else {
        return None;
    };
    let (lo, hi) = (number_in(lo, min, max)?, number_in(hi, min, max)?);
    (hi >= lo).then_some([lo, hi])
}

fn vec3(value: &Value) -> Option<[f64; 3]> {
    let [x, y, z] = value.as_array()?.as_slice() else {
        return None;
    };
    let finite = |v: &Value| v.as_f64().filter(|n| n.is_finite());
    Some([finite(x)?, finite(y)?, finite(z)?])
}

fn parse_velocity(object: &Map<String, Value>, name: &str) -> Result<Velocity, FxProgramError> {
    let Some(raw) = optional(object, "velocity") else {
        return Ok(Velocity { speed: [0.0, 0.0], cone_deg: 0.0 });
    };
    let refusal = || refuse("velocity-invalid", Some(name.to_owned()));
    let fields = raw.as_object().ok_or_else(refusal)?;
    let speed = match optional(fields, "speed") {
        None => [0.0, 0.0],
        Some(value) => pair_in(value, 0.0, MAX_SPEED_MPS).ok_or_else(refusal)?,
    };
    let cone_deg = match optional(fields, "coneDeg") {
        None => 0.0,
        Some(value) => number_in(value, 0.0, MAX_CONE_DEG).ok_or_else(refusal)?,
    };
    Ok(Velocity { speed, cone_deg })
}

fn parse_accelerate(object: &Map<String, Value>, name: &str) -> Result<Accelerate, FxProgramError> {
    let Some(raw) = optional(object, "accelerate") else {
        return Ok(Accelerate { gravity: [0.0; 3], drag: 0.0 });
    };
    let refusal = || refuse("accelerate-invalid", Some(name.to_owned()));
    let fields = raw.as_object().ok_or_else(refusal)?;
    let gravity = match optional(fields, "gravity") {
        None => [0.0; 3],
        Some(value) => vec3(value).ok_or_else(refusal)?,
    };
    let drag = match optional(fields, "drag") {
        None => 0.0,
        Some(value) => number_in(value, 0.0, MAX_DRAG).ok_or_else(refusal)?,
    };
    Ok(Accelerate { gravity, drag })
}

fn parse_population(raw: &Value, index: usize) -> Result<Population, FxProgramError> {
    let object = raw
        .as_object()
        .ok_or_else(|| refuse("population-not-object", Some(format!("index {index}"))))?;
    let name = object
        .get("name")
        .and_then(Value::as_str)
        .filter(|name| !name.is_empty())
        .ok_or_else(|| refuse("population-name-missing", Some(format!("index {index}"))))?;
    let named = |code: &'static str| refuse(code, Some(name.to_owned()));
    let spawn = object
        .get("spawn")
        .and_then(Value::as_object)
        .ok_or_else(|| named("spawn-rate-invalid"))?;
    let rate = spawn
        .get("rate")
        .and_then(|value| number_in(value, 0.0, MAX_RATE))
        .ok_or_else(|| named("spawn-rate-invalid"))?;
    let burst = match optional(spawn, "burst") {
        None => 0,
        Some(value) => integer_in(value, 0.0, MAX_BURST).ok_or_else(|| named("spawn-burst-invalid"))? as u64,
    };
    let lifetime = match optional(object, "lifetime") {
        None => [0.0, 0.0],
        Some(value) => pair_in(value, 0.0, MAX_LIFETIME_S).ok_or_else(|| named("lifetime-invalid"))?,
    };
    let sink = object
        .get("render")
        .and_then(Value::as_object)
        .ok_or_else(|| named("render-kind-unsupported"))?;
    let kind = sink
        .get("kind")
        .and_then(Value::as_str)
        .filter(|kind| SINKS.contains(kind))
        .ok_or_else(|| named("render-kind-unsupported"))?;
    let blend = match sink.get("blend") {
        None => "alpha",
        Some(value) => value
            .as_str()
            .filter(|blend| BLENDS.contains(blend))
            .ok_or_else(|| named("blend-unsupported"))?,
    };
    let max_particles = match optional(object, "maxParticles") {
        None => None,
        Some(value) => Some(
            number_in(value, 1.0, MAX_PARTICLES_LIMIT).ok_or_else(|| named("max-particles-invalid"))?,
        ),
    };
    let velocity = parse_velocity(object, name)?;
    let accelerate = parse_accelerate(object, name)?;
    Ok(Population {
        name: name.to_owned(),
        rate,
        burst,
        lifetime,
        velocity,
        accelerate,
        render: Render {
            kind: kind.to_owned(),
            blend: blend.to_owned(),
        },
        max_particles,
    })
}

pub fn parse_fx_program(text: &str) -> Result<FxProgram, FxProgramError> {
    let doc: Value = serde_json::from_str(text).map_err(|_| refuse("json-unreadable", None))?;
    let object = doc.as_object().ok_or_else(|| refuse("populations-missing", None))?;
    let raw_populations = object
        .get("populations")
        .and_then(Value::as_array)
        .ok_or_else(|| refuse("populations-missing", None))?;
    if raw_populations.is_empty() || raw_populations.len() > MAX_POPULATIONS {
        return Err(refuse("population-count-invalid", Some(raw_populations.len().to_string())));
    }
    let seed = match optional(object, "seed") {
        None => 0,
        Some(value) => integer_in(value, 0.0, MAX_SEED).ok_or_else(|| refuse("seed-invalid", None))? as u32,
    };
    let populations = raw_populations
        .iter()
        .enumerate()
        .map(|(index, raw)| parse_population(raw, index))
        .collect::<Result<Vec<_>, _>>()?;
    let mut names = HashSet::new();
    if !populations.iter().all(|p| names.insert(p.name.as_str())) {
        return Err(refuse("population-name-duplicate", None));
    }
    Ok(FxProgram { seed, populations })
}

#[derive(Debug, Clone, PartialEq)]
pub struct Particle {
    pub pos: [f64; 3],
    pub vel: [f64; 3],
    dies: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PopulationSnapshot {
    pub name: String,
    pub alive: usize,
    pub render: Render,
    pub particles: Vec<Particle>,
}

fn hash01(seed: u32, pop: usize, n: u64) -> f64 {
    let pop_plus = ((pop + 1) as u32) as i32;
    let n_plus = ((n + 1) as u32) as i32;
    let mut h = ((seed as i32) ^ (0x9e37_79b9_u32 as i32)).wrapping_mul(0x85eb_ca6b_u32 as i32)
        ^ pop_plus.wrapping_mul(0xc2b2_ae35_u32 as i32)
        ^ n_plus.wrapping_mul(0x27d4_eb2f_u32 as i32);
    h = (h ^ ((h as u32) >> 15) as i32).wrapping_mul(0x2c1b_3c6d_u32 as i32);
    h = (h ^ ((h as u32) >> 12) as i32).wrapping_mul(0x297a_2d39_u32 as i32);
    h ^= ((h as u32) >> 15) as i32;
    f64::from(h as u32) / 4_294_967_296.0
}

struct PopulationState {
    def: Population,
    index: usize,
    carry: f64,
    emitted: u64,
    bursted: bool,
    live: Vec<Particle>,
}

impl PopulationState {
    fn channel(&self, seed: u32, channel: u64) -> f64 {
        hash01(seed, self.index, self.emitted * CHANNELS + channel)
    }

    fn emit(&mut self, seed: u32, time: f64) {
        if self.def.max_particles.is_some_and(|max| (self.live.len() as f64) >= max) {
            return;
        }
        let [lo, hi] = self.def.lifetime;
        let life = lo + (hi - lo) * self.channel(seed, CHANNEL_LIFETIME);
        let [speed_lo, speed_hi] = self.def.velocity.speed;
        let magnitude = speed_lo + (speed_hi - speed_lo) * self.channel(seed, CHANNEL_SPEED);
        let cos_cone = (self.def.velocity.cone_deg * std::f64::consts::PI / 180.0).cos();
        let cos_theta = cos_cone + self.channel(seed, CHANNEL_CONE) * (1.0 - cos_cone);
        let sin_theta = (1.0 - cos_theta * cos_theta).max(0.0).sqrt();
        let azimuth = self.channel(seed, CHANNEL_AZIMUTH) * 2.0 * std::f64::consts::PI;
        self.emitted += 1;
        self.live.push(Particle {
            pos: [0.0; 3],
            vel: [
                magnitude * sin_theta * azimuth.cos(),
                magnitude * cos_theta,
                magnitude * sin_theta * azimuth.sin(),
            ],
            dies: time + life,
        });
    }

    fn integrate(&mut self, dt: f64) {
        let gravity = self.def.accelerate.gravity;
        let damping = (1.0 - self.def.accelerate.drag * dt).max(0.0);
        for p in &mut self.live {
            for axis in 0..3 {
                p.vel[axis] = (p.vel[axis] + gravity[axis] * dt) * damping;
                p.pos[axis] += p.vel[axis] * dt;
            }
        }
    }
}

pub struct FxSimulation {
    seed: u32,
    time: f64,
    pops: Vec<PopulationState>,
}

impl FxSimulation {
    pub fn new(program: &FxProgram) -> Self {
        Self {
            seed: program.seed,
            time: 0.0,
            pops: program
                .populations
                .iter()
                .enumerate()
                .map(|(index, def)| PopulationState {
                    def: def.clone(),
                    index,
                    carry: 0.0,
                    emitted: 0,
                    bursted: false,
                    live: Vec::new(),
                })
                .collect(),
        }
    }

    pub fn step(&mut self, dt: f64) -> Result<Vec<PopulationSnapshot>, FxProgramError> {
        if !(0.0..=1.0).contains(&dt) {
            return Err(refuse("dt-invalid", Some(dt.to_string())));
        }
        self.time += dt;
        let (seed, time) = (self.seed, self.time);
        for pop in &mut self.pops {
            pop.integrate(dt);
            if !pop.bursted {
                pop.bursted = true;
                for _ in 0..pop.def.burst {
                    pop.emit(seed, time);
                }
            }
            pop.carry += pop.def.rate * dt;
            let whole = pop.carry.floor();
            pop.carry -= whole;
            for _ in 0..(whole as u64) {
                pop.emit(seed, time);
            }
            pop.live.retain(|p| p.dies > time);
        }
        Ok(self.snapshot())
    }

    pub fn snapshot(&self) -> Vec<PopulationSnapshot> {
        self.pops
            .iter()
            .map(|pop| PopulationSnapshot {
                name: pop.def.name.clone(),
                alive: pop.live.len(),
                render: pop.def.render.clone(),
                particles: pop.live.clone(),
            })
            .collect()
    }
}

mod tests {
    use super::*;
    use crate::canonical_json::canonical_json;
    use serde_json::json;

    const TOLERANCE: f64 = 1e-9;

    fn program_json(program: &FxProgram) -> Value {
        json!({
            "seed": program.seed,
            "populations": program.populations.iter().map(|p| json!({
                "name": p.name,
                "rate": p.rate,
                "burst": p.burst,
                "lifetime": p.lifetime,
                "velocity": { "speed": p.velocity.speed, "coneDeg": p.velocity.cone_deg },
                "accelerate": { "gravity": p.accelerate.gravity, "drag": p.accelerate.drag },
                "render": { "kind": p.render.kind, "blend": p.render.blend },
                "maxParticles": p.max_particles,
            })).collect::<Vec<Value>>(),
        })
    }

    fn assert_snapshot(label: &str, got: &[PopulationSnapshot], want: &Value) {
        let want = want.as_object().unwrap();
        assert_eq!(got.len(), want.len(), "{label}: population count");
        for pop in got {
            let w = want.get(&pop.name).unwrap_or_else(|| panic!("{label}: missing {}", pop.name));
            assert_eq!(pop.alive as u64, w["alive"].as_u64().unwrap(), "{label} {}: alive", pop.name);
            assert_eq!(
                canonical_json(&json!({ "kind": pop.render.kind, "blend": pop.render.blend })),
                canonical_json(&w["render"]),
                "{label} {}: render",
                pop.name
            );
            let want_particles = w["particles"].as_array().unwrap();
            assert_eq!(pop.particles.len(), want_particles.len(), "{label} {}: particle count", pop.name);
            for (i, (p, want_p)) in pop.particles.iter().zip(want_particles).enumerate() {
                for axis in 0..3 {
                    let want_pos = want_p["pos"][axis].as_f64().unwrap();
                    assert!((p.pos[axis] - want_pos).abs() <= TOLERANCE, "{label} {} particle {i} pos[{axis}]", pop.name);
                    let want_vel = want_p["vel"][axis].as_f64().unwrap();
                    assert!((p.vel[axis] - want_vel).abs() <= TOLERANCE, "{label} {} particle {i} vel[{axis}]", pop.name);
                }
            }
        }
    }

    #[test]
    fn fx_program_matches_spoint() {
        let f: Value = crate::testkit::parse(include_str!("../testdata/parity/spoint-fx-program.json"));
        for case in crate::testkit::cases(&f, "parse") {
            let text = case["text"].as_str().unwrap();
            let want = &case["expect"];
            match (parse_fx_program(text), want["ok"].as_bool()) {
                (Ok(program), Some(true)) => assert_eq!(
                    canonical_json(&program_json(&program)),
                    canonical_json(&want["value"]),
                    "parseFxProgram {text}"
                ),
                (Err(err), Some(false)) => {
                    assert_eq!(Some(err.reason_code), want["reasonCode"].as_str(), "parseFxProgram {text}: reason");
                    if err.reason_code != "json-unreadable" {
                        assert_eq!(err.detail.as_deref(), want["detail"].as_str(), "parseFxProgram {text}: detail");
                    }
                }
                (got, _) => panic!("parseFxProgram {text}: spoint returns {want}, bevy returns {got:?}"),
            }
        }
        for case in crate::testkit::cases(&f, "simulate") {
            let text = case["text"].as_str().unwrap();
            let program = parse_fx_program(text).unwrap();
            let mut sim = FxSimulation::new(&program);
            assert_snapshot("initial", &sim.snapshot(), &case["initial"]);
            for step in case["steps"].as_array().unwrap() {
                let dt = step["dt"].as_f64().unwrap();
                let want = &step["expect"];
                match (sim.step(dt), want["ok"].as_bool()) {
                    (Ok(snapshot), Some(true)) => assert_snapshot(&format!("step dt={dt}"), &snapshot, &want["value"]),
                    (Err(err), Some(false)) => {
                        assert_eq!(Some(err.reason_code), want["reasonCode"].as_str(), "step dt={dt}: reason");
                        assert_eq!(err.detail.as_deref(), want["detail"].as_str(), "step dt={dt}: detail");
                    }
                    (got, _) => panic!("step dt={dt}: spoint returns {want}, bevy returns {got:?}"),
                }
            }
        }
    }
}
