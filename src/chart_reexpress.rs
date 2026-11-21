
use std::collections::HashSet;

use crate::character::ChartTransfer;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LookAngles {
    pub yaw: Option<f64>,
    pub pitch: Option<f64>,
}

pub struct ReexpressPass<'a, T: ChartTransfer> {
    transfer: &'a T,
    seen: HashSet<usize>,
}

fn look_angles<T: ChartTransfer>(transfer: &T, psi: f64, pitch: f64) -> (f64, f64) {
    let cp = pitch.cos();
    let forward = [psi.sin() * cp, pitch.sin(), psi.cos() * cp];
    let v = transfer.vec(forward);
    (v[0].atan2(v[2]), v[1].clamp(-1.0, 1.0).asin())
}

fn yaw_of_quat(q: [f64; 4]) -> f64 {
    2.0 * q[1].atan2(q[3])
}

fn set_yaw_quat(q: &mut [f64; 4], yaw: f64) {
    let half = yaw / 2.0;
    *q = [0.0, half.sin(), 0.0, half.cos()];
}

fn reexpress_push_slot<T: ChartTransfer>(transfer: &T, s: &mut [f64; 4]) {
    let a = transfer.vec([s[0], 0.0, s[1]]);
    let b = transfer.vec([s[2], 0.0, s[3]]);
    *s = [a[0], a[2], b[0], b[2]];
}

pub fn reexpress_yaw_quat<T: ChartTransfer>(transfer: &T, q: &mut [f64; 4]) {
    let (yaw, _) = look_angles(transfer, yaw_of_quat(*q), 0.0);
    set_yaw_quat(q, yaw);
}

pub fn reexpress_look<T: ChartTransfer>(transfer: &T, holder: &mut LookAngles) {
    let yaw = holder.yaw.filter(|v| v.is_finite());
    let pitch = holder.pitch.filter(|v| v.is_finite());
    if yaw.is_none() && pitch.is_none() {
        return;
    }
    let (new_yaw, new_pitch) = look_angles(transfer, yaw.unwrap_or(0.0), pitch.unwrap_or(0.0));
    if yaw.is_some() {
        holder.yaw = Some(new_yaw);
    }
    if pitch.is_some() {
        holder.pitch = Some(new_pitch);
    }
}

pub fn create_reexpress_pass<T: ChartTransfer>(transfer: &T) -> ReexpressPass<'_, T> {
    ReexpressPass { transfer, seen: HashSet::new() }
}

impl<'a, T: ChartTransfer> ReexpressPass<'a, T> {
    pub fn distinct_targets(&self) -> usize {
        self.seen.len()
    }

    fn first<X>(&mut self, target: &X) -> bool {
        self.seen.insert(target as *const X as usize)
    }

    pub fn point(&mut self, p: &mut [f64; 3]) {
        if self.first(&*p) {
            *p = self.transfer.point(*p);
        }
    }

    pub fn vector(&mut self, v: &mut [f64; 3]) {
        if self.first(&*v) {
            *v = self.transfer.vec(*v);
        }
    }

    pub fn yaw_rotation(&mut self, q: &mut [f64; 4]) {
        if self.first(&*q) {
            reexpress_yaw_quat(self.transfer, q);
        }
    }

    pub fn look(&mut self, holder: &mut LookAngles) {
        if self.first(&*holder) {
            reexpress_look(self.transfer, holder);
        }
    }

    pub fn clear<X>(&mut self, list: &mut Vec<X>) {
        if self.first(&*list) {
            list.clear();
        }
    }

    pub fn push_slot(&mut self, slot: &mut [f64; 4]) {
        if self.first(&*slot) {
            reexpress_push_slot(self.transfer, slot);
        }
    }
}

mod tests {
    use super::*;
    use crate::chart_transfer::{create_chart_transfer, fixture_snapshot};
    use serde_json::Value;

    const TOLERANCE: f64 = 1e-9;

    fn vec3(v: &Value) -> [f64; 3] {
        [v[0].as_f64().unwrap(), v[1].as_f64().unwrap(), v[2].as_f64().unwrap()]
    }

    fn quat(v: &Value) -> [f64; 4] {
        [v[0].as_f64().unwrap(), v[1].as_f64().unwrap(), v[2].as_f64().unwrap(), v[3].as_f64().unwrap()]
    }

    fn look_of(v: &Value) -> LookAngles {
        LookAngles { yaw: v["yaw"].as_f64(), pitch: v["pitch"].as_f64() }
    }

