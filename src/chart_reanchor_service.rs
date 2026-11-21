
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::f64::consts::PI;

use crate::chart_transfer::{create_chart_transfer, ChartEpochTransfer, ChartSnapshot};
use crate::height::ops_jsgen::V3;
use crate::height::placement::PlacementLattice;
use crate::height::planet_frame::anchor_basis;

const QUARTER_TURN: f64 = PI / 2.0;
const CHUNK_M_SAFETY: f64 = 1.0 + 1e-12;
const RAD_TO_DEG: f64 = 180.0 / PI;
const SAME_ANCHOR_TOLERANCE: f64 = 1e-9;
const SAME_ANCHOR_SQ_TOLERANCE: f64 = 1e-18;

pub const CHART_ANCHORS_PER_FACE: f64 = 32.0;
pub const CHART_REANCHOR_HYSTERESIS_DEG: f64 = 0.75;
pub const DEFAULT_MAX_SLOPE_ANGLE_RAD: f64 = 0.7854;
pub const WALKABLE_CHART_LIMIT_DEG: f64 = DEFAULT_MAX_SLOPE_ANGLE_RAD * 180.0 / PI;

fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn norm(a: V3) -> f64 {
    dot(a, a).sqrt()
}

fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn angle_deg(up: V3, dir: V3) -> f64 {
    let length = norm(dir);
    let length = if length > 0.0 { length } else { 1.0 };
    (dot(dir, up) / length).clamp(-1.0, 1.0).acos() * RAD_TO_DEG
}

pub fn chart_anchor_lattice(radius: f64, anchors_per_face: f64) -> Result<PlacementLattice, String> {
    if !(radius.is_finite() && radius > 0.0) || !(anchors_per_face >= 1.0) {
        return Err(format!("chart anchor lattice needs a positive finite radius and anchorsPerFace >= 1, got radius {radius} and anchorsPerFace {anchors_per_face}"));
    }
    let chunk_m = (QUARTER_TURN * radius / anchors_per_face) * CHUNK_M_SAFETY;
    PlacementLattice::new(radius, chunk_m, 1.0)
}

