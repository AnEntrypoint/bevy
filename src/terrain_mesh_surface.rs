use crate::height::anchor::FACE_FRAME;
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;
use crate::js_math::round_half_up;

pub type V3 = [f64; 3];

const QUARTER_PI: f64 = std::f64::consts::FRAC_PI_4;
const ATAN_INV_K: f64 = 4.0 / std::f64::consts::PI;
const MESH_SOLVE_ITERS: usize = 4;
const MESH_SOLVE_TOL_M: f64 = 1e-6;
const DEFAULT_FOCUS_RADIUS_M: f64 = 240.0;
const DEFAULT_MAX_QUADS: f64 = 512.0;
const FOOTPRINT_INFLATE_M: f64 = 1.0;
const FOOTPRINT_BLOAT: f64 = 1.06;
const REACH_MUL: f64 = 1.6;
const QUAD_CORNER_U: [f64; 4] = [0.0, 1.0, 1.0, 0.0];
const QUAD_CORNER_V: [f64; 4] = [0.0, 0.0, 1.0, 1.0];
const NEAR_GRID_MAX_FAN: i64 = 64;
const NEAR_GRID_KEY_BIAS: i64 = 32768;
const NEAR_GRID_KEY_STRIDE: i64 = 65536;
const TRI_TOLERANCE: f64 = -1e-12;

#[derive(Clone, Copy, Debug)]
pub struct MeshFrame {
    pub radius: f64,
    pub anchor_height: f64,
    pub offset_y: f64,
    pub up: V3,
    pub east: V3,
    pub north: V3,
    pub chart_epoch: Option<f64>,
}

pub trait MeshLattice {
    fn chart_epoch(&self) -> Option<f64>;
    fn covers(&self, x: f64, z: f64) -> bool;
    fn height_at_local(&self, x: f64, z: f64) -> f64;
}

#[derive(Clone, Debug)]
pub struct DrawnQuads {
    pub count: f64,
    pub grid_size: f64,
    pub offsets: Vec<f64>,
    pub faces: Vec<f64>,
}

pub struct MeshOptions<H, Q> {
    pub frame: MeshFrame,
    pub height_at_dir: H,
    pub quads: Q,
    pub lattice: Option<Box<dyn MeshLattice>>,
    pub focus_radius_m: Option<f64>,
    pub max_quads: Option<f64>,
}

