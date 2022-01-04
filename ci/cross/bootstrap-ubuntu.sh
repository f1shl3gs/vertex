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
  wget \
  libxml2-utils \
  xsltproc \
  libglib2.0-dev \
  libgnutls28-dev \
  libxml2-dev \
  pip \
  libtirpc-dev

# Install tools for building libvirt
pip install meson ninja docutils

# Locales
locale-gen en_US.UTF-8
dpkg-reconfigure locales

# Cleanup temporary files
apt clean
rm -rf "$(pip cache dir)"