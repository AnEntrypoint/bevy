#!/usr/bin/env bash
cd "$(dirname "$0")/.." || exit 2
WINDOW_S=${WINDOW_S:-600}
INTERVAL_S=${INTERVAL_S:-30}
GATE_KB=${GATE_KB:-1000}
CPU_IDLE_MIN=${CPU_IDLE_MIN:-70}
CPU_SAMPLE_S=${CPU_SAMPLE_S:-10}
CPU_STAT_FILE=${CPU_STAT_FILE:-/proc/stat}
HOT_CPU_PCT=${HOT_CPU_PCT:-30}
MON_INTERVAL_S=${MON_INTERVAL_S:-5}
NOISE_GUARD=${NOISE_GUARD:-1}
MIN_REMAINING_S=${MIN_REMAINING_S:-0}
LOG=${WINDOW_LOG:-target/witness/window-job.log}
NOISE_FILE="$LOG.noise"
VMSTAT_CMD=${VMSTAT_CMD:-vmstat 1 2}
HEAVY_CMD=${HEAVY_CMD:-node scripts/heavy-jobs.cjs}
mkdir -p "$(dirname "$LOG")"
: > "$LOG"
if [ "$#" -gt 0 ] && [ "$1" = "--" ]; then shift; fi
deadline=$(( $(date +%s) + WINDOW_S ))

cpu_totals() { awk '/^cpu /{ t = 0; for (i = 2; i <= NF; i++) t += $i; print $5 + $6, t }' "$CPU_STAT_FILE"; }

cpu_idle_pct() {
  read -r idle0 total0 < <(cpu_totals)
  sleep "$CPU_SAMPLE_S"
  read -r idle1 total1 < <(cpu_totals)
  awk -v di=$((idle1 - idle0)) -v dt=$((total1 - total0)) 'BEGIN { print (dt > 0) ? int(100 * di / dt) : 0 }'
}

monitor() {
  local bench=$1 prev="" hot pid cmd sid names now
  while kill -0 "$bench" 2>/dev/null; do
    sleep "$MON_INTERVAL_S"
    hot=$(top -b -n 2 -d 1 -w 200 | awk -v hot="$HOT_CPU_PCT" '/^ *PID/ { n++; next } n == 2 && $9 + 0 > hot { print $1, $12 }')
    names=""
    now=""
    while read -r pid cmd; do
      [ -n "$pid" ] || continue
      sid=$(ps -o sid= -p "$pid" 2>/dev/null | tr -d ' ')
      [ "$sid" = "$bench" ] && continue
      now="$now $pid"
      case " $prev " in *" $pid "*) names="$names $cmd($pid)" ;; esac
    done <<< "$hot"
    prev=$now
    if [ -n "$names" ]; then echo "NOISE $(date +%T)$names" | tee -a "$LOG" "$NOISE_FILE" > /dev/null; fi
  done
}

guarded() {
  if [ "$NOISE_GUARD" != 1 ]; then "$@"; return; fi
  : > "$NOISE_FILE"
  setsid "$@" &
  local bench=$! mon rc
  monitor "$bench" &
  mon=$!
  wait "$bench"
  rc=$?
  kill "$mon" 2>/dev/null
  wait "$mon" 2>/dev/null
  if [ -s "$NOISE_FILE" ]; then
    echo "INCONCLUSIVE $(date +%T): other processes above ${HOT_CPU_PCT}% CPU for two samples:$(sed 's/^NOISE [0-9:]*//' "$NOISE_FILE" | tr -s ' ' | sort -u | tr '
' ' ')" | tee -a "$LOG"
    [ "$rc" -eq 0 ] && rc=77
  fi
  return $rc
}

sample() {
  read -r si so < <($VMSTAT_CMD | tail -n 1 | awk '{print $7, $8}')
  idle=$(cpu_idle_pct)
  heavy_out=$($HEAVY_CMD)
  heavy_rc=$?
  gate=closed
  if [ "$si" -lt "$GATE_KB" ] && [ "$so" -lt "$GATE_KB" ] && [ "$idle" -ge "$CPU_IDLE_MIN" ]; then gate=open; fi
  heavy=clear
  if [ "$heavy_rc" -ne 0 ]; then heavy=busy; fi
  echo "reading $(date +%T) si=$si so=$so idle=$idle% gate=$gate heavy=$heavy remainS=$((deadline - $(date +%s))) $(echo "$heavy_out" | paste -sd ';' -)" | tee -a "$LOG"
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
    guarded "$@"
    rc=$?
    echo "WITNESS rc=$rc $(date +%T)" | tee -a "$LOG"
    echo "END $(date +%T) overall=$rc" | tee -a "$LOG"
    exit $rc
  fi
  wait_s=$(( INTERVAL_S - ($(date +%s) - t) ))
  if [ "$wait_s" -gt 0 ]; then sleep "$wait_s"; fi
done
