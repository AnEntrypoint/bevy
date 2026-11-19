#!/usr/bin/env bash
set -u
repo=$(cd "$(dirname "$0")/.." && pwd)
label=$1
shift
record=${RECORD:-$repo/target/witness/build-measure.jsonl}
mkdir -p "$(dirname "$record")" "$repo/target/witness"

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

(
    while :; do
        read -r kib linking < <(ps -eo rss=,comm= | awk '$2 ~ /^(rustc|cargo|cc1|cc1plus|collect2|ld|ld\.bfd|ld\.gold|ld\.lld|lld|rust-lld|mold|cc|gcc)$|^build-script-/ {total += $1} $2 ~ /^(collect2|ld|ld\.bfd|ld\.gold|ld\.lld|lld|rust-lld|mold)$/ {link = 1} END {print total + 0, link + 0}')
        current=$(cat "$peak_file")
        [ "$kib" -gt "$current" ] && echo "$kib" > "$peak_file"
        if [ "$linking" -eq 1 ]; then
            echo $(( $(cat "$link_file") + 1 )) > "$link_file"
        fi
        sleep 0.5
    done
) &
sampler=$!

json=$("$repo/scripts/build-timed.sh" "$@")
rc=$?

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
rm -f "$peak_file" "$link_file"

target_root=${CARGO_TARGET_DIR:-$repo/target}
timings=$(find "$target_root"/cargo-timings -name 'cargo-timing-*.html' -newer "$marker" 2>/dev/null | sort | tail -n 1)
rm -f "$marker"
if [ -n "$timings" ]; then
    cp "$timings" "$repo/target/witness/timings-$label.html"
fi

awk -v label="$label" -v rc="$rc" -v json="$json" -v peak="$peak_kib" -v si="$((swap_in1 - swap_in0))" -v so="$((swap_out1 - swap_out0))" \
    -v a0="$avail0" -v a1="$avail1" -v f0="$swapfree0" -v f1="$swapfree1" -v ls="$link_samples" -v gate="$gate" -v gateout="$so_now" -v slots="$slots" -v nm="${NOISY_MIB:-10}" -v timings="${timings##*/}" 'BEGIN {
    nz = (si * 4 / 1024 > nm || so * 4 / 1024 > nm) ? "true" : "false"
    printf "{\"label\":\"%s\",\"rc\":%d,\"build\":%s,\"peakRssMiB\":%.1f,\"sampleS\":0.5,\"slotsAtStart\":\"%s\",\"gateSwapInKiBps\":%d,\"gateSwapOutKiBps\":%d,\"linkS\":%.1f,\"swapInPages\":%d,\"swapOutPages\":%d,\"swapInMiB\":%.1f,\"swapOutMiB\":%.1f,\"memAvailStartMiB\":%.1f,\"memAvailEndMiB\":%.1f,\"swapFreeStartMiB\":%.1f,\"swapFreeEndMiB\":%.1f,\"swapNoisy\":%s,\"timings\":\"%s\"}\n", label, rc, json, peak / 1024, slots, gate, gateout, ls * 0.5, si, so, si * 4 / 1024, so * 4 / 1024, a0 / 1024, a1 / 1024, f0 / 1024, f1 / 1024, nz, timings
}' | tee -a "$record"
exit "$rc"
