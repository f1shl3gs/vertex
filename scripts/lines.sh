#!/usr/bin/env bash

set -euo pipefail

SRC=$(find ./src -name "*.rs" -print0 |xargs -0 cat| wc -l)
LIB=$(find ./lib -name "*.rs" -print0 |xargs -0 cat| wc -l)
DOC=$(find ./ -name "*.md" -not -path "./target/*" -print0 | xargs -0 cat | wc -l)

echo "doc:   ${DOC}"
echo ""
echo "src:   ${SRC}"
echo "lib:   ${LIB}"
echo "total: $(expr ${SRC} + ${LIB})"
