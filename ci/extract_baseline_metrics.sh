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

# shellcheck disable=SC2012
SIZE=$(ls -all target/release/vertex | awk '{print $5/1024.0}')

echo "[{\"name\": \"Baseline\", \"unit\": \"s\", \"value\": ${DURATION}}, {\"name\": \"Binary size\", \"unit\": \"KiB\", \"value\": ${SIZE}}]"
