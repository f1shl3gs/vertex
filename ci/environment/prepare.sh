#! /usr/bin/env bash

set -e -o verbose

rustup show # causes installation of version from rust-toolchain.toml
rustup default "$(rustup show active-toolchain | awk '{print $1;}')"

if [[ "$(cross --version | grep cross)" != "cross 0.2.4" ]]; then
  rustup run stable cargo install cross --version 0.2.4 --force
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

# Setup protoc
#
# prost need `protoc` to be installed
PROTOC_VERSION=3.20.1
curl -L "https://github.com/protocolbuffers/protobuf/releases/download/v${PROTOC_VERSION}/protoc-${PROTOC_VERSION}-linux-x86_64.zip" -o protoc-${PROTOC_VERSION}-linux-x86_64.zip
unzip protoc-${PROTOC_VERSION}-linux-x86_64.zip
cp -r include/google /usr/local/include/
rm protoc-${PROTOC_VERSION}-linux-x86_64.zip
