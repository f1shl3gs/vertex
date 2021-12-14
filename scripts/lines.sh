#!/usr/bin/env bash

set -euo pipefail

# benchs
BENCH=$(find ./ -name "*.rs" | grep benches | xargs cat | grep -vc '^$')

# docs
DOCS_LINES=$(find ./ -name "*.md" | grep -v target | xargs cat | wc -l)
DOCS_WORDS=$(find ./ -name "*.md" | grep -v target | xargs cat | wc -w)

SRC=$(find ./src -name "*.rs" -print0 |xargs -0 grep -v '^$' | wc -l)
LIB=$(find ./lib/*/src -name "*.rs" -print0 |xargs -0 grep -v '^$' | wc -l)
TESTS=$(find ./tests -name "*.rs" -print0 | xargs -0 grep -v '^$' | wc -l)

echo "src:   ${SRC}"
echo "lib:   ${LIB}"
echo "tests: ${TESTS}"
echo "bench: ${BENCH}"
echo "doc:   ${DOCS_LINES} (words: ${DOCS_WORDS})"
echo ""
# shellcheck disable=SC2003
echo "total: $(expr "${SRC}" + "${LIB}" + "${BENCH}" + "${TESTS}")"
