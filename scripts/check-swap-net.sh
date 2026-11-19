#!/usr/bin/env bash
set -u
repo=$(cd "$(dirname "$0")/.." && pwd)
. "$repo/scripts/lib/swap-owners.sh"
proc=$(mktemp -d)
before=$(mktemp)
after=$(mktemp)
tree=$(mktemp)
owners=$(mktemp)
status() {
    mkdir -p "$proc/$2"
    printf 'Name:\t%s\nPid:\t%s\nVmSwap:\t%s kB\n' "$1" "$2" "$3" > "$proc/$2/status"
}
status alpha 100 0
status beta 200 0
status gamma 290 50
status shrink 400 500
status exited 600 120
mkdir -p "$proc/300"
printf 'Name:\tkthread\nPid:\t300\n' > "$proc/300/status"
PROC_ROOT="$proc" swap_owners_sample > "$before"
status alpha 100 0
status beta 200 300
status gamma 290 290
status shrink 400 100
status late 500 80
rm -rf "$proc/600"
PROC_ROOT="$proc" swap_owners_sample > "$after"
printf '200\n500\n400\n' > "$tree"
printf '200 beta 50\n200 beta 300\n500 late 80\n400 shrink 500\n400 shrink 100\n290 gamma 290\n' > "$owners"
net=$(swap_owners_net "$before" "$after")
peak=$(swap_owners_tree_peak "$tree" "$owners")
rm -rf "$proc" "$before" "$after" "$tree" "$owners"
[ "$net" = "220" ] && [ "$peak" = "880" ] && ok=true || ok=false
echo "{\"ok\":$ok,\"net\":$net,\"treePeak\":$peak}"
$ok
