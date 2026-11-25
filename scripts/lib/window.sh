cd /workspace/bevy || exit 2
lock=${TREE_LOCK:-/workspace/tmp/bevy/tree.lock}
if [ -z "$TREE_LOCK_HELD" ] && [ "$(readlink /proc/$$/fd/9 2>/dev/null)" != "$lock" ]; then
  exec 9>"$lock"
  flock 9
  export TREE_LOCK_HELD=1
fi
scratch=/workspace/tmp/bevy/window-$$
mkdir -p target/witness "$scratch"
overall=0
inconclusive=0
note() { echo "$@" | tee -a $log; }
selected() { [ -z "$ONLY" ] || [[ " $ONLY " == *" $1 "* ]]; }
run() {
  name=$1; shift
  selected "$name" || return 0
  timeout 1200 "$@" > target/witness/$name.out 2>&1 &
  pid=$!
  note "PID $name pid=$pid $(date -u +%T)Z"
  wait $pid; rc=$?
  if [ "$rc" -eq 77 ]; then inconclusive=1; elif [ "$rc" -ne 0 ]; then overall=1; fi
  note "WITNESS $name rc=$rc"
  [ "$rc" -eq 77 ] && note "INCONCLUSIVE $name: the host load was too high to measure; not passed and not failed"
  grep -a 'test result\|FAIL\|panicked\|differs\|differ from\|greenPercent\|sha256\|frozen\|ok   ' target/witness/$name.out | tail -n 14 | tee -a $log
}
mutant() {
  name=$1; file=$2; old=$3; new=$4; cmd=$5
  selected "$name" || return 0
  cp "$file" $scratch/$name.good
  python3 - "$file" "$old" "$new" <<'PY' || { note "MUTANT-APPLY-FAILED $name"; overall=1; cp $scratch/$name.good "$file"; return; }
import sys
p, o, n = sys.argv[1], sys.argv[2], sys.argv[3]
t = open(p, encoding='utf-8').read()
assert t.count(o) == 1, t.count(o)
open(p, 'w', encoding='utf-8').write(t.replace(o, n))
PY
  cmp -s "$file" $scratch/$name.good; note "MUTANT $name file-differs-rc=$?"
  timeout 1200 bash -c "$cmd" > target/witness/$name.out 2>&1 &
  pid=$!
  note "PID mutant-$name pid=$pid $(date -u +%T)Z"
  wait $pid; rc=$?
  cp $scratch/$name.good "$file"
  cmp "$file" $scratch/$name.good && note "RESTORED $name identical"
  evidence=$(grep -a -o 'FAILED\|FAIL \|differs from spoint\|centre carve elevation differs\|hashVersion 1 elevation differs\|reliefScale elevation differs\|carved elevation differs\|hpf elevation differs\|elevation differs\|census froze\|census kept\|60 census frames' target/witness/$name.out | sort | uniq -c | tr '\n' ' ')
  if [ "$rc" -ne 0 ] && [ -n "$evidence" ]; then note "DETECTED $name rc=$rc $evidence"; else note "NOT-DETECTED $name rc=$rc evidence=${evidence:-none}"; overall=1; fi
}
mutants() {
  while IFS=$'\t' read -r name file old new cmd; do mutant "$name" "$file" "$old" "$new" "$cmd"; done < "$1"
}
finish() {
  rm -rf "$scratch"
  note "WITNESS-SET overall=$overall inconclusive=$inconclusive $(date -u +%T)Z"
  [ "$overall" -eq 0 ] && [ "$inconclusive" -eq 1 ] && exit 77
  exit $overall
}