#[derive(Clone, Copy, PartialEq)]
struct Footprint {
    face: usize,
    ox: f64,
    oy: f64,
    l: f64,
    level: f64,
    cx: [f64; 4],
    cz: [f64; 4],
    min_x: f64,
    max_x: f64,
    min_z: f64,
    max_z: f64,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct QuadKey {
    face: usize,
    ox: i64,
    oy: i64,
    level: u64,
    epoch: Option<u64>,
}

struct Quad {
    pos: Vec<f64>,
    i11: f64,
    i12: f64,
    i21: f64,
    i22: f64,
    x0: f64,
    z0: f64,
}

pub struct TerrainMeshSurface<H, Q> {
    pub frame: MeshFrame,
    base: f64,
    height_at_dir: H,
    quads: Q,
    lattice: Option<Box<dyn MeshLattice>>,
    focus_radius_m: f64,
    max_quads: f64,
    focus: [f64; 2],
    cam: [V3; 6],
    reach_u: [f64; 6],
    reach_v: [f64; 6],
    near_cell_m: f64,
    near_grid: HashMap<i64, Vec<usize>>,
    near_overflow: Vec<usize>,
    indexed: Vec<Footprint>,
    built: HashMap<QuadKey, Rc<Quad>>,
    built_order: VecDeque<QuadKey>,
    grid_size: i64,
    version: u64,
    synced_quads: usize,
    lazy_sync_version: Option<u64>,
    lazy_sync_drawn: Option<usize>,
}

pub const MESH_SURFACE_NO_GROUND: Option<f64> = None;

pub fn is_mesh_surface_no_ground(v: Option<f64>) -> bool {
    v == MESH_SURFACE_NO_GROUND
}

pub struct RenderFrame {
    pub render_mesh_height_at_local: Option<Box<dyn FnMut(f64, f64) -> Option<f64>>>,
}

pub fn render_mesh_height_at(frame: &mut RenderFrame) -> Option<&mut dyn FnMut(f64, f64) -> Option<f64>> {
    match frame.render_mesh_height_at_local.as_mut() {
        Some(f) => Some(&mut **f),
        None => None,
    }
}

fn refusal(key: &str, why: &str) -> String {
    format!("createTerrainMeshSurface: opts.{key} {why}")
}

pub fn create_terrain_mesh_surface<H, Q>(opts: MeshOptions<H, Q>) -> Result<TerrainMeshSurface<H, Q>, String>
where
    H: Fn(V3) -> f64,
    Q: Fn() -> Option<DrawnQuads>,
{
    let frame = opts.frame;
    if !(frame.radius.is_finite() && frame.radius > 0.0) {
        return Err(refusal("frame.radius", "must be a finite positive radius"));
    }
    if !frame.anchor_height.is_finite() {
        return Err(refusal("frame.anchorHeight", "must be a finite number"));
    }
    if !frame.offset_y.is_finite() {
        return Err(refusal("frame.offsetY", "must be a finite number"));
    }
    let focus_radius_m = opts.focus_radius_m.unwrap_or(DEFAULT_FOCUS_RADIUS_M);
    if !(focus_radius_m.is_finite() && focus_radius_m > 0.0) {
        return Err(refusal("focusRadiusM", "must be a finite positive number of metres"));
    }
    let max_quads = opts.max_quads.unwrap_or(DEFAULT_MAX_QUADS);
    if !(max_quads.is_finite() && max_quads >= 1.0) {
        return Err(refusal("maxQuads", "must be a finite count of at least one"));
    }
    Ok(TerrainMeshSurface {
        base: frame.radius + frame.anchor_height - frame.offset_y,
        frame,
        height_at_dir: opts.height_at_dir,
        quads: opts.quads,
        lattice: opts.lattice,
        focus_radius_m,
        max_quads,
        focus: [0.0; 2],
        cam: [[0.0; 3]; 6],
        reach_u: [0.0; 6],
        reach_v: [0.0; 6],
        near_cell_m: 0.0,
        near_grid: HashMap::new(),
        near_overflow: Vec::new(),
        indexed: Vec::new(),
        built: HashMap::new(),
        built_order: VecDeque::new(),
        grid_size: 0,
        version: 0,
        synced_quads: 0,
        lazy_sync_version: None,
        lazy_sync_drawn: None,
    })
}

fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn world_to_face_local(face: usize, cam: V3, r: f64) -> V3 {
    let (c, u, v) = FACE_FRAME[face];
    let cu = dot(cam, u);
    let cv = dot(cam, v);
    let cc = dot(cam, c);
    let (ox, oy) = if cc > 1.0 {
        (ATAN_INV_K * r * (cu / cc).atan(), ATAN_INV_K * r * (cv / cc).atan())
    } else {
        (if cu >= 0.0 { r } else { -r }, if cv >= 0.0 { r } else { -r })
    };
    [ox, oy, cc]
}

fn deform_dir(face: usize, u: f64, v: f64, r: f64) -> Option<V3> {
    let (c, fu, fv) = FACE_FRAME[face];
    let wx = r * ((u / r) * QUARTER_PI).tan();
    let wy = r * ((v / r) * QUARTER_PI).tan();
    let a = [0, 1, 2].map(|i| fu[i] * wx + fv[i] * wy + c[i] * r);
    let len = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt();
    if !(len > 0.0) {
        return None;
    }
    Some([a[0] / len, a[1] / len, a[2] / len])
}

fn metres_per_face_unit(face: usize, u: f64, v: f64, du: f64, dv: f64, r: f64, e: V3, n: V3) -> f64 {
    let (Some(a), Some(b)) = (deform_dir(face, u, v, r), deform_dir(face, u + du, v + dv, r)) else {
        return f64::NAN;
    };
    let d = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let dx = d[0] * e[0] + d[1] * e[1] + d[2] * e[2];
    let dz = d[0] * n[0] + d[1] * n[1] + d[2] * n[2];
    r * dx.hypot(dz)
}

fn cell_flip(x: i64, y: i64) -> bool {
    let mut h = (x as i32) | ((y as i32) << 16);
    h = (h ^ (((h as u32) >> 16) as i32)).wrapping_mul(0x045d_9f3b);
    h = (h ^ (((h as u32) >> 16) as i32)).wrapping_mul(0x045d_9f3b);
    h ^= ((h as u32) >> 16) as i32;
    (((h as u32) >> 17) & 1) == 1
}

fn grid_key(i: i64, j: i64) -> i64 {
    (i + NEAR_GRID_KEY_BIAS) * NEAR_GRID_KEY_STRIDE + (j + NEAR_GRID_KEY_BIAS)
}

fn inside_quad(q: &Footprint, x: f64, z: f64) -> bool {
    let mut sign = 0i8;
    for i in 0..4 {
        let j = (i + 1) & 3;
        let cross = (q.cx[j] - q.cx[i]) * (z - q.cz[i]) - (q.cz[j] - q.cz[i]) * (x - q.cx[i]);
        if cross == 0.0 {
            continue;
        }
        let s = if cross > 0.0 { 1 } else { -1 };
        if sign == 0 {
            sign = s;
        } else if s != sign {
            return false;
        }
    }
    true
}

fn corner(pos: &[f64], n: usize, i: usize, j: usize) -> (f64, f64) {
    let k = (j * n + i) * 3;
    (pos[k], pos[k + 2])
}

fn tri_y(p: &[f64], ia: usize, ib: usize, ic: usize, x: f64, z: f64) -> Option<f64> {
    let (ax, ay, az) = (p[ia], p[ia + 1], p[ia + 2]);
    let (bx, by, bz) = (p[ib], p[ib + 1], p[ib + 2]);
    let (cx, cy, cz) = (p[ic], p[ic + 1], p[ic + 2]);
    let det = (bz - cz) * (ax - cx) + (cx - bx) * (az - cz);
    if det == 0.0 {
        return None;
    }
    let wa = ((bz - cz) * (x - cx) + (cx - bx) * (z - cz)) / det;
    let wb = ((cz - az) * (x - cx) + (ax - cx) * (z - cz)) / det;
    let wc = 1.0 - wa - wb;
    if wa < TRI_TOLERANCE || wb < TRI_TOLERANCE || wc < TRI_TOLERANCE {
        return None;
    }
    Some(wa * ay + wb * by + wc * cy)
}

impl<H, Q> TerrainMeshSurface<H, Q>
where
    H: Fn(V3) -> f64,
    Q: Fn() -> Option<DrawnQuads>,
{
    pub fn set_focus(&mut self, x: f64, z: f64) {
        if x.is_finite() && z.is_finite() {
            self.focus = [x, z];
        }
    }

    pub fn version(&self) -> u64 {
        self.version
    }

    pub fn sync(&mut self) {
        self.version += 1;
        self.synced_quads = 0;
        let quads = match (self.quads)() {
            Some(q) if q.count.is_finite() && q.count > 0.0 => q,
            _ => return,
        };
        let g = quads.grid_size as i64;
        if g < 1 {
            return;
        }
        self.ensure_grid(g);
        let r = self.frame.radius;
        let (e, up, nn) = (self.frame.east, self.frame.up, self.frame.north);
        let [fx, fy] = self.focus;
        let cam_w: V3 = [0, 1, 2].map(|i| up[i] * (r + self.frame.anchor_height) + e[i] * fx + nn[i] * fy);
        for face in 0..6 {
            self.cam[face] = world_to_face_local(face, cam_w, r);
        }
        let reach = self.focus_radius_m * REACH_MUL;
        for face in 0..6 {
            let cam = self.cam[face];
            self.reach_u[face] = reach / metres_per_face_unit(face, cam[0], cam[1], 1.0, 0.0, r, e, nn);
            self.reach_v[face] = reach / metres_per_face_unit(face, cam[0], cam[1], 0.0, 1.0, r, e, nn);
        }
        let count = quads.count.min((quads.offsets.len() / 4) as f64);
        let mut near = Vec::new();
        let mut n = 0usize;
        while (n as f64) < count {
            let idx = n;
            n += 1;
            let face = quads.faces.get(idx).map_or(0, |f| *f as i64);
            let base = idx * 4;
            let (ox, oy, l, level) = (quads.offsets[base], quads.offsets[base + 1], quads.offsets[base + 2], quads.offsets[base + 3]);
            if !(ox.is_finite() && oy.is_finite() && l.is_finite() && l > 0.0) {
                continue;
            }
            let Some(face) = usize::try_from(face).ok().filter(|f| *f < 6) else {
                continue;
            };
            if !(self.reach_u[face] > 0.0 && self.reach_v[face] > 0.0) {
                continue;
            }
            let cam = self.cam[face];
            if (ox + l * 0.5 - cam[0]).abs() > self.reach_u[face] + l * 0.5 {
                continue;
            }
            if (oy + l * 0.5 - cam[1]).abs() > self.reach_v[face] + l * 0.5 {
                continue;
            }
            if let Some(fp) = self.footprint(face, ox, oy, l, level) {
                near.push(fp);
            }
        }
        self.synced_quads = near.len();
        if near != self.indexed {
            self.indexed = near;
            self.rebuild_near_index();
        }
    }

    fn ensure_grid(&mut self, g: i64) {
        if self.grid_size == g {
            return;
        }
        self.grid_size = g;
        self.built.clear();
        self.built_order.clear();
    }

    fn footprint(&self, face: usize, ox: f64, oy: f64, l: f64, level: f64) -> Option<Footprint> {
        let r = self.frame.radius;
        let (e, nn) = (self.frame.east, self.frame.north);
        let mut cx = [0.0; 4];
        let mut cz = [0.0; 4];
        let (mut min_x, mut max_x) = (f64::INFINITY, f64::NEG_INFINITY);
        let (mut min_z, mut max_z) = (f64::INFINITY, f64::NEG_INFINITY);
        for c in 0..4 {
            let d = deform_dir(face, ox + QUAD_CORNER_U[c] * l, oy + QUAD_CORNER_V[c] * l, r)?;
            let w = [d[0] * r, d[1] * r, d[2] * r];
            let px = w[0] * e[0] + w[1] * e[1] + w[2] * e[2];
            let pz = w[0] * nn[0] + w[1] * nn[1] + w[2] * nn[2];
            cx[c] = px;
            cz[c] = pz;
            min_x = min_x.min(px);
            max_x = max_x.max(px);
            min_z = min_z.min(pz);
            max_z = max_z.max(pz);
        }
        let gx = (cx[0] + cx[1] + cx[2] + cx[3]) * 0.25;
        let gz = (cz[0] + cz[1] + cz[2] + cz[3]) * 0.25;
        for c in 0..4 {
            cx[c] = gx + (cx[c] - gx) * FOOTPRINT_BLOAT;
            cz[c] = gz + (cz[c] - gz) * FOOTPRINT_BLOAT;
        }
        Some(Footprint {
            face,
            ox,
            oy,
            l,
            level,
            cx,
            cz,
            min_x: min_x - FOOTPRINT_INFLATE_M,
            max_x: max_x + FOOTPRINT_INFLATE_M,
            min_z: min_z - FOOTPRINT_INFLATE_M,
            max_z: max_z + FOOTPRINT_INFLATE_M,
        })
    }

    fn rebuild_near_index(&mut self) {
        self.near_grid.clear();
        self.near_overflow.clear();
        if self.indexed.is_empty() {
            return;
        }
        let mut extents: Vec<f64> = self.indexed.iter().map(|n| (n.max_x - n.min_x).max(n.max_z - n.min_z)).collect();
        extents.sort_by(f64::total_cmp);
        self.near_cell_m = extents[extents.len() >> 1].max(1.0);
        let cell = self.near_cell_m;
        for k in 0..self.indexed.len() {
            let n = self.indexed[k];
            let (i0, i1) = ((n.min_x / cell).floor() as i64, (n.max_x / cell).floor() as i64);
            let (j0, j1) = ((n.min_z / cell).floor() as i64, (n.max_z / cell).floor() as i64);
            if i0 < -NEAR_GRID_KEY_BIAS
                || i1 >= NEAR_GRID_KEY_BIAS
                || j0 < -NEAR_GRID_KEY_BIAS
                || j1 >= NEAR_GRID_KEY_BIAS
                || (i1 - i0 + 1) * (j1 - j0 + 1) > NEAR_GRID_MAX_FAN
            {
                self.near_overflow.push(k);
                continue;
            }
            for i in i0..=i1 {
                for j in j0..=j1 {
                    self.near_grid.entry(grid_key(i, j)).or_default().push(k);
                }
            }
        }
    }

    fn lattice_height_at_dir(&self, d: V3) -> f64 {
        let Some(lattice) = self.lattice.as_deref() else {
            return f64::NAN;
        };
        if lattice.chart_epoch().is_some() && self.frame.chart_epoch != lattice.chart_epoch() {
            return f64::NAN;
        }
        let len = dot(d, d).sqrt();
        if !(len > 0.0) {
            return f64::NAN;
        }
        let de = dot(d, self.frame.east) / len;
        let du = dot(d, self.frame.up) / len;
        let dn = dot(d, self.frame.north) / len;
        if !(du.abs() > 1e-9) {
            return f64::NAN;
        }
        let mut rho = self.frame.radius + self.frame.anchor_height;
        for _ in 0..MESH_SOLVE_ITERS {
            let x = rho * de;
            let z = rho * dn;
            if !lattice.covers(x, z) {
                return f64::NAN;
            }
            let y = lattice.height_at_local(x, z);
            if !y.is_finite() {
                return f64::NAN;
            }
            let next = (y + self.base) / du;
            if !next.is_finite() {
                return f64::NAN;
            }
            let step = next - rho;
            rho = next;
            if step.abs() <= MESH_SOLVE_TOL_M {
                return rho - self.frame.radius;
            }
        }
        f64::NAN
    }

    fn build_quad(&self, face: usize, ox: f64, oy: f64, l: f64) -> Option<Quad> {
        let g = self.grid_size;
        let gf = g as f64;
        let n = (g + 1) as usize;
        let r = self.frame.radius;
        let (e, up, nn) = (self.frame.east, self.frame.up, self.frame.north);
        let mut pos = vec![0.0; n * n * 3];
        for j in 0..=g {
            let v = oy + (j as f64 / gf) * l;
            for i in 0..=g {
                let u = ox + (i as f64 / gf) * l;
                let dir = deform_dir(face, u, v, r)?;
                let mut h = self.lattice_height_at_dir(dir);
                if !h.is_finite() {
                    h = (self.height_at_dir)(dir);
                }
                if !h.is_finite() {
                    return None;
                }
                let s = r + h;
                let w = [dir[0] * s, dir[1] * s, dir[2] * s];
                let k = ((j as usize) * n + i as usize) * 3;
                pos[k] = w[0] * e[0] + w[1] * e[1] + w[2] * e[2];
                pos[k + 1] = w[0] * up[0] + w[1] * up[1] + w[2] * up[2] - self.base;
                pos[k + 2] = w[0] * nn[0] + w[1] * nn[1] + w[2] * nn[2];
            }
        }
        let gu = g as usize;
        let (x00, z00) = corner(&pos, n, 0, 0);
        let (x10, z10) = corner(&pos, n, gu, 0);
        let (x01, z01) = corner(&pos, n, 0, gu);
        let (x11, z11) = corner(&pos, n, gu, gu);
        let dxdi = ((x10 - x00) + (x11 - x01)) / (2.0 * gf);
        let dxdj = ((x01 - x00) + (x11 - x10)) / (2.0 * gf);
        let dzdi = ((z10 - z00) + (z11 - z01)) / (2.0 * gf);
        let dzdj = ((z01 - z00) + (z11 - z10)) / (2.0 * gf);
        let det = dxdi * dzdj - dxdj * dzdi;
        if !(det.abs() > 0.0) {
            return None;
        }
        Some(Quad {
            pos,
            i11: dzdj / det,
            i12: -dxdj / det,
            i21: -dzdi / det,
            i22: dxdi / det,
            x0: x00,
            z0: z00,
        })
    }

    fn materialize(&mut self, n: Footprint) -> Option<Rc<Quad>> {
        let key = QuadKey {
            face: n.face,
            ox: round_half_up(n.ox * 1000.0) as i64,
            oy: round_half_up(n.oy * 1000.0) as i64,
            level: n.level.to_bits(),
            epoch: self.frame.chart_epoch.map(f64::to_bits),
        };
        if let Some(hit) = self.built.get(&key) {
            return Some(Rc::clone(hit));
        }
        let q = Rc::new(self.build_quad(n.face, n.ox, n.oy, n.l)?);
        self.built.insert(key, Rc::clone(&q));
        self.built_order.push_back(key);
        if self.built.len() as f64 > self.max_quads {
            if let Some(old) = self.built_order.pop_front() {
                self.built.remove(&old);
            }
        }
        Some(q)
    }

    fn cell_y(&self, q: &Quad, i: i64, j: i64, x: f64, z: f64) -> Option<f64> {
        let g = self.grid_size;
        if i < 0 || j < 0 || i >= g || j >= g {
            return None;
        }
        let n = (g + 1) as usize;
        let (iu, ju) = (i as usize, j as usize);
        let a = (ju * n + iu) * 3;
        let b = (ju * n + iu + 1) * 3;
        let c = ((ju + 1) * n + iu) * 3;
        let d = ((ju + 1) * n + iu + 1) * 3;
        let p = &q.pos;
        if cell_flip(i + 1, j + 1) {
            tri_y(p, a, c, d, x, z).or_else(|| tri_y(p, a, d, b, x, z))
        } else {
            tri_y(p, a, c, b, x, z).or_else(|| tri_y(p, b, c, d, x, z))
        }
    }

    fn quad_y(&self, q: &Quad, x: f64, z: f64) -> Option<f64> {
        let dx = x - q.x0;
        let dz = z - q.z0;
        let fi = q.i11 * dx + q.i12 * dz;
        let fj = q.i21 * dx + q.i22 * dz;
        let i0 = fi.floor() as i64;
        let j0 = fj.floor() as i64;
        for j in (j0 - 1)..=(j0 + 1) {
            for i in (i0 - 1)..=(i0 + 1) {
                if let Some(y) = self.cell_y(q, i, j, x, z) {
                    return Some(y);
                }
            }
        }
        for j in 0..self.grid_size {
            for i in 0..self.grid_size {
                if let Some(y) = self.cell_y(q, i, j, x, z) {
                    return Some(y);
                }
            }
        }
        None
    }

    fn drawn_count(&self) -> usize {
        match (self.quads)() {
            Some(q) if q.count.is_finite() && q.count > 0.0 => (q.count as i64).min((q.offsets.len() / 4) as i64) as usize,
            _ => 0,
        }
    }

    fn ensure_coverage(&mut self, x: f64, z: f64) {
        if self.synced_quads > 0 {
            return;
        }
        if self.lazy_sync_version == Some(self.version) {
            return;
        }
        let drawn = self.drawn_count();
        if self.lazy_sync_drawn == Some(drawn) {
            return;
        }
        self.set_focus(x, z);
        self.sync();
        if self.synced_quads > 0 {
            self.lazy_sync_version = Some(self.version);
        } else {
            self.lazy_sync_drawn = Some(self.drawn_count());
        }
    }

    pub fn height_at_local(&mut self, x: f64, z: f64) -> Option<f64> {
        if !(x.is_finite() && z.is_finite()) {
            return None;
        }
        self.ensure_coverage(x, z);
        let cell = self.near_cell_m;
        let mut candidates: Vec<usize> = Vec::new();
        if cell > 0.0 {
            let ci = (x / cell).floor() as i64;
            let cj = (z / cell).floor() as i64;
            let in_range = ci > -NEAR_GRID_KEY_BIAS && ci < NEAR_GRID_KEY_BIAS && cj > -NEAR_GRID_KEY_BIAS && cj < NEAR_GRID_KEY_BIAS;
            let bucket = if in_range { self.near_grid.get(&grid_key(ci, cj)) } else { None };
            for &k in bucket.into_iter().flatten().chain(self.near_overflow.iter()) {
                let n = &self.indexed[k];
                if x < n.min_x || x > n.max_x || z < n.min_z || z > n.max_z {
                    continue;
                }
                if !inside_quad(n, x, z) {
                    continue;
                }
                candidates.push(k);
            }
        }
        if candidates.is_empty() {
            return None;
        }
        candidates.sort_by(|&a, &b| self.indexed[a].l.total_cmp(&self.indexed[b].l).then(a.cmp(&b)));
        for k in candidates {
            let n = self.indexed[k];
            let Some(q) = self.materialize(n) else {
                continue;
            };
            if let Some(y) = self.quad_y(&q, x, z) {
                return Some(y);
            }
        }
        None
    }
}

mod tests {
    use super::*;
    use serde_json::Value;
    use std::cell::RefCell;

