#!/usr/bin/env bash
cd /workspace/bevy || exit 2
want_open=$1; want_present=$2; want_cand=$3; want_blocked=$4
tsv=docs/parity-matrix.tsv
open=$(awk -F'\t' 'NR>1 && $6 !~ /not applicable/ && $6 !~ /^present$/ {n++} END{print n+0}' $tsv)
doc=$(grep -o 'Totals: open [0-9]*' docs/REMAINING_PARITY.md | grep -o '[0-9]*$')
present=$(awk -F'\t' 'NR>1 && $6=="present" {n++} END{print n+0}' $tsv)
blocked=$(awk -F'\t' 'NR>1 && $6=="blocked" {n++} END{print n+0}' $tsv)
cand=$(awk -F'\t' 'NR>1 && $6=="candidate-only" {n++} END{print n+0}' $tsv)
dups=$(cut -f1 $tsv | sort | uniq -d | wc -l)
echo "open_tsv=$open open_doc=$doc present=$present blocked=$blocked candidate=$cand dup_numbers=$dups"
[ "$open" = "$doc" ] && [ "$open" = "$want_open" ] && [ "$present" = "$want_present" ] && [ "$blocked" = "$want_blocked" ] && [ "$cand" = "$want_cand" ] && [ "$dups" = 0 ]