    fn values<T>(v: &Value, f: fn(&Value) -> T) -> Vec<T> {
        v.as_array().unwrap().iter().map(f).collect()
    }

    fn assert_close(got: &[f64], want: &[f64], what: &str) {
        assert_eq!(got.len(), want.len(), "{what}");
        for k in 0..got.len() {
            assert!((got[k] - want[k]).abs() <= TOLERANCE, "{what}: {got:?} against spoint {want:?}");
        }
    }

    fn assert_angle(got: Option<f64>, want: Option<f64>, what: &str) {
        match (got, want) {
            (Some(g), Some(w)) => assert!((g - w).abs() <= TOLERANCE, "{what}: {g} against spoint {w}"),
            (None, None) => {}
            _ => panic!("{what}: {got:?} against spoint {want:?}"),
        }
    }

    fn assert_look(got: LookAngles, want: &Value, what: &str) {
        assert_angle(got.yaw, want["yaw"].as_f64(), what);
        assert_angle(got.pitch, want["pitch"].as_f64(), what);
    }

    #[test]
    fn chart_reexpress_matches_spoint_chart_reexpress() {
        let fixture: Value = serde_json::from_str(include_str!("../testdata/parity/spoint-chart-reexpress.json")).unwrap();
        let transfer = create_chart_transfer(&fixture_snapshot(&fixture["from"]), &fixture_snapshot(&fixture["to"]));
        assert!((transfer.tilt_rad() - fixture["tiltRad"].as_f64().unwrap()).abs() <= TOLERANCE);

        for (input, want) in values(&fixture["yawQuat"]["inputs"], quat).iter().zip(values(&fixture["yawQuat"]["results"], quat)) {
            let mut q = *input;
            reexpress_yaw_quat(&transfer, &mut q);
            assert_close(&q, &want, "reexpress_yaw_quat");
        }

        for (input, want) in fixture["look"]["inputs"].as_array().unwrap().iter().zip(fixture["look"]["results"].as_array().unwrap()) {
            let mut holder = look_of(input);
            reexpress_look(&transfer, &mut holder);
            assert_look(holder, want, "reexpress_look");
        }

        let pass_fixture = &fixture["pass"];
        let mut pass = create_reexpress_pass(&transfer);

        let mut points = values(&pass_fixture["points"], vec3);
        points.iter_mut().for_each(|p| pass.point(p));
        for (got, want) in points.iter().zip(values(&pass_fixture["pointResults"], vec3)) {
            assert_close(got, &want, "pass point");
        }

        let mut vectors = values(&pass_fixture["vectors"], vec3);
        vectors.iter_mut().for_each(|v| pass.vector(v));
        for (got, want) in vectors.iter().zip(values(&pass_fixture["vectorResults"], vec3)) {
            assert_close(got, &want, "pass vector");
        }

        let mut quats = values(&pass_fixture["quats"], quat);
        quats.iter_mut().for_each(|q| pass.yaw_rotation(q));
        for (got, want) in quats.iter().zip(values(&pass_fixture["quatResults"], quat)) {
            assert_close(got, &want, "pass yaw rotation");
        }

        let mut holders = values(&pass_fixture["holders"], look_of);
        holders.iter_mut().for_each(|h| pass.look(h));
        for (got, want) in holders.iter().zip(pass_fixture["holderResults"].as_array().unwrap()) {
            assert_look(*got, want, "pass look");
        }

        let mut lists: Vec<Vec<f64>> = pass_fixture["lists"].as_array().unwrap().iter().map(|l| l.as_array().unwrap().iter().map(|x| x.as_f64().unwrap()).collect()).collect();
        lists.iter_mut().for_each(|l| pass.clear(l));
        for (got, want) in lists.iter().zip(pass_fixture["listLengths"].as_array().unwrap()) {
            assert_eq!(got.len() as u64, want.as_u64().unwrap(), "pass clear");
        }

        let mut slots = values(&pass_fixture["slots"], quat);
        slots.iter_mut().for_each(|s| pass.push_slot(s));
        for (got, want) in slots.iter().zip(values(&pass_fixture["slotResults"], quat)) {
            assert_close(got, &want, "pass push slot");
        }

        let mut repeated = vec3(&pass_fixture["repeatedPoint"]);
        pass.point(&mut repeated);
        pass.point(&mut repeated);
        assert_close(&repeated, &vec3(&pass_fixture["pointRepeat"]), "pass point repeat");
        assert_eq!(pass.distinct_targets() as u64, pass_fixture["distinctTargets"].as_u64().unwrap(), "distinct targets");
    }
}
