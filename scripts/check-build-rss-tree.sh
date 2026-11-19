#!/usr/bin/env bash
set -u
repo=$(cd "$(dirname "$0")/.." && pwd)
. "${BUILD_RSS_LIB:-$repo/scripts/lib/build-rss.sh}"
work=$(mktemp -d)
ln -s "$(command -v perl)" "$work/rustc"
rss_of() {
    awk '/VmRSS/ {print $2}' "/proc/$1/status" 2>/dev/null
}
outside_script='$x = "a" x (200 << 20); sleep 60'
inside_script='$x = "a" x (60 << 20); sleep 60'
"$work/rustc" -e "$outside_script" &
outside_pid=$!
bash -c '"$0" -e "$1" & wait' "$work/rustc" "$inside_script" &
root_pid=$!
sleep 3
inside_pid=$(ps -o pid= --ppid "$root_pid" | tr -d ' ' | head -n 1)
comm=$(cat "/proc/$outside_pid/comm")
outside_kib=$(rss_of "$outside_pid")
expected_kib=$(( $(rss_of "$root_pid") + $(rss_of "$inside_pid") ))
read -r tree_kib link < <(build_tree_sample "$root_pid")
kill "$outside_pid" "$inside_pid" "$root_pid" 2>/dev/null
wait 2>/dev/null
rm -rf "$work"
ok=true
[ "$comm" = rustc ] || ok=false
[ "$outside_kib" -ge 153600 ] || ok=false
[ $(( tree_kib - expected_kib )) -le 16384 ] && [ $(( expected_kib - tree_kib )) -le 16384 ] || ok=false
echo "{\"ok\":$ok,\"comm\":\"$comm\",\"outsideTreeRssKiB\":$outside_kib,\"insideTreeRssKiB\":$expected_kib,\"attributedRssKiB\":$tree_kib,\"linkProcesses\":$link}"
$ok
