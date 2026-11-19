build_tree_sample() {
    ps -eo pid=,ppid=,rss=,comm= | awk -v root="$1" '
        { pid[NR] = $1; ppid[NR] = $2; rss[NR] = $3; comm[NR] = $4 }
        END {
            tree[root] = 1
            grew = 1
            while (grew) {
                grew = 0
                for (i = 1; i <= NR; i++) if (!(pid[i] in tree) && (ppid[i] in tree)) { tree[pid[i]] = 1; grew = 1 }
            }
            for (i = 1; i <= NR; i++) if (pid[i] in tree) {
                total += rss[i]
                if (comm[i] ~ /^(collect2|ld|ld\.bfd|ld\.gold|ld\.lld|lld|rust-lld|mold)$/) link = 1
            }
            print total + 0, link + 0
        }'
}
