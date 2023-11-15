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

# Rust/Cargo should already be installed on both GH Actions-provided Ubuntu 20.04,
# so this is really just make sure the path is configured.
if [ -n "${CI-}" ] ; then
    echo "${HOME}/.cargo/bin" >> "${GITHUB_PATH}"
else
    echo "export PATH=\"$HOME/.cargo/bin:\$PATH\"" >> "${HOME}/.bash_profile"
fi

# Install mold, because the system linker wastes a bunch of time.
#
# Notably, we don't install/configure it when we're going to do anything with `cross`, as `cross` takes the Cargo
# configuration from the host system and ships it over...  which isn't good when we're overriding the `rustc-wrapper`
# and all of that.
if [ -z "${DISABLE_MOLD:-""}" ] ; then
    # We explicitly put `mold-wrapper.so` right beside `mold` itself because it's hard-coded to look in the same directory
    # first when trying to load the shared object, so we can dodge having to care about the "right" lib folder to put it in.
    TEMP=$(mktemp -d)
    MOLD_VERSION=2.3.3
    MOLD_TARGET=mold-${MOLD_VERSION}-$(uname -m)-linux
    curl -fsSL "https://github.com/rui314/mold/releases/download/v${MOLD_VERSION}/${MOLD_TARGET}.tar.gz" \
        --output "$TEMP/${MOLD_TARGET}.tar.gz"
    tar \
        -xvf "${TEMP}/${MOLD_TARGET}.tar.gz" \
        -C "${TEMP}"
    cp "${TEMP}/${MOLD_TARGET}/bin/mold" /usr/bin/mold
    cp "${TEMP}/${MOLD_TARGET}/lib/mold/mold-wrapper.so" /usr/bin/mold-wrapper.so
    rm -rf "$TEMP"

    # Create our rustc wrapper script that we'll use to actually invoke `rustc` such that `mold` will wrap it and intercept
    # anything linking calls to use `mold` instead of `ld`, etc.
    CARGO_BIN_DIR="${CARGO_OVERRIDE_DIR}/bin"
    mkdir -p "$CARGO_BIN_DIR"

    RUSTC_WRAPPER="${CARGO_BIN_DIR}/wrap-rustc"
    cat <<EOF >"$RUSTC_WRAPPER"
#!/bin/sh
exec mold -run "\$@"
EOF
    chmod +x "$RUSTC_WRAPPER"

    # Now configure Cargo to use our rustc wrapper script.
    cat <<EOF >>"$CARGO_OVERRIDE_CONF"
[build]
rustc-wrapper = "$RUSTC_WRAPPER"
EOF
fi