    type Surface = TerrainMeshSurface<fn(V3) -> f64, Box<dyn Fn() -> Option<DrawnQuads>>>;

    fn fixture() -> Value {
        serde_json::from_str(include_str!("../testdata/parity/spoint-terrain-mesh-surface.json")).unwrap()
    }

    fn vec3(value: &Value) -> V3 {
        [value[0].as_f64().unwrap(), value[1].as_f64().unwrap(), value[2].as_f64().unwrap()]
    }

    fn frame_of(value: &Value) -> MeshFrame {
        MeshFrame {
            radius: value["radius"].as_f64().unwrap(),
            anchor_height: value["anchorHeight"].as_f64().unwrap(),
            offset_y: value["offsetY"].as_f64().unwrap(),
            up: vec3(&value["up"]),
            east: vec3(&value["east"]),
            north: vec3(&value["north"]),
            chart_epoch: value["chartEpoch"].as_f64(),
        }
    }

    fn numbers(value: &Value) -> Vec<f64> {
        value.as_array().unwrap().iter().map(|x| x.as_f64().unwrap()).collect()
    }

    fn payload_of(value: &Value) -> Option<DrawnQuads> {
        if value.is_null() {
            return None;
        }
        Some(DrawnQuads {
            count: value["count"].as_f64().unwrap(),
            grid_size: value["gridSize"].as_f64().unwrap(),
            offsets: numbers(&value["offsets"]),
            faces: numbers(&value["faces"]),
        })
    }

