#!/usr/bin/env bash

set -euo pipefail

SRC=$(find ./src -name "*.rs" |xargs cat| wc -l)
LIB=$(find ./lib -name "*.rs" |xargs cat| wc -l)
DOC=$(find ./ -name "*.md" -not -path "./target/*" | xargs cat | wc -l)

echo "doc:   ${DOC}"
echo ""
echo "src:   ${SRC}"
echo "lib:   ${LIB}"
echo "total: $(expr ${SRC} + ${LIB})"