fn chart_anchor_for_dir(lattice: &PlacementLattice, dir: V3) -> V3 {
    lattice.chunk_centre_dir(lattice.chunk_key_of_dir(dir))
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChartState {
    radius: f64,
    offset_y: f64,
    anchor_height: f64,
    chart_epoch: u32,
    up: V3,
    east: V3,
    north: V3,
}

impl ChartState {
    fn anchored(anchor_dir: V3, radius: f64, offset_y: f64, height: &dyn Fn(V3) -> f64) -> Self {
        let basis = anchor_basis(anchor_dir);
        Self {
            radius,
            offset_y,
            anchor_height: height(basis.up),
            chart_epoch: 0,
            up: basis.up,
            east: basis.east,
            north: basis.north,
        }
    }

    fn reanchor(&mut self, dir: V3, height: &dyn Fn(V3) -> f64) {
        let basis = anchor_basis(dir);
        self.up = basis.up;
        self.east = basis.east;
        self.north = basis.north;
        self.anchor_height = height(self.up);
        self.chart_epoch += 1;
    }

    pub fn snapshot(&self) -> ChartSnapshot {
        ChartSnapshot {
            radius: self.radius,
            offset_y: self.offset_y,
            anchor_height: self.anchor_height,
            east: self.east,
            up: self.up,
            north: self.north,
        }
    }

    pub fn chart_epoch(&self) -> u32 {
        self.chart_epoch
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Decision {
    Stay { reason: &'static str, gain_deg: Option<f64> },
    Refusal { reason: &'static str, worst_player_angle_deg: Option<f64> },
    Reanchor { anchor_dir: V3, centroid_dir: V3, gain_deg: f64, worst_player_angle_deg: f64 },
}

fn compare_dirs(a: &V3, b: &V3) -> Ordering {
    a[0].partial_cmp(&b[0]).unwrap_or(Ordering::Equal)
        .then(a[1].partial_cmp(&b[1]).unwrap_or(Ordering::Equal))
        .then(a[2].partial_cmp(&b[2]).unwrap_or(Ordering::Equal))
}

fn player_centroid_dir(dirs: &[V3]) -> Result<Option<V3>, &'static str> {
    if dirs.is_empty() {
        return Ok(None);
    }
    let mut unit_dirs: Vec<V3> = Vec::with_capacity(dirs.len());
    for dir in dirs {
        let length = norm(*dir);
        if !(length > 0.0) {
            return Err("player-direction-has-no-length");
        }
        unit_dirs.push([dir[0] / length, dir[1] / length, dir[2] / length]);
    }
    unit_dirs.sort_by(compare_dirs);
    let sum = unit_dirs.iter().fold([0.0; 3], |acc, d| [acc[0] + d[0], acc[1] + d[1], acc[2] + d[2]]);
    let length = norm(sum);
    if !(length > 0.0) {
        return Err("players-straddle-the-planet");
    }
    Ok(Some([sum[0] / length, sum[1] / length, sum[2] / length]))
}

fn chart_anchor_decision(lattice: &PlacementLattice, frame: &ChartState, dirs: &[V3], hysteresis_deg: f64, walkable_limit_deg: f64) -> Decision {
    let centroid_dir = match player_centroid_dir(dirs) {
        Ok(Some(centroid)) => centroid,
        Ok(None) => return Decision::Stay { reason: "no-players", gain_deg: None },
        Err(reason) => return Decision::Refusal { reason, worst_player_angle_deg: None },
    };
    let anchor_dir = chart_anchor_for_dir(lattice, centroid_dir);
    let worst_player_angle_deg = dirs.iter().fold(0.0_f64, |worst, dir| worst.max(angle_deg(anchor_dir, *dir)));
    if worst_player_angle_deg >= walkable_limit_deg {
        return Decision::Refusal { reason: "party-spread-exceeds-walkable-chart", worst_player_angle_deg: Some(worst_player_angle_deg) };
    }
    if norm(sub(anchor_dir, frame.up)) < SAME_ANCHOR_TOLERANCE {
        return Decision::Stay { reason: "anchor-unchanged", gain_deg: Some(0.0) };
    }
    let gain_deg = angle_deg(frame.up, centroid_dir) - angle_deg(anchor_dir, centroid_dir);
    if gain_deg < hysteresis_deg {
        return Decision::Stay { reason: "gain-below-hysteresis", gain_deg: Some(gain_deg) };
    }
    Decision::Reanchor { anchor_dir, centroid_dir, gain_deg, worst_player_angle_deg }
}

struct Shift {
    from: ChartSnapshot,
    to: ChartSnapshot,
    angle_before_deg: f64,
    angle_after_deg: f64,
    transfer: ChartEpochTransfer,
}

fn reanchor_chart_for(frame: &mut ChartState, lattice: &PlacementLattice, dir: V3, height: &dyn Fn(V3) -> f64) -> Option<Shift> {
    let target = chart_anchor_for_dir(lattice, dir);
    let delta = sub(target, frame.up);
    if dot(delta, delta) <= SAME_ANCHOR_SQ_TOLERANCE {
        return None;
    }
    let from = frame.snapshot();
    frame.reanchor(target, height);
    let to = frame.snapshot();
    let transfer = create_chart_transfer(&from, &to);
    Some(Shift {
        from,
        to,
        angle_before_deg: angle_deg(from.up, dir),
        angle_after_deg: angle_deg(to.up, dir),
        transfer,
    })
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChartReanchorEvent {
    pub from: ChartSnapshot,
    pub to: ChartSnapshot,
    pub anchor_dir: V3,
    pub angle_before_deg: f64,
    pub angle_after_deg: f64,
    pub transfer: ChartEpochTransfer,
    pub decision: Decision,
    pub epoch: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Refusal {
    pub reason: &'static str,
    pub epoch: u32,
    pub decision: Decision,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepError {
    NoStateMigrator,
}

struct TerrainMigrator {
    gate: Box<dyn Fn(&Decision, u32) -> Option<&'static str>>,
    migrate: Box<dyn Fn(&ChartReanchorEvent)>,
}

pub struct ChartReanchorService {
    lattice: PlacementLattice,
    frame: ChartState,
    sampler: Box<dyn Fn(V3) -> f64>,
    player_dirs: Box<dyn Fn() -> Vec<V3>>,
    hysteresis_deg: f64,
    migrators: Vec<Box<dyn Fn(&ChartReanchorEvent)>>,
    terrain_migrators: Vec<TerrainMigrator>,
    reanchor_count: u64,
    refusal_count: u64,
    last_refusal: Option<Refusal>,
    refusals_by_reason: BTreeMap<&'static str, u64>,
}

impl ChartReanchorService {
    pub fn new(radius: f64, offset_y: f64, anchor_dir: V3, sampler: impl Fn(V3) -> f64 + 'static, player_dirs: impl Fn() -> Vec<V3> + 'static) -> Result<Self, String> {
        let lattice = chart_anchor_lattice(radius, CHART_ANCHORS_PER_FACE)?;
        let frame = ChartState::anchored(anchor_dir, radius, offset_y, &sampler);
        Ok(Self {
            lattice,
            frame,
            sampler: Box::new(sampler),
            player_dirs: Box::new(player_dirs),
            hysteresis_deg: CHART_REANCHOR_HYSTERESIS_DEG,
            migrators: Vec::new(),
            terrain_migrators: Vec::new(),
            reanchor_count: 0,
            refusal_count: 0,
            last_refusal: None,
            refusals_by_reason: BTreeMap::new(),
        })
    }

    pub fn on_reanchor(&mut self, migrate: impl Fn(&ChartReanchorEvent) + 'static) {
        self.migrators.push(Box::new(migrate));
    }

    pub fn add_terrain_migrator(&mut self, gate: impl Fn(&Decision, u32) -> Option<&'static str> + 'static, migrate: impl Fn(&ChartReanchorEvent) + 'static) {
        self.terrain_migrators.push(TerrainMigrator { gate: Box::new(gate), migrate: Box::new(migrate) });
    }

    fn record_refusal(&mut self, reason: &'static str, decision: Decision) {
        self.refusal_count += 1;
        *self.refusals_by_reason.entry(reason).or_insert(0) += 1;
        self.last_refusal = Some(Refusal { reason, epoch: self.frame.chart_epoch, decision });
    }

    fn terrain_refusal(&self, decision: Decision) -> Option<&'static str> {
        self.terrain_migrators.iter().find_map(|m| (m.gate)(&decision, self.frame.chart_epoch))
    }

    pub fn step(&mut self) -> Result<Option<ChartReanchorEvent>, StepError> {
        if self.migrators.is_empty() {
            return Err(StepError::NoStateMigrator);
        }
        let dirs = (self.player_dirs)();
        let decision = chart_anchor_decision(&self.lattice, &self.frame, &dirs, self.hysteresis_deg, WALKABLE_CHART_LIMIT_DEG);
        let centroid_dir = match decision {
            Decision::Stay { .. } => return Ok(None),
            Decision::Refusal { reason, .. } => {
                self.record_refusal(reason, decision);
                return Ok(None);
            }
            Decision::Reanchor { centroid_dir, .. } => centroid_dir,
        };
        if let Some(reason) = self.terrain_refusal(decision) {
            self.record_refusal(reason, decision);
            return Ok(None);
        }
        let Some(shift) = reanchor_chart_for(&mut self.frame, &self.lattice, centroid_dir, self.sampler.as_ref()) else {
            return Ok(None);
        };
        self.reanchor_count += 1;
        let event = ChartReanchorEvent {
            from: shift.from,
            to: shift.to,
            anchor_dir: shift.to.up,
            angle_before_deg: shift.angle_before_deg,
            angle_after_deg: shift.angle_after_deg,
            transfer: shift.transfer,
            decision,
            epoch: self.frame.chart_epoch,
        };
        for terrain in &self.terrain_migrators {
            (terrain.migrate)(&event);
        }
        for migrate in &self.migrators {
            migrate(&event);
        }
        Ok(Some(event))
    }

    pub fn frame(&self) -> &ChartState {
        &self.frame
    }

    pub fn reanchor_count(&self) -> u64 {
        self.reanchor_count
    }

    pub fn refusal_count(&self) -> u64 {
        self.refusal_count
    }

    pub fn migrator_count(&self) -> usize {
        self.migrators.len()
    }

    pub fn last_refusal(&self) -> Option<&Refusal> {
        self.last_refusal.as_ref()
    }

    pub fn refusals_by_reason(&self) -> &BTreeMap<&'static str, u64> {
        &self.refusals_by_reason
    }
}

mod tests {
    use super::*;
    use crate::character::ChartTransfer;
    use serde_json::Value;
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    const TOLERANCE: f64 = 1e-9;

    fn fixture() -> Value {
        serde_json::from_str(include_str!("../testdata/parity/spoint-chart-reanchor-service.json")).unwrap()
    }

    fn vec3(v: &Value) -> V3 {
        [v[0].as_f64().unwrap(), v[1].as_f64().unwrap(), v[2].as_f64().unwrap()]
    }

    fn height_at(d: V3) -> f64 {
        0.5 + 0.25 * d[0] - 0.1 * d[2]
    }

    fn assert_close(got: f64, want: &Value, what: &str) {
        let want = want.as_f64().unwrap();
        assert!((got - want).abs() <= TOLERANCE, "{what}: {got} against spoint {want}");
    }

    fn assert_vec(got: V3, want: &Value, what: &str) {
        for k in 0..3 {
            assert_close(got[k], &want[k], what);
        }
    }

    fn assert_snapshot(got: &ChartSnapshot, want: &Value, what: &str) {
        assert_close(got.radius, &want["radius"], what);
        assert_close(got.offset_y, &want["offsetY"], what);
        assert_close(got.anchor_height, &want["anchorHeight"], what);
        assert_vec(got.east, &want["east"], what);
        assert_vec(got.up, &want["up"], what);
        assert_vec(got.north, &want["north"], what);
    }

    fn assert_event(event: &ChartReanchorEvent, want: &Value, what: &str) {
        assert_eq!(event.epoch as u64, want["epoch"].as_u64().unwrap(), "{what}: epoch");
        assert_eq!(want["decisionKind"].as_str(), Some("reanchor"), "{what}: decision kind");
        assert_vec(event.anchor_dir, &want["anchorDir"], what);
        assert_close(event.angle_before_deg, &want["angleBeforeDeg"], what);
        assert_close(event.angle_after_deg, &want["angleAfterDeg"], what);
        assert_close(event.transfer.tilt_rad(), &want["tiltRad"], what);
        assert_snapshot(&event.from, &want["from"], what);
        assert_snapshot(&event.to, &want["to"], what);
        match event.decision {
            Decision::Reanchor { centroid_dir, gain_deg, .. } => {
                assert_vec(centroid_dir, &want["centroidDir"], what);
                assert_close(gain_deg, &want["gainDeg"], what);
            }
            other => panic!("{what}: decision {other:?} against spoint reanchor"),
        }
    }

    fn assert_last_refusal(got: Option<&Refusal>, want: &Value, what: &str) {
        match (got, want.is_null()) {
            (None, true) => {}
            (Some(refusal), false) => {
                assert_eq!(refusal.reason, want["reason"].as_str().unwrap(), "{what}: refusal reason");
                assert_eq!(refusal.epoch as u64, want["epoch"].as_u64().unwrap(), "{what}: refusal epoch");
                if !want["worstPlayerAngleDeg"].is_null() {
                    let angle = match refusal.decision {
                        Decision::Refusal { worst_player_angle_deg: Some(angle), .. } => angle,
                        Decision::Reanchor { worst_player_angle_deg, .. } => worst_player_angle_deg,
                        other => panic!("{what}: refusal decision {other:?} against spoint worst angle"),
                    };
                    assert_close(angle, &want["worstPlayerAngleDeg"], what);
                }
            }
            (got, _) => panic!("{what}: last refusal {got:?} against spoint {want}"),
        }
    }

    fn assert_refusals(got: &BTreeMap<&'static str, u64>, want: &Value, what: &str) {
        let want = want.as_object().unwrap();
        assert_eq!(got.len(), want.len(), "{what}: refusal reasons");
        for (reason, count) in want {
            assert_eq!(got.get(reason.as_str()).copied(), count.as_u64(), "{what}: refusals for {reason}");
        }
    }

    #[test]
    fn chart_reanchor_service_matches_spoint_steps() {
        let fixture = fixture();
        assert_close(WALKABLE_CHART_LIMIT_DEG, &fixture["walkableLimitDeg"], "walkable limit");
        assert_close(CHART_REANCHOR_HYSTERESIS_DEG, &fixture["hysteresisDeg"], "hysteresis");
        assert_close(CHART_ANCHORS_PER_FACE, &fixture["anchorsPerFace"], "anchors per face");
        let gain_gate = fixture["gainGateDeg"].as_f64().unwrap();
        let players: Rc<RefCell<Vec<V3>>> = Rc::new(RefCell::new(Vec::new()));
        let source = players.clone();
        let mut service = ChartReanchorService::new(
            fixture["radius"].as_f64().unwrap(),
            fixture["offsetY"].as_f64().unwrap(),
            vec3(&fixture["anchorDir"]),
            height_at,
            move || source.borrow().clone(),
        )
        .unwrap();
        assert_snapshot(&service.frame().snapshot(), &fixture["initial"], "initial");
        assert_eq!(service.frame().chart_epoch() as u64, fixture["initial"]["chartEpoch"].as_u64().unwrap());
        let migrated = Rc::new(Cell::new(0_u64));
        let migrated_counter = migrated.clone();
        service.on_reanchor(move |_| migrated_counter.set(migrated_counter.get() + 1));
        let terrain_migrated = Rc::new(Cell::new(0_u64));
        let terrain_counter = terrain_migrated.clone();
        service.add_terrain_migrator(
            move |decision: &Decision, _epoch: u32| match decision {
                Decision::Reanchor { gain_deg, .. } if *gain_deg > gain_gate => Some("terrain-stream-busy"),
                _ => None,
            },
            move |_| terrain_counter.set(terrain_counter.get() + 1),
        );
        for step in fixture["steps"].as_array().unwrap() {
            let name = step["name"].as_str().unwrap();
            *players.borrow_mut() = step["dirs"].as_array().unwrap().iter().map(vec3).collect();
            let refusals_before = service.refusal_count();
            let result = service.step().unwrap();
            let refused_now = service.refusal_count() > refusals_before;
            match (&result, step["result"].is_null()) {
                (None, true) => {}
                (Some(event), false) => assert_event(event, &step["result"], name),
                _ => panic!("{name}: step result {result:?} against spoint {}", step["result"]),
            }
            assert_last_refusal(if refused_now { service.last_refusal() } else { None }, &step["lastRefusal"], name);
            assert_eq!(service.reanchor_count(), step["reanchorCount"].as_u64().unwrap(), "{name}: reanchorCount");
            assert_eq!(service.refusal_count(), step["refusalCount"].as_u64().unwrap(), "{name}: refusalCount");
            assert_eq!(service.migrator_count() as u64, step["migratorCount"].as_u64().unwrap(), "{name}: migratorCount");
            assert_eq!(migrated.get(), step["migrated"].as_u64().unwrap(), "{name}: migrated");
            assert_eq!(terrain_migrated.get(), step["terrainMigrated"].as_u64().unwrap(), "{name}: terrainMigrated");
            assert_refusals(service.refusals_by_reason(), &step["refusalsByReason"], name);
        }
    }

    #[test]
    fn chart_reanchor_service_without_migrator_refuses_to_step() {
        let fixture = fixture();
        let mut bare = ChartReanchorService::new(
            fixture["radius"].as_f64().unwrap(),
            fixture["offsetY"].as_f64().unwrap(),
            vec3(&fixture["anchorDir"]),
            height_at,
            || vec![[0.0, 1.0, 0.0]],
        )
        .unwrap();
        assert_eq!(bare.step(), Err(StepError::NoStateMigrator));
        assert!(fixture["bareError"].as_str().unwrap().contains("no registered state migrator"));
    }
}
