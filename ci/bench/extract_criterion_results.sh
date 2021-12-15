#!/usr/bin/env bash

set -euo pipefail

OUTPUT=bench.json
CONTENT="["

IFS=$'\n\n';
for LINE in $(grep -v ignore bench.txt | grep time: | grep -v %); do
  [[ $LINE =~ ([a-zA-Z0-9/_]*).*\[[0-9.]*\ (us|ns|ms|s)\ ([0-9.]*)\ (us|ns|ms|s) ]]
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
  esac

  CONTENT="${CONTENT}{\"name\": \"${BENCH}\",\"unit\":\"ns/op\",\"value\": ${VALUE}},"
  # echo "${BENCH}: ${VALUE} ns/op"
done

echo "${CONTENT%,} ]" > ${OUTPUT}
