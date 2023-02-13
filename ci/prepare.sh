#! /usr/bin/env bash

set -e -o verbose

rustup show # causes installation of version from rust-toolchain.toml
rustup default "$(rustup show active-toolchain | awk '{print $1;}')"

# Setup cargo-cross
if [[ "$(cross --version | grep cross)" != "cross 0.2.5" ]]; then
  rustup run stable cargo install cross --version 0.2.5 --force
fi

# Make sure our release build settings are present.
#
# We want to ensure we're building using "full" release capabilities when possible, which
# means full LTO and a single codegen unit.  This maximizes performance of the resulting
# code, but increases compilation time.  We only set this if we're in CI _and_ we haven't
# been instructed to use the debug profile (via PROFILE environment variable).
if [[ "${CI-}" == "true" && "${PROFILE-}" != "debug" ]]; then
  {
    echo "CARGO_PROFILE_RELEASE_LTO=fat";
    echo "CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1";
    echo "CARGO_PROFILE_RELEASE_DEBUG=false";
  } >> "${GITHUB_ENV}"
fi

bash ci/install_protoc.sh
