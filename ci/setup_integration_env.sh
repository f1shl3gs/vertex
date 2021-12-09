#!/usr/bin/env bash

rustup show # causes installation of version from rust-toolchain.toml
rustup default "$(rustup show active-toolchain | awk '{print $1;}')"

# Make sure our release build settings are present.
. scripts/environment/release-flags.sh


# Updates the Cargo config to product disk optimized builds(for CI, not users)
mkdir -p .cargo
cat <<-EOF >> ./.cargo/config

[build]
# On the CI, where this script runs, we won't be caching build artifacts.
# So we don't need to keep these around.
incremental = false
EOF
