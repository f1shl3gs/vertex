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
    curl \
    libclang-dev \
    locales \
    pkg-config \
    shellcheck \
    wget \
    unzip

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
