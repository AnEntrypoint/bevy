use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};
use bevy::platform::time::Instant;

pub const SECTIONS: usize = 17;
pub const NAMES: [&str; SECTIONS] = ["plan_leaves", "start_tasks", "finish_tasks", "retire_patches", "publish_stats", "start_water", "finish_water", "update_params", "stream_vegetation", "start_builds", "finish_builds", "rebase", "pre_fixed", "update", "post_update", "after_propagate", "after_visibility"];

static NANOS: [AtomicU64; SECTIONS] = [const { AtomicU64::new(0) }; SECTIONS];
static MARK: AtomicU64 = AtomicU64::new(0);

pub fn timed<T>(section: usize, body: impl FnOnce() -> T) -> T {
    let start = Instant::now();
    let out = body();
    NANOS[section].fetch_add(start.elapsed().as_nanos() as u64, Ordering::Relaxed);
    out
}

pub fn totals_ms() -> [f64; SECTIONS] {
    NANOS.each_ref().map(|nanos| nanos.load(Ordering::Relaxed) as f64 / 1e6)
}

fn now_ns() -> u64 {
    static EPOCH: OnceLock<Instant> = OnceLock::new();
    EPOCH.get_or_init(Instant::now).elapsed().as_nanos() as u64
}

pub fn restart() {
    MARK.store(now_ns(), Ordering::Relaxed);
}

pub fn mark(section: usize) {
    let now = now_ns();
    let previous = MARK.swap(now, Ordering::Relaxed);
    NANOS[section].fetch_add(now - previous, Ordering::Relaxed);
}
