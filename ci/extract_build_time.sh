#!/usr/bin/env bash

set -euo pipefail

MATCHED=$(grep 'Finished release \[optimized] target(s)' build.txt | grep -oE "([0-9]+)")
DURATION=0
FACTOR=60
for VALUE in $MATCHED; do
  if [[ $FACTOR -ne 0 ]]; then
    DURATION=$((VALUE * FACTOR))
    FACTOR=0
  else
    DURATION=$((DURATION + VALUE))
  fi
done

echo "[{\"name\": \"Baseline\", \"unit\": \"s\", \"value\": ${DURATION}}]"