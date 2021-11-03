#!/usr/bin/env bash

set -euo pipefail

SRC=$(find ./src -name "*.rs" -print0 |xargs -0 grep -v '^$' | wc -l)
LIB=$(find ./lib -name "*.rs" -print0 |xargs -0 grep -v '^$' | wc -l)
BENCH=$(find ./benches -name "*.rs" -print0 | xargs -0 grep -v '^$' | wc -l)
DOC=$(find ./ -name "*.md" -not -path "./target/*" -print0 | xargs -0 grep -v '^$' | wc -l)
TESTS=$(find ./tests -name "*.rs" -print0 | xargs -0 grep -v '^$' | wc -l)

echo "src:   ${SRC}"
echo "lib:   ${LIB}"
echo "tests: ${TESTS}"
echo "bench: ${BENCH}"
echo "doc:   ${DOC}"
echo ""
echo "total: $(expr ${SRC} + ${LIB} + ${BENCH} + ${TESTS})"
