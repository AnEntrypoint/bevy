#![cfg_attr(not(test), allow(dead_code))]

const FIELD_WASM_BUDGET_BYTES: f64 = 32.0 * 1024.0 * 1024.0;
const FIELD_WASM_BYTES_PER_SAMPLE: f64 = 4.0;

pub trait HeightfieldHost {
    fn add_height_field(&mut self, n: usize, spacing: f64, origin: [f64; 2]) -> Option<u64>;
    fn remove_body(&mut self, id: u64);
    fn set_terrain_body_id(&mut self, id: Option<u64>);
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StreamerOptions {
    pub extent: f64,
    pub resolution: f64,
    pub rebuild_at: f64,
    pub max_fields: Option<usize>,
}

impl Default for StreamerOptions {
    fn default() -> Self {
        Self { extent: 510.0, resolution: 4.0, rebuild_at: 0.4, max_fields: None }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Field {
    body_id: u64,
    center: [f64; 2],
    n: usize,
}

#[derive(Debug)]
pub struct TerrainStreamer {
    n: usize,
    spacing: f64,
    half: f64,
    cover_radius: f64,
    max_fields: usize,
    fields: Vec<Field>,
    disposed: bool,
    refusal_count: u64,
    rebuild_count: u64,
    extent: f64,
}

fn js_round(x: f64) -> f64 {
    (x + 0.5).floor()
}

fn chebyshev(a: [f64; 2], b: [f64; 2]) -> f64 {
    (a[0] - b[0]).abs().max((a[1] - b[1]).abs())
}

fn finite_centers(raw: &[[f64; 2]]) -> Vec<[f64; 2]> {
    raw.iter().copied().filter(|c| c[0].is_finite() && c[1].is_finite()).collect()
}

fn field_within(p: [f64; 2], fields: &[Field], radius: f64) -> bool {
    fields.iter().any(|f| chebyshev(p, f.center) <= radius)
}

fn next_uncovered_centre(players: &[[f64; 2]], fields: &[Field], cover_radius: f64, half: f64) -> Option<[f64; 2]> {
    let uncovered: Vec<[f64; 2]> = players.iter().copied().filter(|p| !field_within(*p, fields, cover_radius)).collect();
    if uncovered.is_empty() {
        return None;
    }
    let seed = uncovered.iter().copied().find(|p| !field_within(*p, fields, half)).unwrap_or(uncovered[0]);
    let members: Vec<[f64; 2]> = uncovered.iter().copied().filter(|p| chebyshev(*p, seed) <= cover_radius).collect();
    let (mut min_x, mut max_x, mut min_z, mut max_z) = (f64::INFINITY, f64::NEG_INFINITY, f64::INFINITY, f64::NEG_INFINITY);
    for m in &members {
        min_x = min_x.min(m[0]);
        max_x = max_x.max(m[0]);
        min_z = min_z.min(m[1]);
        max_z = max_z.max(m[1]);
    }
    Some([(min_x + max_x) / 2.0, (min_z + max_z) / 2.0])
}

fn loads(players: &[[f64; 2]], fields: &[Field], half: f64) -> Vec<usize> {
    let mut load = vec![0_usize; fields.len()];
    for p in players {
        let mut best: Option<usize> = None;
        let mut best_d = f64::INFINITY;
        for (f, field) in fields.iter().enumerate() {
            let d = chebyshev(*p, field.center);
            if d <= half && d < best_d {
                best = Some(f);
                best_d = d;
            }
        }
        if let Some(f) = best {
            load[f] += 1;
        }
    }
    load
}

impl TerrainStreamer {
    pub fn new(options: StreamerOptions) -> Self {
        let extent = if options.extent.is_finite() && options.extent > 0.0 { options.extent } else { 510.0 };
        let resolution = if options.resolution.is_finite() && options.resolution > 0.0 { options.resolution } else { 4.0 };
        let rebuild_at = if options.rebuild_at.is_finite() { options.rebuild_at } else { 0.4 };
        let mut n = js_round(extent / resolution).max(2.0);
        if n % 2.0 != 0.0 {
            n += 1.0;
        }
        let n = n as usize;
        let max_fields = match options.max_fields {
            Some(m) if m > 0 => m,
            _ => (FIELD_WASM_BUDGET_BYTES / (n * n) as f64 / FIELD_WASM_BYTES_PER_SAMPLE).floor().max(1.0) as usize,
        };
        Self {
            n,
            spacing: extent / (n as f64 - 1.0),
            half: extent / 2.0,
            cover_radius: extent * rebuild_at,
            max_fields,
            fields: Vec::new(),
            disposed: false,
            refusal_count: 0,
            rebuild_count: 0,
            extent,
        }
    }

    fn snap_corner(&self, c: f64) -> f64 {
        js_round((c - self.half) / self.spacing) * self.spacing
    }

    fn build_field(&self, cx: f64, cz: f64, grid_n: usize, host: &mut dyn HeightfieldHost) -> Option<Field> {
        let corner_x = self.snap_corner(cx);
        let corner_z = self.snap_corner(cz);
        let grid_spacing = self.extent / (grid_n as f64 - 1.0);
        let body_id = host.add_height_field(grid_n, grid_spacing, [corner_x, corner_z])?;
        Some(Field { body_id, center: [corner_x + self.half, corner_z + self.half], n: grid_n })
    }

    fn retire_unowned(&mut self, players: &[[f64; 2]], host: &mut dyn HeightfieldHost) {
        if players.is_empty() {
            return;
        }
        let load = loads(players, &self.fields, self.half);
        for f in (0..self.fields.len()).rev() {
            if load[f] > 0 {
                continue;
            }
            host.remove_body(self.fields[f].body_id);
            self.fields.remove(f);
        }
    }

    fn publish_primary(&self, players: &[[f64; 2]], host: &mut dyn HeightfieldHost) {
        if self.fields.is_empty() {
            host.set_terrain_body_id(None);
            return;
        }
        let load = loads(players, &self.fields, self.half);
        let mut best = 0;
        for f in 1..self.fields.len() {
            if load[f] > load[best] {
                best = f;
            }
        }
        host.set_terrain_body_id(Some(self.fields[best].body_id));
    }

    pub fn setup(options: StreamerOptions, raw: &[[f64; 2]], fallback: [f64; 2], host: &mut dyn HeightfieldHost) -> Self {
        let mut streamer = Self::new(options);
        streamer.start(raw, fallback, host);
        streamer
    }

    fn start(&mut self, raw: &[[f64; 2]], fallback: [f64; 2], host: &mut dyn HeightfieldHost) {
        let players = finite_centers(raw);
        let seed = players.first().copied().unwrap_or(fallback);
        let coarse_n = if self.n >= 16 {
            let half_n = js_round(self.n as f64 / 2.0) as usize;
            half_n + half_n % 2
        } else {
            0
        };
        let boot_n = if coarse_n > 0 { coarse_n } else { self.n };
        let Some(first) = self.build_field(seed[0], seed[1], boot_n, host) else {
            return;
        };
        self.fields.push(first);
        host.set_terrain_body_id(Some(first.body_id));
        if coarse_n > 0 && coarse_n < self.n {
            self.refine_field(first.body_id, seed, host);
        }
    }

    fn refine_field(&mut self, coarse_id: u64, seed: [f64; 2], host: &mut dyn HeightfieldHost) {
        let Some(fine) = self.build_field(seed[0], seed[1], self.n, host) else {
            return;
        };
        let Some(index) = self.fields.iter().position(|f| f.body_id == coarse_id) else {
            host.remove_body(fine.body_id);
            return;
        };
        let old = self.fields[index].body_id;
        self.fields[index] = fine;
        if old != fine.body_id {
            host.remove_body(old);
        }
        host.set_terrain_body_id(Some(fine.body_id));
    }

    pub fn pass(&mut self, raw: &[[f64; 2]], host: &mut dyn HeightfieldHost) {
        let players = finite_centers(raw);
        if players.is_empty() {
            return;
        }
        let mut need = next_uncovered_centre(&players, &self.fields, self.cover_radius, self.half);
        while let Some(centre) = need {
            if self.disposed {
                break;
            }
            if self.fields.len() >= self.max_fields {
                self.retire_unowned(&players, host);
            }
            if self.fields.len() >= self.max_fields {
                self.refusal_count += 1;
                break;
            }
            let Some(built) = self.build_field(centre[0], centre[1], self.n, host) else {
                self.refusal_count += 1;
                break;
            };
            self.fields.push(built);
            self.rebuild_count += 1;
            need = next_uncovered_centre(&finite_centers(raw), &self.fields, self.cover_radius, self.half);
        }
        if self.disposed {
            return;
        }
        let now = finite_centers(raw);
        self.retire_unowned(&now, host);
        self.publish_primary(&now, host);
    }

    pub fn stop(&mut self, host: &mut dyn HeightfieldHost) {
        self.disposed = true;
        for f in self.fields.drain(..) {
            host.remove_body(f.body_id);
        }
        host.set_terrain_body_id(None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[derive(Debug, Clone, PartialEq)]
    enum Event {
        Add { n: usize, spacing: f64, origin: [f64; 2], id: u64 },
        Remove { id: u64 },
        Primary { id: Option<u64> },
    }

    #[derive(Default)]
    struct Recorder {
        next_id: u64,
        events: Vec<Event>,
    }

    impl HeightfieldHost for Recorder {
        fn add_height_field(&mut self, n: usize, spacing: f64, origin: [f64; 2]) -> Option<u64> {
            self.next_id += 1;
            self.events.push(Event::Add { n, spacing, origin, id: self.next_id });
            Some(self.next_id)
        }

        fn remove_body(&mut self, id: u64) {
            self.events.push(Event::Remove { id });
        }

        fn set_terrain_body_id(&mut self, id: Option<u64>) {
            self.events.push(Event::Primary { id });
        }
    }

    fn num(v: &Value) -> f64 {
        match v.as_str() {
            Some("NaN") => f64::NAN,
            _ => v.as_f64().unwrap(),
        }
    }

    fn expected(v: &Value) -> Event {
        match v["e"].as_str().unwrap() {
            "add" => Event::Add {
                n: v["n"].as_u64().unwrap() as usize,
                spacing: num(&v["spacing"]),
                origin: [num(&v["origin"][0]), num(&v["origin"][1])],
                id: v["id"].as_u64().unwrap(),
            },
            "remove" => Event::Remove { id: v["id"].as_u64().unwrap() },
            "primary" => Event::Primary { id: v["id"].as_u64() },
            other => panic!("unknown event {other}"),
        }
    }

    fn same(got: &Event, want: &Event) -> bool {
        match (got, want) {
            (Event::Add { n: a, spacing: sa, origin: oa, id: ia }, Event::Add { n: b, spacing: sb, origin: ob, id: ib }) => {
                a == b && ia == ib && (sa - sb).abs() <= 1e-9 && (oa[0] - ob[0]).abs() <= 1e-9 && (oa[1] - ob[1]).abs() <= 1e-9
            }
            _ => got == want,
        }
    }

    fn check_steps(label: &str, got: &[Event], want: &[Value]) {
        let want: Vec<Event> = want.iter().map(expected).collect();
        assert_eq!(got.len(), want.len(), "{label}: event count bevy {:?} against spoint {:?}", got, want);
        for (i, (g, w)) in got.iter().zip(want.iter()).enumerate() {
            assert!(same(g, w), "{label}: event {i} bevy {g:?} against spoint {w:?}");
        }
    }

    #[test]
    fn terrain_streaming_request_and_release_order_matches_spoint() {
        let fixture: Value = serde_json::from_str(include_str!("../testdata/parity/spoint-terrain-stream.json")).unwrap();
        for scenario in fixture["scenarios"].as_array().unwrap() {
            let name = scenario["name"].as_str().unwrap();
            let options = StreamerOptions {
                max_fields: scenario["maxFields"].as_u64().map(|m| m as usize),
                ..StreamerOptions::default()
            };
            let path: Vec<Vec<[f64; 2]>> = scenario["path"]
                .as_array()
                .unwrap()
                .iter()
                .map(|tick| tick.as_array().unwrap().iter().map(|p| [num(&p[0]), num(&p[1])]).collect())
                .collect();
            let steps = scenario["trace"].as_array().unwrap();
            let mut host = Recorder::default();
            let mut streamer = TerrainStreamer::setup(options, &path[0], [0.0, 0.0], &mut host);
            check_steps(&format!("{name} start"), &host.events, steps[0]["events"].as_array().unwrap());
            host.events.clear();
            for (t, tick) in path.iter().enumerate() {
                streamer.pass(tick, &mut host);
                check_steps(&format!("{name} tick {t}"), &host.events, steps[t + 1]["events"].as_array().unwrap());
                host.events.clear();
            }
            streamer.stop(&mut host);
            check_steps(&format!("{name} stop"), &host.events, steps[steps.len() - 1]["events"].as_array().unwrap());
        }
    }
}
