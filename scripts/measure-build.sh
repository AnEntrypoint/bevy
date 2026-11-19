#!/usr/bin/env bash
set -u
repo=$(cd "$(dirname "$0")/.." && pwd)
label=$1
shift
record=${RECORD:-$repo/target/witness/build-measure.jsonl}
mkdir -p "$(dirname "$record")" "$repo/target/witness"
. "$repo/scripts/lib/build-rss.sh"

vmstat_value() {
    awk -v key="$1" '$1 == key {print $2}' /proc/vmstat
}

meminfo_kib() {
    awk -v key="$1:" '$1 == key {print $2}' /proc/meminfo
}

swap_rates() {
    local window=${GATE_WINDOW_S:-0.5} i0 o0 i1 o1
    read -r i0 o0 < <(awk '$1 == "pswpin" {i = $2} $1 == "pswpout" {o = $2} END {print i, o}' /proc/vmstat)
    sleep "$window"
    read -r i1 o1 < <(awk '$1 == "pswpin" {i = $2} $1 == "pswpout" {o = $2} END {print i, o}' /proc/vmstat)
    awk -v i0="$i0" -v i1="$i1" -v o0="$o0" -v o1="$o1" -v w="$window" 'BEGIN { printf "%d %d\n", (i1 - i0) * 4 / w, (o1 - o0) * 4 / w }'
}

waited=0
while :; do
    read -r gate so_now < <(swap_rates)
    [ "$gate" -le 1000 ] && [ "$so_now" -le 1000 ] && break
    waited=$((waited + 1))
    if [ "$waited" -ge 1080 ]; then
        echo "measure-build: swap-in ${gate} and swap-out ${so_now} KiB/s for 540 s; not starting $label" >&2
        exit 75
    fi
done
echo "measure-build: $label gate swap-in ${gate} KiB/s, swap-out ${so_now} KiB/s" >&2
slots=""
for s in 1 2; do if flock -n /tmp/cargo-slots/$s.lock -c true; then slots="$slots slot$s=free"; else slots="$slots slot$s=busy"; fi; done
slots=${slots# }

marker=$(mktemp)
swap_in0=$(vmstat_value pswpin)
swap_out0=$(vmstat_value pswpout)
avail0=$(meminfo_kib MemAvailable)
swapfree0=$(meminfo_kib SwapFree)
peak_file=$(mktemp)
link_file=$(mktemp)
echo 0 > "$peak_file"
echo 0 > "$link_file"
trace_file=$(mktemp)
echo "$(date +%s.%N) $swap_in0 $swap_out0" > "$trace_file"
build_pid_file=$(mktemp)

(
    until [ -s "$build_pid_file" ]; do
        sleep 0.1
    done
    build_pid=$(cat "$build_pid_file")
    while :; do
        read -r kib linking < <(build_tree_sample "$build_pid")
        current=$(cat "$peak_file")
        [ "$kib" -gt "$current" ] && echo "$kib" > "$peak_file"
        if [ "$linking" -eq 1 ]; then
            echo $(( $(cat "$link_file") + 1 )) > "$link_file"
        fi
        echo "$(date +%s.%N) $(awk '$1 == "pswpin" {i = $2} $1 == "pswpout" {o = $2} END {print i, o}' /proc/vmstat)" >> "$trace_file"
        sleep 0.5
    done
) &
sampler=$!

json_file=$(mktemp)
"$repo/scripts/build-timed.sh" "$@" > "$json_file" &
build_pid=$!
echo "$build_pid" > "$build_pid_file"
wait "$build_pid"
rc=$?
json=$(cat "$json_file")
rm -f "$json_file"

for child in $(ps -o pid= --ppid "$sampler"); do
    kill "$child" 2>/dev/null
done
kill "$sampler" 2>/dev/null
wait "$sampler" 2>/dev/null

swap_in1=$(vmstat_value pswpin)
swap_out1=$(vmstat_value pswpout)
avail1=$(meminfo_kib MemAvailable)
swapfree1=$(meminfo_kib SwapFree)
peak_kib=$(cat "$peak_file")
link_samples=$(cat "$link_file")
rm -f "$peak_file" "$link_file" "$build_pid_file"
echo "$(date +%s.%N) $swap_in1 $swap_out1" >> "$trace_file"
read -r peak_in peak_out < <(awk 'NR > 1 { dt = $1 - pt; if (dt > 0) { di = ($2 - pi) * 4 / dt; dout = ($3 - po) * 4 / dt; if (di > mi) mi = di; if (dout > mo) mo = dout } } { pt = $1; pi = $2; po = $3 } END { printf "%d %d\n", mi + 0, mo + 0 }' "$trace_file")
rm -f "$trace_file"

target_root=${CARGO_TARGET_DIR:-$repo/target}
timings=$(find "$target_root"/cargo-timings -name 'cargo-timing-*.html' -newer "$marker" 2>/dev/null | sort | tail -n 1)
rm -f "$marker"
if [ -n "$timings" ]; then
    cp "$timings" "$repo/target/witness/timings-$label.html"
fi

awk -v label="$label" -v rc="$rc" -v json="$json" -v peak="$peak_kib" -v si="$((swap_in1 - swap_in0))" -v so="$((swap_out1 - swap_out0))" \
    -v a0="$avail0" -v a1="$avail1" -v f0="$swapfree0" -v f1="$swapfree1" -v ls="$link_samples" -v gate="$gate" -v gateout="$so_now" -v slots="$slots" -v pin="$peak_in" -v pout="$peak_out" -v nm="${NOISY_MIB:-10}" -v timings="${timings##*/}" 'BEGIN {
    nz = (si * 4 / 1024 > nm || so * 4 / 1024 > nm) ? "true" : "false"
    printf "{\"label\":\"%s\",\"rc\":%d,\"build\":%s,\"peakRssMiB\":%.1f,\"sampleS\":0.5,\"slotsAtStart\":\"%s\",\"gateSwapInKiBps\":%d,\"gateSwapOutKiBps\":%d,\"linkS\":%.1f,\"swapInPages\":%d,\"swapOutPages\":%d,\"swapInMiB\":%.1f,\"swapOutMiB\":%.1f,\"memAvailStartMiB\":%.1f,\"memAvailEndMiB\":%.1f,\"swapFreeStartMiB\":%.1f,\"swapFreeEndMiB\":%.1f,\"rssScope\":\"build process tree\",\"swapScope\":\"host\",\"swapNoisy\":%s,\"swapPeakInKiBps\":%d,\"swapPeakOutKiBps\":%d,\"timings\":\"%s\"}\n", label, rc, json, peak / 1024, slots, gate, gateout, ls * 0.5, si, so, si * 4 / 1024, so * 4 / 1024, a0 / 1024, a1 / 1024, f0 / 1024, f1 / 1024, nz, pin, pout, timings
}' | tee -a "$record"
exit "$rc"
