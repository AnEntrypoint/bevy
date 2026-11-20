use super::ops_jsgen::V3;

pub fn normalise(v: V3) -> V3 {
    let length = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    [v[0] / length, v[1] / length, v[2] / length]
}

fn cross(a: V3, b: V3) -> V3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

pub struct Basis {
    pub up: V3,
    pub east: V3,
    pub north: V3,
}

pub fn anchor_basis(anchor: V3) -> Basis {
    let up = normalise(anchor);
    let reference = if up[1].abs() < 0.99 { [0.0, 1.0, 0.0] } else { [1.0, 0.0, 0.0] };
    let east = normalise(cross(reference, up));
    let north = cross(east, up);
    Basis { up, east, north }
}

pub fn tangent_local_to_dir(basis: &Basis, radius: f64, x: f64, z: f64) -> V3 {
    normalise([0, 1, 2].map(|i| basis.up[i] * radius + basis.east[i] * x + basis.north[i] * z))
}

#[cfg_attr(not(test), allow(dead_code))]
pub struct Frame {
    pub offset_y: f64,
    pub anchor_height: f64,
    pub radius: f64,
}

#[cfg_attr(not(test), allow(dead_code))]
impl Frame {
    fn sphere_radius(&self) -> f64 {
        if self.radius.is_finite() && self.radius > 0.0 { self.radius } else { f64::INFINITY }
    }

    fn is_finite(&self) -> bool {
        self.offset_y.is_finite() && self.anchor_height.is_finite()
    }

    pub fn sphere_surface_local_y(&self, x: f64, z: f64, elevation: f64) -> Option<f64> {
        if !self.is_finite() || !elevation.is_finite() {
            return None;
        }
        let radius = self.sphere_radius();
        let drop = if radius == f64::INFINITY { 0.0 } else { sphere_drop_below_tangent(x * x + z * z, radius + elevation) };
        Some(self.offset_y + (elevation - self.anchor_height) - drop)
    }

    pub fn waterline_local_y(&self, x: f64, z: f64) -> Option<f64> {
        self.sphere_surface_local_y(x, z, 0.0)
    }

    pub fn elevation_at_local(&self, x: f64, y: f64, z: f64) -> Option<f64> {
        if !self.is_finite() {
            return None;
        }
        let radius = self.sphere_radius();
        if radius == f64::INFINITY {
            return Some(y - self.offset_y + self.anchor_height);
        }
        let t = radius + self.anchor_height + (y - self.offset_y);
        let r2 = x * x + z * z;
        let l = (t * t + r2).sqrt();
        Some((t - radius) + r2 / (l + t))
    }
}

#[cfg_attr(not(test), allow(dead_code))]
fn sphere_drop_below_tangent(r2: f64, sphere_radius: f64) -> f64 {
    let q = sphere_radius * sphere_radius - r2;
    if q > 0.0 { r2 / (sphere_radius + q.sqrt()) } else { sphere_radius }
}

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, PartialEq)]
pub enum GroundFault {
    Solve,
    Other,
}

#[cfg_attr(not(test), allow(dead_code))]
pub fn guarded_ground_height(height: impl Fn(f64, f64) -> Result<Option<f64>, GroundFault>, x: f64, z: f64) -> Result<Option<f64>, GroundFault> {
    match height(x, z) {
        Ok(value) => Ok(value),
        Err(GroundFault::Solve) => Ok(None),
        Err(GroundFault::Other) => Err(GroundFault::Other),
    }
}

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug)]
pub struct RayHit {
    pub distance: f64,
    pub point: V3,
    pub normal: V3,
    pub cell: V3,
    pub adjacent: V3,
}

const RAYCAST_BISECT_STEPS: usize = 16;

#[cfg_attr(not(test), allow(dead_code))]
pub fn raycast_ground_local(ground: impl Fn(f64, f64) -> Option<f64>, origin: V3, direction: V3, max_distance: f64, step_m: f64, cell_m: f64) -> Option<RayHit> {
    let len = (direction[0] * direction[0] + direction[1] * direction[1] + direction[2] * direction[2]).sqrt();
    if !(len > 0.0) || !max_distance.is_finite() || max_distance <= 0.0 {
        return None;
    }
    let d = [direction[0] / len, direction[1] / len, direction[2] / len];
    let sample_at = |t: f64| -> (V3, bool) {
        let p = [origin[0] + d[0] * t, origin[1] + d[1] * t, origin[2] + d[2] * t];
        let above = match ground(p[0], p[2]) {
            None => true,
            Some(g) => p[1] > g,
        };
        (p, above)
    };
    let hit_at = |distance: f64, p: V3| -> RayHit {
        let e = cell_m * 0.5;
        let ground_at = |x: f64, z: f64| ground(x, z);
        let gx = match (ground_at(p[0] + e, p[2]), ground_at(p[0] - e, p[2])) {
            (Some(plus), Some(minus)) => (plus - minus) / (2.0 * e),
            _ => 0.0,
        };
        let gz = match (ground_at(p[0], p[2] + e), ground_at(p[0], p[2] - e)) {
            (Some(plus), Some(minus)) => (plus - minus) / (2.0 * e),
            _ => 0.0,
        };
        let nl = (gx * gx + 1.0 + gz * gz).sqrt();
        let normal = [-gx / nl, 1.0 / nl, -gz / nl];
        let cell = p.map(|c| (c / cell_m).floor());
        let adjacent = [0, 1, 2].map(|i| ((p[i] + normal[i] * cell_m) / cell_m).floor());
        RayHit { distance, point: p, normal, cell, adjacent }
    };
    let (p0, above0) = sample_at(0.0);
    if !above0 {
        return Some(hit_at(0.0, p0));
    }
    let mut prev_t = 0.0;
    let mut t = step_m;
    while t <= max_distance + step_m * 0.5 {
        let tc = t.min(max_distance);
        let (_, above) = sample_at(tc);
        if !above {
            let (mut lo, mut hi) = (prev_t, tc);
            for _ in 0..RAYCAST_BISECT_STEPS {
                let mid = 0.5 * (lo + hi);
                if sample_at(mid).1 {
                    lo = mid;
                } else {
                    hi = mid;
                }
            }
            return Some(hit_at(hi, sample_at(hi).0));
        }
        if tc >= max_distance {
            break;
        }
        prev_t = tc;
        t += step_m;
    }
    None
}

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Debug, PartialEq)]
pub enum SurfaceSolve {
    Tolerance,
    ChartRange,
    NoGround,
    NoConvergence { residual: f64 },
    Argument,
}

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SolveStats {
    pub solves: u64,
    pub evaluations: u64,
}

