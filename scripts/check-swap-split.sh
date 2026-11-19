#!/usr/bin/env bash
set -u
repo=$(cd "$(dirname "$0")/.." && pwd)
. "$repo/scripts/lib/swap-owners.sh"
. "$repo/scripts/lib/build-rss.sh"
proc=$(mktemp -d)
before=$(mktemp)
after=$(mktemp)
tree=$(mktemp)
status() {
    mkdir -p "$proc/$2"
    printf 'Name:\t%s\nPid:\t%s\nVmSwap:\t%s kB\n' "$1" "$2" "$3" > "$proc/$2/status"
}
status alpha 100 0
status beta 200 0
status gamma 290 50
status shrink 400 500
mkdir -p "$proc/300"
printf 'Name:\tkthread\nPid:\t300\n' > "$proc/300/status"
PROC_ROOT="$proc" swap_owners_sample > "$before"
status alpha 100 0
status beta 200 300
status gamma 290 290
status shrink 400 100
status late 500 80
PROC_ROOT="$proc" swap_owners_sample > "$after"
printf '200\n500\n400\n' > "$tree"
split=$(swap_owners_growth_split "$before" "$after" "$tree")
treesize=$(swap_owners_tree_size "$tree")
rm -rf "$proc" "$before" "$after" "$tree"
sleep 30 &
child=$!
pids=$(build_tree_pids "$$")
kill "$child" 2>/dev/null
wait "$child" 2>/dev/null
tree_ok=false
printf '%s\n' "$pids" | grep -qx "$child" && tree_ok=true
[ "$split" = "380 240" ] && [ "$treesize" = "3" ] && $tree_ok && ok=true || ok=false
echo "{\"ok\":$ok,\"split\":\"$split\",\"treeSize\":$treesize,\"childInTree\":$tree_ok}"
$ok
