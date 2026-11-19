swap_owners_sample() {
    awk 'FNR == 1 { pid = ""; name = "" } /^Pid:/ { pid = $2 } /^Name:/ { name = $2 } /^VmSwap:/ { if (pid != "" && $2 > 0) print pid, name, $2 }' "${PROC_ROOT:-/proc}"/[0-9]*/status 2>/dev/null
}

swap_owners_top() {
    awk '{ key = $1 " " $2; if (!(key in max) || $3 > max[key]) max[key] = $3 } END { for (key in max) { split(key, parts, " "); print max[key], parts[1], parts[2] } }' "$1" | sort -rn | head -n 5
}
swap_owners_growth() {
    awk 'FILENAME == ARGV[1] { before[$1] = $3; next } { after[$1] = $3; name[$1] = $2 } END { for (p in after) { d = after[p] - before[p]; if (d > 0) print d, p, name[p] } }' "$1" "$2" | sort -rn | head -n 5
}
swap_owners_growth_total() {
    awk 'FILENAME == ARGV[1] { before[$1] = $3; next } { after[$1] = $3 } END { for (p in after) { d = after[p] - before[p]; if (d > 0) sum += d } printf "%d\n", sum + 0 }' "$1" "$2"
}
swap_owners_growth_split() {
    awk 'FILENAME == ARGV[1] { tree[$1] = 1; next } FILENAME == ARGV[2] { before[$1] = $3; next } { d = $3 - before[$1]; if (d > 0) { if ($1 in tree) b += d; else o += d } } END { printf "%d %d\n", b + 0, o + 0 }' "$3" "$1" "$2"
}