thread_local! {
    static SOLVE_STATS: std::cell::Cell<SolveStats> = const { std::cell::Cell::new(SolveStats { solves: 0, evaluations: 0 }) };
    static MEMO_STATS: std::cell::Cell<MemoStats> = const { std::cell::Cell::new(MemoStats { hits: 0, misses: 0, evictions: 0, epoch_stale: 0, cell_collisions: 0 }) };
}

#[cfg_attr(not(test), allow(dead_code))]
pub fn surface_solve_stats() -> SolveStats {
    SOLVE_STATS.with(|c| c.get())
}

#[cfg_attr(not(test), allow(dead_code))]
pub fn reset_surface_solve_stats() {
    SOLVE_STATS.with(|c| c.set(SolveStats::default()))
}

#[cfg_attr(not(test), allow(dead_code))]
pub fn ground_memo_stats() -> MemoStats {
    MEMO_STATS.with(|c| c.get())
}

#[cfg_attr(not(test), allow(dead_code))]
pub fn reset_ground_memo_stats() {
    MEMO_STATS.with(|c| c.set(MemoStats::default()))
}

#[cfg_attr(not(test), allow(dead_code))]
pub fn is_surface_root_within(residual_m: f64, tolerance_m: f64) -> bool {
    residual_m.is_finite() && tolerance_m.is_finite() && tolerance_m > 0.0 && residual_m.abs() <= tolerance_m
}

fn bump_solve(f: impl FnOnce(&mut SolveStats)) {
    SOLVE_STATS.with(|c| {
        let mut s = c.get();
        f(&mut s);
        c.set(s);
    })
}

fn bump_memo(f: impl FnOnce(&mut MemoStats)) {
    MEMO_STATS.with(|c| {
        let mut s = c.get();
        f(&mut s);
        c.set(s);
    })
}

const SURFACE_SOLVE_MAX_EVALS: usize = 64;
const BRACKET_HALVING_WINDOW: usize = 6;

#[cfg_attr(not(test), allow(dead_code))]
pub fn solve_surface_y(basis: &Basis, radius: f64, anchor_height: f64, x: f64, z: f64, height: impl Fn(V3) -> Option<f64>, tolerance_m: f64, guess: Option<f64>) -> Result<f64, SurfaceSolve> {
    if !tolerance_m.is_finite() || tolerance_m <= 0.0 {
        return Err(SurfaceSolve::Tolerance);
    }
    let r2 = x * x + z * z;
    if radius.is_finite() && r2 >= radius * radius {
        return Err(SurfaceSolve::ChartRange);
    }
    bump_solve(|s| s.solves += 1);
    let mut y = match guess {
        Some(g) if g.is_finite() => g,
        _ => render_y_on_sphere(r2, 0.0, anchor_height, radius),
    };
    let (mut y_prev, mut g_prev) = (0.0, 0.0);
    let (mut below, mut above) = (f64::NEG_INFINITY, f64::INFINITY);
    let (mut g_below, mut g_above) = (0.0, 0.0);
    let mut checkpoint: Option<(usize, f64)> = None;
    let mut last_g = f64::INFINITY;
    for k in 0..SURFACE_SOLVE_MAX_EVALS {
        bump_solve(|s| s.evaluations += 1);
        let dir = tangent_local_to_dir(basis, radius + anchor_height + y, x, z);
        let h = match height(dir) {
            Some(h) if h.is_finite() => h,
            _ => return Err(SurfaceSolve::NoGround),
        };
        let y_f = render_y_on_sphere(r2, h, anchor_height, radius);
        let g = y_f - y;
        last_g = g;
        if g.is_finite() && g.abs() <= tolerance_m {
            return Ok(y);
        }
        if g > 0.0 {
            if y > below {
                below = y;
                g_below = g;
            }
        } else if y < above {
            above = y;
            g_above = g;
        }
        if below > f64::NEG_INFINITY && above < f64::INFINITY && above - below <= tolerance_m {
            return Ok(0.5 * ((below + g_below) + (above + g_above)));
        }
        let bracketed = below > f64::NEG_INFINITY && above < f64::INFINITY;
        let mut must_bisect = false;
        if bracketed {
            let width = above - below;
            match checkpoint {
                None => checkpoint = Some((k, width)),
                Some((checkpoint_k, checkpoint_width)) if k - checkpoint_k >= BRACKET_HALVING_WINDOW => {
                    must_bisect = width > 0.5 * checkpoint_width;
                    checkpoint = Some((k, width));
                }
                Some(_) => {}
            }
        }
        let mut y_next = if k > 0 && g != g_prev { y - g * (y - y_prev) / (g - g_prev) } else { y_f };
        if must_bisect || !(y_next > below && y_next < above) {
            y_next = if bracketed { 0.5 * (below + above) } else { y_f };
        }
        y_prev = y;
        g_prev = g;
        y = y_next;
    }
    Err(SurfaceSolve::NoConvergence { residual: last_g })
}

#[cfg_attr(not(test), allow(dead_code))]
const SURFACE_SOLVE_TOLERANCE_M: f64 = 1e-4;