    fn height(d: V3) -> f64 {
        40.0 * d[0] * d[1] + 25.0 * d[2] * d[2] - 10.0 * d[0] + 3.0 * d[0] * d[1] * d[2] + 5.0
    }

    struct MockLattice {
        chart_epoch: Option<f64>,
    }

    impl MeshLattice for MockLattice {
        fn chart_epoch(&self) -> Option<f64> {
            self.chart_epoch
        }

        fn covers(&self, x: f64, z: f64) -> bool {
            x.abs() <= 500.0 && z.abs() <= 400.0
        }

        fn height_at_local(&self, x: f64, z: f64) -> f64 {
            0.5 + 0.002 * x - 0.001 * z + 1e-6 * x * z
        }
    }

    fn lattice_of(value: &Value) -> Option<Box<dyn MeshLattice>> {
        if value.is_null() {
            return None;
        }
        Some(Box::new(MockLattice { chart_epoch: value["chartEpoch"].as_f64() }))
    }

    fn surface_of(case: &Value, payloads: &[Option<DrawnQuads>]) -> (Result<Surface, String>, Rc<RefCell<Option<DrawnQuads>>>) {
        let cell = Rc::new(RefCell::new(payloads[case["quads"].as_u64().unwrap() as usize].clone()));
        let source = Rc::clone(&cell);
        let quads: Box<dyn Fn() -> Option<DrawnQuads>> = Box::new(move || source.borrow().clone());
        let opts = MeshOptions {
            frame: frame_of(&case["frame"]),
            height_at_dir: height as fn(V3) -> f64,
            quads,
            lattice: lattice_of(&case["lattice"]),
            focus_radius_m: case["focusRadiusM"].as_f64(),
            max_quads: case["maxQuads"].as_f64(),
        };
        (create_terrain_mesh_surface(opts), cell)
    }

