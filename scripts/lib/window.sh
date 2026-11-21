cd /workspace/bevy || exit 2
scratch=/workspace/tmp/bevy/window
mkdir -p target/witness "$scratch"
overall=0
note() { echo "$@" | tee -a $log; }
run() {
  name=$1; shift
  timeout 1200 "$@" > target/witness/$name.out 2>&1 &
  pid=$!
  note "PID $name pid=$pid $(date -u +%T)Z"
  wait $pid; rc=$?
  [ "$rc" -ne 0 ] && overall=1
  note "WITNESS $name rc=$rc"
  grep -a 'test result\|FAIL\|panicked\|differs\|differ from\|greenPercent\|sha256\|frozen\|ok   ' target/witness/$name.out | tail -n 14 | tee -a $log
}
mutant() {
  name=$1; file=$2; old=$3; new=$4; cmd=$5
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
  note "WITNESS-SET overall=$overall $(date -u +%T)Z"
  exit $overall
}
