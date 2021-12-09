#!/usr/bin/env bash

set -euo pipefail

mkdir -p .cargo

cat <<-EOF >> ./.cargo/config

[build]
# On the CI, where this script runs, we won't be caching build artifacts.
# So we don't need to keep these around.
incremental = false
EOF
