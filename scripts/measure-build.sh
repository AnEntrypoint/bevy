#!/usr/bin/env bash
set -u
repo=$(cd "$(dirname "$0")/.." && pwd)
label=$1
shift
record=${RECORD:-$repo/target/witness/build-measure.jsonl}
mkdir -p "$(dirname "$record")" "$repo/target/witness"
. "$repo/scripts/lib/build-rss.sh"
. "$repo/scripts/lib/swap-owners.sh"

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
mkdir -p /tmp/cargo-slots
slot_wait_start=$(date +%s.%N)
slot_tries=0
until flock -n /tmp/cargo-slots/1.lock true || flock -n /tmp/cargo-slots/2.lock true; do
    slot_tries=$((slot_tries + 1))
    if [ "$slot_tries" -ge 72000 ]; then
        echo "measure-build: no build slot free for 7200 s; not starting $label" >&2
        exit 75
    fi
    sleep 0.1
done
slot_wait=$(awk -v s="$slot_wait_start" -v e="$(date +%s.%N)" 'BEGIN { printf "%.2f", e - s }')
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
owners_file=$(mktemp)
tree_file=$(mktemp)

(
    until [ -s "$build_pid_file" ]; do
        sleep 0.1
    done
    build_pid=$(cat "$build_pid_file")
    owner_tick=0
    while :; do
        owner_tick=$((owner_tick + 1))
        [ $(((owner_tick - 1) % 4)) -eq 0 ] && swap_owners_sample >> "$owners_file"
        read -r kib linking < <(build_tree_sample "$build_pid")
        build_tree_pids "$build_pid" >> "$tree_file"
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
owners_before=$(mktemp)
swap_owners_sample > "$owners_before"
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
owners_after=$(mktemp)
swap_owners_sample > "$owners_after"
avail1=$(meminfo_kib MemAvailable)
swapfree1=$(meminfo_kib SwapFree)
peak_kib=$(cat "$peak_file")
link_samples=$(cat "$link_file")
rm -f "$peak_file" "$link_file" "$build_pid_file"
echo "$(date +%s.%N) $swap_in1 $swap_out1" >> "$trace_file"
read -r peak_in peak_out < <(awk 'NR > 1 { dt = $1 - pt; if (dt > 0) { di = ($2 - pi) * 4 / dt; dout = ($3 - po) * 4 / dt; if (di > mi) mi = di; if (dout > mo) mo = dout } } { pt = $1; pi = $2; po = $3 } END { printf "%d %d\n", mi + 0, mo + 0 }' "$trace_file")
rm -f "$trace_file"
owners=$(swap_owners_top "$owners_file" | awk '{ printf "%s(%s) %.1f MiB; ", $3, $2, $1 / 1024 }')
grow_tree_peak=$(swap_owners_tree_peak "$tree_file" "$owners_file")
rm -f "$owners_file"
growth=$(swap_owners_growth "$owners_before" "$owners_after" | awk '{ printf "%s(%s) %.1f MiB; ", $3, $2, $1 / 1024 }')
growth_total=$(swap_owners_growth_total "$owners_before" "$owners_after")
growth_top=$(swap_owners_growth "$owners_before" "$owners_after" | awk '{ s += $1 } END { printf "%d", s + 0 }')
sort -u -o "$tree_file" "$tree_file"
read -r grow_build grow_other < <(swap_owners_growth_split "$owners_before" "$owners_after" "$tree_file")
grow_tree=$(swap_owners_tree_size "$tree_file")
grow_net=$(swap_owners_net "$owners_before" "$owners_after")
rm -f "$tree_file"
rm -f "$owners_before" "$owners_after"

crates=$(printf '%s' "$json" | grep -o '"cratesCompiled":[0-9]*' | cut -d: -f2)
IFS='|' read -r nz rule < <(node "$repo/scripts/lib/swap-noisy.cjs" "${crates:-0}" "$((swap_in1 - swap_in0))" "$((swap_out1 - swap_out0))")
target_root=${CARGO_TARGET_DIR:-$repo/target}
timings=$(find "$target_root"/cargo-timings -name 'cargo-timing-*.html' -newer "$marker" 2>/dev/null | sort | tail -n 1)
rm -f "$marker"
if [ -n "$timings" ]; then
    cp "$timings" "$repo/target/witness/timings-$label.html"
fi

awk -v label="$label" -v rc="$rc" -v json="$json" -v peak="$peak_kib" -v si="$((swap_in1 - swap_in0))" -v so="$((swap_out1 - swap_out0))" \
    -v a0="$avail0" -v a1="$avail1" -v f0="$swapfree0" -v f1="$swapfree1" -v ls="$link_samples" -v gate="$gate" -v gateout="$so_now" -v slots="$slots" -v slotwait="$slot_wait" -v pin="$peak_in" -v pout="$peak_out" -v nz="$nz" -v rule="$rule" -v owners="$owners" -v growth="$growth" -v gtot="$growth_total" -v gtop="$growth_top" -v gbuild="$grow_build" -v gother="$grow_other" -v gtree="$grow_tree" -v gnet="$grow_net" -v gtpeak="$grow_tree_peak" -v timings="${timings##*/}" 'BEGIN {
    printf "{\"label\":\"%s\",\"rc\":%d,\"build\":%s,\"peakRssMiB\":%.1f,\"sampleS\":0.5,\"slotsAtStart\":\"%s\",\"slotWaitS\":%s,\"gateSwapInKiBps\":%d,\"gateSwapOutKiBps\":%d,\"linkS\":%.1f,\"swapInPages\":%d,\"swapOutPages\":%d,\"swapInMiB\":%.1f,\"swapOutMiB\":%.1f,\"memAvailStartMiB\":%.1f,\"memAvailEndMiB\":%.1f,\"swapFreeStartMiB\":%.1f,\"swapFreeEndMiB\":%.1f,\"rssScope\":\"build process tree\",\"swapScope\":\"host\",\"swapNoisy\":%s,\"swapRule\":\"%s\",\"swapOwnersTop\":\"%s\",\"swapOwnersGrowth\":\"%s\",\"swapOwnersGrowthMiB\":%.1f,\"swapOwnersTop5Pct\":%.1f,\"swapOwnersBuildPct\":%.1f,\"swapOwnersOtherPct\":%.1f,\"swapOwnersUnattributedPct\":%.1f,\"swapOwnersBuildMiB\":%.1f,\"swapOwnersOtherMiB\":%.1f,\"swapOwnersTreePids\":%d,\"swapOwnersNetMiB\":%.1f,\"swapExitFreedMiB\":%.1f,\"swapExitFreedPct\":%.1f,\"swapOwnersTreePeakMiB\":%.1f,\"swapAttribution\":\"net per pid, exit-freed residual, tree peak sampled at 2 s\",\"swapPeakInKiBps\":%d,\"swapPeakOutKiBps\":%d,\"timings\":\"%s\"}\n", label, rc, json, peak / 1024, slots, slotwait, gate, gateout, ls * 0.5, si, so, si * 4 / 1024, so * 4 / 1024, a0 / 1024, a1 / 1024, f0 / 1024, f1 / 1024, nz, rule, owners, growth, gtot / 1024, (so > 0 ? gtop * 100 / (so * 4) : 0), (so > 0 ? gbuild * 100 / (so * 4) : 0), (so > 0 ? gother * 100 / (so * 4) : 0), (so > 0 ? 100 - gbuild * 100 / (so * 4) - gother * 100 / (so * 4) : 0), gbuild / 1024, gother / 1024, gtree, gnet / 1024, ((so - si) * 4 / 1024) - gnet / 1024, (so > 0 ? (((so - si) * 4 / 1024) - gnet / 1024) * 100 / (so * 4 / 1024) : 0), gtpeak / 1024, pin, pout, timings
}' | tee -a "$record"
exit "$rc"
