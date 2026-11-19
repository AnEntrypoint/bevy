swap_owners_sample() {
    awk 'FNR == 1 { pid = ""; name = "" } /^Pid:/ { pid = $2 } /^Name:/ { name = $2 } /^VmSwap:/ { if (pid != "" && $2 > 0) print pid, name, $2 }' "${PROC_ROOT:-/proc}"/[0-9]*/status 2>/dev/null
}

swap_owners_top() {
    awk '{ key = $1 " " $2; if (!(key in max) || $3 > max[key]) max[key] = $3 } END { for (key in max) { split(key, parts, " "); print max[key], parts[1], parts[2] } }' "$1" | sort -rn | head -n 5
}
