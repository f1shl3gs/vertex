#! /usr/bin/env bash

set -e -o verbose

export DEBIAN_FRONTEND=noninteractive
export ACCEPT_EULA=Y

echo 'APT::Acquire::Retries "5";' > /etc/apt/apt.conf.d/80-retries

apt update --yes

apt install --yes \
  software-properties-common \
  apt-utils \
  apt-transport-https

# Deps
apt install --yes \
    bc \
    build-essential \
    ca-certificates \
    cmake \
    curl \
    gawk \
    libclang-dev \
    libsasl2-dev \
    libssl-dev \
    llvm \
    locales \
    pkg-config \
    shellcheck \
    sudo \
    wget

# Apt cleanup
apt clean

# Locales
locale-gen en_US.UTF-8
dpkg-reconfigure locales

# Install rust
if ! command -v rustup ; then
  # Rust/Cargo should already be installed on both GH Actions-provided Ubuntu 20.04,
  # so this is really just make sure the path is configured.
  curl https://sh.rustup.rs -sSf | sh -s -- -y --profile minimal
fi

# Rust/Cargo should already be installed on both GH Actions-provided Ubuntu 20.04,
# so this is really just make sure the path is configured.
if [ -n "${CI-}" ] ; then
    echo "${HOME}/.cargo/bin" >> "${GITHUB_PATH}"
else
    echo "export PATH=\"$HOME/.cargo/bin:\$PATH\"" >> "${HOME}/.bash_profile"
fi

# Setup cargo-cross
source "$HOME/.cargo/env"
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
