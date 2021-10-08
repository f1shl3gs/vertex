#!/usr/bin/env bash

set -euo pipefail

SRC=$(find ./src -name "*.rs" |xargs cat|grep -vc ^$$)
LIB=$(find ./lib -name "*.rs" |xargs cat|grep -vc ^$$)

echo "src:   ${SRC}"
echo "lib:   ${LIB}"
echo "total: $(expr ${SRC} + ${LIB})"
