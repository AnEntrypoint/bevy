#!/usr/bin/env bash
cd /workspace/bevy || exit 2
dir=/workspace/tmp/bevy/cpu-guard-$$
mkdir -p "$dir"
overall=0
report() { echo "WITNESS $1 rc=$2"; [ "$2" -ne 0 ] && overall=1; }
spinners() { for _ in $(seq 1 "$1"); do yes > /dev/null & echo $!; done; }
stop() { kill "$@" 2>/dev/null; wait "$@" 2>/dev/null; }
printf '%s\n' 'procs x' '1 0 0 100 0 0 10 10 0 0 0 0 0 0 0 0' > "$dir/vmstat.txt"
export WINDOW_LOG="$dir/busy.log" HEAVY_CMD="echo HEAVY_JOBS clear" VMSTAT_CMD="cat $dir/vmstat.txt"

mapfile -t busy < <(spinners 6)
CPU_IDLE_MIN=70 CPU_SAMPLE_S=5 WINDOW_S=25 INTERVAL_S=3 NOISE_GUARD=0 bash scripts/window-job.sh -- true > /dev/null 2>&1
rc=$?
stop "${busy[@]}"
grep -q 'gate=closed' "$dir/busy.log" && [ "$rc" -eq 3 ]; report busy-host-refuses-start $?
echo "$(grep -c 'idle=' "$dir/busy.log") readings; last: $(grep 'idle=' "$dir/busy.log" | tail -1 | cut -c1-120)"

export WINDOW_LOG="$dir/noise.log"
mapfile -t noisy < <(spinners 1)
CPU_IDLE_MIN=0 CPU_SAMPLE_S=0 NOISE_GUARD=1 MON_INTERVAL_S=2 HOT_CPU_PCT=30 WINDOW_S=60 INTERVAL_S=1 bash scripts/window-job.sh -- sleep 14 > "$dir/noise.out" 2>&1
rc=$?
stop "${noisy[@]}"
grep -q "INCONCLUSIVE" "$dir/noise.out" && grep -q "yes(" "$dir/noise.out"; named=$?
[ "$rc" -eq 77 ] && [ "$named" -eq 0 ]; report competing-process-is-named-inconclusive $?
tail -2 "$dir/noise.out" | cut -c1-200

rm -rf "$dir"
echo "WITNESS-SET overall=$overall"
exit $overall