#[cfg_attr(not(test), allow(dead_code))]
fn render_y_on_sphere(r2: f64, elevation: f64, anchor_height: f64, radius: f64) -> f64 {
    (elevation - anchor_height) - sphere_drop_below_tangent(r2, radius + elevation)
}

const GROUND_MEMO_SLOTS: usize = 4096;
const GROUND_MEMO_CELLS_PER_M: f64 = 4.0;
const GROUND_MEMO_HASH_X: i32 = 73_856_093;
const GROUND_MEMO_HASH_Z: i32 = 19_349_663;

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MemoStats {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub epoch_stale: u64,
    pub cell_collisions: u64,
}

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone)]
struct GroundMemoSlot {
    x: f64,
    z: f64,
    height: f64,
    epoch: i64,
    failure: Option<SurfaceSolve>,
}

impl GroundMemoSlot {
    fn empty() -> Self {
        Self { x: f64::NAN, z: 0.0, height: 0.0, epoch: -1, failure: None }
    }
}

#[cfg_attr(not(test), allow(dead_code))]
pub type GroundFn = Box<dyn Fn(f64, f64, Option<f64>) -> Result<f64, SurfaceSolve>>;

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, PartialEq)]
pub enum GroundSourceError {
    EmptyName,
    NotFinite,
    Probe(SurfaceSolve),
}

const CPU_SOLVE_SOURCE: &str = "cpu-solve";

#[cfg_attr(not(test), allow(dead_code))]
pub struct ChartFrame {
    basis: Basis,
    radius: f64,
    anchor_height: f64,
    offset_y: f64,
    relief_scale: f64,
    chart_epoch: u32,
    memo: Vec<GroundMemoSlot>,
    ground_sources: Vec<(String, GroundFn)>,
    active_source: String,
    anonymous_installs: u32,
    cpu_height_divergent: bool,
    divergence_reported: bool,
}

#[cfg_attr(not(test), allow(dead_code))]
impl ChartFrame {
    pub fn new(anchor_dir: V3, radius: f64, offset_y: f64, relief_scale: Option<f64>, height: impl Fn(V3) -> f64) -> Self {
        let basis = anchor_basis(anchor_dir);
        let anchor_height = height(basis.up);
        let relief_scale = relief_scale.unwrap_or(0.01);
        Self { basis, radius, anchor_height, offset_y, relief_scale, chart_epoch: 0, memo: vec![GroundMemoSlot::empty(); GROUND_MEMO_SLOTS], ground_sources: Vec::new(), active_source: CPU_SOLVE_SOURCE.to_string(), anonymous_installs: 0, cpu_height_divergent: false, divergence_reported: false }
    }

    pub fn chart_epoch(&self) -> u32 {
        self.chart_epoch
    }

    pub fn reanchor(&mut self, dir: V3, height: impl Fn(V3) -> f64) -> V3 {
        self.basis = anchor_basis(dir);
        self.anchor_height = height(self.basis.up);
        self.chart_epoch += 1;
        self.basis.up
    }

    pub fn relief_scale(&self) -> f64 {
        self.relief_scale
    }

    pub fn anchor_surface_world(&self) -> V3 {
        self.basis.up.map(|c| c * (self.radius + self.anchor_height))
    }

    pub fn local_to_world(&self, x: f64, y: f64, z: f64) -> V3 {
        let s = self.radius + self.anchor_height + y;
        [0, 1, 2].map(|i| self.basis.up[i] * s + (self.basis.east[i] * x + self.basis.north[i] * z))
    }

    pub fn local_to_dir(&self, x: f64, z: f64, y: Option<f64>) -> V3 {
        let render_y = match y {
            Some(y) => y - self.offset_y,
            None => render_y_on_sphere(x * x + z * z, 0.0, self.anchor_height, self.radius),
        };
        tangent_local_to_dir(&self.basis, self.radius + self.anchor_height + render_y, x, z)
    }

    pub fn register_ground_source(&mut self, name: &str, source: GroundFn) -> Result<(), GroundSourceError> {
        if name.is_empty() {
            return Err(GroundSourceError::EmptyName);
        }
        match source(0.0, 0.0, None) {
            Ok(y) if !y.is_finite() => return Err(GroundSourceError::NotFinite),
            Err(err @ SurfaceSolve::Tolerance) => return Err(GroundSourceError::Probe(err)),
            _ => {}
        }
        match self.ground_sources.iter_mut().find(|(installed, _)| installed == name) {
            Some(entry) => entry.1 = source,
            None => self.ground_sources.push((name.to_string(), source)),
        }
        self.active_source = name.to_string();
        Ok(())
    }

    pub fn install_anonymous_ground_source(&mut self, source: GroundFn) -> Result<String, GroundSourceError> {
        self.anonymous_installs += 1;
        let name = if self.anonymous_installs == 1 { "substituted".to_string() } else { format!("substituted#{}", self.anonymous_installs) };
        self.register_ground_source(&name, source)?;
        Ok(name)
    }

    pub fn use_cpu_solve(&mut self) {
        self.active_source = CPU_SOLVE_SOURCE.to_string();
    }

    pub fn ground_source(&self) -> &str {
        &self.active_source
    }

    pub fn ground_sources(&self) -> Vec<String> {
        std::iter::once(CPU_SOLVE_SOURCE.to_string()).chain(self.ground_sources.iter().map(|(name, _)| name.clone())).collect()
    }

    pub fn has_ground_source(&self, name: &str) -> bool {
        name == CPU_SOLVE_SOURCE || self.ground_sources.iter().any(|(installed, _)| installed == name)
    }

    pub fn set_cpu_height_divergent(&mut self, divergent: bool) {
        self.cpu_height_divergent = divergent;
    }

    pub fn ground_height_active(&mut self, x: f64, z: f64, height: impl Fn(V3) -> Option<f64>) -> Result<f64, SurfaceSolve> {
        if self.active_source == CPU_SOLVE_SOURCE {
            return self.ground_height_local(x, z, height);
        }
        let (_, source) = self.ground_sources.iter().find(|(name, _)| *name == self.active_source).expect("the active ground source is installed");
        source(x, z, None)
    }

