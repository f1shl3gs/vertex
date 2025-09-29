#! /usr/bin/env bash

set -e -o verbose

# ensure active toolchain is installed
if ! command -v rustup >/dev/null 2>&1; then
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
fi

# Determine desired toolchain and ensure it's installed.
ACTIVE_TOOLCHAIN="$(rustup show active-toolchain 2>/dev/null || true)"
ACTIVE_TOOLCHAIN="${ACTIVE_TOOLCHAIN%% *}"  # keep only the first token
if [ -z "${ACTIVE_TOOLCHAIN}" ]; then
  # No active toolchain yet: fall back to env override or ultimately to stable.
  ACTIVE_TOOLCHAIN="${RUSTUP_TOOLCHAIN:-stable}"
  rustup default "${ACTIVE_TOOLCHAIN}"
fi

rustup toolchain install "${ACTIVE_TOOLCHAIN}"
rustup show

# Setup cargo-cross
if ! cross --version 2>/dev/null | grep -q '^cross 0.2.5'; then
  cargo install cross --version 0.2.5 --force --locked
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
