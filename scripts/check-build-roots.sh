#!/usr/bin/env bash
set -u
roots=$(ls -d /tmp/bevy-clean.* 2>/dev/null)
if [ -n "$roots" ]; then
  echo "$roots" | sed 's/^/stale build root: /'
  exit 1
fi
echo "no stale build roots under /tmp"
