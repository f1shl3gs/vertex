#! /usr/bin/env bash
set -e -o verbose

rustup show # causes installation of version from rust-toolchain.toml
rustup default "$(rustup show active-toolchain | awk '{print $1;}')"
rustup run stable cargo install cross --version 0.2.1

# Make sure our release build settings are present.
. scripts/environment/release-flags.sh
