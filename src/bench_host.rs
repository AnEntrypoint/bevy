use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::Duration;

pub const IDLE_MIN: f64 = 0.70;
pub const COMPETITOR_CPU_PCT: f64 = 80.0;
pub const IDLE_WINDOW: Duration = Duration::from_secs(10);
pub const SAMPLE_INTERVAL: Duration = Duration::from_secs(5);
const POLL: Duration = Duration::from_millis(100);
const CLOCK_TICKS_PER_S: f64 = 100.0;

type Snapshot = HashMap<u32, (String, u64)>;

fn cpu_times(stat: &str) -> Option<(u64, u64)> {
    let fields: Vec<u64> = stat.lines().next()?.split_whitespace().skip(1).filter_map(|field| field.parse().ok()).collect();
    let idle = fields.get(3)? + fields.get(4).copied().unwrap_or(0);
    Some((idle, fields.iter().sum()))
}

fn idle_fraction(before: &str, after: &str) -> Option<f64> {
    let ((idle_before, total_before), (idle_after, total_after)) = (cpu_times(before)?, cpu_times(after)?);
    let total = total_after.checked_sub(total_before).filter(|total| *total > 0)?;
    Some(idle_after.saturating_sub(idle_before) as f64 / total as f64)
}

pub fn measure_idle() -> f64 {
    let read = || std::fs::read_to_string("/proc/stat").unwrap_or_default();
    let before = read();
    std::thread::sleep(IDLE_WINDOW);
    idle_fraction(&before, &read()).unwrap_or(1.0)
}

fn after_comm(stat: &str) -> Option<Vec<&str>> {
    let close = stat.rfind(')')?;
    Some(stat[close + 1..].split_whitespace().collect())
}

fn process_ticks(stat: &str) -> Option<(String, u64)> {
    let comm = stat[stat.find('(')? + 1..stat.rfind(')')?].to_string();
    let fields = after_comm(stat)?;
    Some((comm, fields.get(11)?.parse::<u64>().ok()? + fields.get(12)?.parse::<u64>().ok()?))
}

fn parent_of(stat: &str) -> Option<u32> {
    after_comm(stat)?.get(1)?.parse().ok()
}

fn snapshot() -> Snapshot {
    let Ok(entries) = std::fs::read_dir("/proc") else { return Snapshot::new() };
    entries
        .filter_map(|entry| {
            let pid: u32 = entry.ok()?.file_name().to_str()?.parse().ok()?;
            let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
            Some((pid, process_ticks(&stat)?))
        })
        .collect()
}

fn busy_processes(before: &Snapshot, after: &Snapshot, interval: Duration, ignored: &HashSet<u32>) -> Vec<(u32, String, f64)> {
    after
        .iter()
        .filter(|(pid, _)| !ignored.contains(*pid))
        .filter_map(|(pid, (comm, ticks))| {
            let (_, earlier) = before.get(pid)?;
            let percent = ticks.saturating_sub(*earlier) as f64 / CLOCK_TICKS_PER_S / interval.as_secs_f64() * 100.0;
            (percent > COMPETITOR_CPU_PCT).then(|| (*pid, comm.clone(), percent))
        })
        .collect()
}

fn persistent_competitor(previous: &HashSet<u32>, busy: &[(u32, String, f64)]) -> Option<String> {
    busy.iter().find(|(pid, ..)| previous.contains(pid)).map(|(pid, comm, percent)| format!("{comm} pid {pid} at {percent:.0}% cpu"))
}

pub fn unquiet(idle_before: f64, competitor: Option<&str>) -> Option<String> {
    if idle_before < IDLE_MIN {
        return Some(format!("host {:.0}% idle before the run, {:.0}% needed", idle_before * 100.0, IDLE_MIN * 100.0));
    }
    competitor.map(|competitor| format!("competing process {competitor}"))
}