    fn assert_value(got: Option<f64>, want: &Value, what: &str) {
        match (got, want.as_f64()) {
            (None, None) => {}
            (Some(g), Some(w)) => assert!((g - w).abs() <= 1e-9, "{what}: bevy {g} against spoint {w}"),
            (g, w) => panic!("{what}: spoint {w:?} against bevy {g:?}"),
        }
    }

    #[test]
    fn terrain_mesh_surface_matches_spoint() {
        let fixture = fixture();
        let payloads: Vec<Option<DrawnQuads>> = fixture["payloads"].as_array().unwrap().iter().map(payload_of).collect();
        for case in fixture["cases"].as_array().unwrap() {
            let name = case["name"].as_str().unwrap();
            let (surface, cell) = surface_of(case, &payloads);
            let mut surface = surface.unwrap_or_else(|e| panic!("{name}: {e}"));
            for step in case["steps"].as_array().unwrap() {
                match step["op"].as_str().unwrap() {
                    "query" => assert_value(surface.height_at_local(step["x"].as_f64().unwrap(), step["z"].as_f64().unwrap()), &step["expect"], name),
                    "setFocus" => surface.set_focus(step["x"].as_f64().unwrap(), step["z"].as_f64().unwrap()),
                    "sync" => surface.sync(),
                    "version" => assert_eq!(surface.version(), step["expect"].as_u64().unwrap(), "{name}: version"),
                    "setQuads" => *cell.borrow_mut() = payloads[step["index"].as_u64().unwrap() as usize].clone(),
                    other => panic!("unknown op {other}"),
                }
            }
        }
        for refusal in fixture["refusals"].as_array().unwrap() {
            let (result, _) = surface_of(refusal, &payloads);
            assert_eq!(result.err().as_deref(), refusal["error"].as_str(), "{}", refusal["name"]);
        }
        let render = &fixture["render"];
        let mut absent = RenderFrame { render_mesh_height_at_local: None };
        assert!(render_mesh_height_at(&mut absent).is_none());
        assert!(render["absentIsNull"].as_bool().unwrap());
        let (surface, _) = surface_of(&render["case"], &payloads);
        let boxed: Box<dyn FnMut(f64, f64) -> Option<f64>> = {
            let mut surface = surface.unwrap();
            Box::new(move |x, z| surface.height_at_local(x, z))
        };
        let mut present = RenderFrame { render_mesh_height_at_local: Some(boxed) };
        let probe = render_mesh_height_at(&mut present).expect("present frame returns its height function");
        for point in render["probes"].as_array().unwrap() {
            assert_value(probe(point["x"].as_f64().unwrap(), point["z"].as_f64().unwrap()), &point["expect"], "render probe");
        }
        assert!(fixture["sentinel"]["constant"].is_null());
        assert!(MESH_SURFACE_NO_GROUND.is_none());
        for item in fixture["sentinel"]["cases"].as_array().unwrap() {
            let value = item["value"].as_f64();
            assert_eq!(is_mesh_surface_no_ground(value), item["expect"].as_bool().unwrap(), "sentinel {value:?}");
        }
    }
}