    pub fn elevation_at_dir(&mut self, d: V3, height: impl Fn(V3) -> f64, hash_version: u32) -> (f64, Option<String>) {
        let warning = if self.cpu_height_divergent && self.active_source != CPU_SOLVE_SOURCE && !self.divergence_reported {
            self.divergence_reported = true;
            Some(format!("[terrain] elevationAtDir is reading the CPU height sampler while this frame takes its ground from another source: at terrain hashVersion {hash_version} the CPU sampler and any GPU terrain disagree by a measured 1.4 m mean / 4.1 m max over 256 m, so this elevation describes ground nobody walks on -- read the frame ground height and convert it with elevationAtLocal, or seed a solve with cpuElevationAtDir"))
        } else {
            None
        };
        (height(normalise(d)), warning)
    }

    pub fn ground_height_local(&mut self, x: f64, z: f64, height: impl Fn(V3) -> Option<f64>) -> Result<f64, SurfaceSolve> {
        if !x.is_finite() || !z.is_finite() {
            return Err(SurfaceSolve::Argument);
        }
        let cell_x = (x * GROUND_MEMO_CELLS_PER_M).floor() as i64;
        let cell_z = (z * GROUND_MEMO_CELLS_PER_M).floor() as i64;
        let slot = ((((cell_x as i32).wrapping_mul(GROUND_MEMO_HASH_X)) ^ ((cell_z as i32).wrapping_mul(GROUND_MEMO_HASH_Z))) & (GROUND_MEMO_SLOTS as i32 - 1)) as usize;
        let epoch = i64::from(self.chart_epoch);
        let entry = &self.memo[slot];
        if entry.x == x && entry.z == z && entry.epoch == epoch {
            bump_memo(|s| s.hits += 1);
            return match &entry.failure {
                Some(err) => Err(err.clone()),
                None => Ok(entry.height),
            };
        }
        bump_memo(|s| s.misses += 1);
        let previous = &self.memo[slot];
        if previous.epoch != -1 {
            bump_memo(|s| s.evictions += 1);
            if previous.x == x && previous.z == z {
                bump_memo(|s| s.epoch_stale += 1);
            } else if (previous.x * GROUND_MEMO_CELLS_PER_M).floor() as i64 == cell_x && (previous.z * GROUND_MEMO_CELLS_PER_M).floor() as i64 == cell_z {
                bump_memo(|s| s.cell_collisions += 1);
            }
        }
        self.memo[slot] = GroundMemoSlot { x, z, height: 0.0, epoch, failure: None };
        match solve_surface_y(&self.basis, self.radius, self.anchor_height, x, z, height, SURFACE_SOLVE_TOLERANCE_M, None) {
            Ok(y) => {
                self.memo[slot].height = y + self.offset_y;
                Ok(self.memo[slot].height)
            }
            Err(err) => {
                if err == SurfaceSolve::Tolerance {
                    self.memo[slot].epoch = -1;
                } else {
                    self.memo[slot].failure = Some(err.clone());
                }
                Err(err)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{anchor_basis, guarded_ground_height, raycast_ground_local, render_y_on_sphere, solve_surface_y, tangent_local_to_dir, ChartFrame, Frame, GroundFault, GroundFn, GroundSourceError, SurfaceSolve};
    use serde_json::Value;
    use super::{ground_memo_stats, is_surface_root_within, reset_ground_memo_stats, reset_surface_solve_stats, sphere_drop_below_tangent, surface_solve_stats};

    const PLANET_FRAME_TOLERANCE_M: f64 = 1e-9;

    fn fixture_ground() -> Value {
        serde_json::from_str(include_str!("../../testdata/parity/spoint-planet-ground.json")).unwrap()
    }

    fn fixture() -> Value {
        serde_json::from_str(include_str!("../../testdata/parity/spoint-planet-frame.json")).unwrap()
    }

    fn vec3(value: &Value) -> [f64; 3] {
        [value[0].as_f64().unwrap(), value[1].as_f64().unwrap(), value[2].as_f64().unwrap()]
    }

    fn frame_of(value: &Value) -> Frame {
        let read = |key: &str| value[key].as_f64().unwrap_or(f64::NAN);
        Frame { offset_y: read("offsetY"), anchor_height: read("anchorHeight"), radius: read("radius") }
    }

    fn compare(worst: &mut f64, got: Option<f64>, want: &Value) {
        match (got, want.as_f64()) {
            (None, None) => {}
            (Some(g), Some(w)) => *worst = worst.max((g - w).abs()),
            (g, w) => panic!("spoint returns {w:?}, bevy returns {g:?}"),
        }
    }

    #[test]
    fn anchor_basis_matches_the_spoint_frame() {
        for case in fixture()["basis"].as_array().unwrap() {
            let basis = anchor_basis(vec3(&case["anchor"]));
            for (got, want) in [(basis.up, &case["up"]), (basis.east, &case["east"]), (basis.north, &case["north"])] {
                let want = vec3(want);
                for i in 0..3 {
                    assert!((got[i] - want[i]).abs() <= PLANET_FRAME_TOLERANCE_M, "basis {got:?} differs from spoint {want:?}");
                }
            }
        }
    }

    #[test]
    fn tangent_local_to_dir_matches_the_spoint_frame() {
        let mut worst = 0.0_f64;
        for case in fixture()["tangent"].as_array().unwrap() {
            let dir = tangent_local_to_dir(&anchor_basis(vec3(&case["anchor"])), case["radius"].as_f64().unwrap(), case["x"].as_f64().unwrap(), case["z"].as_f64().unwrap());
            let want = vec3(&case["dir"]);
            for i in 0..3 {
                worst = worst.max((dir[i] - want[i]).abs());
            }
        }
        assert!(worst <= PLANET_FRAME_TOLERANCE_M, "tangent direction differs from spoint by up to {worst}");
    }

    #[test]
    fn frame_geometry_matches_the_spoint_frame() {
        let mut worst = 0.0_f64;
        for frame_case in fixture()["frames"].as_array().unwrap() {
            let frame = frame_of(&frame_case["frame"]);
            for case in frame_case["cases"].as_array().unwrap() {
                let (x, y, z) = (case["x"].as_f64().unwrap(), case["y"].as_f64().unwrap(), case["z"].as_f64().unwrap());
                if case.get("waterline").is_some() {
                    compare(&mut worst, frame.waterline_local_y(x, z), &case["waterline"]);
                }
                if case.get("surface").is_some() {
                    compare(&mut worst, frame.sphere_surface_local_y(x, z, case["elevation"].as_f64().unwrap()), &case["surface"]);
                }
                compare(&mut worst, frame.elevation_at_local(x, y, z), &case["elevationAtLocal"]);
            }
        }
        assert!(worst <= PLANET_FRAME_TOLERANCE_M, "frame geometry differs from spoint by up to {worst} m");
    }

    fn mock_ground(x: f64, z: f64) -> Option<f64> {
        if x < -450.0 { None } else { Some(3.0 + 0.02 * x - 0.015 * z + 0.0001 * x * z) }
    }

    fn mock_height(x: f64, z: f64) -> Result<Option<f64>, GroundFault> {
        if z > 300.0 {
            return Err(GroundFault::Other);
        }
        if x > 100.0 {
            return Err(GroundFault::Solve);
        }
        Ok(if x < -400.0 { None } else { Some(3.0 + 0.02 * x - 0.015 * z) })
    }

    #[test]
    fn raycast_ground_local_matches_the_spoint_frame() {
        let mut worst = 0.0_f64;
        for case in fixture_ground()["raycast"].as_array().unwrap() {
            let origin = vec3(&case["origin"]);
            let direction = vec3(&case["direction"]);
            let got = raycast_ground_local(mock_ground, origin, direction, case["maxDistance"].as_f64().unwrap(), case["stepM"].as_f64().unwrap(), case["cellM"].as_f64().unwrap());
            match (got, case["hit"].is_null()) {
                (None, true) => {}
                (Some(hit), false) => {
                    let want = &case["hit"];
                    worst = worst.max((hit.distance - want["distance"].as_f64().unwrap()).abs());
                    for (got, key) in [(hit.point, "point"), (hit.normal, "normal"), (hit.cell, "cell"), (hit.adjacent, "adjacent")] {
                        let want = vec3(&want[key]);
                        for i in 0..3 {
                            worst = worst.max((got[i] - want[i]).abs());
                        }
                    }
                }
                (got, _) => panic!("spoint hit {} but bevy returns {got:?}", case["hit"]),
            }
        }
        assert!(worst <= PLANET_FRAME_TOLERANCE_M, "raycast differs from spoint by up to {worst}");
    }

    #[test]
    fn guarded_ground_height_matches_the_spoint_frame() {
        for case in fixture_ground()["guarded"].as_array().unwrap() {
            let (x, z) = (case["x"].as_f64().unwrap(), case["z"].as_f64().unwrap());
            let got = guarded_ground_height(mock_height, x, z);
            if case["rethrows"].as_bool().unwrap() {
                assert_eq!(got, Err(GroundFault::Other), "spoint rethrows at ({x}, {z})");
            } else {
                match (got, case["value"].as_f64()) {
                    (Ok(None), None) => {}
                    (Ok(Some(g)), Some(w)) => assert!((g - w).abs() <= PLANET_FRAME_TOLERANCE_M, "guarded differs at ({x}, {z}): {g} against {w}"),
                    (got, want) => panic!("spoint {want:?}, bevy {got:?} at ({x}, {z})"),
                }
            }
        }
    }

    fn fixture_surface() -> Value {
        serde_json::from_str(include_str!("../../testdata/parity/spoint-planet-surface.json")).unwrap()
    }

    fn surface_height(kind: &str, d: [f64; 3]) -> Option<f64> {
        let smooth = 40.0 + 25.0 * d[0] - 15.0 * d[1] + 10.0 * d[2] * d[2];
        match kind {
            "null" => None,
            "zigzag" => Some(40.0 + 50.0 * ((d[2] * 2000.0).floor() as i64 & 1) as f64),
            "smooth" => Some(smooth),
            "step" => Some(if d[0] > 0.3 { 100.0 } else { 40.0 }),
            "nullish" => if d[0] > 0.9 { None } else { Some(smooth) },
            other => panic!("unknown height kind {other}"),
        }
    }

    fn surface_error_name(err: &SurfaceSolve) -> &'static str {
        match err {
            SurfaceSolve::Tolerance => "SolveToleranceError",
            SurfaceSolve::ChartRange => "ChartRangeError",
            SurfaceSolve::NoGround => "NoGroundHeightError",
            SurfaceSolve::NoConvergence { .. } => "SurfaceSolveError",
            SurfaceSolve::Argument => "SurfaceSolveArgumentError",
        }
    }

    #[test]
    fn solve_surface_y_matches_the_spoint_frame() {
        let fixture = fixture_surface();
        let basis = anchor_basis(vec3(&fixture["anchorDir"]));
        let radius = fixture["radius"].as_f64().unwrap();
        let anchor_height = fixture["anchorHeight"].as_f64().unwrap();
        let mut worst = 0.0_f64;
        for case in fixture["cases"].as_array().unwrap() {
            let (x, z) = (case["x"].as_f64().unwrap(), case["z"].as_f64().unwrap());
            let kind = case["kind"].as_str().unwrap();
            let tolerance = case["tol"].as_f64().unwrap();
            let guess = case["guess"].as_f64();
            let got = solve_surface_y(&basis, radius, anchor_height, x, z, |d| surface_height(kind, d), tolerance, guess);
            match (got, case["error"].as_str()) {
                (Ok(y), None) => worst = worst.max((y - case["y"].as_f64().unwrap()).abs()),
                (Err(err), Some(name)) => assert_eq!(surface_error_name(&err), name, "error at {kind} ({x}, {z})"),
                (got, want) => panic!("spoint {want:?} with y {:?}, bevy {got:?} at {kind} ({x}, {z})", case["y"]),
            }
        }
        assert!(worst <= PLANET_FRAME_TOLERANCE_M, "solveSurfaceY differs from spoint by up to {worst} m");
    }

    fn fixture_chart() -> Value {
        serde_json::from_str(include_str!("../../testdata/parity/spoint-planet-chart.json")).unwrap()
    }

    #[test]
    fn chart_frame_matches_the_spoint_frame() {
        let fixture = fixture_chart();
        let mut frame = ChartFrame::new(vec3(&fixture["anchorDir"]), fixture["radius"].as_f64().unwrap(), fixture["offsetY"].as_f64().unwrap(), Some(0.001), |d| surface_height("smooth", d).unwrap());
        let mut worst = (frame.anchor_height - fixture["anchorHeight"].as_f64().unwrap()).abs();
        let mut measure = |got: [f64; 3], want: [f64; 3]| {
            for i in 0..3 {
                worst = worst.max((got[i] - want[i]).abs());
            }
        };
        measure(frame.anchor_surface_world(), vec3(&fixture["anchorSurfaceWorld"]));
        for case in fixture["localToWorld"].as_array().unwrap() {
            let got = frame.local_to_world(case["x"].as_f64().unwrap(), case["y"].as_f64().unwrap(), case["z"].as_f64().unwrap());
            measure(got, vec3(&case["out"]));
        }
        for case in fixture["localToDir"].as_array().unwrap() {
            let got = frame.local_to_dir(case["x"].as_f64().unwrap(), case["z"].as_f64().unwrap(), case["y"].as_f64());
            measure(got, vec3(&case["out"]));
        }
        for case in fixture["groundHeightLocal"].as_array().unwrap() {
            let (x, z) = (case["x"].as_f64().unwrap(), case["z"].as_f64().unwrap());
            match (frame.ground_height_local(x, z, |d| surface_height("smooth", d)), case["error"].as_str()) {
                (Ok(y), None) => worst = worst.max((y - case["y"].as_f64().unwrap()).abs()),
                (got, want) => panic!("spoint {want:?}, bevy {got:?} at ({x}, {z})"),
            }
        }
        assert!(worst <= PLANET_FRAME_TOLERANCE_M, "chart frame differs from spoint by up to {worst} m");
    }

    #[test]
    fn solve_surface_y_reaches_the_closure_and_cap_paths_like_spoint() {
        let fixture: Value = serde_json::from_str(include_str!("../../testdata/parity/spoint-solve-paths.json")).unwrap();
        let basis = anchor_basis([0.0, 1.0, 0.0]);
        let radius = fixture["radius"].as_f64().unwrap();
        let mut worst = 0.0_f64;
        for case in fixture["cases"].as_array().unwrap() {
            let (x, z, tol) = (case["x"].as_f64().unwrap(), case["z"].as_f64().unwrap(), case["tol"].as_f64().unwrap());
            let (c, a, b) = (case["c"].as_f64().unwrap(), case["A"].as_f64().unwrap(), case["B"].as_f64().unwrap());
            let height = move |d: [f64; 3]| Some(if d[0] > c { a } else { b });
            let anchor_height = height(basis.up).unwrap();
            match (solve_surface_y(&basis, radius, anchor_height, x, z, height, tol, None), case["error"].as_str()) {
                (Ok(y), None) => {
                    worst = worst.max((y - case["y"].as_f64().unwrap()).abs());
                    let dir = tangent_local_to_dir(&basis, radius + anchor_height + y, x, z);
                    let residual = render_y_on_sphere(x * x + z * z, height(dir).unwrap(), anchor_height, radius) - y;
                    worst = worst.max((residual - case["residual"].as_f64().unwrap()).abs());
                    assert!(residual.abs() > tol, "the closure return leaves a residual above the tolerance at ({x}, {z})");
                }
                (Err(err), Some(name)) => assert_eq!(surface_error_name(&err), name, "error at ({x}, {z})"),
                (got, want) => panic!("spoint {want:?} with y {:?}, bevy {got:?} at ({x}, {z})", case["y"]),
            }
        }
        assert!(worst <= PLANET_FRAME_TOLERANCE_M, "solveSurfaceY closure and cap paths differ from spoint by up to {worst} m");
    }

    #[test]
    fn chart_frame_relief_scale_matches_spoint() {
        let fixture: Value = serde_json::from_str(include_str!("../../testdata/parity/spoint-planet-frame-parts.json")).unwrap();
        for case in fixture["relief"].as_array().unwrap() {
            let given = case["reliefScale"].as_f64();
            let frame = ChartFrame::new([0.0, 1.0, 0.0], 6360.0, 0.0, given, |d| surface_height("smooth", d).unwrap());
            let want = case["value"].as_f64().unwrap();
            assert!(frame.relief_scale() == want, "reliefScale {given:?} gives {} where spoint gives {want}", frame.relief_scale());
        }
    }

    fn chart_deviation(frame: &mut ChartFrame, want: &Value) -> f64 {
        let mut worst = (frame.anchor_height - want["anchorHeight"].as_f64().unwrap()).abs();
        let anchor_world = frame.anchor_surface_world();
        let want_world = vec3(&want["anchorSurfaceWorld"]);
        for i in 0..3 {
            worst = worst.max((anchor_world[i] - want_world[i]).abs());
        }
        let dirs = [frame.local_to_dir(100.0, 50.0, None), frame.local_to_dir(100.0, 50.0, Some(20.0))];
        for (got, want) in dirs.iter().zip(want["localToDir"].as_array().unwrap()) {
            for i in 0..3 {
                worst = worst.max((got[i] - vec3(want)[i]).abs());
            }
        }
        let worlds = [frame.local_to_world(0.0, 0.0, 0.0), frame.local_to_world(100.0, 5.0, -50.0)];
        for (got, want) in worlds.iter().zip(want["localToWorld"].as_array().unwrap()) {
            for i in 0..3 {
                worst = worst.max((got[i] - vec3(want)[i]).abs());
            }
        }
        let grounds = [frame.ground_height_local(0.0, 0.0, |d| Some(surface_height("smooth", d).unwrap())), frame.ground_height_local(100.0, 50.0, |d| Some(surface_height("smooth", d).unwrap()))];
        for (got, want) in grounds.iter().zip(want["groundHeightLocal"].as_array().unwrap()) {
            worst = worst.max((got.as_ref().unwrap() - want.as_f64().unwrap()).abs());
        }
        worst
    }

    #[test]
    fn chart_frame_reanchor_matches_spoint() {
        let fixture: Value = serde_json::from_str(include_str!("../../testdata/parity/spoint-planet-frame-parts.json")).unwrap();
        let reanchor = &fixture["reanchor"];
        let mut frame = ChartFrame::new([0.6, 0.0, 0.8], 6360.0, 5.0, Some(0.001), |d| surface_height("smooth", d).unwrap());
        let mut worst = chart_deviation(&mut frame, &reanchor["before"]);
        assert_eq!(frame.chart_epoch() as u64, reanchor["before"]["chartEpoch"].as_u64().unwrap(), "chartEpoch before");
        let up = frame.reanchor([0.0, 1.0, 0.0], |d| surface_height("smooth", d).unwrap());
        for i in 0..3 {
            worst = worst.max((up[i] - vec3(&reanchor["returned"])[i]).abs());
        }
        assert_eq!(frame.chart_epoch() as u64, reanchor["after"]["chartEpoch"].as_u64().unwrap(), "chartEpoch after");
        worst = worst.max(chart_deviation(&mut frame, &reanchor["after"]));
        let up = frame.reanchor([0.6, 0.0, 0.8], |d| surface_height("smooth", d).unwrap());
        for i in 0..3 {
            worst = worst.max((up[i] - vec3(&reanchor["returnedAgain"])[i]).abs());
        }
        assert_eq!(frame.chart_epoch() as u64, reanchor["back"]["chartEpoch"].as_u64().unwrap(), "chartEpoch back");
        worst = worst.max(chart_deviation(&mut frame, &reanchor["back"]));
        assert!(worst <= PLANET_FRAME_TOLERANCE_M, "reanchor differs from spoint by up to {worst}");
    }

    fn json_num(v: &Value) -> f64 {
        match v.as_str() {
            Some("NaN") => f64::NAN,
            Some("Infinity") => f64::INFINITY,
            Some("-Infinity") => f64::NEG_INFINITY,
            _ => v.as_f64().unwrap(),
        }
    }

    #[test]
    fn surface_root_and_drop_match_spoint() {
        let fixture: Value = serde_json::from_str(include_str!("../../testdata/parity/spoint-surface-root.json")).unwrap();
        for case in fixture["surfaceRoot"].as_array().unwrap() {
            let got = is_surface_root_within(json_num(&case["residualM"]), json_num(&case["toleranceM"]));
            assert_eq!(got, case["within"].as_bool().unwrap(), "isSurfaceRootWithin({}, {})", case["residualM"], case["toleranceM"]);
        }
        let mut worst = 0.0_f64;
        for case in fixture["dropBelowTangent"].as_array().unwrap() {
            let got = sphere_drop_below_tangent(json_num(&case["r2"]), json_num(&case["sphereRadius"]));
            worst = worst.max((got - json_num(&case["value"])).abs());
        }
        assert!(worst <= 1e-9, "sphereDropBelowTangent differs from spoint by up to {worst}");
    }

    #[test]
    fn ground_memo_matches_spoint() {
        let fixture: Value = serde_json::from_str(include_str!("../../testdata/parity/spoint-planet-frame-parts.json")).unwrap();
        let mut frame = ChartFrame::new([0.6, 0.0, 0.8], 6360.0, 5.0, Some(0.001), |d| surface_height("smooth", d).unwrap());
        reset_ground_memo_stats();
        reset_surface_solve_stats();
        for step in fixture["memo"].as_array().unwrap() {
            if let Some(dir) = step.get("reanchor") {
                frame.reanchor(vec3(dir), |d| surface_height("smooth", d).unwrap());
                assert_eq!(frame.chart_epoch() as u64, step["chartEpoch"].as_u64().unwrap(), "chartEpoch after reanchor");
            } else {
                let (x, z) = (step["x"].as_f64().unwrap_or(f64::NAN), step["z"].as_f64().unwrap_or(f64::NAN));
                match (frame.ground_height_local(x, z, |d| surface_height("smooth", d)), step["error"].as_str()) {
                    (Ok(y), None) => assert!((y - step["value"].as_f64().unwrap()).abs() <= PLANET_FRAME_TOLERANCE_M, "memo value differs at ({x}, {z}): {y}"),
                    (Err(err), Some(name)) => assert_eq!(surface_error_name(&err), name, "memo error at ({x}, {z})"),
                    (got, want) => panic!("spoint {want:?}, bevy {got:?} at ({x}, {z})"),
                }
            }
            let want = &step["stats"];
            let got = ground_memo_stats();
            let solve = surface_solve_stats();
            assert_eq!(solve.solves, step["solve"]["solves"].as_u64().unwrap(), "solves");
            assert_eq!(solve.evaluations, step["solve"]["evaluations"].as_u64().unwrap(), "evaluations");
            assert_eq!(got.hits, want["hits"].as_u64().unwrap(), "hits");
            assert_eq!(got.misses, want["misses"].as_u64().unwrap(), "misses");
            assert_eq!(got.evictions, want["evictions"].as_u64().unwrap(), "evictions");
            assert_eq!(got.epoch_stale, want["epochStale"].as_u64().unwrap(), "epochStale");
            assert_eq!(got.cell_collisions, want["cellCollisions"].as_u64().unwrap(), "cellCollisions");
        }
    }

    fn gpu_source() -> GroundFn {
        Box::new(|x, z, _| if x > 5000.0 { Err(SurfaceSolve::NoConvergence { residual: 0.5 }) } else { Ok(0.5 + 0.01 * x - 0.02 * z) })
    }

    fn source_error_kind(err: &GroundSourceError) -> &'static str {
        match err {
            GroundSourceError::EmptyName | GroundSourceError::NotFinite => "TypeError",
            GroundSourceError::Probe(err) => surface_error_name(err),
        }
    }

    #[test]
    fn ground_sources_match_spoint() {
        let fixture: Value = serde_json::from_str(include_str!("../../testdata/parity/spoint-planet-frame-parts.json")).unwrap();
        let sources = &fixture["sources"];
        let names = |v: &Value| -> Vec<String> { v.as_array().unwrap().iter().map(|n| n.as_str().unwrap().to_string()).collect() };
        let smooth = |d: [f64; 3]| surface_height("smooth", d);
        let mut frame = ChartFrame::new([0.6, 0.0, 0.8], 6360.0, 5.0, Some(0.001), |d| surface_height("smooth", d).unwrap());
        assert_eq!(frame.ground_sources(), names(&sources["initial"]["names"]), "initial names");
        assert_eq!(frame.ground_source(), sources["initial"]["source"].as_str().unwrap());
        let cpu = frame.ground_height_active(100.0, 50.0, smooth).unwrap();
        assert!((cpu - sources["initial"]["cpu"].as_f64().unwrap()).abs() <= PLANET_FRAME_TOLERANCE_M, "cpu ground");
        frame.register_ground_source("gpu", gpu_source()).unwrap();
        assert_eq!(frame.ground_sources(), names(&sources["gpu"]["names"]), "gpu names");
        assert_eq!(frame.ground_source(), "gpu");
        for ((x, z), want) in [(100.0, 0.0), (0.0, 50.0)].iter().zip(sources["gpu"]["values"].as_array().unwrap()) {
            let got = frame.ground_height_active(*x, *z, smooth).unwrap();
            assert!((got - want.as_f64().unwrap()).abs() <= PLANET_FRAME_TOLERANCE_M, "gpu ground at ({x}, {z})");
        }
        let err = frame.ground_height_active(6000.0, 0.0, smooth).unwrap_err();
        assert_eq!(surface_error_name(&err), sources["gpuThrows"]["error"].as_str().unwrap(), "gpu throws");
        let err = frame.register_ground_source("", gpu_source()).unwrap_err();
        assert_eq!(source_error_kind(&err), sources["emptyName"]["error"].as_str().unwrap(), "empty name");
        let err = frame.register_ground_source("nullish", Box::new(|_, _, _| Ok(f64::NAN))).unwrap_err();
        assert_eq!(source_error_kind(&err), sources["nullishSource"]["error"].as_str().unwrap(), "nullish source");
        assert!(frame.has_ground_source("gpu"));
        assert!(!frame.has_ground_source("nope"));
        let name = frame.install_anonymous_ground_source(Box::new(|x, _, _| Ok(1.0 + 0.001 * x))).unwrap();
        assert_eq!(name, "substituted");
        assert_eq!(frame.ground_sources(), names(&sources["anonymousA"]["names"]), "anonymous A names");
        let got = frame.ground_height_active(100.0, 50.0, smooth).unwrap();
        assert!((got - sources["anonymousA"]["values"][0].as_f64().unwrap()).abs() <= PLANET_FRAME_TOLERANCE_M, "anonymous A value");
        let name = frame.install_anonymous_ground_source(Box::new(|_, _, _| Ok(2.0))).unwrap();
        assert_eq!(name, sources["anonymousB"]["source"].as_str().unwrap(), "anonymous B name");
        frame.use_cpu_solve();
        assert_eq!(frame.ground_sources(), names(&sources["cpuSetter"]["names"]), "cpu setter names");
        assert_eq!(frame.ground_source(), sources["cpuSetter"]["source"].as_str().unwrap());
        let got = frame.ground_height_active(100.0, 50.0, smooth).unwrap();
        assert!((got - sources["cpuSetter"]["values"][0].as_f64().unwrap()).abs() <= PLANET_FRAME_TOLERANCE_M, "cpu setter value");
    }

    #[test]
    fn elevation_at_dir_warns_once_like_spoint() {
        let fixture: Value = serde_json::from_str(include_str!("../../testdata/parity/spoint-planet-frame-parts.json")).unwrap();
        for label in ["divergentGpu", "plainGpu", "divergentCpu"] {
            let want = &fixture["cpuWarning"][label];
            let mut frame = ChartFrame::new([0.0, 1.0, 0.0], 6360.0, 0.0, Some(0.001), |d| surface_height("smooth", d).unwrap());
            frame.set_cpu_height_divergent(label != "plainGpu");
            if label != "divergentCpu" {
                frame.register_ground_source("gpu", gpu_source()).unwrap();
            }
            let mut messages = Vec::new();
            let mut values = Vec::new();
            for _ in 0..2 {
                let (value, warning) = frame.elevation_at_dir([0.6, 0.3, 0.7], |d| surface_height("smooth", d).unwrap(), 2);
                values.push(value);
                messages.extend(warning);
            }
            assert_eq!(messages.len() as u64, want["messages"].as_u64().unwrap(), "{label} message count");
            if let Some(text) = want["text"].as_str() {
                assert_eq!(messages[0], text, "{label} message text");
            }
            for (got, want) in values.iter().zip(want["values"].as_array().unwrap()) {
                assert!((got - want.as_f64().unwrap()).abs() <= PLANET_FRAME_TOLERANCE_M, "{label} elevation");
            }
        }
    }
}
