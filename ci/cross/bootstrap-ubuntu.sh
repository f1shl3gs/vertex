#!/bin/sh

set -e -o verbose

export DEBIAN_FRONTEND=noninteractive
export ACCEPT_EULA=Y

echo 'APT::Acquire::Retries "5";' > /etc/apt/apt.conf.d/80-retries

apt update --yes

apt install --yes \
  apt-utils \
  apt-transport-https \
  software-properties-common

apt upgrade --yes

# Install deps
apt install --yes \
  build-essential \
  ca-certificates \
  cmake \
  gawk \
  curl \
  libclang-dev \
  libsasl2-dev \
  libssl-dev \
  llvm \
  locales \
  pkg-config \
  unzip \
  zlib \
  zlib1g-dev

# Setup protoc
#
# prost-build need protoc to be installed
PROTOC_VERSION=3.20.1
curl -L "https://github.com/protocolbuffers/protobuf/releases/download/v${PROTOC_VERSION}/protoc-${PROTOC_VERSION}-linux-x86_64.zip" -o protoc-${PROTOC_VERSION}-linux-x86_64.zip
unzip protoc-${PROTOC_VERSION}-linux-x86_64.zip
cp bin/protoc /usr/local/bin
cp -r include/google /usr/local/include/
rm protoc-${PROTOC_VERSION}-linux-x86_64.zip

# Locales
locale-gen en_US.UTF-8
dpkg-reconfigure locales

# Cleanup temporary files
apt clean