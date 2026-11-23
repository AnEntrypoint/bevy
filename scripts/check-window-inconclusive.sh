#!/usr/bin/env bash
dir=/workspace/tmp/bevy/inconclusive-witness-$$
mkdir -p "$dir"
export TREE_LOCK="$dir/tree.lock"
unset TREE_LOCK_HELD
overall=0
cat > "$dir/part.sh" <<'EOF'
log=$PART_LOG
. /workspace/bevy/scripts/lib/window.sh
run stub bash -c "exit $STUB_RC"
finish
EOF
expect() {
  name=$1; rc=$2; want_exit=$3; want_line=$4
  STUB_RC=$rc PART_LOG="$dir/$name.log" bash "$dir/part.sh" > /dev/null 2>&1
  got=$?
  if [ "$got" -eq "$want_exit" ] && grep -q "$want_line" "$dir/$name.log"; then echo "WITNESS $name rc=0"; else echo "WITNESS $name FAILED exit=$got"; overall=1; fi
}
expect pass 0 0 'WITNESS-SET overall=0 inconclusive=0'
expect fail 1 1 'WITNESS-SET overall=1 inconclusive=0'
expect inconclusive 77 77 'WITNESS-SET overall=0 inconclusive=1'
grep -q 'INCONCLUSIVE stub' "$dir/inconclusive.log" && echo 'WITNESS says-inconclusive rc=0' || { echo 'WITNESS says-inconclusive FAILED'; overall=1; }
rm -rf "$dir"
echo "WITNESS-SET overall=$overall"
exit $overall
