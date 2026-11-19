#!/usr/bin/env bash
set -u
repo=$(cd "$(dirname "$0")/.." && pwd)
. "$repo/scripts/lib/swap-owners.sh"
proc=$(mktemp -d)
before=$(mktemp)
after=$(mktemp)
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
got=$(swap_owners_growth "$before" "$after")
total=$(swap_owners_growth_total "$before" "$after")
rm -rf "$proc" "$before" "$after"
want='300 200 beta
240 290 gamma
80 500 late'
[ "$got" = "$want" ] && [ "$total" = "620" ] && ok=true || ok=false
echo "{\"ok\":$ok,\"growth\":\"$(printf '%s' "$got" | tr '\n' ';')\",\"total\":$total}"
$ok
