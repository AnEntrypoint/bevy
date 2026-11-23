use std::sync::atomic::{AtomicU64, Ordering};

pub const SECTIONS: usize = 12;
pub const NAMES: [&str; SECTIONS] = ["plan_leaves", "start_tasks", "finish_tasks", "retire_patches", "publish_stats", "start_water", "finish_water", "update_params", "stream_vegetation", "start_builds", "finish_builds", "rebase"];

static NANOS: [AtomicU64; SECTIONS] = [const { AtomicU64::new(0) }; SECTIONS];

pub fn timed<T>(section: usize, body: impl FnOnce() -> T) -> T {
    let start = std::time::Instant::now();
    let out = body();
    NANOS[section].fetch_add(start.elapsed().as_nanos() as u64, Ordering::Relaxed);
    out
}

pub fn totals_ms() -> [f64; SECTIONS] {
    NANOS.each_ref().map(|nanos| nanos.load(Ordering::Relaxed) as f64 / 1e6)
}
