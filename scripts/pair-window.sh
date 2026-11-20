#!/usr/bin/env bash
cd "$(dirname "$0")/.." || exit 2
format="$1"
case "$format" in
  astc4x4|etc2) ;;
  *) echo "unknown format $format; expected astc4x4 or etc2" >&2; exit 2 ;;
esac
read -r window_s min_s < <(node -e "const b = require('./scripts/lib/pair-budget.cjs'); console.log(b.PAIR_WINDOW_S + ' ' + b.armBudgets(process.env).pairS)")
if [ "$2" = "--print-env" ]; then
  echo "WINDOW_S=$window_s"
  echo "MIN_REMAINING_S=$min_s"
  exit 0
fi
WINDOW_S=$window_s MIN_REMAINING_S=$min_s exec bash scripts/window-job.sh -- node scripts/measure-astc-pair.cjs "$format"