fn watch(stop: &AtomicBool, interval: Duration) -> Option<String> {
    let own = std::process::id();
    let parent = std::fs::read_to_string("/proc/self/stat").ok().and_then(|stat| parent_of(&stat));
    let ignored: HashSet<u32> = [Some(own), parent].into_iter().flatten().collect();
    let (mut before, mut previous, mut found) = (snapshot(), HashSet::new(), None);
    while !stop.load(Ordering::SeqCst) {
        let mut waited = Duration::ZERO;
        while waited < interval && !stop.load(Ordering::SeqCst) {
            std::thread::sleep(POLL);
            waited += POLL;
        }
        if stop.load(Ordering::SeqCst) {
            break;
        }
        let after = snapshot();
        let busy = busy_processes(&before, &after, interval, &ignored);
        found = found.or_else(|| persistent_competitor(&previous, &busy));
        previous = busy.iter().map(|(pid, ..)| *pid).collect();
        before = after;
    }
    found
}

pub struct Watch {
    stop: Arc<AtomicBool>,
    thread: JoinHandle<Option<String>>,
}

impl Watch {
    pub fn start(interval: Duration) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let flag = stop.clone();
        Self { stop, thread: std::thread::spawn(move || watch(&flag, interval)) }
    }

    pub fn finish(self) -> Option<String> {
        self.stop.store(true, Ordering::SeqCst);
        self.thread.join().ok().flatten()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot_of(rows: &[(u32, &str, u64)]) -> Snapshot {
        rows.iter().map(|(pid, comm, ticks)| (*pid, (comm.to_string(), *ticks))).collect()
    }

    #[test]
    fn the_idle_share_is_the_idle_and_iowait_ticks_over_all_ticks_between_two_reads() {
        let before = "cpu  100 0 100 800 0 0 0 0 0 0\ncpu0 1 1 1 1 1 1 1 1 1 1\n";
        let after = "cpu  200 0 200 1000 0 0 0 0 0 0\ncpu0 2 2 2 2 2 2 2 2 2 2\n";
        assert_eq!(idle_fraction(before, after), Some(0.5));
        assert_eq!(idle_fraction(before, before), None);
    }

    #[test]
    fn a_process_line_with_brackets_in_its_name_still_gives_its_cpu_ticks_and_parent() {
        let stat = "1234 (my (odd) name) S 77 2 3 4 5 6 7 8 9 10 40 60 0 0";
        assert_eq!(process_ticks(stat), Some(("my (odd) name".to_string(), 100)));
        assert_eq!(parent_of(stat), Some(77));
    }

    #[test]
    fn only_a_process_above_the_cpu_limit_that_is_not_ignored_is_busy() {
        let before = snapshot_of(&[(1, "a", 0), (2, "b", 0), (3, "bench", 0), (4, "find", 0)]);
        let after = snapshot_of(&[(1, "a", 100), (2, "b", 450), (3, "bench", 900), (4, "find", 190)]);
        let ignored: HashSet<u32> = [3].into_iter().collect();
        let busy = busy_processes(&before, &after, Duration::from_secs(5), &ignored);
        assert_eq!(busy.iter().map(|(pid, comm, _)| (*pid, comm.as_str())).collect::<Vec<_>>(), vec![(2, "b")]);
    }

    #[test]
    fn a_competitor_needs_two_busy_samples_in_a_row_and_is_named() {
        let busy = vec![(2, "b".to_string(), 50.0)];
        assert_eq!(persistent_competitor(&HashSet::new(), &busy), None);
        let before: HashSet<u32> = [2].into_iter().collect();
        assert_eq!(persistent_competitor(&before, &busy), Some("b pid 2 at 50% cpu".to_string()));
    }

    #[test]
    fn a_busy_host_or_a_competing_process_makes_the_run_unquiet() {
        assert!(unquiet(0.5, None).is_some());
        assert!(unquiet(IDLE_MIN, None).is_none());
        assert!(unquiet(0.9, Some("cc pid 1 at 90% cpu")).is_some());
    }

    #[test]
    fn a_simulated_competing_busy_process_is_caught_and_named() {
        let mut child = std::process::Command::new("sh").args(["-c", "while :; do :; done"]).spawn().expect("the busy loop");
        let watch = Watch::start(Duration::from_secs(1));
        std::thread::sleep(Duration::from_millis(3500));
        child.kill().expect("stop the busy loop");
        child.wait().expect("reap the busy loop");
        let found = watch.finish().expect("the busy loop was not caught");
        assert!(found.starts_with("sh pid "), "{found}");
    }
}
