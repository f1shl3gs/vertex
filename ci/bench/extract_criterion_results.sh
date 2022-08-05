#!/usr/bin/env bash

set -euo pipefail

OUTPUT=bench.json
CONTENT="["

while IFS= read -r LINE; do
  [[ $LINE =~ ([a-zA-Z0-9/_\/\-]*).*\[[0-9.]*\ (µs|us|ns|ms|s)\ ([0-9.]*)\ (µs|us|ns|ms|s) ]]
  BENCH=${BASH_REMATCH[1]}
  VALUE=${BASH_REMATCH[3]}
  UNIT=${BASH_REMATCH[4]}

  # convert to ns
  case $UNIT in
  s)
    VALUE=$(echo "${VALUE} * 1000.0 * 1000.0 * 1000.0" | bc -l)
    ;;
  ms)
    VALUE=$(echo "${VALUE} * 1000.0 * 1000.0" | bc -l)
    ;;
  us)
    VALUE=$(echo "${VALUE} * 1000.0" | bc -l)
    ;;
  µs)
    VALUE=$(echo "${VALUE} * 1000.0" | bc -l)
    ;;
  esac

  CONTENT="${CONTENT}{\"name\": \"${BENCH}\",\"unit\":\"ns/op\",\"value\": ${VALUE}},"
done < <(grep -v ignore bench.txt | grep time: | grep -v %)

echo "${CONTENT%,} ]" > ${OUTPUT}
