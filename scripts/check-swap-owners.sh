#!/usr/bin/env bash
set -u
repo=$(cd "$(dirname "$0")/.." && pwd)
. "$repo/scripts/lib/swap-owners.sh"
proc=$(mktemp -d)
samples=$(mktemp)
mkdir -p "$proc/100" "$proc/200" "$proc/290" "$proc/300"
printf 'Name:\talpha\nPid:\t100\nVmSwap:\t0 kB\n' > "$proc/100/status"
printf 'Name:\tbeta\nPid:\t200\nVmSwap:\t300 kB\n' > "$proc/200/status"
printf 'Name:\tgamma\nPid:\t290\nVmSwap:\t50 kB\n' > "$proc/290/status"
printf 'Name:\tkthread\nPid:\t300\n' > "$proc/300/status"
PROC_ROOT="$proc" swap_owners_sample >> "$samples"
printf 'Name:\tbeta\nPid:\t200\nVmSwap:\t120 kB\n' > "$proc/200/status"
printf 'Name:\tgamma\nPid:\t290\nVmSwap:\t290 kB\n' > "$proc/290/status"
PROC_ROOT="$proc" swap_owners_sample >> "$samples"
top=$(swap_owners_top "$samples")
rm -rf "$proc" "$samples"
first=$(printf '%s\n' "$top" | head -n 1)
lines=$(printf '%s\n' "$top" | grep -c .)
ok=true
[ "$first" = "300 200 beta" ] || ok=false
[ "$lines" -eq 2 ] || ok=false
echo "{\"ok\":$ok,\"top\":\"$(printf '%s' "$top" | tr '\n' ';')\"}"
$ok
