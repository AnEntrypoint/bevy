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

waited=0
while :; do
    so=$(vmstat 1 2 | tail -n 1 | awk '{print $8}')
    [ "$so" -lt 1000 ] && break
    waited=$((waited + 1))
    if [ "$waited" -ge 540 ]; then
        echo "measure-build: swap-out stayed at ${so} KiB/s for 540 s; not starting $label" >&2
        exit 75
    fi
done

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

timings=$(find "$repo"/target/cargo-timings -name 'cargo-timing-*.html' -newer "$marker" 2>/dev/null | sort | tail -n 1)
rm -f "$marker"
if [ -n "$timings" ]; then
    cp "$timings" "$repo/target/witness/timings-$label.html"
fi

awk -v label="$label" -v rc="$rc" -v json="$json" -v peak="$peak_kib" -v si="$((swap_in1 - swap_in0))" -v so="$((swap_out1 - swap_out0))" \
    -v a0="$avail0" -v a1="$avail1" -v f0="$swapfree0" -v f1="$swapfree1" -v ls="$link_samples" -v timings="${timings##*/}" 'BEGIN {
    printf "{\"label\":\"%s\",\"rc\":%d,\"build\":%s,\"peakRssMiB\":%.1f,\"sampleS\":0.5,\"linkS\":%.1f,\"swapInPages\":%d,\"swapOutPages\":%d,\"swapInMiB\":%.1f,\"swapOutMiB\":%.1f,\"memAvailStartMiB\":%.1f,\"memAvailEndMiB\":%.1f,\"swapFreeStartMiB\":%.1f,\"swapFreeEndMiB\":%.1f,\"timings\":\"%s\"}\n", label, rc, json, peak / 1024, ls * 0.5, si, so, si * 4 / 1024, so * 4 / 1024, a0 / 1024, a1 / 1024, f0 / 1024, f1 / 1024, timings
}' | tee -a "$record"
exit "$rc"
