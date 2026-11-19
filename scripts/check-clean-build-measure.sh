#!/usr/bin/env bash
set -u
repo=$(cd "$(dirname "$0")/.." && pwd)
script="$repo/scripts/measure-clean-build.sh"
work=$(mktemp -d /tmp/check-clean-build.XXXXXX)
fail=0
report() {
  if [ "$2" = true ]; then
    echo "ok   $1"
  else
    echo "FAIL $1"
    fail=1
  fi
}
cat > "$work/fake-cargo" <<'EOF'
#!/usr/bin/env bash
printf '%s\n' "$*" > "$FAKE_LOG"
printf '%s\n' "$CARGO_TARGET_DIR" >> "$FAKE_LOG"
echo "   Compiling fake v0.1.0"
echo "    Finished dev profile [unoptimized + debuginfo] target(s) in 2.50s"
[ -n "$FAKE_RC" ] || FAKE_RC=0
exit $FAKE_RC
EOF
mkdir -p "$work/bin"
cat > "$work/bin/vmstat" <<'EOF'
#!/usr/bin/env bash
echo "procs -----------memory----------"
echo " r  b   swpd   free   buff  cache   si   so    bi    bo   in   cs us sy id wa st"
echo " 0  0      0 100000   1000 200000 $FAKE_SI $FAKE_SO     0     0  100   200  1  1 98  0  0"
echo " 0  0      0 100000   1000 200000 $FAKE_SI $FAKE_SO     0     0  100   200  1  1 98  0  0"
EOF
chmod +x "$work/fake-cargo" "$work/bin/vmstat"
export FAKE_LOG="$work/fake.log"
export CLEAN_BUILD_LOG_DIR="$work/logs"
out="$work/out.txt"
cfg=$(sed -n 's/^jobs = \([0-9][0-9]*\)$/\1/p' "$repo/.cargo/config.toml" | head -n 1)

MEASURE_NO_GATE=1 CARGO_BIN="$work/fake-cargo" bash "$script" --bogus > "$out" 2>/dev/null; rc=$?
t=false; [ $rc -eq 2 ] && [ ! -s "$out" ] && t=true; report "unknown argument exits 2 with no stdout" $t

MEASURE_NO_GATE=1 CARGO_BIN="$work/fake-cargo" bash "$script" --jobs 0 > "$out" 2>/dev/null; rc=$?
t=false; [ $rc -eq 2 ] && [ ! -s "$out" ] && t=true; report "jobs 0 exits 2" $t

MEASURE_NO_GATE=1 CARGO_BIN="$work/fake-cargo" bash "$script" --jobs x > "$out" 2>/dev/null; rc=$?
t=false; [ $rc -eq 2 ] && [ ! -s "$out" ] && t=true; report "jobs non-numeric exits 2" $t

MEASURE_NO_GATE=1 CARGO_BIN="$work/fake-cargo" bash "$script" --jobs 7 --target-root "$work/debug" > "$out" 2>/dev/null; rc=$?
t=false; [ $rc -eq 2 ] && [ ! -d "$work/debug" ] && t=true; report "profile directory as target root exits 2" $t

FAKE_SI=40000 FAKE_SO=0 PATH="$work/bin:$PATH" CARGO_BIN="$work/fake-cargo" bash "$script" --jobs 7 --target-root "$work/gate" > "$out" 2>/dev/null; rc=$?
t=false; [ $rc -eq 3 ] && [ ! -s "$out" ] && [ ! -e "$FAKE_LOG" ] && t=true; report "closed gate exits 3 before any cargo call" $t

MEASURE_NO_GATE=1 CARGO_BIN="$work/fake-cargo" FAKE_RC=0 bash "$script" --jobs 7 --target-root "$work/root" --keep > "$out" 2>/dev/null; rc=$?
lines=$(wc -l < "$out")
t=false; [ $rc -eq 0 ] && [ "$lines" -eq 1 ] && t=true; report "successful run prints exactly one line and exits 0" $t
pattern='^CLEAN_BUILD jobs=7 rc=0 wall_s=[0-9]+[.][0-9]+ cargo_finished_s=2[.]50 peak_load1=[0-9]+[.][0-9]+ gate_si=0 gate_so=0 load1_before=[0-9]+[.][0-9]+ target_root='
t=false; grep -Eq "$pattern" "$out" && t=true; report "result line has the fixed field order and values" $t
t=false; [ "$(sed -n 1p "$FAKE_LOG")" = "build --jobs 7" ] && [ "$(sed -n 2p "$FAKE_LOG")" = "$work/root" ] && t=true; report "cargo got build --jobs 7 with the target root" $t
t=false; [ -d "$work/root" ] && t=true; report "--keep keeps the target root" $t

MEASURE_NO_GATE=1 CARGO_BIN="$work/fake-cargo" bash "$script" --jobs 7 --target-root "$work/root2" > "$out" 2>/dev/null
t=false; [ ! -e "$work/root2" ] && t=true; report "without --keep the target root is removed" $t

MEASURE_NO_GATE=1 CARGO_BIN="$work/fake-cargo" bash "$script" --jobs 7 --target-root "$work/root5" > "$out" 2> "$work/err"
logpath=$(sed -n 's/^measure-clean-build: log //p' "$work/err")
t=false; [ -f "$logpath" ] && case "$logpath" in "$work/logs/"*) t=true ;; esac; report "the run log is kept in CLEAN_BUILD_LOG_DIR and named on stderr" $t
t=false; grep -Eq ' peak_tree_rss_mib=[0-9]+ mem_avail_before_mib=[0-9]+ cgroup_max_mib=(na|[0-9]+)$' "$out" && t=true; report "the result line ends with tree peak RSS, MemAvailable and the cgroup limit" $t
t=false; echo "$(basename "$logpath")" | grep -Eq '^[0-9]{8}-[0-9]{6}-[0-9]+[.]log$' && t=true; report "the run log name carries a timestamp and the PID" $t

MEASURE_NO_GATE=1 CARGO_BIN="$work/fake-cargo" bash "$script" --target-root "$work/root3" > "$out" 2>/dev/null
t=false; grep -Eq "^CLEAN_BUILD jobs=$cfg rc=0 " "$out" && t=true; report "default jobs comes from the repo .cargo/config.toml ($cfg)" $t

MEASURE_NO_GATE=1 CARGO_BIN="$work/fake-cargo" FAKE_RC=3 bash "$script" --jobs 7 --target-root "$work/root4" > "$out" 2>/dev/null; rc=$?
t=false; [ $rc -eq 3 ] && grep -Eq "^CLEAN_BUILD jobs=7 rc=3 " "$out" && t=true; report "a failing cargo is reported as rc=3 and exits 3" $t

rm -rf "$work"
exit $fail
