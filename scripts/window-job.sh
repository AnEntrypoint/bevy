#!/usr/bin/env bash
cd "$(dirname "$0")/.." || exit 2
WINDOW_S=${WINDOW_S:-600}
INTERVAL_S=${INTERVAL_S:-30}
GATE_KB=${GATE_KB:-1000}
LOAD_MAX=${LOAD_MAX:-4}
MIN_REMAINING_S=${MIN_REMAINING_S:-0}
LOG=${WINDOW_LOG:-target/witness/window-job.log}
VMSTAT_CMD=${VMSTAT_CMD:-vmstat 1 2}
HEAVY_CMD=${HEAVY_CMD:-node scripts/heavy-jobs.cjs}
LOADAVG_FILE=${LOADAVG_FILE:-/proc/loadavg}
mkdir -p "$(dirname "$LOG")"
: > "$LOG"
if [ "$#" -gt 0 ] && [ "$1" = "--" ]; then shift; fi
deadline=$(( $(date +%s) + WINDOW_S ))

sample() {
  read -r si so < <($VMSTAT_CMD | tail -n 1 | awk '{print $7, $8}')
  load=$(cut -d' ' -f1 "$LOADAVG_FILE")
  heavy_out=$($HEAVY_CMD)
  heavy_rc=$?
  gate=closed
  if [ "$si" -lt "$GATE_KB" ] && [ "$so" -lt "$GATE_KB" ] && awk -v l="$load" -v m="$LOAD_MAX" 'BEGIN { exit !(l <= m) }'; then gate=open; fi
  heavy=clear
  if [ "$heavy_rc" -ne 0 ]; then heavy=busy; fi
  echo "reading $(date +%T) si=$si so=$so load1=$load gate=$gate heavy=$heavy remainS=$((deadline - $(date +%s))) $(echo "$heavy_out" | paste -sd ';' -)" | tee -a "$LOG"
  [ "$gate" = open ] && [ "$heavy" = clear ]
}

while :; do
  t=$(date +%s)
  if [ "$t" -ge "$deadline" ]; then
    echo "WINDOW-ENDED-WITHOUT-CLEAR $(date +%T)" | tee -a "$LOG"
    exit 3
  fi
  remaining=$((deadline - t))
  if [ "$remaining" -lt "$MIN_REMAINING_S" ]; then
    echo "START-REFUSED $(date +%T) remainS=$remaining min=$MIN_REMAINING_S" | tee -a "$LOG"
    exit 4
  fi
  if sample; then
    echo "GATE-CLEAR $(date +%T) remainS=$((deadline - $(date +%s)))" | tee -a "$LOG"
    if [ "$#" -eq 0 ]; then
      echo "END $(date +%T) no-witness" | tee -a "$LOG"
      exit 0
    fi
    "$@"
    rc=$?
    echo "WITNESS rc=$rc $(date +%T)" | tee -a "$LOG"
    echo "END $(date +%T) overall=$rc" | tee -a "$LOG"
    exit $rc
  fi
  wait_s=$(( INTERVAL_S - ($(date +%s) - t) ))
  if [ "$wait_s" -gt 0 ]; then sleep "$wait_s"; fi
done
