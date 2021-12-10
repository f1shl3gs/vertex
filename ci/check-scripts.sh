#!/usr/bin/env bash

set -euo pipefail

FILES=()
while IFS='' read -r LINE; do FILES+=("$LINE"); done < <(git ls-files | grep '\.sh')

shellcheck --external-sources --shell bash "${FILES[@]}"