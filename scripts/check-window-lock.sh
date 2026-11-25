#!/usr/bin/env bash
dir=/workspace/tmp/bevy/lock-witness-$$
mkdir -p "$dir"
export TREE_LOCK="$dir/tree.lock"
unset TREE_LOCK_HELD
overall=0
report() { echo "WITNESS $1 rc=$2"; [ "$2" -ne 0 ] && overall=1; }
cat > "$dir/child.sh" <<'EOF'
log=${CHILD_LOG:?}
. /workspace/bevy/scripts/lib/window.sh
finish
EOF
cat > "$dir/parent.sh" <<'EOF'
log=${PARENT_LOG:?}
. /workspace/bevy/scripts/lib/window.sh
$CHILD_PREFIX bash "$CHILD"
echo $? > "$RESULT"
finish
EOF
cat > "$dir/turn.sh" <<'EOF'
log=${TURN_LOG:?}
. /workspace/bevy/scripts/lib/window.sh
echo start $(date +%s%N) >> "$ORDER"
sleep 2
echo end $(date +%s%N) >> "$ORDER"
finish
EOF
export CHILD="$dir/child.sh" CHILD_LOG="$dir/child.log" PARENT_LOG="$dir/parent.log" RESULT="$dir/nested.result" CHILD_PREFIX=env
timeout 20 bash "$dir/parent.sh" > /dev/null 2>&1
report nested-run-finishes $?
[ "$(cat "$dir/nested.result" 2>/dev/null)" = 0 ]; report nested-child-ran $?
export ORDER="$dir/order.txt"
: > "$ORDER"
TURN_LOG="$dir/turn-a.log" bash "$dir/turn.sh" > /dev/null 2>&1 &
TURN_LOG="$dir/turn-b.log" bash "$dir/turn.sh" > /dev/null 2>&1 &
wait
[ "$(cut -d" " -f1 "$ORDER" | tr "\n" " ")" = "start end start end " ]; report two-runs-take-turns $?
rm -f "$dir/nested.result"
CHILD_PREFIX="env -u TREE_LOCK_HELD" timeout 20 bash "$dir/parent.sh" > /dev/null 2>&1
report nesting-without-flag-finishes $?
[ "$(cat "$dir/nested.result" 2>/dev/null)" = 0 ]; report nesting-without-flag-child-ran $?
cat > "$dir/old-window.sh" <<'EOF'
exec 9>"$TREE_LOCK"
flock 9
EOF
cat > "$dir/old-parent.sh" <<'EOF'
log=/dev/null
. /workspace/bevy/scripts/lib/window.sh
env -u TREE_LOCK_HELD timeout 6 bash "$DIR/old-window.sh"
echo $? > "$DIR/old.result"
EOF
DIR="$dir" TREE_LOCK="$TREE_LOCK" bash "$dir/old-parent.sh" > /dev/null 2>&1
[ "$(cat "$dir/old.result" 2>/dev/null)" = 124 ]; report old-nesting-still-deadlocks-red-check $?
rm -rf "$dir"
echo "WITNESS-SET overall=$overall"
exit $overall
