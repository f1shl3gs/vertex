FROM ubuntu:18.04

# The OpenSSL version to use. Here is the place to check for new releases:
#
# - https://www.openssl.org/source/
#
# ALSO UPDATE hooks/build!
ARG OPENSSL_VERSION=1.1.1l
# The Rust toolchain to use when building our image.  Set by `hooks/build`.
ARG TOOLCHAIN=stable
ARG ZLIB_VERSION=1.2.11


# Make sure we have basic dev tools for building C libraries. Our goal here is to support
# the musl-libc builds and Cargo builds needed for a large selection of the most popular
# crates.
RUN apt-get update && \
    apt-get install -yq \
        build-essential \
        curl \
        file \
        git \
        musl-dev \
        musl-tools \
        linux-libc-dev \
        pkgconf \
        xutils-dev && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/* && \
    useradd rust --user-group --create-home --shell /bin/bash --groups sudo

# Static linking for C++ code
RUN ln -s "/usr/bin/g++" "/usr/bin/musl-g++"

# Build a static library version of OpenSSL using musl-libc. This is needed by the popular
# Rust `hyper` crate.
#
# We point /usr/local/musl/include/linux at some Linux kernel headers(not necessarily the
# right ones) in an effort to compile OpenSSL 1.1's "engine" component. It's possible
# that this will cause bizarre and terrible things to happen. Tere may be "sanitized"
# header
RUN echo "Building OpenSSL $OPENSSL_VERSION" && \
    mkdir -p /usr/local/musl/include && \
    ln -s /usr/include/linux /usr/local/musl/include/linux && \
    ln -s /usr/include/x86_64-linux-gnu/asm /usr/local/musl/include/asm && \
    ln -s /usr/include/asm-generic /usr/local/musl/include/asm-generic && \
    cd /tmp && \
    curl -fLO "https://www.openssl.org/source/openssl-$OPENSSL_VERSION.tar.gz" && \
    tar xvzf "openssl-$OPENSSL_VERSION.tar.gz" && \
    cd "openssl-$OPENSSL_VERSION" && \
    env CC=musl-gcc ./Configure no-shared no-zlib -fPIC --prefix=/usr/local/musl -DOPENSSL_NO_SECURE_MEMORY linux-x86_64 && \
    env C_INCLUDE_PATH=/usr/local/musl/include/ make depend && \
    env C_INCLUDE_PATH=/usr/local/musl/include/ make && \
    make install && \
    rm /usr/local/musl/include/linux /usr/local/musl/include/asm /usr/local/musl/include/asm-generic && \
    rm -rf /tmp/*

RUN echo "Building zlib-$ZLIB_VERSION" && \
    curl -fLO "http://zlib.net/zlib-$ZLIB_VERSION.tar.gz" && \
    tar xzf "zlib-$ZLIB_VERSION.tar.gz" && \
    cd "zlib-$ZLIB_VERSION" && \
    CC=musl-gcc ./configure --static --prefix=/usr/local/musl && \
    make && make install && \
    rm -rf /tmp/*

# Set up our path with all our binary directories, including those for the musl-gcc
# toolchain and for our Rust toolchain.
#
# We use the instructions at https://github.com/rust-lang/rustup/issues/2383
# to install the rustup toolchain as root
ENV RUSTUP_HOME=/opt/rust/rustup \
    PATH=/home/rust/.cargo/bin:/opt/rust/cargo/bin:/usr/local/musl/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin

# Install our Rust toolchain and the `musl` target. We path the command-line we pass to the
# installer so that it won't attempt to interact with the user or foll around with TTYs. We
# also set the default `--target` to musl so that our users don't need to keep overriding
# it manually.
RUN curl https://sh.rustup.rs -sSf | \
    env CARGO_HOME=/opt/rust/cargo \
        sh -s -- -y --default-toolchain $TOOLCHAIN  --profile minimal --no-modify-path && \
    env CARGO_HOME=/opt/rust/cargo \
        rustup component add rustfmt && \
    env CARGO_HOME=/opt/rust/cargo \
        rustup component add clippy && \
    env CARGO_HOME=/opt/rust/cargo \
        rustup target add x86_64-unknown-linux-musl
ADD cargo-config.toml /opt/rust/cargo/config

# Set up our environment variables so that we cross-compile using musl-libc by default
ENV X86_64_UNKNOWN_LINUX_MUSL_OPENSSL_DIR=/usr/local/musl/ \
    X86_64_UNKNOWN_LINUX_MUSL_OPENSSL_STATIC=1 \
    PG_CONFIG_X86_64_UNKNOWN_LINUX_GNU=/usr/bin/pg_config \
    PKG_CONFIG_ALLOW_CROSS=true \
    PKG_CONFIG_ALL_STATIC=true \
    LIBZ_SYS_STATIC=1 \
    TARGET=musl

# Run all further code as user `rust`, create our working directories, install our config file,
# and set up our credential helper.
USER rust
RUN mkdir -p /home/rust/libs /home/rust/src /home/rust/.cargo && \
    ln -s /opt/rust/cargo/config /home/rust/.cargo/config

# Expect our source code to live in /home/rust/src. We'll run the build as user `rust`, which will
# be uid 1000, gid 1000 outside the container.
WORKDIR /home/rust/src
